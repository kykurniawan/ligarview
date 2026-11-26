package app.ligarview.client

import android.graphics.Bitmap
import android.view.Choreographer
import android.view.View
import android.widget.ImageView

/**
 * Draws the Mac's cursor above the video. The position is applied once per vsync so the
 * cursor never waits for video decoding; only the network delay remains.
 */
class CursorOverlay(private val view: ImageView, private val root: View) : Choreographer.FrameCallback {
    /** Cursor image with its size and hotspot as fractions of the display. */
    class Shape(val bitmap: Bitmap, val w: Float, val h: Float, val hotX: Float, val hotY: Float)

    @Volatile private var x = 0f
    @Volatile private var y = 0f
    @Volatile private var inside = false
    @Volatile private var pending: Shape? = null
    @Volatile var enabled = false

    private var shape: Shape? = null
    private var running = false

    fun setPosition(x: Float, y: Float, inside: Boolean) {
        this.x = x
        this.y = y
        this.inside = inside
    }

    fun setShape(shape: Shape) {
        pending = shape
    }

    fun start() {
        if (running) return
        running = true
        Choreographer.getInstance().postFrameCallback(this)
    }

    fun stop() {
        running = false
        Choreographer.getInstance().removeFrameCallback(this)
    }

    override fun doFrame(frameTimeNanos: Long) {
        if (!running) return
        val rootW = root.width.toFloat()
        val rootH = root.height.toFloat()

        pending?.let {
            pending = null
            shape = it
            view.setImageBitmap(it.bitmap)
            view.layoutParams = view.layoutParams.apply {
                width = (it.w * rootW).toInt().coerceAtLeast(1)
                height = (it.h * rootH).toInt().coerceAtLeast(1)
            }
        }

        val s = shape
        if (enabled && inside && s != null) {
            view.visibility = View.VISIBLE
            view.translationX = x * rootW - s.hotX * rootW
            view.translationY = y * rootH - s.hotY * rootH
        } else {
            view.visibility = View.INVISIBLE
        }
        Choreographer.getInstance().postFrameCallback(this)
    }
}
