package app.ligarview.client

import android.view.MotionEvent
import android.view.View
import java.nio.ByteBuffer

/**
 * Turns S Pen (stylus) events on the full-screen picture into pointer messages for the Mac.
 * Fingers are not handled here and fall through to the normal view behaviour. Wire format:
 * see host/src/input.rs.
 */
class PenInput(
    private val enabled: () -> Boolean,
    private val send: (ByteArray) -> Unit,
    private val onStrokeEnd: () -> Unit = {},
) {
    fun onTouch(view: View, e: MotionEvent): Boolean {
        if (!isPen(e) || !enabled()) return false
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN -> emit(ACT_DOWN, view, e, e.x, e.y, e.pressure)
            MotionEvent.ACTION_MOVE -> {
                // Samples that arrived between two events keep fast strokes smooth.
                for (i in 0 until e.historySize) {
                    emit(ACT_MOVE, view, e, e.getHistoricalX(i), e.getHistoricalY(i), e.getHistoricalPressure(i))
                }
                emit(ACT_MOVE, view, e, e.x, e.y, e.pressure)
            }
            MotionEvent.ACTION_UP -> {
                emit(ACT_UP, view, e, e.x, e.y, 0f)
                onStrokeEnd()
            }
            MotionEvent.ACTION_CANCEL -> {
                emit(ACT_CANCEL, view, e, e.x, e.y, 0f)
                onStrokeEnd()
            }
            else -> return false
        }
        return true
    }

    /** The pen hovering above the screen moves the Mac's pointer without clicking. */
    fun onHover(view: View, e: MotionEvent): Boolean {
        if (!isPen(e) || !enabled()) return false
        if (e.actionMasked != MotionEvent.ACTION_HOVER_MOVE && e.actionMasked != MotionEvent.ACTION_HOVER_ENTER) return false
        emit(ACT_HOVER, view, e, e.x, e.y, 0f)
        return true
    }

    private fun isPen(e: MotionEvent): Boolean {
        val tool = e.getToolType(0)
        return tool == MotionEvent.TOOL_TYPE_STYLUS || tool == MotionEvent.TOOL_TYPE_ERASER
    }

    private fun emit(action: Int, view: View, e: MotionEvent, x: Float, y: Float, pressure: Float) {
        if (view.width <= 0 || view.height <= 0) return
        val tool = if (e.getToolType(0) == MotionEvent.TOOL_TYPE_ERASER) TOOL_ERASER else TOOL_PEN
        val buttons = if (e.buttonState and MotionEvent.BUTTON_STYLUS_PRIMARY != 0) BTN_BARREL else 0
        val body = ByteBuffer.allocate(16)
            .put(MSG_POINTER.toByte())
            .put(action.toByte())
            .put(tool.toByte())
            .put(buttons.toByte())
            .putFloat((x / view.width).coerceIn(0f, 1f))
            .putFloat((y / view.height).coerceIn(0f, 1f))
            .putFloat(pressure.coerceIn(0f, 1f))
            .array()
        send(body)
    }

    private companion object {
        const val MSG_POINTER = 1
        const val ACT_HOVER = 0
        const val ACT_DOWN = 1
        const val ACT_MOVE = 2
        const val ACT_UP = 3
        const val ACT_CANCEL = 4
        const val TOOL_PEN = 2
        const val TOOL_ERASER = 3
        const val BTN_BARREL = 1
    }
}
