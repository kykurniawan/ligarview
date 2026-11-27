package app.ligarview.client

import android.app.Activity
import android.graphics.Color
import android.hardware.usb.UsbAccessory
import android.hardware.usb.UsbManager
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.widget.TextView
import java.io.BufferedInputStream
import java.io.DataInputStream
import java.io.FileInputStream
import java.io.FileOutputStream
import java.nio.ByteBuffer
import kotlin.concurrent.thread

/**
 * Experiment: talks to the Mac host over USB Android Open Accessory (no USB debugging needed).
 *
 * Protocol: 'P' + 8 bytes   -> echoed back (round-trip latency)
 *           'B' + u32 + data -> discarded and counted (throughput)
 *           'D'              -> replies 'R' + u64 bytes counted since the last 'D'
 */
class AccessoryProbeActivity : Activity() {
    private lateinit var out: TextView
    private val lines = ArrayDeque<String>()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        out = TextView(this).apply {
            setBackgroundColor(Color.BLACK)
            setTextColor(Color.GREEN)
            textSize = 14f
            setPadding(32, 32, 32, 32)
        }
        setContentView(out)

        val usb = getSystemService(UsbManager::class.java)
        @Suppress("DEPRECATION")
        val accessory: UsbAccessory? =
            intent.getParcelableExtra(UsbManager.EXTRA_ACCESSORY) ?: usb.accessoryList?.firstOrNull()
        if (accessory == null) {
            say("No accessory attached")
            return
        }
        say("Accessory: ${accessory.manufacturer} / ${accessory.model} v${accessory.version}")
        if (!usb.hasPermission(accessory)) {
            say("No permission for the accessory")
            return
        }
        val pfd = usb.openAccessory(accessory)
        if (pfd == null) {
            say("openAccessory failed")
            return
        }
        say("Accessory opened")
        thread(name = "aoa-probe") { serve(pfd) }
    }

    private fun say(line: String) {
        android.util.Log.d("LigarviewAOA", line)
        runOnUiThread {
            lines.addLast(line)
            while (lines.size > 20) lines.removeFirst()
            out.text = lines.joinToString("\n")
        }
    }

    private fun serve(pfd: ParcelFileDescriptor) {
        // Reads must be at least one USB packet; a 16 KiB buffer is the recommended size.
        val input = DataInputStream(BufferedInputStream(FileInputStream(pfd.fileDescriptor), 16384))
        val output = FileOutputStream(pfd.fileDescriptor)
        val chunk = ByteArray(16384)
        var total = 0L
        var pings = 0
        try {
            while (true) {
                when (val type = input.readUnsignedByte()) {
                    'P'.code -> {
                        val payload = ByteArray(8).also(input::readFully)
                        output.write(ByteBuffer.allocate(9).put('P'.code.toByte()).put(payload).array())
                        pings++
                    }
                    'B'.code -> {
                        var left = input.readInt()
                        while (left > 0) {
                            val n = input.read(chunk, 0, minOf(left, chunk.size))
                            if (n < 0) return
                            left -= n
                            total += n
                        }
                    }
                    'D'.code -> {
                        say("received $total bytes, answered $pings pings")
                        output.write(ByteBuffer.allocate(9).put('R'.code.toByte()).putLong(total).array())
                        total = 0
                        pings = 0
                    }
                    else -> say("unexpected byte $type")
                }
            }
        } catch (e: Exception) {
            say("link closed: ${e.message}")
        } finally {
            pfd.close()
        }
    }
}
