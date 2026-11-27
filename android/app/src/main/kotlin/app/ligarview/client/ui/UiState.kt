package app.ligarview.client.ui

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

enum class LinkPhase { Idle, Permission, Connecting, Connected, Streaming, Retrying }

enum class Transport { None, Usb, Network }

/** What the tablet itself reports, refreshed while the information screen is showing. */
data class DeviceInfo(
    val panelHz: Float = 0f,
    val maxHz: Float = 0f,
    val powerSave: Boolean = false,
    val ignoresBatteryOptimizations: Boolean = true,
    val usbCableConnected: Boolean = false,
    /** The Mac has switched the tablet into accessory mode: the host is running and ready. */
    val usbAccessoryReady: Boolean = false,
)

/** Everything the Compose screens read. The activity writes it, the UI only observes it. */
@Stable
class UiState(initialHost: String, val startOnWifiTab: Boolean) {
    var phase by mutableStateOf(LinkPhase.Idle)
    var transport by mutableStateOf(Transport.None)

    /** True once a stream was live in this session; tells "waiting for the Mac" from "connection lost". */
    var everConnected by mutableStateOf(false)

    /** True while the picture fills the screen; otherwise it is the small preview on the information screen. */
    var fullscreen by mutableStateOf(false)

    /** Resolution, frame rate and codec of the stream, once connected. */
    var streamInfo by mutableStateOf("")

    /** One line of live numbers (fps, bitrate, decode time) while streaming. */
    var liveStats by mutableStateOf("")

    /** Stream description (Connected) or the reason of the last failure (Retrying). */
    var detail by mutableStateOf("")
    var hostText by mutableStateOf(initialHost)
    var device by mutableStateOf(DeviceInfo())
}

class Actions(
    val connect: () -> Unit,
    val connectUsb: () -> Unit,
    val disconnect: () -> Unit,
    val openFullscreen: () -> Unit,
    /** Reports where the preview box is (pixels, in the root view), so the video can be placed behind it. */
    val onPreviewBounds: (left: Float, top: Float, width: Float, height: Float) -> Unit,
    val openDisplaySettings: () -> Unit,
    val openBatterySaver: () -> Unit,
    val openBatteryOptimization: () -> Unit,
)
