package app.ligarview.client

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Build
import android.os.Handler
import android.os.Looper
import java.net.Inet4Address

/** A Mac announcing itself on the local network. */
data class FoundMac(val name: String, val host: String, val port: Int)

/**
 * Finds Ligarview hosts announced over Bonjour (`_ligarview._tcp`), so nobody has to type an
 * address. [onChange] runs on the main thread with the first Mac found, or null when none is left.
 */
class NsdDiscovery(context: Context, private val onChange: (FoundMac?) -> Unit) {
    private val nsd = context.getSystemService(NsdManager::class.java)
    private val main = Handler(Looper.getMainLooper())
    private val found = LinkedHashMap<String, FoundMac>()
    private val pending = ArrayDeque<NsdServiceInfo>()
    private var resolving = false
    private var listener: NsdManager.DiscoveryListener? = null

    fun start() {
        if (listener != null) return
        val l = object : NsdManager.DiscoveryListener {
            override fun onServiceFound(info: NsdServiceInfo) {
                main.post {
                    pending.add(info)
                    resolveNext()
                }
            }

            override fun onServiceLost(info: NsdServiceInfo) {
                main.post {
                    found.remove(info.serviceName)
                    onChange(found.values.firstOrNull())
                }
            }

            override fun onDiscoveryStarted(serviceType: String) = Unit
            override fun onDiscoveryStopped(serviceType: String) = Unit
            override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                main.post { listener = null }
            }

            override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) = Unit
        }
        listener = l
        try {
            nsd.discoverServices(SERVICE_TYPE, NsdManager.PROTOCOL_DNS_SD, l)
        } catch (_: Exception) {
            listener = null
        }
    }

    fun stop() {
        listener?.let {
            try {
                nsd.stopServiceDiscovery(it)
            } catch (_: Exception) {
            }
        }
        listener = null
        pending.clear()
        resolving = false
        found.clear()
    }

    @Suppress("DEPRECATION")
    private fun resolveNext() {
        if (resolving) return
        val info = pending.removeFirstOrNull() ?: return
        resolving = true
        nsd.resolveService(info, object : NsdManager.ResolveListener {
            override fun onResolveFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                main.post {
                    resolving = false
                    // Another resolve was running: try this one again shortly.
                    if (errorCode == NsdManager.FAILURE_ALREADY_ACTIVE) {
                        pending.add(serviceInfo)
                        main.postDelayed({ resolveNext() }, 500)
                    } else {
                        resolveNext()
                    }
                }
            }

            override fun onServiceResolved(serviceInfo: NsdServiceInfo) {
                main.post {
                    resolving = false
                    val address = if (Build.VERSION.SDK_INT >= 34) {
                        serviceInfo.hostAddresses.firstOrNull { it is Inet4Address } ?: serviceInfo.hostAddresses.firstOrNull()
                    } else {
                        serviceInfo.host
                    }
                    val host = address?.hostAddress
                    if (host != null) {
                        found[serviceInfo.serviceName] = FoundMac(serviceInfo.serviceName, host, serviceInfo.port)
                        onChange(found.values.firstOrNull())
                    }
                    resolveNext()
                }
            }
        })
    }

    private companion object {
        const val SERVICE_TYPE = "_ligarview._tcp"
    }
}
