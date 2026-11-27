package app.ligarview.client

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.graphics.Color
import android.hardware.usb.UsbManager
import android.os.Bundle
import android.provider.Settings
import android.view.Gravity
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.TextView
import androidx.activity.ComponentActivity
import androidx.activity.OnBackPressedCallback
import androidx.compose.ui.platform.ComposeView
import app.ligarview.client.ui.Actions
import app.ligarview.client.ui.DeviceChecks
import app.ligarview.client.ui.InfoOverlay
import app.ligarview.client.ui.LigarviewTheme
import app.ligarview.client.ui.LinkPhase
import app.ligarview.client.ui.Transport
import app.ligarview.client.ui.UiState

/**
 * Hosts the live picture (a SurfaceView fed by the video decoder), the Mac's cursor drawn on top,
 * and a Compose overlay that explains how to connect and what to adjust while nothing is streaming.
 */
class MainActivity : ComponentActivity(), SurfaceHolder.Callback {
    private lateinit var surfaceView: SurfaceView
    private lateinit var stats: TextView
    private lateinit var statsMirror: ImageView
    private lateinit var cursorOverlay: CursorOverlay
    private lateinit var state: UiState

    private var client: StreamClient? = null

    /** Identifies the running client; status reports from a stopped one are ignored. */
    private var clientToken: Any? = null
    private var cursorClient: CursorClient? = null
    private lateinit var usbManager: UsbManager
    private var usbLink: UsbLink? = null
    private lateinit var permissionIntent: PendingIntent
    private val handler = android.os.Handler(android.os.Looper.getMainLooper())

    /** True from the moment the stream was lost until it is live again. */
    private var graceStarted = false
    private val releaseAwake = Runnable {
        window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    }
    private var surfaceReady = false

    private val backCallback = object : OnBackPressedCallback(false) {
        override fun handleOnBackPressed() {
            wantConnection = false
            stopClient()
            state.phase = LinkPhase.Idle
            state.detail = ""
            state.everConnected = false
            setStatsVisible(false)
        }
    }

    /** True while the user wants a connection. Back leaves it and returns to the information screen. */
    private var wantConnection = false
        set(value) {
            field = value
            backCallback.isEnabled = value
        }

