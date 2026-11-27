package app.ligarview.client

import java.io.Closeable
import java.io.InputStream
import java.net.InetSocketAddress
import java.net.Socket

/**
 * Where the stream and cursor clients read from: a TCP connection (Wi-Fi, or USB through adb)
 * or one channel of the USB accessory pipe. The byte streams are identical either way.
 */
interface StreamSource {
    val label: String
    fun open(): Opened
}

/**
 * `send` writes one input message body back to the host on this connection (null when the
 * source cannot carry input); it blocks while the link is slow, so call it off the main thread.
 */
class Opened(
    val input: InputStream,
    val send: ((ByteArray) -> Unit)? = null,
    private val onClose: () -> Unit = {},
) : Closeable {
    override fun close() = onClose()
}

/** `code` is the pairing code shown in the Mac's window; the host checks it before it sends anything. */
class TcpSource(private val host: String, private val port: Int, private val code: Int = 0) : StreamSource {
    override val label: String get() = "$host:$port"

    override fun open(): Opened {
        val socket = Socket()
        try {
            socket.tcpNoDelay = true
            socket.receiveBufferSize = 1 shl 20
            // The host sends a heartbeat every second, so silence means a dead link.
            socket.soTimeout = READ_TIMEOUT_MS
            socket.connect(InetSocketAddress(host, port), CONNECT_TIMEOUT_MS)
            socket.getOutputStream().apply {
                write(PREAMBLE)
                write(java.nio.ByteBuffer.allocate(4).putInt(code).array())
                flush()
            }
            val out = socket.getOutputStream()
            val send = { body: ByteArray ->
                synchronized(out) {
                    out.write(byteArrayOf((body.size shr 8).toByte(), body.size.toByte()))
                    out.write(body)
                    out.flush()
                }
            }
            return Opened(socket.getInputStream(), send) { socket.close() }
        } catch (e: Exception) {
            socket.close()
            throw e
        }
    }

    private companion object {
        val PREAMBLE = "LGRP".toByteArray()
        const val READ_TIMEOUT_MS = 4000
        const val CONNECT_TIMEOUT_MS = 3000
    }
}
