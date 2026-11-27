package app.ligarview.client

import android.view.MotionEvent
import android.view.View
import java.nio.ByteBuffer
import kotlin.math.hypot

/**
 * One- and two-finger drags on the full-screen picture scroll the Mac, like Sidecar. Fingers never
 * click; the pen does the pointing. Until the fingers have moved past the touch slop nothing is
 * claimed, so a long press still toggles the statistics overlay. Three or more fingers are ignored.
 * Wire format: see host/src/input.rs.
 */
class FingerScroll(
    private val enabled: () -> Boolean,
    private val send: (ByteArray) -> Unit,
    private val slopPx: Float,
) {
    private var tracking = false
    private var scrolling = false
    private var startX = 0f
    private var startY = 0f
    private var lastX = 0f
    private var lastY = 0f

    fun onTouch(view: View, e: MotionEvent): Boolean {
        if (e.getToolType(0) != MotionEvent.TOOL_TYPE_FINGER || !enabled()) return false
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                tracking = true
                scrolling = false
                val (cx, cy) = centroid(e, -1)
                lastX = cx
                lastY = cy
                startX = cx
                startY = cy
                return false
            }
            MotionEvent.ACTION_POINTER_DOWN -> {
                if (e.pointerCount > 2) {
                    finish(view, e, cancel = true)
                    tracking = false
                    return false
                }
                if (!tracking) return false
                rebase(e, -1)
                return scrolling
            }
            MotionEvent.ACTION_POINTER_UP -> {
                if (!tracking) return false
                // The finger that lifts must not drag the centre along.
                rebase(e, e.actionIndex)
                return scrolling
            }
            MotionEvent.ACTION_MOVE -> {
                if (!tracking || e.pointerCount > 2) return false
                val (cx, cy) = centroid(e, -1)
                if (!scrolling) {
                    if (hypot(cx - startX, cy - startY) < slopPx) return false
                    scrolling = true
                    view.cancelLongPress()
                    emit(PH_BEGIN, view, cx, cy, 0f, 0f)
                    // The content must not lag behind by the slop the fingers already travelled.
                    emit(PH_CHANGE, view, cx, cy, cx - startX, cy - startY)
                    lastX = cx
                    lastY = cy
                    return true
                }
                emit(PH_CHANGE, view, cx, cy, cx - lastX, cy - lastY)
                lastX = cx
                lastY = cy
                return true
            }
            MotionEvent.ACTION_UP -> return finish(view, e, cancel = false)
            MotionEvent.ACTION_CANCEL -> return finish(view, e, cancel = true)
        }
        return scrolling
    }

    private fun finish(view: View, e: MotionEvent, cancel: Boolean): Boolean {
        val was = scrolling
        if (was) emit(if (cancel) PH_CANCEL else PH_END, view, lastX, lastY, 0f, 0f)
        tracking = false
        scrolling = false
        return was
    }

    private fun rebase(e: MotionEvent, skip: Int) {
        val (cx, cy) = centroid(e, skip)
        lastX = cx
        lastY = cy
        if (!scrolling) {
            startX = lastX
            startY = lastY
        }
    }

    /** Average position of the fingers, leaving out index [skip]. */
    private fun centroid(e: MotionEvent, skip: Int): Pair<Float, Float> {
        var x = 0f
        var y = 0f
        var n = 0
        for (i in 0 until e.pointerCount) {
            if (i == skip) continue
            x += e.getX(i)
            y += e.getY(i)
            n++
        }
        if (n == 0) return lastX to lastY
        return (x / n) to (y / n)
    }

    private fun emit(phase: Int, view: View, x: Float, y: Float, dx: Float, dy: Float) {
        if (view.width <= 0 || view.height <= 0) return
        val body = ByteBuffer.allocate(18)
            .put(MSG_SCROLL.toByte())
            .put(phase.toByte())
            .putFloat((x / view.width).coerceIn(0f, 1f))
            .putFloat((y / view.height).coerceIn(0f, 1f))
            .putFloat(dx / view.width)
            .putFloat(dy / view.height)
            .array()
        send(body)
    }

    private companion object {
        const val MSG_SCROLL = 2
        const val PH_BEGIN = 0
        const val PH_CHANGE = 1
        const val PH_END = 2
        const val PH_CANCEL = 3
    }
}
