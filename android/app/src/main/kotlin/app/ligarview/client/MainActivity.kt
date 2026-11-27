package app.ligarview.client

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ValueAnimator
import android.graphics.Color
import android.graphics.RectF
import android.hardware.usb.UsbManager
import android.os.Bundle
import android.provider.Settings
import android.view.Gravity
import android.view.InputDevice
import android.view.KeyEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.WindowInsets
import android.view.WindowInsetsController
import android.view.ViewOutlineProvider
import android.view.WindowManager
import android.view.animation.PathInterpolator
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
import kotlin.math.roundToInt

/**
 * Hosts the live picture (a SurfaceView fed by the video decoder), the Mac's cursor drawn on top,
 * and a Compose overlay that explains how to connect and what to adjust while nothing is streaming.
 */
class MainActivity : ComponentActivity(), SurfaceHolder.Callback {
    private lateinit var root: FrameLayout
    private lateinit var surfaceView: SurfaceView
    private lateinit var stats: TextView
    private lateinit var statsMirror: ImageView
    private lateinit var cursorOverlay: CursorOverlay
    private lateinit var state: UiState

    private var client: StreamClient? = null

    private lateinit var discovery: NsdDiscovery
    private lateinit var pairingServer: PairingServer

    /** The Mac announced on the network, if any. */
    private var found: FoundMac? = null

    /** `host:port` the running network client is connected to. */
    private var activeTarget: String? = null

    /** The user left on the tablet; stay disconnected until they pair again or reopen the app. */
    private var userStopped = false

    /** Identifies the running client; status reports from a stopped one are ignored. */
    private var clientToken: Any? = null
    private var cursorClient: CursorClient? = null
    private var sidebar: Sidebar? = null
    private lateinit var keyboardInput: KeyboardInput
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

    // The video view sits behind the Compose screen. On the information screen it is placed exactly
    // behind the preview box, in full screen it covers everything; moving it never recreates the
    // surface, so the decoder keeps running.
    private val previewRect = RectF()
    private val surfaceRect = RectF()
    private var surfaceAnimator: ValueAnimator? = null
    private var cornerRadius = 0f

    /** Back leaves the full screen and returns to the information screen. */
    private val backCallback = object : OnBackPressedCallback(false) {
        override fun handleOnBackPressed() = setFullscreen(false)
    }

