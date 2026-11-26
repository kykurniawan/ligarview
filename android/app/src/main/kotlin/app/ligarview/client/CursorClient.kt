package app.ligarview.client

import android.graphics.BitmapFactory
import java.io.BufferedInputStream
import java.io.DataInputStream
import java.io.IOException
import java.net.InetSocketAddress
import java.net.Socket
import kotlin.concurrent.thread

/**
 * Receives cursor position and shape from the host (video port + 1) and feeds [overlay].
 * Reconnects until [stop]. Wire format: see host/src/cursor_server.rs.
 */
class CursorClient(
    private val host: String,
    private val port: Int,
    private val overlay: CursorOverlay,
) {
    @Volatile private var running = false
    @Volatile private var socket: Socket? = null
    private var worker: Thread? = null

    fun start() {
        running = true
        worker = thread(name = "cursor-client") { loop() }
    }

    fun stop() {
        running = false
        try {
            socket?.close()
        } catch (_: IOException) {
        }
        worker?.join(1000)
    }

    private fun loop() {
        while (running) {
            try {
                session()
            } catch (_: Exception) {
                // Host not up yet or connection lost; retry below.
            }
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
        Socket().also { socket = it }.use { sock ->
            sock.tcpNoDelay = true
            sock.connect(InetSocketAddress(host, port), 3000)
            val input = DataInputStream(BufferedInputStream(sock.getInputStream(), 16 * 1024))
            while (running) {
                val length = input.readInt()
                if (length < 1 || length > MAX_MESSAGE_BYTES) throw IOException("Bad cursor message size $length")
                when (input.readUnsignedByte()) {
                    TYPE_POSITION -> {
                        val x = input.readFloat()
                        val y = input.readFloat()
                        overlay.setPosition(x, y, input.readUnsignedByte() != 0)
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
        const val MAX_MESSAGE_BYTES = 1 shl 20
    }
}
