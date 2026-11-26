package app.ligarview.client

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Rect
import android.graphics.RectF
import android.view.Choreographer
import android.view.View
import android.widget.ImageView

/**
 * A transparent, always-visible view that paints the cursor bitmap itself.
 *
 * An earlier version moved an `ImageView` with `translationX/Y` and toggled its visibility. On
 * the Galaxy Tab S9 that left the view reported as visible, correctly placed and drawing, yet
 * absent from the screen after a fast crossing from the Mac's screen, until it was hidden and
 * shown again. Painting directly avoids both the property-driven placement and the toggling.
 */
class CursorView(context: Context) : View(context) {
    private val paint = Paint(Paint.FILTER_BITMAP_FLAG)
    private val dst = RectF()
    private val dirty = Rect()
    private var bitmap: Bitmap? = null
    private var shown = false

    init {
        setWillNotDraw(false)
        // Never takes part in hit testing or accessibility.
        importantForAccessibility = IMPORTANT_FOR_ACCESSIBILITY_NO
    }

    /** Moves/shows/hides the cursor, invalidating only the old and new rectangles. */
    fun update(bitmap: Bitmap?, left: Float, top: Float, width: Float, height: Float, show: Boolean) {
        dirty.setEmpty()
        if (shown) dirty.union(bounds())
        this.bitmap = bitmap
        shown = show && bitmap != null
        dst.set(left, top, left + width, top + height)
        if (shown) dirty.union(bounds())
        if (!dirty.isEmpty) invalidate(dirty)
    }

    private fun bounds() = Rect(
        dst.left.toInt() - 1, dst.top.toInt() - 1, dst.right.toInt() + 2, dst.bottom.toInt() + 2,
    )

    override fun onDraw(canvas: Canvas) {
        val b = bitmap ?: return
        if (shown) canvas.drawBitmap(b, null, dst, paint)
    }
}

/**
 * Applies the Mac's cursor to a [CursorView]. The position is applied once per vsync so the
 * cursor never waits for video decoding; only the network delay remains.
 */
class CursorOverlay(private val view: CursorView, private val root: View) : Choreographer.FrameCallback {
    /** Cursor image with its size and hotspot as fractions of the display. */
    class Shape(val bitmap: Bitmap, val w: Float, val h: Float, val hotX: Float, val hotY: Float)

    @Volatile private var x = 0f
    @Volatile private var y = 0f
    @Volatile private var inside = false
    @Volatile private var pending: Shape? = null
    @Volatile var enabled = false
    @Volatile private var visible = false
    @Volatile private var frames = 0L
    @Volatile private var messages = 0L

    private var shape: Shape? = null
    private var running = false

    /** Debug aid: shows the bitmap currently in use, so a missing cursor can be told apart from a missing bitmap. */
    var mirror: ImageView? = null

    /** One-line summary for diagnosing a cursor that is not shown. */
    fun debugState(): String {
        val s = shape
        return "cursor[in=$inside shape=${s?.let { "%.3fx%.3f".format(it.w, it.h) } ?: "none"} " +
            "pos=%.3f,%.3f vis=$visible en=$enabled n=$messages fr=$frames]".format(x, y)
    }

    fun setPosition(x: Float, y: Float, inside: Boolean) {
        this.x = x
        this.y = y
        this.inside = inside
        messages++
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
        frames++
        Choreographer.getInstance().postFrameCallback(this)
        val rootW = root.width.toFloat()
        val rootH = root.height.toFloat()
        if (rootW <= 0f || rootH <= 0f) return // not laid out yet

        pending?.let {
            pending = null
            shape = it
            mirror?.setImageBitmap(it.bitmap)
        }
        val s = shape
        val show = enabled && inside && s != null
        if (show != visible) {
            android.util.Log.d("Ligarview", "cursor visible=$show ${debugState()}")
            visible = show
        }
        if (s != null) {
            view.update(
                s.bitmap,
                x * rootW - s.hotX * rootW,
                y * rootH - s.hotY * rootH,
                s.w * rootW,
                s.h * rootH,
                show,
            )
        }
    }
}
