package app.ligarview.client

import android.os.SystemClock
import android.view.MotionEvent
import android.view.VelocityTracker
import android.view.View
import java.nio.ByteBuffer
import kotlin.math.abs
import kotlin.math.exp
import kotlin.math.hypot

/**
 * Finger gestures on the full-screen picture, like Sidecar's. Fingers never click; the pen does
 * the pointing.
 *
 * - One or two fingers dragging scroll the Mac, and a flick keeps gliding after the fingers lift.
 * - Two fingers moving apart or together zoom (the host sends Ctrl + scroll, which Chromium-based
 *   apps, Figma, Excalidraw, maps and most browsers read as zoom).
 *
 * The first touch is claimed so that the picture keeps receiving the finger's movement; nothing is
 * sent until the fingers have moved past the touch slop. Three or more fingers are ignored. Wire
 * format: see host/src/input.rs.
 */
class FingerScroll(
    private val enabled: () -> Boolean,
    private val send: (ByteArray) -> Unit,
    private val slopPx: Float,
) {
    private var tracking = false
    private var scrolling = false
    private var pinching = false
    private var startX = 0f
    private var startY = 0f
    private var lastX = 0f
    private var lastY = 0f
    private var startSpan = 0f
    private var lastSpan = 0f
    private var velocity: VelocityTracker? = null

    // Fling state: the glide continues on its own after the fingers lift.
    private var flingView: View? = null
    private var flingVx = 0f
    private var flingVy = 0f
    private var flingTime = 0L
    private var flingRunning = false
    private val flingStep = object : Runnable {
        override fun run() {
            val view = flingView
            if (!flingRunning || view == null || !enabled()) {
                stopFling()
                return
            }
            val now = SystemClock.uptimeMillis()
            val dt = ((now - flingTime) / 1000f).coerceIn(0f, 0.05f)
            flingTime = now
            val decay = exp(-FLING_FRICTION * dt)
            flingVx *= decay
            flingVy *= decay
            if (hypot(flingVx, flingVy) < FLING_STOP_PX_S) {
                stopFling()
                return
            }
            emit(PH_CHANGE, view, lastX, lastY, flingVx * dt, flingVy * dt)
            view.postOnAnimation(this)
        }
    }

    fun onTouch(view: View, e: MotionEvent): Boolean {
        if (e.getToolType(0) != MotionEvent.TOOL_TYPE_FINGER || !enabled()) return false
        // A finger touching the screen stops a glide in progress.
        if (e.actionMasked == MotionEvent.ACTION_DOWN) stopFling()
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                tracking = true
                scrolling = false
                pinching = false
                velocity?.recycle()
                velocity = VelocityTracker.obtain().also { it.addMovement(e) }
                val (cx, cy) = centroid(e, -1)
                lastX = cx
                lastY = cy
                startX = cx
                startY = cy
                return true
            }
            MotionEvent.ACTION_POINTER_DOWN -> {
                if (e.pointerCount > 2) {
                    finish(view, cancel = true)
                    tracking = false
                    return false
                }
                if (!tracking) return false
                rebase(e, -1)
                startSpan = span(e)
                lastSpan = startSpan
                return scrolling || pinching
            }
            MotionEvent.ACTION_POINTER_UP -> {
                if (!tracking) return false
                if (pinching) {
                    // Lifting one finger ends the zoom; the other must not start a scroll.
                    emit(PH_END, view, lastX, lastY, 0f, 0f, pinch = true)
                    pinching = false
                    tracking = false
                    return true
                }
                // The finger that lifts must not drag the centre along.
                rebase(e, e.actionIndex)
                velocity?.clear()
                return scrolling
            }
            MotionEvent.ACTION_MOVE -> {
                if (!tracking || e.pointerCount > 2) return false
                velocity?.addMovement(e)
                val (cx, cy) = centroid(e, -1)
                if (pinching) {
                    val s = span(e)
                    emit(PH_CHANGE, view, cx, cy, s - lastSpan, 0f, pinch = true)
                    lastSpan = s
                    lastX = cx
                    lastY = cy
                    return true
                }
                if (!scrolling) {
                    val travel = hypot(cx - startX, cy - startY)
                    val spread = if (e.pointerCount == 2) abs(span(e) - startSpan) else 0f
                    if (spread >= slopPx && spread > travel) {
                        // Two fingers moving apart or together: a zoom, not a scroll.
                        pinching = true
                        val s = span(e)
                        emit(PH_BEGIN, view, cx, cy, 0f, 0f, pinch = true)
                        emit(PH_CHANGE, view, cx, cy, s - startSpan, 0f, pinch = true)
                        lastSpan = s
                        lastX = cx
                        lastY = cy
                        return true
                    }
                    if (travel < slopPx) return false
                    scrolling = true
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
            MotionEvent.ACTION_UP -> {
                velocity?.addMovement(e)
                if (scrolling && startFling(view)) {
                    // The scroll ends when the glide does.
                    tracking = false
                    scrolling = false
                    return true
                }
                return finish(view, cancel = false)
            }
            MotionEvent.ACTION_CANCEL -> return finish(view, cancel = true)
        }
        return scrolling || pinching
    }

    /** Starts the glide when the fingers lifted while still moving fast. Returns true if it started. */
    private fun startFling(view: View): Boolean {
        val tracker = velocity ?: return false
        tracker.computeCurrentVelocity(1000, MAX_FLING_PX_S)
        // VelocityTracker follows the first finger; for two fingers that is close enough.
        val vx = tracker.xVelocity
        val vy = tracker.yVelocity
        if (hypot(vx, vy) < FLING_START_PX_S) return false
        flingView = view
        flingVx = vx
        flingVy = vy
        flingTime = SystemClock.uptimeMillis()
        flingRunning = true
        view.postOnAnimation(flingStep)
        return true
    }

    /** Ends the glide, and with it the scroll the Mac still believes is going on. */
    private fun stopFling() {
        if (!flingRunning) return
        flingRunning = false
        flingView?.let { emit(PH_END, it, lastX, lastY, 0f, 0f) }
        flingView = null
    }

    private fun finish(view: View, cancel: Boolean): Boolean {
        val was = scrolling || pinching
        if (scrolling) emit(if (cancel) PH_CANCEL else PH_END, view, lastX, lastY, 0f, 0f)
        if (pinching) emit(if (cancel) PH_CANCEL else PH_END, view, lastX, lastY, 0f, 0f, pinch = true)
        tracking = false
        scrolling = false
        pinching = false
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

    /** Distance between the first two fingers. */
    private fun span(e: MotionEvent): Float =
        if (e.pointerCount < 2) 0f else hypot(e.getX(0) - e.getX(1), e.getY(0) - e.getY(1))

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

    /**
     * Scroll: (dx, dy) is the movement in pixels. Pinch: dx is the change of finger distance in
     * pixels, sent as a fraction of the view's diagonal.
     */
    private fun emit(phase: Int, view: View, x: Float, y: Float, dx: Float, dy: Float, pinch: Boolean = false) {
        if (view.width <= 0 || view.height <= 0) return
        val px = (x / view.width).coerceIn(0f, 1f)
        val py = (y / view.height).coerceIn(0f, 1f)
        val body = if (pinch) {
            ByteBuffer.allocate(14)
                .put(MSG_PINCH.toByte())
                .put(phase.toByte())
                .putFloat(px)
                .putFloat(py)
                .putFloat(dx / hypot(view.width.toFloat(), view.height.toFloat()))
                .array()
        } else {
            ByteBuffer.allocate(18)
                .put(MSG_SCROLL.toByte())
                .put(phase.toByte())
                .putFloat(px)
                .putFloat(py)
                .putFloat(dx / view.width)
                .putFloat(dy / view.height)
                .array()
        }
        send(body)
    }

    private companion object {
        const val MSG_SCROLL = 2
        const val MSG_PINCH = 5
        const val PH_BEGIN = 0
        const val PH_CHANGE = 1
        const val PH_END = 2
        const val PH_CANCEL = 3

        // The glide slows down exponentially; it travels about velocity / friction in total.
        const val FLING_FRICTION = 3.2f
        const val FLING_START_PX_S = 500f
        const val FLING_STOP_PX_S = 60f
        const val MAX_FLING_PX_S = 9000f
    }
}
