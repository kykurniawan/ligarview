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
import android.widget.TextView

/**
 * Sidecar-style sidebar on the left edge of the full-screen picture: Cmd, Shift, Ctrl and Option
 * that stay held for the next pen stroke (tap) or until tapped again (double-tap), plus Esc, Undo
 * and Redo. A small tab expands and collapses it so it does not cover the Mac's screen.
 * Wire format: see host/src/input.rs.
 */
class Sidebar(context: Context, private val send: (ByteArray) -> Unit) {
    private enum class Hold { OFF, ONCE, LOCKED }

    private class Mod(val bit: Int, val label: String) {
        var hold = Hold.OFF
        var lastTap = 0L
        lateinit var view: TextView
    }

    private val density = context.resources.displayMetrics.density
    private fun dp(v: Int) = (v * density).toInt()

    private val mods = listOf(Mod(1, "⌘"), Mod(2, "⇧"), Mod(4, "⌃"), Mod(8, "⌥"))
    private val strip = LinearLayout(context).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_HORIZONTAL
        setPadding(dp(6), dp(8), dp(6), dp(8))
        background = GradientDrawable().apply {
            setColor(0xE6141820.toInt())
            cornerRadii = floatArrayOf(0f, 0f, dp(16).toFloat(), dp(16).toFloat(), dp(16).toFloat(), dp(16).toFloat(), 0f, 0f)
        }
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
        setOnClickListener { setExpanded(strip.visibility != View.VISIBLE) }
    }

    /** The views to add to the full-screen layout; they sit above the picture. */
    val container = FrameLayout(context).apply {
        addView(strip, FrameLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT, Gravity.START or Gravity.CENTER_VERTICAL))
        addView(tab, FrameLayout.LayoutParams(dp(26), dp(76), Gravity.START or Gravity.CENTER_VERTICAL))
    }

    init {
        for (m in mods) {
            m.view = button(m.label) { tapModifier(m) }
            strip.addView(m.view, LinearLayout.LayoutParams(dp(52), dp(52)).apply { bottomMargin = dp(4) })
        }
        strip.addView(spacer(), LinearLayout.LayoutParams(dp(36), dp(1)).apply { topMargin = dp(4); bottomMargin = dp(8) })
        strip.addView(button("esc") { key(KEY_ESCAPE, 0) }, LinearLayout.LayoutParams(dp(52), dp(52)).apply { bottomMargin = dp(4) })
        strip.addView(button("↶") { key(KEY_Z, MOD_CMD) }, LinearLayout.LayoutParams(dp(52), dp(52)).apply { bottomMargin = dp(4) })
        strip.addView(button("↷") { key(KEY_Z, MOD_CMD or MOD_SHIFT) }, LinearLayout.LayoutParams(dp(52), dp(52)))
        container.visibility = View.GONE
    }

    private fun spacer() = View(strip.context).apply { setBackgroundColor(0x33FFFFFF) }

    private fun button(label: String, onClick: () -> Unit) = TextView(strip.context).apply {
        text = label
        setTextColor(TEXT)
        textSize = if (label.length > 1) 15f else 24f
        typeface = Typeface.DEFAULT_BOLD
        gravity = Gravity.CENTER
        paint(this, Hold.OFF)
        setOnClickListener { onClick() }
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

    /** Called when a pen stroke ends: modifiers armed for one use are let go. */
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

    private fun mask() = mods.filter { it.hold != Hold.OFF }.fold(0) { acc, m -> acc or m.bit }

    private fun sendMask() = send(byteArrayOf(MSG_MODIFIERS.toByte(), mask().toByte()))

    private fun key(code: Int, mask: Int) =
        send(byteArrayOf(MSG_KEY.toByte(), (code shr 8).toByte(), code.toByte(), mask.toByte()))

    private fun setExpanded(open: Boolean) {
        strip.visibility = if (open) View.VISIBLE else View.GONE
        tab.text = if (open) "‹" else "›"
        tab.translationX = if (open) strip.width.toFloat().coerceAtLeast(dp(64).toFloat()) else 0f
    }

    /** Shown only while the picture is full screen and the host accepts input. */
    fun setAvailable(available: Boolean) {
        if (available == (container.visibility == View.VISIBLE)) return
        container.visibility = if (available) View.VISIBLE else View.GONE
        tab.visibility = if (available) View.VISIBLE else View.GONE
        if (!available) {
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
        const val MOD_CMD = 1
        const val MOD_SHIFT = 2
        const val KEY_ESCAPE = 53
        const val KEY_Z = 6
        const val DOUBLE_TAP_MS = 350L
        val TEXT = Color.parseColor("#ECEFF4")
        val ACCENT = Color.parseColor("#4FD1C5")
        val ON_ACCENT = Color.parseColor("#00201C")
    }
}
