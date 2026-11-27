package app.ligarview.client.ui

import androidx.compose.foundation.layout.size
import androidx.compose.ui.res.painterResource
import androidx.compose.foundation.Image
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import app.ligarview.client.R
import kotlin.math.roundToInt

/**
 * The screen the app starts on and returns to: what the tablet is doing, a small live preview of
 * the Mac, how to connect, and what to change on the tablet. It sits on the app's dark background;
 * the video is a separate view that the activity places behind the preview box.
 */
@Composable
fun InfoOverlay(state: UiState, actions: Actions, versionName: String) {
    AnimatedVisibility(
        visible = !state.fullscreen,
        enter = fadeIn(tween(220)),
        exit = fadeOut(tween(160)),
    ) {
        InfoScreen(state, actions, versionName)
    }
}

@Composable
private fun InfoScreen(state: UiState, actions: Actions, versionName: String) {
    BoxWithConstraints(Modifier.fillMaxSize()) {
        // Two columns need room on both axes; a tablet held upright gets one column.
        val twoColumns = maxWidth >= 840.dp && maxWidth > maxHeight
        if (twoColumns) {
            // Centered vertically; scrolls only when the content is taller than the screen.
            Column(
                Modifier.fillMaxSize().verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.Center,
            ) {
                Row(
                    Modifier.fillMaxWidth().padding(horizontal = 64.dp, vertical = 40.dp),
                    horizontalArrangement = Arrangement.spacedBy(72.dp),
                ) {
                    Column(Modifier.weight(1f)) { LeftColumn(state, actions) }
                    Column(Modifier.weight(1f)) { RightColumn(state, actions, versionName) }
                }
            }
        } else {
            val padding = if (maxWidth >= 600.dp) 40.dp else 24.dp
            Column(
                Modifier.fillMaxSize().verticalScroll(rememberScrollState()),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                // One readable column, centered, however wide the tablet is.
                Column(Modifier.widthIn(max = 720.dp).fillMaxWidth().padding(padding)) {
                    LeftColumn(state, actions)
                    Spacer(Modifier.height(40.dp))
                    RightColumn(state, actions, versionName)
                    // Room for the system's gesture handle and bars at the bottom of an upright tablet.
                    Spacer(Modifier.height(72.dp))
                }
            }
        }
    }
}

// ------------------------------------------------------------------ left: status and preview

@Composable
private fun LeftColumn(state: UiState, actions: Actions) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
        Image(painterResource(R.drawable.logo_mark), contentDescription = null, modifier = Modifier.size(48.dp))
        Column {
            Text(stringResource(R.string.app_name), style = MaterialTheme.typography.titleLarge, color = Palette.Text)
            Text(stringResource(R.string.tagline), style = MaterialTheme.typography.bodyMedium, color = Palette.TextDim)
        }
    }
    Spacer(Modifier.height(36.dp))
    Status(state, actions)
    Spacer(Modifier.height(28.dp))
    Preview(state, actions)
}

@Composable
private fun Status(state: UiState, actions: Actions) {
    val device = state.device
    val title: String
    val body: String
    val color: Color
    var pulsing = true
    when (state.phase) {
        LinkPhase.Idle -> {
            pulsing = false
            when {
                device.usbAccessoryReady -> {
                    title = stringResource(R.string.status_mac_title)
                    body = stringResource(R.string.status_mac_body)
                    color = Palette.Success
                }
                device.usbCableConnected -> {
                    title = stringResource(R.string.status_cable_title)
                    body = stringResource(R.string.status_cable_body)
                    color = Palette.Info
                }
                else -> {
                    title = stringResource(R.string.status_idle_title)
                    body = stringResource(R.string.status_idle_body)
                    color = Palette.TextFaint
                }
            }
        }
        LinkPhase.Permission -> {
            title = stringResource(R.string.status_permission_title)
            body = stringResource(R.string.status_permission_body)
            color = Palette.Warning
        }
        LinkPhase.Connecting -> {
            title = stringResource(R.string.status_connecting_title)
            body = if (state.transport == Transport.Usb) {
                stringResource(R.string.status_connecting_usb)
            } else {
                stringResource(R.string.status_connecting_net, state.hostText)
            }
            color = Palette.Warning
        }
        LinkPhase.Connected -> {
            title = stringResource(R.string.status_connected_title)
            body = stringResource(R.string.status_connected_body, state.detail)
            color = Palette.Accent
        }
        LinkPhase.Streaming -> {
            pulsing = false
            title = stringResource(R.string.status_streaming_title)
            val transport = stringResource(
                if (state.transport == Transport.Usb) R.string.transport_usb else R.string.transport_wifi,
            )
            body = stringResource(R.string.status_streaming_body, transport, state.streamInfo)
            color = Palette.Success
        }
        LinkPhase.Retrying -> {
            if (state.everConnected) {
                title = stringResource(R.string.status_retry_title)
                body = stringResource(R.string.status_retry_body, state.detail)
            } else {
                title = stringResource(R.string.status_wait_title)
                body = stringResource(R.string.status_wait_body)
            }
            color = Palette.Warning
        }
    }
    Column {
        Row(horizontalArrangement = Arrangement.spacedBy(14.dp), verticalAlignment = Alignment.CenterVertically) {
            StatusMark(color, pulsing, size = 12.dp)
            Text(title, style = MaterialTheme.typography.headlineLarge, color = Palette.Text)
        }
        Spacer(Modifier.height(6.dp))
        Text(body, style = MaterialTheme.typography.bodyLarge, color = Palette.TextDim)
        if (state.phase == LinkPhase.Streaming && state.liveStats.isNotEmpty()) {
            Text(state.liveStats, style = MaterialTheme.typography.bodySmall, color = Palette.TextFaint)
        }
        if (state.phase != LinkPhase.Idle) {
            TextButton(onClick = actions.disconnect, contentPadding = PaddingValues(0.dp)) {
                Text(
                    stringResource(R.string.action_disconnect),
                    style = MaterialTheme.typography.labelLarge,
                    color = Palette.TextDim,
                )
            }
        }
    }
}

