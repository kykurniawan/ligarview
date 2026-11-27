package app.ligarview.client.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import app.ligarview.client.R
import kotlin.math.roundToInt

/**
 * The screen shown whenever the picture is not live: what the tablet is doing, how to connect it
 * to the Mac, and what to change on the tablet for the best result. It fades away once the
 * stream starts.
 */
@Composable
fun InfoOverlay(state: UiState, actions: Actions, versionName: String) {
    AnimatedVisibility(
        visible = state.phase != LinkPhase.Streaming,
        enter = fadeIn(tween(250)),
        exit = fadeOut(tween(350)),
    ) {
        InfoScreen(state, actions, versionName)
    }
}

@Composable
private fun InfoScreen(state: UiState, actions: Actions, versionName: String) {
    BoxWithConstraints(
        Modifier
            .fillMaxSize()
            .background(Brush.linearGradient(listOf(Palette.Background, Palette.BackgroundGlow))),
    ) {
        if (maxWidth >= 840.dp) {
            // Centered vertically; scrolls only when the content is taller than the screen.
            Column(
                Modifier.fillMaxSize().verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.Center,
            ) {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 56.dp, vertical = 40.dp),
                    horizontalArrangement = Arrangement.spacedBy(40.dp),
                ) {
                    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(24.dp)) {
                        ConnectColumn(state, actions)
                    }
                    Column(Modifier.weight(1.1f), verticalArrangement = Arrangement.spacedBy(16.dp)) {
                        ReadyColumn(state, actions, versionName)
                    }
                }
            }
        } else {
            Column(
                Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(24.dp),
            ) {
                ConnectColumn(state, actions)
                ReadyColumn(state, actions, versionName)
            }
        }
    }
}

// ---------------------------------------------------------------- left: status and connection

@Composable
private fun ConnectColumn(state: UiState, actions: Actions) {
    Brand()
    StatusPanel(state)
    ConnectPanel(state, actions)
}

@Composable
private fun Brand() {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
        Canvas(Modifier.size(48.dp)) {
            val stroke = Stroke(width = 4.dp.toPx())
            drawRoundRect(
                Palette.Accent,
                topLeft = Offset(2.dp.toPx(), 2.dp.toPx()),
                size = Size(size.width * 0.66f, size.height * 0.62f),
                cornerRadius = CornerRadius(8.dp.toPx()),
                style = stroke,
            )
            drawRoundRect(
                Palette.Accent,
                topLeft = Offset(size.width * 0.34f, size.height * 0.34f),
                size = Size(size.width * 0.64f, size.height * 0.62f),
                cornerRadius = CornerRadius(8.dp.toPx()),
            )
        }
        Column {
            Text(
                stringResource(R.string.app_name),
                style = MaterialTheme.typography.displaySmall,
                color = Palette.Text,
            )
            Text(
                stringResource(R.string.tagline),
                style = MaterialTheme.typography.bodyLarge,
                color = Palette.TextDim,
            )
        }
    }
}

@Composable
private fun StatusPanel(state: UiState) {
    val cable = state.device.usbCableConnected
    val title: String
    val body: String
    val color: Color
    var pulsing = true
    when (state.phase) {
        LinkPhase.Idle -> {
            pulsing = false
            if (state.device.usbAccessoryReady) {
                title = stringResource(R.string.status_mac_title)
                body = stringResource(R.string.status_mac_body)
                color = Palette.Success
            } else if (cable) {
                title = stringResource(R.string.status_cable_title)
                body = stringResource(R.string.status_cable_body)
                color = Palette.Info
            } else {
                title = stringResource(R.string.status_idle_title)
                body = stringResource(R.string.status_idle_body)
                color = Palette.TextDim
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
        LinkPhase.Streaming -> {
            pulsing = false
            title = ""
            body = ""
            color = Palette.Success
        }
    }
    Panel(Modifier.fillMaxWidth(), borderColor = color.copy(alpha = 0.35f)) {
        Row(horizontalArrangement = Arrangement.spacedBy(16.dp), verticalAlignment = Alignment.CenterVertically) {
            StatusDot(color, pulsing)
            Column(Modifier.weight(1f)) {
                Text(title, style = MaterialTheme.typography.headlineSmall, color = Palette.Text)
                Spacer(Modifier.height(4.dp))
                Text(body, style = MaterialTheme.typography.bodyLarge, color = Palette.TextDim)
            }
        }
    }
}

@Composable
private fun ConnectPanel(state: UiState, actions: Actions) {
    var tab by rememberSaveable { mutableIntStateOf(if (state.startOnWifiTab) 1 else 0) }
    // Show the tab of the connection that is actually being used.
    LaunchedEffect(state.transport) {
        when (state.transport) {
            Transport.Usb -> tab = 0
            Transport.Network -> tab = 1
            Transport.None -> Unit
        }
    }
    Panel(Modifier.fillMaxWidth()) {
        SectionTitle(stringResource(R.string.section_connect))
        Spacer(Modifier.height(16.dp))
        SegmentedTabs(
            labels = listOf(stringResource(R.string.tab_usb), stringResource(R.string.tab_wifi)),
            selected = tab,
            onSelect = { tab = it },
        )
        Spacer(Modifier.height(20.dp))
        Crossfade(tab, label = "connectTab") { selected ->
            if (selected == 0) UsbSteps(state, actions) else WifiForm(state, actions)
        }
    }
}

@Composable
private fun UsbSteps(state: UiState, actions: Actions) {
    Column(verticalArrangement = Arrangement.spacedBy(20.dp)) {
        StepRow(1, stringResource(R.string.step1_title), stringResource(R.string.step1_body))
        StepRow(2, stringResource(R.string.step2_title), stringResource(R.string.step2_body)) {
            if (state.device.usbCableConnected) Chip(stringResource(R.string.step2_detected), Palette.Success)
        }
        StepRow(3, stringResource(R.string.step3_title), stringResource(R.string.step3_body))
        if (state.phase == LinkPhase.Idle && state.device.usbAccessoryReady) {
            Button(
                onClick = actions.connectUsb,
                shape = MaterialTheme.shapes.medium,
                colors = ButtonDefaults.buttonColors(containerColor = Palette.Accent, contentColor = Palette.OnAccent),
                contentPadding = PaddingValues(horizontal = 28.dp, vertical = 14.dp),
            ) {
                Text(stringResource(R.string.usb_connect_now), style = MaterialTheme.typography.labelLarge)
            }
        }
        Text(
            stringResource(R.string.usb_note),
            style = MaterialTheme.typography.bodySmall,
            color = Palette.TextDim,
        )
    }
}

@Composable
private fun WifiForm(state: UiState, actions: Actions) {
    Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
        OutlinedTextField(
            value = state.hostText,
            onValueChange = { state.hostText = it },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true,
            label = { Text(stringResource(R.string.wifi_field_label)) },
            placeholder = { Text(stringResource(R.string.wifi_field_hint)) },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = ImeAction.Go),
            keyboardActions = KeyboardActions(onGo = { actions.connect() }),
            shape = MaterialTheme.shapes.medium,
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = Palette.Accent,
                unfocusedBorderColor = Palette.Outline,
                focusedLabelColor = Palette.Accent,
                unfocusedLabelColor = Palette.TextDim,
                cursorColor = Palette.Accent,
                focusedTextColor = Palette.Text,
                unfocusedTextColor = Palette.Text,
                focusedPlaceholderColor = Palette.TextDim.copy(alpha = 0.6f),
                unfocusedPlaceholderColor = Palette.TextDim.copy(alpha = 0.6f),
            ),
        )
        Button(
            onClick = actions.connect,
            shape = MaterialTheme.shapes.medium,
            colors = ButtonDefaults.buttonColors(containerColor = Palette.Accent, contentColor = Palette.OnAccent),
            contentPadding = PaddingValues(horizontal = 28.dp, vertical = 14.dp),
        ) {
            Text(stringResource(R.string.wifi_connect), style = MaterialTheme.typography.labelLarge)
        }
        Text(stringResource(R.string.wifi_help), style = MaterialTheme.typography.bodyMedium, color = Palette.TextDim)
        Text(stringResource(R.string.wifi_dev), style = MaterialTheme.typography.bodySmall, color = Palette.TextDim)
    }
}

