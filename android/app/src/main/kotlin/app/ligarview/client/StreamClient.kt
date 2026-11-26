package app.ligarview.client

import android.media.MediaFormat
import android.view.Surface
import java.io.BufferedInputStream
import java.io.DataInputStream
import java.io.IOException
import java.net.InetSocketAddress
import java.net.Socket
import java.util.concurrent.atomic.AtomicLong
import kotlin.concurrent.thread

/**
 * Connects to the Ligarview host and feeds the decoder. Reconnects automatically
 * until [stop] is called.
 *
 * Wire protocol (big-endian): see host/src/protocol.rs.
 */
class StreamClient(
    private val host: String,
    private val port: Int,
    private val surface: Surface,
    private val onStatus: (message: String, streaming: Boolean) -> Unit,
) {
    @Volatile
    private var running = false

    @Volatile
    private var socket: Socket? = null
    private var worker: Thread? = null

    @Volatile
    private var decoder: VideoDecoder? = null

    /** Total payload bytes received. */
    val bytesReceived = AtomicLong()

    /** Total frames shown by the decoder (0 when not streaming). */
    val framesRendered: Long get() = decoder?.framesRendered ?: 0L

    /** Smoothed in-decoder latency in ms (0 when not streaming). */
    val decodeLatencyMs: Float get() = decoder?.decodeLatencyMs ?: 0f

    fun start() {
        running = true
        worker = thread(name = "stream-client") { loop() }
    }

    fun stop() {
        running = false
        try {
            socket?.close()
        } catch (_: IOException) {
        }
        worker?.join(1500)
    }

    private fun loop() {
        while (running) {
            try {
                onStatus("Connecting to $host:$port...", false)
                session()
            } catch (e: Exception) {
                if (running) onStatus("${e.message ?: e.javaClass.simpleName}. Retrying...", false)
            }
            if (running) {
                try {
                    Thread.sleep(RETRY_DELAY_MS)
                } catch (_: InterruptedException) {
                    return
                }
            }
        }
    }

    private fun session() {
        val sock = Socket().also { socket = it }
        sock.use {
            it.tcpNoDelay = true
            it.receiveBufferSize = 1 shl 20
            it.connect(InetSocketAddress(host, port), CONNECT_TIMEOUT_MS)
            val input = DataInputStream(BufferedInputStream(it.getInputStream(), 256 * 1024))

            val magic = ByteArray(4).also(input::readFully)
            if (!magic.contentEquals(MAGIC)) throw IOException("Not a Ligarview host")
            val version = input.readUnsignedByte()
            if (version != VERSION) throw IOException("Unsupported protocol version $version")
            val codec = input.readUnsignedByte()
            val mime = when (codec) {
                CODEC_H264 -> MediaFormat.MIMETYPE_VIDEO_AVC
                CODEC_HEVC -> MediaFormat.MIMETYPE_VIDEO_HEVC
                else -> throw IOException("Unsupported codec $codec")
            }
            val width = input.readUnsignedShort()
            val height = input.readUnsignedShort()
            val fps = input.readUnsignedShort()

            if (fps > 0) {
                if (android.os.Build.VERSION.SDK_INT >= 31) {
                    surface.setFrameRate(fps.toFloat(), Surface.FRAME_RATE_COMPATIBILITY_DEFAULT, Surface.CHANGE_FRAME_RATE_ALWAYS)
                } else {
                    surface.setFrameRate(fps.toFloat(), Surface.FRAME_RATE_COMPATIBILITY_DEFAULT)
                }
            }
            val decoder = VideoDecoder(surface, width, height, mime).also { this.decoder = it }
            try {
                onStatus("Connected: ${width}x$height @ $fps fps (${if (codec == CODEC_HEVC) "HEVC" else "H.264"})", false)
                var buffer = ByteArray(512 * 1024)
                var first = true
                while (running) {
                    val length = input.readInt()
                    val flags = input.readUnsignedByte()
                    val ptsUs = input.readLong()
                    if (length < 0 || length > MAX_FRAME_BYTES) throw IOException("Bad frame size $length")
                    if (buffer.size < length) buffer = ByteArray(length)
                    input.readFully(buffer, 0, length)
                    bytesReceived.addAndGet(length + 13L)
                    if (decoder.feed(buffer, length, ptsUs, repeat = flags and FLAG_REPEAT != 0) && first) {
                        first = false
                        onStatus("Streaming", true)
                    }
                }
            } finally {
                this.decoder = null
                decoder.release()
            }
        }
    }

    private companion object {
        val MAGIC = "LGRV".toByteArray()
        const val VERSION = 1
        const val CODEC_H264 = 1
        const val CODEC_HEVC = 2
        const val FLAG_REPEAT = 2
        const val CONNECT_TIMEOUT_MS = 3000
        const val RETRY_DELAY_MS = 1000L
        const val MAX_FRAME_BYTES = 16 * 1024 * 1024
    }
}
