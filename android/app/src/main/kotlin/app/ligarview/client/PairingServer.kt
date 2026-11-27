package app.ligarview.client

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Build
import android.os.Handler
import android.os.Looper
import java.io.DataInputStream
import java.net.ServerSocket
import kotlin.concurrent.thread

/**
 * Lets a Mac pair with this tablet. The tablet shows a code and announces itself over Bonjour
 * (`_ligarview-tablet._tcp`); the user types the code into the Mac, which sends it here.
 * When it matches, [onPaired] gets the Mac's address and the code, which the tablet then
 * presents whenever it connects to that Mac.
 */
class PairingServer(
    context: Context,
    private val currentCode: () -> String,
    private val onPaired: (host: String?, port: Int, code: Int) -> Unit,
) {
    private val nsd = context.getSystemService(NsdManager::class.java)
    private val main = Handler(Looper.getMainLooper())
    private var server: ServerSocket? = null
    private var registration: NsdManager.RegistrationListener? = null

    fun start() {
        if (server != null) return
        val socket = try {
            ServerSocket(0)
        } catch (_: Exception) {
            return
        }
        server = socket
        thread(name = "pairing-server") {
            while (!socket.isClosed) {
                try {
                    handle(socket.accept())
                } catch (_: Exception) {
                }
            }
        }
        val info = NsdServiceInfo().apply {
            serviceName = Build.MODEL
            serviceType = SERVICE_TYPE
            port = socket.localPort
        }
        val l = object : NsdManager.RegistrationListener {
            override fun onServiceRegistered(serviceInfo: NsdServiceInfo) = Unit
            override fun onRegistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) = Unit
            override fun onServiceUnregistered(serviceInfo: NsdServiceInfo) = Unit
            override fun onUnregistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) = Unit
        }
        registration = l
        try {
            nsd.registerService(info, NsdManager.PROTOCOL_DNS_SD, l)
        } catch (_: Exception) {
            registration = null
        }
    }

    fun stop() {
        registration?.let {
            try {
                nsd.unregisterService(it)
            } catch (_: Exception) {
            }
        }
        registration = null
        try {
            server?.close()
        } catch (_: Exception) {
        }
        server = null
    }

    private fun handle(client: java.net.Socket) {
        client.use {
            it.soTimeout = 5000
            val input = DataInputStream(it.getInputStream())
            val magic = ByteArray(4).also(input::readFully)
            if (!magic.contentEquals("LGRQ".toByteArray())) return
            val code = input.readInt()
            val port = input.readUnsignedShort()
            val ok = code.toString() == currentCode()
            it.getOutputStream().apply {
                write(if (ok) 1 else 0)
                flush()
            }
            // Only an IPv4 address is usable: the host listens on IPv4 and Bonjour supplies the rest.
            val host = ((it.remoteSocketAddress as? java.net.InetSocketAddress)?.address as? java.net.Inet4Address)?.hostAddress
            if (ok) main.post { onPaired(host, port, code) }
        }
    }

    private companion object {
        const val SERVICE_TYPE = "_ligarview-tablet._tcp"
    }
}
