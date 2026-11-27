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

class Opened(val input: InputStream, private val onClose: () -> Unit = {}) : Closeable {
    override fun close() = onClose()
}

class TcpSource(private val host: String, private val port: Int) : StreamSource {
    override val label: String get() = "$host:$port"

    override fun open(): Opened {
        val socket = Socket()
        try {
            socket.tcpNoDelay = true
            socket.receiveBufferSize = 1 shl 20
            // The host sends a heartbeat every second, so silence means a dead link.
            socket.soTimeout = READ_TIMEOUT_MS
            socket.connect(InetSocketAddress(host, port), CONNECT_TIMEOUT_MS)
            return Opened(socket.getInputStream()) { socket.close() }
        } catch (e: Exception) {
            socket.close()
            throw e
        }
    }

    private companion object {
        const val READ_TIMEOUT_MS = 4000
        const val CONNECT_TIMEOUT_MS = 3000
    }
}