    /** True while the user wants a connection. */
    private var wantConnection = false

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
        // The keyboard overlays the picture instead of squeezing it.
        window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_NOTHING)

        // Older versions let the user type an address and saved it; it is no longer used.
        prefs().edit().remove(KEY_TARGET).apply()
        val secret = prefs().getString(KEY_SECRET, "").orEmpty()
        state = UiState(initialCode = newCode(), startOnWifiTab = secret.length == CODE_LENGTH)
        intent.getStringExtra(EXTRA_HOST)?.let { state.hostText = it.trim() }
        discovery = NsdDiscovery(this, ::onMacFound)
        pairingServer = PairingServer(this, { state.pairingCode }, ::onPaired)
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
        if (hasWifiPlan() || state.hostText.isNotEmpty() || usbManager.accessoryList?.isNotEmpty() == true) {
            wantConnection = true
            state.phase = LinkPhase.Connecting
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        // Developer route: `am start ... --es host 127.0.0.1:7878` connects to that address (adb reverse).
        intent.getStringExtra(EXTRA_HOST)?.let {
            state.hostText = it.trim()
            userStopped = false
            state.everConnected = false
            wantConnection = true
            startClient()
            return
        }
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
        discovery.stop()
        pairingServer.stop()
        handler.removeCallbacks(deviceTick)
        super.onPause()
    }

    override fun onResume() {
        super.onResume()
        cursorOverlay.start()
        discovery.start()
        pairingServer.start()
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
        root = FrameLayout(this).apply { setBackgroundColor(0xFF0E1013.toInt()) }

        surfaceView = SurfaceView(this)
        surfaceView.holder.addCallback(this)
        surfaceView.outlineProvider = object : ViewOutlineProvider() {
            override fun getOutline(view: View, outline: android.graphics.Outline) {
                outline.setRoundRect(0, 0, view.width, view.height, cornerRadius)
            }
        }
        surfaceView.clipToOutline = true
        // Tiny until the Compose screen reports where the preview box is.
        root.addView(
            surfaceView,
            FrameLayout.LayoutParams(dp(16), dp(10), Gravity.TOP or Gravity.START),
        )

        val cursorView = CursorView(this)
        root.addView(cursorView, FrameLayout.LayoutParams(MATCH_PARENT, MATCH_PARENT))
        cursorOverlay = CursorOverlay(cursorView, root)

        val actions = Actions(
            connectUsb = {
                userStopped = false
                state.everConnected = false
                wantConnection = true
                startClient()
            },
            disconnect = ::disconnect,
            openFullscreen = { if (state.phase == LinkPhase.Streaming) setFullscreen(true) },
            onPreviewBounds = ::onPreviewBounds,
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
        // Long-press the full-screen picture to toggle the statistics overlay.
        surfaceView.setOnLongClickListener {
            if (!state.fullscreen) return@setOnLongClickListener false
            setStatsVisible(stats.visibility != View.VISIBLE)
            true
        }
        // The S Pen controls the Mac's pointer in full screen (the host must allow it).
        val sidebar = Sidebar(
            this,
            send = { cursorClient?.sendInput(it) },
            onKeyboard = ::toggleKeyboard,
            onDisconnect = ::disconnect,
        )
        this.sidebar = sidebar
        root.addView(sidebar.container, FrameLayout.LayoutParams(MATCH_PARENT, MATCH_PARENT))
        // The keyboard overlays the picture; the sidebar moves up out of its way and scrolls if needed.
        root.setOnApplyWindowInsetsListener { v, insets ->
            sidebar.container.setPadding(0, 0, 0, insets.getInsets(WindowInsets.Type.ime()).bottom)
            v.onApplyWindowInsets(insets)
        }
        keyboardInput = KeyboardInput(
            this,
            send = { cursorClient?.sendInput(it) },
            sidebarMask = sidebar::mask,
            keyDone = sidebar::strokeEnded,
        )
        root.addView(keyboardInput.view, FrameLayout.LayoutParams(1, 1))
        val pen = PenInput(
            enabled = { state.fullscreen && cursorClient?.canSendInput == true },
            send = { cursorClient?.sendInput(it) },
            onStrokeEnd = sidebar::strokeEnded,
        )
        val fingers = FingerScroll(
            enabled = { state.fullscreen && cursorClient?.canSendInput == true },
            send = { cursorClient?.sendInput(it) },
            slopPx = android.view.ViewConfiguration.get(this).scaledTouchSlop.toFloat(),
        )
        surfaceView.setOnTouchListener { v, e -> pen.onTouch(v, e) || fingers.onTouch(v, e) }
        surfaceView.setOnHoverListener { v, e -> pen.onHover(v, e) }
        // Rotating the tablet changes the root size: refit the full-screen picture (the preview is
        // re-placed by the Compose screen, which reports its new bounds).
        root.addOnLayoutChangeListener { _, left, top, right, bottom, oldLeft, oldTop, oldRight, oldBottom ->
            val changed = right - left != oldRight - oldLeft || bottom - top != oldBottom - oldTop
            if (changed && state.fullscreen && surfaceAnimator == null) placeSurface(fitRect(), 0f)
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
                    state.liveStats = "%.0f fps · %.1f Mbps · %.0f ms decode".format(fps, mbps, c.decodeLatencyMs)
                    stats.text = text + "\n" + cursorOverlay.debugState()
                    android.util.Log.d("Ligarview", text + " " + cursorOverlay.debugState())
                }
                lastFrames = frames; lastBytes = bytes; lastTime = now
            }
            sidebar?.setAvailable(state.fullscreen && cursorClient?.canSendInput == true)
            surfaceView.postDelayed(this, 1000)
        }
    }

    private fun onPreviewBounds(left: Float, top: Float, width: Float, height: Float) {
        previewRect.set(left, top, left + width, top + height)
        if (!state.fullscreen && surfaceAnimator == null) placeSurface(previewRect, dp(PREVIEW_RADIUS_DP).toFloat())
    }

    private fun placeSurface(rect: RectF, radius: Float) {
        surfaceRect.set(rect)
        cornerRadius = radius
        val lp = surfaceView.layoutParams as FrameLayout.LayoutParams
        lp.width = rect.width().roundToInt().coerceAtLeast(1)
        lp.height = rect.height().roundToInt().coerceAtLeast(1)
        lp.leftMargin = rect.left.roundToInt()
        lp.topMargin = rect.top.roundToInt()
        lp.gravity = Gravity.TOP or Gravity.START
        surfaceView.layoutParams = lp
        surfaceView.invalidateOutline()
        cursorOverlay.setFrame(rect.left, rect.top, rect.width(), rect.height())
    }

    private fun animateSurfaceTo(target: RectF, radius: Float) {
        surfaceAnimator?.cancel()
        val from = RectF(surfaceRect)
        val fromRadius = cornerRadius
        val to = RectF(target)
        surfaceAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 280
            interpolator = PathInterpolator(0.2f, 0f, 0f, 1f)
            addUpdateListener {
                val f = it.animatedFraction
                fun lerp(a: Float, b: Float) = a + (b - a) * f
                placeSurface(
                    RectF(lerp(from.left, to.left), lerp(from.top, to.top), lerp(from.right, to.right), lerp(from.bottom, to.bottom)),
                    lerp(fromRadius, radius),
                )
            }
            addListener(object : AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: Animator) {
                    surfaceAnimator = null
                }
            })
            start()
        }
    }

    /**
     * The picture at full size: as large as fits while keeping the Mac's proportions. Landscape
     * tablets of the same shape are filled; an upright tablet gets bars above and below.
     */
    private fun fitRect(): RectF {
        val w = root.width.toFloat()
        val h = root.height.toFloat()
        val aspect = state.streamAspect
        return if (w / h > aspect) {
            val fitted = h * aspect
            RectF((w - fitted) / 2f, 0f, (w + fitted) / 2f, h)
        } else {
            val fitted = w / aspect
            RectF(0f, (h - fitted) / 2f, w, (h + fitted) / 2f)
        }
    }

    private fun onVideoSize(width: Int, height: Int) {
        runOnUiThread {
            if (width <= 0 || height <= 0) return@runOnUiThread
            state.streamAspect = width.toFloat() / height
            if (state.fullscreen && surfaceAnimator == null) placeSurface(fitRect(), 0f)
        }
    }

    /** Full screen picture, or the small preview on the information screen. */
    private fun setFullscreen(value: Boolean) {
        if (state.fullscreen == value) return
        state.fullscreen = value
        backCallback.isEnabled = value
        if (value) {
            animateSurfaceTo(fitRect(), 0f)
        } else {
            setStatsVisible(false)
            animateSurfaceTo(previewRect, dp(PREVIEW_RADIUS_DP).toFloat())
        }
    }

    /** Leaves the Mac: stops streaming and returns to the idle information screen. */
    private fun disconnect() {
        setFullscreen(false)
        userStopped = true
        wantConnection = false
        stopClient()
        state.phase = LinkPhase.Idle
        state.detail = ""
        state.everConnected = false
        setStatsVisible(false)
    }

    private fun setStatsVisible(visible: Boolean) {
        val v = if (visible) View.VISIBLE else View.GONE
        stats.visibility = v
        statsMirror.visibility = v
    }

    private fun dp(v: Int) = (v * resources.displayMetrics.density).toInt()

    /** Shows or hides the on-screen keyboard; what is typed goes to the Mac. */
    private fun toggleKeyboard() {
        val controller = window.insetsController ?: return
        val shown = root.rootWindowInsets?.isVisible(WindowInsets.Type.ime()) == true
        if (shown) {
            controller.hide(WindowInsets.Type.ime())
        } else {
            keyboardInput.view.requestFocus()
            controller.show(WindowInsets.Type.ime())
        }
    }

    /** A hardware keyboard attached to the tablet types on the Mac while the picture is full screen. */
    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        val usable = state.fullscreen && cursorClient?.canSendInput == true &&
            event.keyCode != KeyEvent.KEYCODE_BACK && event.device?.isVirtual == false &&
            event.isFromSource(InputDevice.SOURCE_KEYBOARD)
        if (usable && keyboardInput.hardwareKey(event)) return true
        return super.dispatchKeyEvent(event)
    }

    private fun prefs() = getSharedPreferences("ligarview", Context.MODE_PRIVATE)

    private fun openSettings(action: String) {
        try {
            startActivity(Intent(action))
        } catch (_: Exception) {
            startActivity(Intent(Settings.ACTION_SETTINGS))
        }
    }

    private fun secret() = prefs().getString(KEY_SECRET, "").orEmpty()

    private fun hasWifiPlan() = secret().length == CODE_LENGTH

    private fun newCode() = (100_000 + java.security.SecureRandom().nextInt(900_000)).toString()

    /** The Mac accepted this tablet's code: remember it as the secret and connect right away. */
    private fun onPaired(host: String?, port: Int, code: Int) {
        prefs().edit().putString(KEY_SECRET, code.toString()).apply()
        state.pairingCode = newCode()
        // Keep the Mac Bonjour found unless the pairing connection gave a usable address.
        if (host != null && found?.host != host) {
            found = FoundMac(found?.name ?: "Mac", host, port)
            state.foundMac = found?.name
        }
        userStopped = false
        state.everConnected = false
        wantConnection = true
        startClient()
    }

    /** A Mac appeared or went away on the network: connect to it when the tablet is paired. */
    private fun onMacFound(mac: FoundMac?) {
        found = mac
        state.foundMac = mac?.name
        if (mac == null || state.hostText.isNotBlank() || secret().length != CODE_LENGTH) return
        if (usbManager.accessoryList?.isNotEmpty() == true) return
        if (!wantConnection) {
            if (userStopped) return
            state.everConnected = false
            wantConnection = true
            startClient()
        } else if (state.transport != Transport.Usb && activeTarget != "${mac.host}:${mac.port}") {
            startClient()
        }
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
            val manual = state.hostText.trim()
            val (host, port) = when {
                manual.isNotEmpty() -> parseTarget(manual)
                else -> found?.let { it.host to it.port }
            } ?: run {
                // Paired but the Mac is not announced yet: wait for the discovery callback.
                state.transport = Transport.None
                state.phase = LinkPhase.Idle
                return
            }
            val code = secret().toIntOrNull() ?: 0
            videoSource = TcpSource(host, port, code)
            cursorSource = TcpSource(host, port + 1, code)
            activeTarget = "$host:$port"
            state.transport = Transport.Network
        }
        state.phase = LinkPhase.Connecting
        val token = Any()
        clientToken = token
        client = StreamClient(
            videoSource,
            surfaceView.holder.surface,
            onStatus = { status, detail -> onLinkStatus(token, status, detail) },
            onSize = ::onVideoSize,
        ).also { it.start() }
        cursorClient = CursorClient(cursorSource, cursorOverlay).also {
            it.onCaps = { bits -> runOnUiThread { sidebar?.setCaps(bits) } }
            it.start()
        }
        cursorOverlay.enabled = true
    }

    private fun onLinkStatus(token: Any, status: LinkStatus, detail: String) {
        runOnUiThread {
            // A client that was stopped (Back, new connection) may still report a late status.
            if (token !== clientToken) return@runOnUiThread
            if (status == LinkStatus.Connected || status == LinkStatus.Streaming) state.everConnected = true
            if (status == LinkStatus.Connected) state.streamInfo = detail
            // If the picture goes away while full screen, show the information screen with the reason.
            if (status != LinkStatus.Streaming && state.fullscreen) setFullscreen(false)
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
        if (hasWifiPlan() && !userStopped) {
            startClient()
        } else {
            wantConnection = false
            state.phase = LinkPhase.Idle
            state.detail = ""
            setFullscreen(false)
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
        activeTarget = null
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
        const val PREVIEW_RADIUS_DP = 12
        const val DEVICE_POLL_MS = 1500L
        const val ACTION_USB_PERMISSION = "app.ligarview.client.USB_PERMISSION"
        /** Saved by older versions; only cleared now. */
        const val KEY_TARGET = "manual_address"
        const val EXTRA_HOST = "host"
        const val KEY_SECRET = "pairing_secret"
        const val CODE_LENGTH = 6
        const val DEFAULT_PORT = 7878
        const val DEFAULT_TARGET = "127.0.0.1:$DEFAULT_PORT"
    }
}
