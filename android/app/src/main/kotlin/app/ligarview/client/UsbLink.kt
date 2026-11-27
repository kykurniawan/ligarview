package app.ligarview.client

import android.os.ParcelFileDescriptor
import java.io.BufferedInputStream
import java.io.Closeable
import java.io.DataInputStream
import java.io.EOFException
import java.io.FileInputStream
import java.io.FileOutputStream
import java.io.IOException
import java.io.InputStream
import java.net.SocketTimeoutException
import java.util.concurrent.Executors
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.TimeUnit
import kotlin.concurrent.thread

/**
 * One logical channel of the USB pipe, readable like a socket. Payloads are queued by the
 * demultiplexer; while the channel is not armed (no client running) they are dropped.
 */
class ChannelStream : InputStream() {
    private val queue = LinkedBlockingQueue<ByteArray>()
    private var current: ByteArray? = null
    private var position = 0

    @Volatile private var armed = false
    @Volatile private var failed = false

    fun push(payload: ByteArray) {
        if (armed) queue.offer(payload)
    }

    /** Starts buffering from a clean state. */
    fun arm() {
        queue.clear()
        current = null
        position = 0
        armed = true
        if (failed) queue.offer(EOF)
    }

    fun disarm() {
        armed = false
        queue.clear()
    }

    fun fail() {
        failed = true
        queue.offer(EOF)
    }

    override fun read(): Int {
        val one = ByteArray(1)
        return if (read(one, 0, 1) < 0) -1 else one[0].toInt() and 0xff
    }

    override fun read(b: ByteArray, off: Int, len: Int): Int {
        if (len == 0) return 0
        var chunk = current
        if (chunk == null || position >= chunk.size) {
            // The host sends heartbeats, so a long silence means the link is dead.
            chunk = queue.poll(READ_TIMEOUT_MS, TimeUnit.MILLISECONDS)
                ?: throw SocketTimeoutException("USB link silent")
            if (chunk === EOF) {
                queue.offer(EOF)
                return -1
            }
            current = chunk
            position = 0
        }
        val n = minOf(len, chunk.size - position)
        System.arraycopy(chunk, position, b, off, n)
        position += n
        return n
    }

    private companion object {
        val EOF = ByteArray(0)
        const val READ_TIMEOUT_MS = 4000L
    }
}

/**
 * The USB Android Open Accessory pipe to the Mac host. Frames (both directions):
 * `channel u8 | payload_len u16 | payload`; channels 0 control, 1 video, 2 cursor. See
 * host/src/usb.rs for the full description.
 *
 * The link lives as long as the accessory is attached. Reading never stops (a blocked read on
 * the accessory file cannot be interrupted, and the accessory cannot be reopened while it is
 * pending); instead the client asks the host to pause and resume the stream.
 */
class UsbLink(private val pfd: ParcelFileDescriptor) : Closeable {
    val video = ChannelStream()
    val cursor = ChannelStream()

    @Volatile var alive = true
        private set

    /** True between [resume] and [pause]: the app wants a stream. */
    @Volatile private var wanted = false

    @Volatile private var lastResume = 0L

    /** Called when the host (re)started and asks whether a stream is wanted; the clients must restart. */
    @Volatile var onHostAsk: (() -> Unit)? = null

    private val output = FileOutputStream(pfd.fileDescriptor)

    /**
     * Writes to the accessory only complete when the host reads them, so with the host gone a
     * write blocks indefinitely. Control frames therefore go through their own thread, and only
     * the newest pending command is kept (a RESUME that was never delivered is not worth
     * repeating ten times).
     */
    private val writer = Executors.newSingleThreadExecutor { r -> Thread(r, "usb-writer").apply { isDaemon = true } }
    private val pendingControl = AtomicInteger(NONE)

    /** Opening the video stream (first time and every retry) asks the host for a fresh one. */
    val videoSource = object : StreamSource {
        override val label = "USB"
        override fun open(): Opened {
            resume()
            return Opened(video)
        }
    }
    val cursorSource = object : StreamSource {
        override val label = "USB cursor"
        override fun open() = Opened(cursor)
    }

    init {
        thread(name = "usb-demux", isDaemon = true) { demux() }
    }

    /** Asks the host to (re)start streaming; it answers with a fresh hello and a keyframe. */
    fun resume() {
        wanted = true
        lastResume = System.nanoTime()
        video.arm()
        cursor.arm()
        sendControl(CTL_RESUME)
    }

    /** Asks the host to stop sending (screen off, app in the background). */
    fun pause() {
        wanted = false
        sendControl(CTL_PAUSE)
        video.disarm()
        cursor.disarm()
    }

    private fun sendControl(code: Int) {
        pendingControl.set(code)
        writer.execute {
            val next = pendingControl.getAndSet(NONE)
            if (next == NONE) return@execute
            val frame = byteArrayOf(CH_CONTROL.toByte(), 0, 1, next.toByte())
            try {
                output.write(frame)
            } catch (_: IOException) {
                shutdown()
            }
        }
    }

    private fun demux() {
        try {
            // Reads must be at least one USB packet; 16 KiB is the recommended size.
            val input = DataInputStream(BufferedInputStream(FileInputStream(pfd.fileDescriptor), 16384))
            skipToSync(input)
            while (true) {
                val channel = input.readUnsignedByte()
                val length = input.readUnsignedShort()
                val payload = ByteArray(length).also(input::readFully)
                when (channel) {
                    CH_VIDEO -> video.push(payload)
                    CH_CURSOR -> cursor.push(payload)
                    CH_CONTROL -> if (length >= 1 && payload[0].toInt() == CTL_ASK && wanted && askIsStale()) onHostAsk?.invoke()
                }
            }
        } catch (_: Exception) {
            // Cable pulled or accessory closed.
        } finally {
            shutdown()
        }
    }

    /** An ASK right after our own request only crosses it on the wire; ignore those. */
    private fun askIsStale() = System.nanoTime() - lastResume > 1_500_000_000L

    /** After (re)opening the accessory the first bytes may be the tail of an old frame. */
    private fun skipToSync(input: DataInputStream) {
        val window = ByteArray(SYNC_MAGIC.size)
        var filled = 0
        while (true) {
            val b = input.read()
            if (b < 0) throw EOFException()
            if (filled < window.size) {
                window[filled++] = b.toByte()
            } else {
                System.arraycopy(window, 1, window, 0, window.size - 1)
                window[window.size - 1] = b.toByte()
            }
            if (filled == window.size && window.contentEquals(SYNC_MAGIC)) return
        }
    }

    private fun shutdown() {
        if (!alive) return
        alive = false
        video.fail()
        cursor.fail()
        writer.shutdownNow()
        try {
            pfd.close()
        } catch (_: IOException) {
        }
    }

    override fun close() = shutdown()

    private companion object {
        const val CH_CONTROL = 0
        const val CH_VIDEO = 1
        const val CH_CURSOR = 2
        const val CTL_RESUME = 1
        const val CTL_PAUSE = 2
        const val CTL_ASK = 1
        const val NONE = -1
        val SYNC_MAGIC = "LGRV-SYNC-v1\u0000\u0000\u0000\u0000".toByteArray(Charsets.ISO_8859_1)
    }
}
