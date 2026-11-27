package app.ligarview.client

import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.SystemClock
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast

/**
 * Sidecar-style sidebar on the left edge of the full-screen picture: Cmd, Shift, Ctrl and Option
 * that stay held for the next pen stroke or key (tap) or until tapped again (double-tap), Esc, Undo
 * and Redo, Dock and menu bar, the on-screen keyboard, and Disconnect. A small tab expands and
 * collapses it so it does not cover the Mac's screen. Wire format: see host/src/input.rs.
 */
class Sidebar(
    context: Context,
    private val send: (ByteArray) -> Unit,
    private val onKeyboard: () -> Unit,
    private val onDisconnect: () -> Unit,
) {
    private enum class Hold { OFF, ONCE, LOCKED }

    private class Mod(val bit: Int, val label: String) {
        var hold = Hold.OFF
        var lastTap = 0L
        lateinit var view: TextView
    }

    private val density = context.resources.displayMetrics.density
    private fun dp(v: Int) = (v * density).toInt()

    /** What the Mac allows (bits 1 pen and sidebar, 4 typing); -1 until the Mac has said. */
    private var caps = -1

    private val mods = listOf(Mod(1, "⌘"), Mod(2, "⇧"), Mod(4, "⌃"), Mod(8, "⌥"))
    private val sidebarButtons = mutableListOf<TextView>()
    private val typingButtons = mutableListOf<TextView>()

    private val strip = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_HORIZONTAL
        setPadding(dp(6), dp(8), dp(6), dp(8))
    }
    private val scroller = ScrollView(context).apply {
        isVerticalScrollBarEnabled = false
        overScrollMode = View.OVER_SCROLL_NEVER
        background = GradientDrawable().apply {
            setColor(0xE6141820.toInt())
            cornerRadii = floatArrayOf(0f, 0f, dp(16).toFloat(), dp(16).toFloat(), dp(16).toFloat(), dp(16).toFloat(), 0f, 0f)
        }
        addView(strip, FrameLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT))
        visibility = View.GONE
    }
    private val tab = TextView(context).apply {
        text = "›"
        setTextColor(TEXT)
        textSize = 22f
        gravity = Gravity.CENTER
        background = GradientDrawable().apply {
            setColor(0xCC141820.toInt())
            cornerRadii = floatArrayOf(0f, 0f, dp(12).toFloat(), dp(12).toFloat(), dp(12).toFloat(), dp(12).toFloat(), 0f, 0f)
        }
        visibility = View.GONE
        setOnClickListener { setExpanded(scroller.visibility != View.VISIBLE) }
    }

    /** The views to add to the full-screen layout; they sit above the picture. */
    val container = FrameLayout(context).apply {
        addView(scroller, FrameLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT, Gravity.START or Gravity.CENTER_VERTICAL))
        addView(tab, FrameLayout.LayoutParams(dp(26), dp(76), Gravity.START or Gravity.CENTER_VERTICAL))
    }

    private lateinit var dockView: TextView
    private lateinit var menuView: TextView

    /** Which edge button is showing its bar (EDGE_BOTTOM, EDGE_TOP) or 0. */
    private var edgeShown = 0
    private val forgetEdge = Runnable { setEdgeShown(0) }

    init {
        // The keyboard comes first so that it stays in reach while the keyboard covers the lower half.
        add(button("aA", typing = true) { onKeyboard() })
        divider()
        for (m in mods) {
            m.view = button(m.label, typing = false) { tapModifier(m) }
            add(m.view)
        }
        divider()
        add(button("esc", typing = false) { key(KEY_ESCAPE, 0) })
        add(button("↶", typing = false) { key(KEY_Z, MOD_CMD) })
        add(button("↷", typing = false) { key(KEY_Z, MOD_CMD or MOD_SHIFT) })
        divider()
        // Pushing the pointer against the tablet's edge brings up an auto-hidden Dock or menu bar
        // there; tapping again moves the pointer back to the middle, which hides it again.
        dockView = button("dock", typing = false) { toggleEdge(EDGE_BOTTOM) }
        menuView = button("menu", typing = false) { toggleEdge(EDGE_TOP) }
        add(dockView)
        add(menuView)
        divider()
        add(button("×", typing = false, always = true) { onDisconnect() })
        container.visibility = View.GONE
        applyCaps()
    }

    private fun toggleEdge(edge: Int) {
        container.removeCallbacks(forgetEdge)
        if (edgeShown == edge) {
            send(byteArrayOf(MSG_EDGE.toByte(), EDGE_AWAY.toByte()))
            setEdgeShown(0)
        } else {
            send(byteArrayOf(MSG_EDGE.toByte(), edge.toByte()))
            setEdgeShown(edge)
            // A bar that was auto-hidden goes away by itself once the pen moves off; do not keep claiming it is up.
            container.postDelayed(forgetEdge, EDGE_MARK_MS)
        }
    }

    private fun setEdgeShown(edge: Int) {
        edgeShown = edge
        paint(dockView, if (edge == EDGE_BOTTOM) Hold.ONCE else Hold.OFF)
        paint(menuView, if (edge == EDGE_TOP) Hold.ONCE else Hold.OFF)
    }

    private fun add(v: TextView) {
        strip.addView(v, LinearLayout.LayoutParams(dp(BUTTON_DP), dp(BUTTON_DP)).apply { bottomMargin = dp(4) })
    }

    private fun divider() {
        strip.addView(
            View(strip.context).apply { setBackgroundColor(0x33FFFFFF) },
            LinearLayout.LayoutParams(dp(36), dp(1)).apply { topMargin = dp(4); bottomMargin = dp(8) },
        )
    }

    /**
     * [typing] buttons need "Allow typing" on the Mac, the rest need "Allow pen control"; [always]
     * ones (Disconnect) work regardless. A button the Mac does not allow is dimmed and explains why.
     */
    private fun button(label: String, typing: Boolean, always: Boolean = false, onClick: () -> Unit) =
        TextView(strip.context).apply {
            text = label
            setTextColor(TEXT)
            textSize = when {
                label.length > 2 -> 13f
                label == "×" -> 28f
                label.length > 1 -> 17f
                else -> 24f
            }
            typeface = Typeface.DEFAULT_BOLD
            gravity = Gravity.CENTER
            paint(this, Hold.OFF)
            if (!always) (if (typing) typingButtons else sidebarButtons).add(this)
            setOnClickListener {
                if (always || allowed(typing)) {
                    onClick()
                } else {
                    Toast.makeText(
                        context,
                        if (typing) R.string.typing_off else R.string.control_off,
                        Toast.LENGTH_SHORT,
                    ).show()
                }
            }
        }

    private fun allowed(typing: Boolean) = when {
        caps < 0 -> true
        typing -> caps and CAP_TYPING != 0
        else -> caps and CAP_PEN != 0 || caps and CAP_TYPING != 0
    }

    private fun applyCaps() {
        sidebarButtons.forEach { it.alpha = if (allowed(false)) 1f else DIMMED }
        typingButtons.forEach { it.alpha = if (allowed(true)) 1f else DIMMED }
    }

    /** What the Mac allows, as announced on the cursor channel. */
    fun setCaps(bits: Int) {
        caps = bits
        applyCaps()
    }

    private fun paint(v: TextView, hold: Hold) {
        v.background = GradientDrawable().apply {
            cornerRadius = dp(12).toFloat()
            when (hold) {
                Hold.OFF -> setColor(0x22FFFFFF)
                Hold.ONCE -> {
                    setColor(0x334FD1C5)
                    setStroke(dp(2), ACCENT)
                }
                Hold.LOCKED -> setColor(ACCENT)
            }
        }
        v.setTextColor(if (hold == Hold.LOCKED) ON_ACCENT else TEXT)
    }

    private fun tapModifier(m: Mod) {
        val now = SystemClock.uptimeMillis()
        m.hold = when {
            m.hold == Hold.OFF -> Hold.ONCE
            m.hold == Hold.ONCE && now - m.lastTap < DOUBLE_TAP_MS -> Hold.LOCKED
            else -> Hold.OFF
        }
        m.lastTap = now
        paint(m.view, m.hold)
        sendMask()
    }

    /** Called when a pen stroke or a typed key is done: modifiers armed for one use are let go. */
    fun strokeEnded() {
        var changed = false
        for (m in mods) {
            if (m.hold == Hold.ONCE) {
                m.hold = Hold.OFF
                paint(m.view, Hold.OFF)
                changed = true
            }
        }
        if (changed) sendMask()
    }

    /** The modifiers held right now, as the bits of the wire format (Cmd 1, Shift 2, Ctrl 4, Option 8). */
    fun mask() = mods.filter { it.hold != Hold.OFF }.fold(0) { acc, m -> acc or m.bit }

    private fun sendMask() = send(byteArrayOf(MSG_MODIFIERS.toByte(), mask().toByte()))

    private fun key(code: Int, mask: Int) =
        send(byteArrayOf(MSG_KEY.toByte(), (code shr 8).toByte(), code.toByte(), mask.toByte()))

    private fun setExpanded(open: Boolean) {
        scroller.visibility = if (open) View.VISIBLE else View.GONE
        tab.text = if (open) "‹" else "›"
        tab.translationX = if (open) dp(BUTTON_DP + 12).toFloat() else 0f
    }

    /** Shown only while the picture is full screen and the host accepts input. */
    fun setAvailable(available: Boolean) {
        if (available == (container.visibility == View.VISIBLE)) return
        container.visibility = if (available) View.VISIBLE else View.GONE
        tab.visibility = if (available) View.VISIBLE else View.GONE
        if (!available) {
            container.removeCallbacks(forgetEdge)
            setEdgeShown(0)
            setExpanded(false)
            // Nothing may stay held on the Mac when the sidebar goes away.
            if (mods.any { it.hold != Hold.OFF }) {
                for (m in mods) {
                    m.hold = Hold.OFF
                    paint(m.view, Hold.OFF)
                }
                sendMask()
            }
        }
    }

    private companion object {
        const val MSG_MODIFIERS = 3
        const val MSG_KEY = 4
        const val MSG_EDGE = 7
        const val EDGE_BOTTOM = 1
        const val EDGE_TOP = 2
        const val EDGE_AWAY = 0
        const val EDGE_MARK_MS = 4000L
        const val MOD_CMD = 1
        const val MOD_SHIFT = 2
        const val KEY_ESCAPE = 53
        const val KEY_Z = 6
        const val CAP_PEN = 1
        const val CAP_TYPING = 4
        const val BUTTON_DP = 48
        const val DIMMED = 0.35f
        const val DOUBLE_TAP_MS = 350L
        val TEXT = Color.parseColor("#ECEFF4")
        val ACCENT = Color.parseColor("#4FD1C5")
        val ON_ACCENT = Color.parseColor("#00201C")
    }
}