/**
 * The preview box is transparent while streaming: the video view sits behind it and the activity
 * keeps it exactly on these bounds. Tapping it opens the full screen.
 */
private val PORTRAIT_PREVIEW_HEIGHT = 440.dp

@Composable
private fun Preview(state: UiState, actions: Actions) {
    val live = state.phase == LinkPhase.Streaming
    Column {
        // A wide picture fills the column; an upright one is kept to a sensible height.
        val shape = if (state.streamAspect >= 1f) {
            Modifier.fillMaxWidth().aspectRatio(state.streamAspect)
        } else {
            Modifier.height(PORTRAIT_PREVIEW_HEIGHT).aspectRatio(state.streamAspect)
        }
        Box(
            shape
                .clip(RoundedCornerShape(12.dp))
                .background(if (live) Color.Transparent else Palette.Surface)
                .onGloballyPositioned { coordinates ->
                    val r = coordinates.boundsInRoot()
                    actions.onPreviewBounds(r.left, r.top, r.width, r.height)
                }
                .clickable(
                    interactionSource = remember { MutableInteractionSource() },
                    indication = null,
                    enabled = live,
                    onClick = actions.openFullscreen,
                ),
            contentAlignment = Alignment.Center,
        ) {
            if (!live) {
                Text(
                    stringResource(R.string.preview_placeholder),
                    style = MaterialTheme.typography.bodyMedium,
                    color = Palette.TextFaint,
                )
            }
        }
        Spacer(Modifier.height(8.dp))
        Text(
            stringResource(R.string.preview_hint),
            style = MaterialTheme.typography.bodySmall,
            color = Palette.TextFaint,
            modifier = Modifier.alpha(if (live) 1f else 0f),
        )
    }
}

// ------------------------------------------------------------------ right: connect and recommendations

@Composable
private fun RightColumn(state: UiState, actions: Actions, versionName: String) {
    ConnectSection(state, actions)
    Spacer(Modifier.height(40.dp))
    ReadySection(state, actions)
    Spacer(Modifier.height(28.dp))
    Text(stringResource(R.string.footer_tip), style = MaterialTheme.typography.bodySmall, color = Palette.TextFaint)
    Text(
        stringResource(R.string.footer_version, versionName),
        style = MaterialTheme.typography.bodySmall,
        color = Palette.TextFaint,
    )
}

@Composable
private fun ConnectSection(state: UiState, actions: Actions) {
    var tab by rememberSaveable { mutableIntStateOf(if (state.startOnWifiTab) 1 else 0) }
    // Show the tab of the connection that is actually being used.
    LaunchedEffect(state.transport) {
        when (state.transport) {
            Transport.Usb -> tab = 0
            Transport.Network -> tab = 1
            Transport.None -> Unit
        }
    }
    SectionTitle(stringResource(R.string.section_connect))
    Spacer(Modifier.height(14.dp))
    TextTabs(
        labels = listOf(stringResource(R.string.tab_usb), stringResource(R.string.tab_wifi)),
        selected = tab,
        onSelect = { tab = it },
    )
    Spacer(Modifier.height(20.dp))
    Crossfade(tab, label = "connectTab") { selected ->
        Column {
            if (selected == 0) UsbSteps(state, actions) else WifiForm(state, actions)
        }
    }
}

@Composable
private fun UsbSteps(state: UiState, actions: Actions) {
    Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Step(1, stringResource(R.string.step1_title), stringResource(R.string.step1_body))
        Step(2, stringResource(R.string.step2_title), stringResource(R.string.step2_body)) {
            if (state.device.usbCableConnected) {
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                    StatusMark(Palette.Success, size = 8.dp)
                    Text(
                        stringResource(R.string.step2_detected),
                        style = MaterialTheme.typography.bodySmall,
                        color = Palette.Success,
                    )
                }
            }
        }
        Step(3, stringResource(R.string.step3_title), stringResource(R.string.step3_body))
        if (state.phase == LinkPhase.Idle && state.device.usbAccessoryReady) {
            AccentButton(stringResource(R.string.usb_connect_now), actions.connectUsb)
        }
        Text(stringResource(R.string.usb_note), style = MaterialTheme.typography.bodySmall, color = Palette.TextFaint)
    }
}

