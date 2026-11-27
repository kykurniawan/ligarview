package app.ligarview.client.ui

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** A bordered surface used for every block of the screen. */
@Composable
fun Panel(modifier: Modifier = Modifier, borderColor: Color = Palette.Outline, content: @Composable ColumnScope.() -> Unit) {
    Surface(
        modifier = modifier,
        shape = MaterialTheme.shapes.large,
        color = Palette.Surface,
        border = BorderStroke(1.dp, borderColor),
    ) {
        Column(Modifier.padding(24.dp), content = content)
    }
}

@Composable
fun SectionTitle(text: String) {
    Text(text, style = MaterialTheme.typography.titleLarge, color = Palette.Text)
}

/** A status light; it breathes while something is in progress. */
@Composable
fun StatusDot(color: Color, pulsing: Boolean, size: Dp = 14.dp) {
    val transition = rememberInfiniteTransition(label = "dot")
    val alpha by transition.animateFloat(
        initialValue = 1f,
        targetValue = if (pulsing) 0.3f else 1f,
        animationSpec = infiniteRepeatable(tween(900), RepeatMode.Reverse),
        label = "dotAlpha",
    )
    val animated by animateColorAsState(color, label = "dotColor")
    Box(
        Modifier
            .size(size + 10.dp)
            .alpha(alpha)
            .background(animated.copy(alpha = 0.18f), CircleShape),
        contentAlignment = Alignment.Center,
    ) {
        Box(Modifier.size(size).background(animated, CircleShape))
    }
}

@Composable
fun SegmentedTabs(labels: List<String>, selected: Int, onSelect: (Int) -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(14.dp))
            .background(Palette.SurfaceHigh)
            .padding(4.dp),
        horizontalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        labels.forEachIndexed { index, label ->
            val active = index == selected
            val background by animateColorAsState(
                if (active) Palette.Accent.copy(alpha = 0.16f) else Color.Transparent,
                label = "tabBg",
            )
            val content by animateColorAsState(if (active) Palette.Accent else Palette.TextDim, label = "tabFg")
            Box(
                Modifier
                    .weight(1f)
                    .clip(RoundedCornerShape(11.dp))
                    .background(background)
                    .clickable { onSelect(index) }
                    .padding(vertical = 12.dp),
                contentAlignment = Alignment.Center,
            ) {
                Text(label, style = MaterialTheme.typography.labelLarge, color = content)
            }
        }
    }
}

@Composable
fun StepRow(number: Int, title: String, body: String, trailing: (@Composable () -> Unit)? = null) {
    Row(horizontalArrangement = Arrangement.spacedBy(16.dp), verticalAlignment = Alignment.Top) {
        Box(
            Modifier
                .size(32.dp)
                .background(Palette.Accent.copy(alpha = 0.14f), CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            Text(number.toString(), style = MaterialTheme.typography.labelLarge, color = Palette.Accent)
        }
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleMedium, color = Palette.Text)
            Spacer(Modifier.size(2.dp))
            Text(body, style = MaterialTheme.typography.bodyMedium, color = Palette.TextDim)
        }
        trailing?.invoke()
    }
}

@Composable
fun Chip(text: String, color: Color) {
    Row(
        Modifier
            .clip(CircleShape)
            .background(color.copy(alpha = 0.14f))
            .padding(horizontal = 12.dp, vertical = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(7.dp).background(color, CircleShape))
        Spacer(Modifier.width(7.dp))
        Text(text, style = MaterialTheme.typography.labelMedium, color = color, fontWeight = FontWeight.SemiBold)
    }
}

enum class RecKind { Ok, Warn, Info }

private fun RecKind.color() = when (this) {
    RecKind.Ok -> Palette.Success
    RecKind.Warn -> Palette.Warning
    RecKind.Info -> Palette.Info
}

@Composable
fun RecommendationCard(
    kind: RecKind,
    title: String,
    body: String,
    actionLabel: String? = null,
    onAction: (() -> Unit)? = null,
) {
    val color = kind.color()
    Panel(
        Modifier.fillMaxWidth(),
        borderColor = if (kind == RecKind.Warn) color.copy(alpha = 0.45f) else Palette.Outline,
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(16.dp), verticalAlignment = Alignment.Top) {
            Box(
                Modifier.size(40.dp).background(color.copy(alpha = 0.14f), CircleShape),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    imageVector = when (kind) {
                        RecKind.Ok -> Icons.Filled.CheckCircle
                        RecKind.Warn -> Icons.Filled.Warning
                        RecKind.Info -> Icons.Filled.Info
                    },
                    contentDescription = null,
                    tint = color,
                    modifier = Modifier.size(22.dp),
                )
            }
            Column(Modifier.weight(1f)) {
                Text(title, style = MaterialTheme.typography.titleMedium, color = Palette.Text)
                Spacer(Modifier.size(4.dp))
                Text(body, style = MaterialTheme.typography.bodyMedium, color = Palette.TextDim)
                if (actionLabel != null && onAction != null) {
                    Spacer(Modifier.size(14.dp))
                    OutlinedButton(
                        onClick = onAction,
                        shape = MaterialTheme.shapes.medium,
                        border = BorderStroke(1.dp, color.copy(alpha = 0.6f)),
                        colors = ButtonDefaults.outlinedButtonColors(contentColor = color),
                        contentPadding = PaddingValues(horizontal = 18.dp, vertical = 10.dp),
                    ) {
                        Text(actionLabel, style = MaterialTheme.typography.labelLarge)
                    }
                }
            }
        }
    }
}