    private val usbReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            when (intent.action) {
                ACTION_USB_PERMISSION -> {
                    if (intent.getBooleanExtra(UsbManager.EXTRA_PERMISSION_GRANTED, false)) {
                        startClient()
                    } else {
                        wantConnection = false
                        state.phase = LinkPhase.Idle
                    }
                }
                UsbManager.ACTION_USB_ACCESSORY_DETACHED -> onUsbDetached()
            }
        }
    }

    private val deviceTick = object : Runnable {
        override fun run() {
            state.device = DeviceChecks.read(this@MainActivity)
            handler.postDelayed(this, DEVICE_POLL_MS)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.attributes = window.attributes.apply {
            layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
        }
        onBackPressedDispatcher.addCallback(this, backCallback)

        val saved = prefs().getString(KEY_TARGET, null)
        state = UiState(initialHost = saved.orEmpty(), startOnWifiTab = saved != null)
        buildUi()
        hideSystemBars()

        usbManager = getSystemService(UsbManager::class.java)
        permissionIntent = PendingIntent.getBroadcast(
            this, 0, Intent(ACTION_USB_PERMISSION).setPackage(packageName), PendingIntent.FLAG_MUTABLE,
        )
        registerReceiver(
            usbReceiver,
            IntentFilter().apply {
                addAction(ACTION_USB_PERMISSION)
                addAction(UsbManager.ACTION_USB_ACCESSORY_DETACHED)
            },
            Context.RECEIVER_NOT_EXPORTED,
        )

        // Connect automatically: over USB when the Mac has switched the tablet into accessory
        // mode, otherwise to the last used host.
        if (saved != null || usbManager.accessoryList?.isNotEmpty() == true) {
            wantConnection = true
            state.phase = LinkPhase.Connecting
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (intent.action == UsbManager.ACTION_USB_ACCESSORY_ATTACHED) {
            wantConnection = true
            startClient()
        }
    }

    override fun onDestroy() {
        unregisterReceiver(usbReceiver)
        usbLink?.close()
        usbLink = null
        super.onDestroy()
    }

    override fun onPause() {
        cursorOverlay.stop()
        handler.removeCallbacks(deviceTick)
        super.onPause()
    }

    override fun onResume() {
        super.onResume()
        cursorOverlay.start()
        requestHighRefreshRate()
        handler.post(deviceTick)
    }

    /**
     * Pin the panel to its fastest mode. Samsung's adaptive refresh drops to 30 Hz when the
     * stream is nearly static and takes a while to ramp back up, which feels like input lag.
     */
    private fun requestHighRefreshRate() {
        val d = display ?: return
        val current = d.mode
        val fastest = d.supportedModes
            .filter { it.physicalWidth == current.physicalWidth && it.physicalHeight == current.physicalHeight }
            .maxByOrNull { it.refreshRate } ?: return
        window.attributes = window.attributes.apply { preferredDisplayModeId = fastest.modeId }
    }

    override fun onWindowFocusChanged(hasFocus: Boolean) {
        super.onWindowFocusChanged(hasFocus)
        if (hasFocus) hideSystemBars()
    }

    private fun hideSystemBars() {
        @Suppress("DEPRECATION")
        window.setDecorFitsSystemWindows(false)
        window.insetsController?.apply {
            hide(WindowInsets.Type.systemBars())
            systemBarsBehavior = WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
    }

    private fun buildUi() {
        val root = FrameLayout(this).apply { setBackgroundColor(Color.BLACK) }

        surfaceView = SurfaceView(this)
        surfaceView.holder.addCallback(this)
        root.addView(surfaceView, FrameLayout.LayoutParams(MATCH_PARENT, MATCH_PARENT))

        val cursorView = CursorView(this)
        root.addView(cursorView, FrameLayout.LayoutParams(MATCH_PARENT, MATCH_PARENT))
        cursorOverlay = CursorOverlay(cursorView, root)

        val actions = Actions(
            connect = ::connect,
            connectUsb = {
                state.everConnected = false
                wantConnection = true
                startClient()
            },
            openDisplaySettings = { openSettings(Settings.ACTION_DISPLAY_SETTINGS) },
            openBatterySaver = { openSettings(Settings.ACTION_BATTERY_SAVER_SETTINGS) },
            openBatteryOptimization = { openSettings(Settings.ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS) },
        )
        val versionName = packageManager.getPackageInfo(packageName, 0).versionName.orEmpty()
        root.addView(
            ComposeView(this).apply {
                setContent { LigarviewTheme { InfoOverlay(state, actions, versionName) } }
            },
            FrameLayout.LayoutParams(MATCH_PARENT, MATCH_PARENT),
        )

        stats = TextView(this).apply {
            setTextColor(Color.GREEN)
            setBackgroundColor(0x99000000.toInt())
            typeface = android.graphics.Typeface.MONOSPACE
            textSize = 13f
            setPadding(dp(10), dp(6), dp(10), dp(6))
            visibility = View.GONE
        }
        root.addView(
            stats,
            FrameLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT, Gravity.TOP or Gravity.START).apply {
                setMargins(dp(16), dp(16), dp(16), dp(16))
            },
        )
        val mirror = ImageView(this).apply {
            scaleType = ImageView.ScaleType.FIT_CENTER
            setBackgroundColor(0xFF808080.toInt())
            visibility = View.GONE
        }
        root.addView(
            mirror,
            FrameLayout.LayoutParams(dp(110), dp(110), Gravity.TOP or Gravity.START).apply {
                setMargins(dp(16), dp(70), 0, 0)
            },
        )
        statsMirror = mirror
        cursorOverlay.mirror = mirror
        // Long-press the picture to toggle the statistics overlay (only while streaming).
        surfaceView.setOnLongClickListener {
            if (state.phase != LinkPhase.Streaming) return@setOnLongClickListener false
            setStatsVisible(stats.visibility != View.VISIBLE)
            true
        }
        setContentView(root)
    }

    private var lastFrames = 0L
    private var lastBytes = 0L
    private var lastTime = 0L

    private val statsTick = object : Runnable {
        override fun run() {
            val c = client
            if (c != null) {
                val now = System.nanoTime()
                val frames = c.framesRendered
                val bytes = c.bytesReceived.get()
                val dt = (now - lastTime) / 1e9
                if (lastTime != 0L && dt > 0 && frames >= lastFrames) {
                    val fps = (frames - lastFrames) / dt
                    val mbps = (bytes - lastBytes) * 8 / dt / 1e6
                    val hz = display?.refreshRate ?: 0f
                    val text = "%.0f fps  %.1f Mbps  decode %.1f ms  panel %.0f Hz  cursor %s".format(
                        fps, mbps, c.decodeLatencyMs, hz, if (cursorClient?.connected == true) "on" else "OFF",
                    )
                    stats.text = text + "\n" + cursorOverlay.debugState()
                    android.util.Log.d("Ligarview", text + " " + cursorOverlay.debugState())
                }
                lastFrames = frames; lastBytes = bytes; lastTime = now
            }
            surfaceView.postDelayed(this, 1000)
        }
    }

    private fun setStatsVisible(visible: Boolean) {
        val v = if (visible) View.VISIBLE else View.GONE
        stats.visibility = v
        statsMirror.visibility = v
    }

    private fun dp(v: Int) = (v * resources.displayMetrics.density).toInt()

    private fun prefs() = getSharedPreferences("ligarview", Context.MODE_PRIVATE)

    private fun openSettings(action: String) {
        try {
            startActivity(Intent(action))
        } catch (_: Exception) {
            startActivity(Intent(Settings.ACTION_SETTINGS))
        }
    }

    private fun connect() {
        val target = state.hostText.trim()
        if (target.isEmpty()) return
        prefs().edit().putString(KEY_TARGET, target).apply()
        state.everConnected = false
        wantConnection = true
        startClient()
    }

    /** The USB accessory link, opened on demand. Null when none is attached or permission is pending. */
    private fun obtainUsbLink(): UsbLink? {
        usbLink?.let {
            if (it.alive) return it
            it.close()
            usbLink = null
        }
        val accessory = usbManager.accessoryList?.firstOrNull() ?: return null
        if (!usbManager.hasPermission(accessory)) {
            usbManager.requestPermission(accessory, permissionIntent)
            return null
        }
        val pfd = try {
            usbManager.openAccessory(accessory)
        } catch (_: Exception) {
            null
        } ?: return null
        return UsbLink(pfd).also { usbLink = it }
    }

    private fun startClient() {
        stopClients()
        if (!wantConnection || !surfaceReady) return
        val link = obtainUsbLink()
        val videoSource: StreamSource
        val cursorSource: StreamSource
        if (link != null) {
            videoSource = link.videoSource
            cursorSource = link.cursorSource
            link.onHostAsk = { runOnUiThread { startClient() } }
            state.transport = Transport.Usb
        } else if (usbManager.accessoryList?.isNotEmpty() == true) {
            state.phase = LinkPhase.Permission
            return
        } else {
            val (host, port) = parseTarget(state.hostText)
            videoSource = TcpSource(host, port)
            cursorSource = TcpSource(host, port + 1)
            state.transport = Transport.Network
        }
        state.phase = LinkPhase.Connecting
        val token = Any()
        clientToken = token
        client = StreamClient(videoSource, surfaceView.holder.surface) { status, detail ->
            onLinkStatus(token, status, detail)
        }.also { it.start() }
        cursorClient = CursorClient(cursorSource, cursorOverlay).also { it.start() }
        cursorOverlay.enabled = true
    }

    private fun onLinkStatus(token: Any, status: LinkStatus, detail: String) {
        runOnUiThread {
            // A client that was stopped (Back, new connection) may still report a late status.
            if (token !== clientToken) return@runOnUiThread
            if (status == LinkStatus.Connected || status == LinkStatus.Streaming) state.everConnected = true
            state.phase = when (status) {
                LinkStatus.Connecting -> LinkPhase.Connecting
                LinkStatus.Connected -> LinkPhase.Connected
                LinkStatus.Streaming -> LinkPhase.Streaming
                LinkStatus.Retrying -> LinkPhase.Retrying
            }
            state.detail = detail
            if (status != LinkStatus.Streaming) setStatsVisible(false)
            keepScreenOn(status == LinkStatus.Streaming)
        }
    }

    /** The cable was pulled: fall back to the saved host, or return to the information screen. */
    private fun onUsbDetached() {
        usbLink?.close()
        usbLink = null
        stopClients()
        if (prefs().getString(KEY_TARGET, null) != null) {
            startClient()
        } else {
            wantConnection = false
            state.phase = LinkPhase.Idle
            state.detail = ""
        }
    }

    /**
     * The tablet is a monitor, so the screen stays on while the stream is live. When the host is
     * gone for a minute the flag is dropped so an unattended tablet can go to sleep instead of
     * showing "Retrying..." all night.
     */
    private fun keepScreenOn(streaming: Boolean) {
        if (streaming) {
            handler.removeCallbacks(releaseAwake)
            graceStarted = false
            window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        } else if (!graceStarted) {
            // Started once; retries that keep reporting "not streaming" must not restart it.
            graceStarted = true
            window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            handler.postDelayed(releaseAwake, KEEP_AWAKE_GRACE_MS)
        }
    }

    private fun stopClients() {
        clientToken = null
        cursorOverlay.enabled = false
        cursorClient?.stop()
        cursorClient = null
        client?.stop()
        client = null
    }

    /** Stops the clients and tells the host over USB to stop sending (the link itself stays up). */
    private fun stopClient() {
        stopClients()
        usbLink?.pause()
    }

    private fun parseTarget(text: String): Pair<String, Int> {
        val t = text.trim().ifEmpty { DEFAULT_TARGET }
        val i = t.lastIndexOf(':')
        if (i < 0) return t to DEFAULT_PORT
        return t.substring(0, i) to (t.substring(i + 1).toIntOrNull() ?: DEFAULT_PORT)
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        surfaceReady = true
        surfaceView.removeCallbacks(statsTick)
        surfaceView.post(statsTick)
        startClient()
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) = Unit

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        surfaceReady = false
        stopClient()
    }

    private companion object {
        const val KEEP_AWAKE_GRACE_MS = 60_000L
        const val DEVICE_POLL_MS = 1500L
        const val ACTION_USB_PERMISSION = "app.ligarview.client.USB_PERMISSION"
        const val KEY_TARGET = "target"
        const val DEFAULT_PORT = 7878
        const val DEFAULT_TARGET = "127.0.0.1:$DEFAULT_PORT"
    }
}