@Composable
private fun WifiForm(state: UiState, actions: Actions) {
    val fieldColors = TextFieldDefaults.colors(
        focusedContainerColor = Palette.Surface,
        unfocusedContainerColor = Palette.Surface,
        focusedIndicatorColor = Palette.Accent,
        unfocusedIndicatorColor = Color.Transparent,
        focusedLabelColor = Palette.Accent,
        unfocusedLabelColor = Palette.TextDim,
        cursorColor = Palette.Accent,
        focusedTextColor = Palette.Text,
        unfocusedTextColor = Palette.Text,
        focusedPlaceholderColor = Palette.TextFaint,
        unfocusedPlaceholderColor = Palette.TextFaint,
    )
    Column(verticalArrangement = Arrangement.spacedBy(14.dp)) {
        Text(stringResource(R.string.wifi_help), style = MaterialTheme.typography.bodyMedium, color = Palette.TextDim)
        Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(stringResource(R.string.wifi_code_label), style = MaterialTheme.typography.bodySmall, color = Palette.TextDim)
            Text(
                state.pairingCode.chunked(3).joinToString(" "),
                style = MaterialTheme.typography.headlineMedium,
                color = Palette.Text,
                fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace,
            )
        }
        val mac = state.foundMac
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            StatusMark(if (mac != null) Palette.Success else Palette.TextFaint, size = 8.dp)
            Text(
                if (mac != null) stringResource(R.string.wifi_found, mac) else stringResource(R.string.wifi_searching),
                style = MaterialTheme.typography.bodySmall,
                color = if (mac != null) Palette.Success else Palette.TextDim,
            )
        }
        TextField(
            value = state.hostText,
            onValueChange = { state.hostText = it },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            label = { Text(stringResource(R.string.wifi_field_label)) },
            placeholder = { Text(stringResource(R.string.wifi_field_hint)) },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = ImeAction.Go),
            keyboardActions = KeyboardActions(onGo = { actions.connect() }),
            shape = MaterialTheme.shapes.medium,
            colors = fieldColors,
        )
        Text(stringResource(R.string.wifi_usb_note), style = MaterialTheme.typography.bodySmall, color = Palette.TextFaint)
    }
}

@Composable
private fun ReadySection(state: UiState, actions: Actions) {
    val device = state.device
    val hasFastPanel = device.maxHz >= 90f
    val refreshOk = hasFastPanel && device.panelHz >= device.maxHz - 1f
    val powerOk = !device.powerSave
    val backgroundOk = device.ignoresBatteryOptimizations

    // Only the settings the user can act on count towards "done".
    val checks = buildList {
        if (hasFastPanel) add(refreshOk)
        add(powerOk)
        add(backgroundOk)
    }
    val done = checks.count { it }
    val allDone = done == checks.size
    SectionTitle(
        stringResource(R.string.section_ready),
        trailing = if (allDone) stringResource(R.string.ready_all_set) else stringResource(R.string.ready_summary, done, checks.size),
        trailingColor = if (allDone) Palette.Success else Palette.TextDim,
    )
    Spacer(Modifier.height(6.dp))

    val panel = device.panelHz.roundToInt()
    val max = device.maxHz.roundToInt()
    Hairline()
    when {
        hasFastPanel && refreshOk -> RecommendationRow(
            RecKind.Ok,
            stringResource(R.string.rec_refresh_ok_title),
            stringResource(R.string.rec_refresh_ok_body, panel),
        )
        hasFastPanel -> RecommendationRow(
            RecKind.Warn,
            stringResource(R.string.rec_refresh_warn_title),
            stringResource(R.string.rec_refresh_warn_body, panel, max),
            stringResource(R.string.action_open_display),
            actions.openDisplaySettings,
        )
        else -> RecommendationRow(
            RecKind.Info,
            stringResource(R.string.rec_refresh_info_title),
            stringResource(R.string.rec_refresh_info_body, panel),
        )
    }
    Hairline()
    if (powerOk) {
        RecommendationRow(
            RecKind.Ok,
            stringResource(R.string.rec_power_ok_title),
            stringResource(R.string.rec_power_ok_body),
        )
    } else {
        RecommendationRow(
            RecKind.Warn,
            stringResource(R.string.rec_power_warn_title),
            stringResource(R.string.rec_power_warn_body),
            stringResource(R.string.action_open_battery),
            actions.openBatterySaver,
        )
    }
    Hairline()
    if (backgroundOk) {
        RecommendationRow(
            RecKind.Ok,
            stringResource(R.string.rec_bg_ok_title),
            stringResource(R.string.rec_bg_ok_body),
        )
    } else {
        RecommendationRow(
            RecKind.Warn,
            stringResource(R.string.rec_bg_warn_title),
            stringResource(R.string.rec_bg_warn_body),
            stringResource(R.string.action_open_bg),
            actions.openBatteryOptimization,
        )
    }
    Hairline()
    RecommendationRow(
        RecKind.Info,
        stringResource(R.string.rec_cable_title),
        stringResource(R.string.rec_cable_body),
    )
}
