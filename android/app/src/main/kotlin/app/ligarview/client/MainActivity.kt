package app.ligarview.client

import android.app.Activity
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbManager
import android.graphics.Color
import android.os.Bundle
import android.text.InputType
import android.view.Gravity
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.View
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.view.WindowManager
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.Button
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView

class MainActivity : Activity(), SurfaceHolder.Callback {
    private lateinit var surfaceView: SurfaceView
    private lateinit var panel: LinearLayout
    private lateinit var status: TextView
    private lateinit var stats: TextView
    private lateinit var hostInput: EditText

    private var client: StreamClient? = null
    private var cursorClient: CursorClient? = null
    private lateinit var usbManager: UsbManager
    private var usbLink: UsbLink? = null
    private lateinit var permissionIntent: PendingIntent
    private val usbReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            when (intent.action) {
                ACTION_USB_PERMISSION -> {
                    if (intent.getBooleanExtra(UsbManager.EXTRA_PERMISSION_GRANTED, false)) startClient()
                    else status.text = "USB permission denied"
                }
                UsbManager.ACTION_USB_ACCESSORY_DETACHED -> onUsbDetached()
            }
        }
    }
    private val handler = android.os.Handler(android.os.Looper.getMainLooper())
    /** True from the moment the stream was lost until it is live again. */
    private var graceStarted = false
    private val releaseAwake = Runnable {
        window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
    }
    private lateinit var cursorOverlay: CursorOverlay
    private var surfaceReady = false

    /** True while the user wants a connection (panel hidden). */
    private var wantConnection = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.attributes = window.attributes.apply {
            layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
        }
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

        val saved = prefs().getString(KEY_TARGET, null)
        hostInput.setText(saved ?: DEFAULT_TARGET)
        // Connect automatically: over USB when the Mac has switched the tablet into accessory
        // mode, otherwise to the last used host.
        if (saved != null || usbManager.accessoryList?.isNotEmpty() == true) {
            wantConnection = true
            panel.visibility = View.GONE
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        if (intent.action == UsbManager.ACTION_USB_ACCESSORY_ATTACHED) {
            wantConnection = true
            panel.visibility = View.GONE
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
        super.onPause()
    }

    override fun onResume() {
        super.onResume()
        cursorOverlay.start()
        requestHighRefreshRate()
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

        status = TextView(this).apply {
            setTextColor(Color.WHITE)
            setBackgroundColor(0x99000000.toInt())
            textSize = 14f
            setPadding(dp(12), dp(6), dp(12), dp(6))
            visibility = View.GONE
        }
        root.addView(
            status,
            FrameLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT, Gravity.BOTTOM or Gravity.START).apply {
                setMargins(dp(16), dp(16), dp(16), dp(16))
            },
        )

        val cursorView = CursorView(this)
        root.addView(cursorView, FrameLayout.LayoutParams(MATCH_PARENT, MATCH_PARENT))
        cursorOverlay = CursorOverlay(cursorView, root)

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
        val mirror = android.widget.ImageView(this).apply {
            scaleType = android.widget.ImageView.ScaleType.FIT_CENTER
            setBackgroundColor(0xFF808080.toInt())
            visibility = View.GONE
        }
        root.addView(
            mirror,
            FrameLayout.LayoutParams(dp(110), dp(110), Gravity.TOP or Gravity.START).apply {
                setMargins(dp(16), dp(70), 0, 0)
            },
        )
        cursorOverlay.mirror = mirror
        // Long-press the picture to toggle the statistics overlay.
        surfaceView.setOnLongClickListener {
            stats.visibility = if (stats.visibility == View.VISIBLE) View.GONE else View.VISIBLE
            mirror.visibility = stats.visibility
            true
        }

        panel = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(0xEE111111.toInt())
            setPadding(dp(32), dp(32), dp(32), dp(32))
        }
        panel.addView(label("Ligarview", 28f, Color.WHITE))
        panel.addView(label("Wi-Fi / adb host address (host:port)", 14f, 0xFFAAAAAA.toInt(), top = 24))
        hostInput = EditText(this).apply {
            setTextColor(Color.WHITE)
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI
            isSingleLine = true
        }
        panel.addView(hostInput, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        panel.addView(Button(this).apply {
            text = "Connect"
            setOnClickListener { connect() }
        }, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = dp(16) })
        panel.addView(
            label(
                "USB: run the Ligarview host on the Mac and plug the tablet in. Android asks once whether " +
                    "to open Ligarview for the USB accessory; accept it (tick \"use by default\" to skip the " +
                    "question next time).\n\n" +
                    "Wi-Fi: enter the Mac's IP address, e.g. 192.168.1.10:7878.\n" +
                    "Developer: with USB debugging, `adb reverse tcp:7878 tcp:7878` and connect to 127.0.0.1:7878.\n\n" +
                    "Back returns to this screen.",
                13f, 0xFF888888.toInt(), top = 24,
            ),
        )
        root.addView(
            panel,
            FrameLayout.LayoutParams(dp(480), WRAP_CONTENT, Gravity.CENTER),
        )
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

    private fun label(text: String, size: Float, color: Int, top: Int = 0) = TextView(this).apply {
        this.text = text
        textSize = size
        setTextColor(color)
        layoutParams = LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = dp(top) }
    }

    private fun dp(v: Int) = (v * resources.displayMetrics.density).toInt()

    private fun prefs() = getSharedPreferences("ligarview", Context.MODE_PRIVATE)

    private fun connect() {
        prefs().edit().putString(KEY_TARGET, hostInput.text.toString().trim()).apply()
        wantConnection = true
        panel.visibility = View.GONE
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
        } else if (usbManager.accessoryList?.isNotEmpty() == true) {
            status.text = "Waiting for USB permission..."
            status.visibility = View.VISIBLE
            return
        } else {
            val (host, port) = parseTarget(hostInput.text.toString())
            videoSource = TcpSource(host, port)
            cursorSource = TcpSource(host, port + 1)
        }
        client = StreamClient(videoSource, surfaceView.holder.surface) { message, streaming ->
            runOnUiThread {
                status.text = message
                status.visibility = if (streaming) View.GONE else View.VISIBLE
                keepScreenOn(streaming)
            }
        }.also { it.start() }
        cursorClient = CursorClient(cursorSource, cursorOverlay).also { it.start() }
        cursorOverlay.enabled = true
    }

    /** The cable was pulled: fall back to the saved host, or show the panel. */
    private fun onUsbDetached() {
        usbLink?.close()
        usbLink = null
        stopClients()
        if (prefs().getString(KEY_TARGET, null) != null) {
            startClient()
        } else {
            wantConnection = false
            status.visibility = View.GONE
            panel.visibility = View.VISIBLE
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

    @Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")
    override fun onBackPressed() {
        if (wantConnection) {
            wantConnection = false
            stopClient()
            status.visibility = View.GONE
            panel.visibility = View.VISIBLE
        } else {
            super.onBackPressed()
        }
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
        const val ACTION_USB_PERMISSION = "app.ligarview.client.USB_PERMISSION"
        const val KEY_TARGET = "target"
        const val DEFAULT_PORT = 7878
        const val DEFAULT_TARGET = "127.0.0.1:$DEFAULT_PORT"
    }
}
