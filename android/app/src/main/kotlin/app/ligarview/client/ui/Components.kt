package app.ligarview.client.ui

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

@Composable
fun SectionTitle(text: String, trailing: String? = null, trailingColor: Color = Palette.TextDim) {
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.Bottom) {
        Text(text, style = MaterialTheme.typography.titleLarge, color = Palette.Text, modifier = Modifier.weight(1f))
        if (trailing != null) {
            Text(trailing, style = MaterialTheme.typography.bodySmall, color = trailingColor)
        }
    }
}

@Composable
fun Hairline() {
    HorizontalDivider(thickness = 1.dp, color = Palette.Divider)
}

/** A small status light; it breathes while something is in progress. */
@Composable
fun StatusMark(color: Color, pulsing: Boolean = false, size: Dp = 10.dp) {
    val transition = rememberInfiniteTransition(label = "mark")
    val alpha by transition.animateFloat(
        initialValue = 1f,
        targetValue = if (pulsing) 0.3f else 1f,
        animationSpec = infiniteRepeatable(tween(900), RepeatMode.Reverse),
        label = "markAlpha",
    )
    val animated by animateColorAsState(color, label = "markColor")
    Box(Modifier.size(size).alpha(alpha).background(animated, CircleShape))
}

/** Plain text tabs: the selected one is brighter and underlined. */
@Composable
fun TextTabs(labels: List<String>, selected: Int, onSelect: (Int) -> Unit) {
    Row(horizontalArrangement = Arrangement.spacedBy(28.dp)) {
        labels.forEachIndexed { index, label ->
            val active = index == selected
            val color by animateColorAsState(if (active) Palette.Text else Palette.TextDim, label = "tab")
            // Each tab is as wide as its label, so the underline matches the text.
            Column(
                Modifier.width(IntrinsicSize.Max).clickable(
                    interactionSource = remember { MutableInteractionSource() },
                    indication = null,
                ) { onSelect(index) },
            ) {
                Text(label, style = MaterialTheme.typography.titleMedium, color = color)
                Spacer(Modifier.height(6.dp))
                Box(
                    Modifier
                        .height(2.dp)
                        .fillMaxWidth()
                        .background(if (active) Palette.Accent else Color.Transparent),
                )
            }
        }
    }
}

@Composable
fun Step(number: Int, title: String, body: String, trailing: (@Composable () -> Unit)? = null) {
    Row(horizontalArrangement = Arrangement.spacedBy(14.dp), verticalAlignment = Alignment.Top) {
        Text(
            number.toString(),
            style = MaterialTheme.typography.titleMedium,
            color = Palette.TextFaint,
            modifier = Modifier.width(18.dp),
        )
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleMedium, color = Palette.Text)
            Text(body, style = MaterialTheme.typography.bodyMedium, color = Palette.TextDim)
        }
        trailing?.invoke()
    }
}

/** A filled button in the accent colour; the only strong colour on the screen. */
@Composable
fun AccentButton(text: String, onClick: () -> Unit) {
    Button(
        onClick = onClick,
        shape = MaterialTheme.shapes.medium,
        colors = ButtonDefaults.buttonColors(containerColor = Palette.Accent, contentColor = Palette.OnAccent),
        elevation = ButtonDefaults.buttonElevation(0.dp, 0.dp, 0.dp, 0.dp, 0.dp),
        contentPadding = PaddingValues(horizontal = 22.dp, vertical = 12.dp),
    ) {
        Text(text, style = MaterialTheme.typography.labelLarge)
    }
}

enum class RecKind { Ok, Warn, Info }

private fun RecKind.color() = when (this) {
    RecKind.Ok -> Palette.Success
    RecKind.Warn -> Palette.Warning
    RecKind.Info -> Palette.Info
}

/** One recommendation: a coloured dot, a title, one line of explanation and an optional link. */
@Composable
fun RecommendationRow(
    kind: RecKind,
    title: String,
    body: String,
    actionLabel: String? = null,
    onAction: (() -> Unit)? = null,
) {
    val color = kind.color()
    Row(
        Modifier.fillMaxWidth().padding(vertical = 14.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalAlignment = Alignment.Top,
    ) {
        Box(Modifier.padding(top = 7.dp)) { StatusMark(color, size = 9.dp) }
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleMedium, color = Palette.Text)
            Text(body, style = MaterialTheme.typography.bodyMedium, color = Palette.TextDim)
            if (actionLabel != null && onAction != null) {
                TextButton(onClick = onAction, contentPadding = PaddingValues(0.dp)) {
                    Text("$actionLabel  →", style = MaterialTheme.typography.labelLarge, color = color)
                }
            }
        }
    }
}
