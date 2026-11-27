package app.ligarview.client

import android.graphics.BitmapFactory
import java.io.BufferedInputStream
import java.io.DataInputStream
import java.io.IOException
import java.util.concurrent.Executors
import java.util.concurrent.RejectedExecutionException
import java.util.concurrent.atomic.AtomicInteger
import kotlin.concurrent.thread

/**
 * Receives cursor position and shape from the host (video port + 1) and feeds [overlay].
 * Reconnects until [stop]. Wire format: see host/src/cursor_server.rs.
 */
class CursorClient(
    private val source: StreamSource,
    private val overlay: CursorOverlay,
) {
    @Volatile private var running = false

    /** True while the cursor connection to the host is up. */
    @Volatile var connected = false
        private set
    @Volatile private var opened: Opened? = null

    /** What the Mac allows (bits 1 pen and sidebar, 2 finger scrolling, 4 typing); -1 until it says. */
    @Volatile var caps = -1
        private set

    /** Called (on the reader thread) when the Mac announces what it allows. */
    @Volatile var onCaps: ((Int) -> Unit)? = null
    private var worker: Thread? = null

    /** Input goes out on its own thread: a slow or dead link must never block the UI. */
    private val sender = Executors.newSingleThreadExecutor { r -> Thread(r, "input-sender").apply { isDaemon = true } }
    private val queued = AtomicInteger(0)

    /** True while the host can receive input from this connection. */
    val canSendInput: Boolean get() = connected && opened?.send != null

    /** Sends one input message body to the host. Dropped when the link is backed up or down. */
    fun sendInput(body: ByteArray) {
        val send = opened?.send ?: return
        if (!connected || queued.get() > MAX_QUEUED_INPUT) return
        queued.incrementAndGet()
        try {
            sender.execute {
                try {
                    send(body)
                } catch (_: Exception) {
                    // The read side notices a dead link and reconnects.
                } finally {
                    queued.decrementAndGet()
                }
            }
        } catch (_: RejectedExecutionException) {
            queued.decrementAndGet()
        }
    }

    fun start() {
        running = true
        worker = thread(name = "cursor-client") { loop() }
    }

    fun stop() {
        running = false
        try {
            opened?.close()
        } catch (_: IOException) {
        }
        worker?.join(1000)
        sender.shutdownNow()
    }

    private fun loop() {
        while (running) {
            try {
                session()
            } catch (_: Exception) {
                // Host not up yet or connection lost; retry below.
            }
            connected = false
            if (running) {
                try {
                    Thread.sleep(1000)
                } catch (_: InterruptedException) {
                    return
                }
            }
        }
    }

    private fun session() {
        source.open().also { opened = it }.use { link ->
            connected = true
            val input = DataInputStream(BufferedInputStream(link.input, 16 * 1024))
            while (running) {
                val length = input.readInt()
                if (length < 1 || length > MAX_MESSAGE_BYTES) throw IOException("Bad cursor message size $length")
                when (input.readUnsignedByte()) {
                    TYPE_POSITION -> {
                        val x = input.readFloat()
                        val y = input.readFloat()
                        overlay.setPosition(x, y, input.readUnsignedByte() != 0)
                    }
                    TYPE_CAPS -> {
                        val bits = input.readUnsignedByte()
                        caps = bits
                        onCaps?.invoke(bits)
                    }
                    TYPE_SHAPE -> {
                        val w = input.readFloat()
                        val h = input.readFloat()
                        val hotX = input.readFloat()
                        val hotY = input.readFloat()
                        val png = ByteArray(length - 17).also(input::readFully)
                        BitmapFactory.decodeByteArray(png, 0, png.size)?.let {
                            overlay.setShape(CursorOverlay.Shape(it, w, h, hotX, hotY))
                        }
                    }
                    else -> input.skipBytes(length - 1)
                }
            }
        }
    }

    private companion object {
        const val TYPE_POSITION = 1
        const val TYPE_SHAPE = 2
        const val TYPE_CAPS = 3
        const val MAX_MESSAGE_BYTES = 1 shl 20
        const val MAX_QUEUED_INPUT = 64
    }
}
