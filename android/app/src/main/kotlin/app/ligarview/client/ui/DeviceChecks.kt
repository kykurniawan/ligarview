package app.ligarview.client.ui

import android.app.Activity
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbManager
import android.os.PowerManager

/** Reads what the tablet reports about refresh rate, power saving and the USB cable. */
object DeviceChecks {
    fun read(activity: Activity): DeviceInfo {
        val display = activity.display
        val current = display?.mode
        val maxHz = if (display != null && current != null) {
            display.supportedModes
                .filter { it.physicalWidth == current.physicalWidth && it.physicalHeight == current.physicalHeight }
                .maxOfOrNull { it.refreshRate } ?: 0f
        } else {
            0f
        }
        val power = activity.getSystemService(PowerManager::class.java)
        // Sticky broadcast: "connected" is true while the tablet is attached to a USB host.
        val usb: Intent? = activity.registerReceiver(null, IntentFilter("android.hardware.usb.action.USB_STATE"))
        return DeviceInfo(
            panelHz = display?.refreshRate ?: 0f,
            maxHz = maxHz,
            powerSave = power?.isPowerSaveMode == true,
            ignoresBatteryOptimizations = power?.isIgnoringBatteryOptimizations(activity.packageName) != false,
            usbCableConnected = usb?.getBooleanExtra("connected", false) == true,
            usbAccessoryReady = activity.getSystemService(UsbManager::class.java)?.accessoryList?.isNotEmpty() == true,
        )
    }
}
