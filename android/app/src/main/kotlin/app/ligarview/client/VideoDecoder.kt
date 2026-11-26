package app.ligarview.client

import android.media.MediaCodec
import android.media.MediaFormat
import android.view.Surface
import java.util.concurrent.ConcurrentHashMap
import kotlin.concurrent.thread

/** Hardware H.264 decoder rendering straight to a [Surface], tuned for low latency. */
class VideoDecoder(surface: Surface, width: Int, height: Int) {
    private val codec = MediaCodec.createDecoderByType(MediaFormat.MIMETYPE_VIDEO_AVC)

    @Volatile
    private var running = true
    private val drainThread: Thread

    /** Number of frames handed to the display so far. */
    @Volatile
    var framesRendered = 0L
        private set

    /** Smoothed time a frame spends inside the decoder (queued -> ready to display), in ms. */
    @Volatile
    var decodeLatencyMs = 0f
        private set

    private val queuedAt = ConcurrentHashMap<Long, Long>()

    init {
        val format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, width, height).apply {
            setInteger(MediaFormat.KEY_LOW_LATENCY, 1)
            setInteger(MediaFormat.KEY_PRIORITY, 0) // real-time
            setInteger(MediaFormat.KEY_OPERATING_RATE, Short.MAX_VALUE.toInt())
            // Qualcomm: output frames as soon as they are decoded instead of buffering for reordering.
            setInteger("vendor.qti-ext-dec-low-latency.enable", 1)
            setInteger(MediaFormat.KEY_MAX_INPUT_SIZE, 4 * 1024 * 1024)
        }
        codec.configure(format, surface, null, 0)
        codec.start()
        drainThread = thread(name = "decoder-out") { drain() }
    }

    /** Queues one Annex-B access unit. Returns false if the decoder had no free input buffer. */
    fun feed(data: ByteArray, length: Int, ptsUs: Long): Boolean {
        val index = codec.dequeueInputBuffer(INPUT_TIMEOUT_US)
        if (index < 0) return false
        val buffer = codec.getInputBuffer(index) ?: return false
        buffer.clear()
        buffer.put(data, 0, length)
        if (queuedAt.size > 256) queuedAt.clear() // frames that never came out
        queuedAt[ptsUs] = System.nanoTime()
        codec.queueInputBuffer(index, 0, length, ptsUs, 0)
        return true
    }

    private fun drain() {
        val info = MediaCodec.BufferInfo()
        while (running) {
            try {
                val index = codec.dequeueOutputBuffer(info, OUTPUT_TIMEOUT_US)
                if (index >= 0) {
                    codec.releaseOutputBuffer(index, true)
                    framesRendered++
                    queuedAt.remove(info.presentationTimeUs)?.let {
                        val ms = (System.nanoTime() - it) / 1_000_000f
                        decodeLatencyMs = if (decodeLatencyMs == 0f) ms else decodeLatencyMs * 0.9f + ms * 0.1f
                    }
                }
            } catch (_: IllegalStateException) {
                break
            }
        }
    }

    fun release() {
        running = false
        drainThread.join(500)
        try {
            codec.stop()
        } catch (_: IllegalStateException) {
        }
        codec.release()
    }

    private companion object {
        const val INPUT_TIMEOUT_US = 100_000L
        const val OUTPUT_TIMEOUT_US = 10_000L
    }
}