// ---------------------------------------------------------------- right: recommendations

@Composable
private fun ReadyColumn(state: UiState, actions: Actions, versionName: String) {
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

    Row(verticalAlignment = Alignment.Bottom, horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
        SectionTitle(stringResource(R.string.section_ready))
        Text(
            if (done == checks.size) {
                stringResource(R.string.ready_all_set)
            } else {
                stringResource(R.string.ready_summary, done, checks.size)
            },
            style = MaterialTheme.typography.bodyMedium,
            color = if (done == checks.size) Palette.Success else Palette.TextDim,
        )
    }
    LinearProgressIndicator(
        progress = { if (checks.isEmpty()) 1f else done / checks.size.toFloat() },
        modifier = Modifier.fillMaxWidth().height(6.dp).clip(CircleShape),
        color = if (done == checks.size) Palette.Success else Palette.Accent,
        trackColor = Palette.SurfaceHigh,
        drawStopIndicator = {},
    )

    val panel = device.panelHz.roundToInt()
    val max = device.maxHz.roundToInt()
    when {
        hasFastPanel && refreshOk -> RecommendationCard(
            RecKind.Ok,
            stringResource(R.string.rec_refresh_ok_title),
            stringResource(R.string.rec_refresh_ok_body, panel),
        )
        hasFastPanel -> RecommendationCard(
            RecKind.Warn,
            stringResource(R.string.rec_refresh_warn_title),
            stringResource(R.string.rec_refresh_warn_body, panel, max),
            stringResource(R.string.action_open_display),
            actions.openDisplaySettings,
        )
        else -> RecommendationCard(
            RecKind.Info,
            stringResource(R.string.rec_refresh_info_title),
            stringResource(R.string.rec_refresh_info_body, panel),
        )
    }

    if (powerOk) {
        RecommendationCard(
            RecKind.Ok,
            stringResource(R.string.rec_power_ok_title),
            stringResource(R.string.rec_power_ok_body),
        )
    } else {
        RecommendationCard(
            RecKind.Warn,
            stringResource(R.string.rec_power_warn_title),
            stringResource(R.string.rec_power_warn_body),
            stringResource(R.string.action_open_battery),
            actions.openBatterySaver,
        )
    }

    if (backgroundOk) {
        RecommendationCard(
            RecKind.Ok,
            stringResource(R.string.rec_bg_ok_title),
            stringResource(R.string.rec_bg_ok_body),
        )
    } else {
        RecommendationCard(
            RecKind.Warn,
            stringResource(R.string.rec_bg_warn_title),
            stringResource(R.string.rec_bg_warn_body),
            stringResource(R.string.action_open_bg),
            actions.openBatteryOptimization,
        )
    }

    RecommendationCard(
        RecKind.Info,
        stringResource(R.string.rec_cable_title),
        stringResource(R.string.rec_cable_body),
    )

    Spacer(Modifier.height(4.dp))
    Text(stringResource(R.string.footer_tip), style = MaterialTheme.typography.bodySmall, color = Palette.TextDim)
    Text(
        stringResource(R.string.footer_version, versionName),
        style = MaterialTheme.typography.bodySmall,
        color = Palette.TextDim.copy(alpha = 0.7f),
    )
}
