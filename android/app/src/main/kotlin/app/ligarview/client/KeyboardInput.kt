package app.ligarview.client

import android.content.Context
import android.text.InputType
import android.view.KeyCharacterMap
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import java.nio.ByteBuffer

/**
 * Turns typing on the tablet into key messages for the Mac. A tiny invisible view takes the
 * on-screen keyboard's input (and a hardware keyboard's); the Mac decides whether it may be used.
 * Wire format: see host/src/input.rs.
 */
class KeyboardInput(
    context: Context,
    private val send: (ByteArray) -> Unit,
    /** Modifiers armed in the sidebar, applied to the next key. */
    private val sidebarMask: () -> Int,
    /** A key went out: single-use modifiers are done. */
    private val keyDone: () -> Unit,
) {
    /** The view that owns the keyboard's connection; add it to the layout, it draws nothing. */
    val view: View = object : View(context) {
        override fun onCheckIsTextEditor() = true

        override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
            // Plain, suggestion-free text: the keyboard then sends what was typed key by key.
            outAttrs.inputType = InputType.TYPE_CLASS_TEXT or
                InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD or InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
            outAttrs.imeOptions = EditorInfo.IME_FLAG_NO_EXTRACT_UI or EditorInfo.IME_FLAG_NO_FULLSCREEN or
                EditorInfo.IME_ACTION_NONE
            return Connection(this)
        }

        override fun onKeyDown(keyCode: Int, event: KeyEvent) = handleKey(event) || super.onKeyDown(keyCode, event)
        override fun onKeyUp(keyCode: Int, event: KeyEvent) = isHandled(event) || super.onKeyUp(keyCode, event)
    }.apply {
        isFocusable = true
        isFocusableInTouchMode = true
        alpha = 0f
    }

    /** What the keyboard has typed into the current word so far, so corrections replace it. */
    private var composing = ""

    private inner class Connection(target: View) : BaseInputConnection(target, false) {
        override fun setComposingText(text: CharSequence, newCursorPosition: Int): Boolean {
            replaceComposing(text.toString())
            return true
        }

        override fun commitText(text: CharSequence, newCursorPosition: Int): Boolean {
            val t = text.toString()
            if (composing.isNotEmpty()) replaceComposing(t) else typeText(t)
            composing = ""
            return true
        }

        override fun finishComposingText(): Boolean {
            composing = ""
            return true
        }

        override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
            repeat(beforeLength.coerceAtMost(MAX_REPEAT)) { pressKey(KEY_BACKSPACE) }
            repeat(afterLength.coerceAtMost(MAX_REPEAT)) { pressKey(KEY_DELETE) }
            composing = ""
            return true
        }

        override fun sendKeyEvent(event: KeyEvent): Boolean {
            if (event.action == KeyEvent.ACTION_DOWN) handleKey(event)
            return true
        }

        override fun performEditorAction(actionCode: Int): Boolean {
            pressKey(KEY_RETURN)
            return true
        }
    }

    /** A composing word grows or is corrected: type only what changed. */
    private fun replaceComposing(now: String) {
        if (now.startsWith(composing)) {
            typeText(now.substring(composing.length))
        } else {
            repeat(composing.length.coerceAtMost(MAX_REPEAT)) { pressKey(KEY_BACKSPACE) }
            typeText(now)
        }
        composing = now
    }

    /** A key from a hardware keyboard attached to the tablet. Returns true when it was consumed. */
    fun hardwareKey(event: KeyEvent): Boolean = if (event.action == KeyEvent.ACTION_DOWN) handleKey(event) else isHandled(event)

    private fun isHandled(event: KeyEvent) = mapped(event.keyCode) != null || event.unicodeChar != 0

    /** One key event, from the on-screen keyboard or a hardware one. */
    private fun handleKey(event: KeyEvent): Boolean {
        mapped(event.keyCode)?.let {
            pressKey(it, extraMask(event))
            return true
        }
        val ch = event.unicodeChar
        if (ch == 0 || event.keyCode == KeyEvent.KEYCODE_BACK) return false
        // Ctrl, Alt and Meta (Cmd) on a hardware keyboard act on the key, so use the base letter.
        val mask = extraMask(event)
        val base = if (mask != 0) event.getUnicodeChar(0) else ch
        // A dead key (accent) has no character yet; the combined one arrives with the next key.
        if (base and KeyCharacterMap.COMBINING_ACCENT != 0) return true
        withMask(mask) { typeText(String(Character.toChars(base))) }
        return true
    }

    /** Modifiers held on a hardware keyboard, in the wire format (Cmd = Meta). */
    private fun extraMask(event: KeyEvent): Int {
        var m = 0
        if (event.isMetaPressed) m = m or 1
        if (event.isCtrlPressed) m = m or 4
        if (event.isAltPressed) m = m or 8
        return m
    }

    private fun mapped(keyCode: Int): Int? = when (keyCode) {
        KeyEvent.KEYCODE_DEL -> KEY_BACKSPACE
        KeyEvent.KEYCODE_FORWARD_DEL -> KEY_DELETE
        KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> KEY_RETURN
        KeyEvent.KEYCODE_TAB -> KEY_TAB
        KeyEvent.KEYCODE_ESCAPE -> KEY_ESCAPE
        KeyEvent.KEYCODE_DPAD_LEFT -> KEY_LEFT
        KeyEvent.KEYCODE_DPAD_RIGHT -> KEY_RIGHT
        KeyEvent.KEYCODE_DPAD_DOWN -> KEY_DOWN
        KeyEvent.KEYCODE_DPAD_UP -> KEY_UP
        else -> null
    }

    /** Presses [code] with the sidebar's armed modifiers plus [extra] ones from a hardware keyboard. */
    private fun pressKey(code: Int, extra: Int = 0) {
        val mask = sidebarMask() or extra
        send(ByteBuffer.allocate(4).put(MSG_KEY.toByte()).putShort(code.toShort()).put(mask.toByte()).array())
        keyDone()
    }

    private fun typeText(text: String) {
        if (text.isEmpty()) return
        // A newline in pasted or autocorrected text is a Return; the rest travels as UTF-8 in chunks.
        val parts = text.split('\n')
        for ((i, part) in parts.withIndex()) {
            if (i > 0) pressKey(KEY_RETURN)
            var rest = part
            while (rest.isNotEmpty()) {
                var n = minOf(rest.length, MAX_CHARS)
                if (n < rest.length && Character.isHighSurrogate(rest[n - 1])) n--
                val bytes = rest.substring(0, n).toByteArray(Charsets.UTF_8)
                send(byteArrayOf(MSG_TEXT.toByte()) + bytes)
                rest = rest.substring(n)
            }
        }
        keyDone()
    }

    /** Applies [mask] around [block] (hardware shortcuts such as Ctrl+C), then restores the sidebar's. */
    private fun withMask(mask: Int, block: () -> Unit) {
        if (mask == 0) return block()
        val held = sidebarMask()
        send(byteArrayOf(MSG_MODIFIERS.toByte(), (held or mask).toByte()))
        block()
        send(byteArrayOf(MSG_MODIFIERS.toByte(), held.toByte()))
    }

    private companion object {
        const val MSG_MODIFIERS = 3
        const val MSG_KEY = 4
        const val MSG_TEXT = 6
        const val MAX_CHARS = 60
        const val MAX_REPEAT = 64
        const val KEY_ESCAPE = 53
        const val KEY_RETURN = 36
        const val KEY_TAB = 48
        const val KEY_BACKSPACE = 51
        const val KEY_DELETE = 117
        const val KEY_LEFT = 123
        const val KEY_RIGHT = 124
        const val KEY_DOWN = 125
        const val KEY_UP = 126
    }
}
