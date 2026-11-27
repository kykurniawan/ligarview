package app.ligarview.client.ui

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** Flat palette: one near-black, a few greys for hierarchy, one accent used sparingly. */
object Palette {
    val Background = Color(0xFF0E1013)
    val Surface = Color(0xFF14181D)
    val Divider = Color(0xFF1D232A)
    val Text = Color(0xFFECEFF4)
    val TextDim = Color(0xFF8C95A3)
    val TextFaint = Color(0xFF5E6773)
    val Accent = Color(0xFF4FD1C5)
    val OnAccent = Color(0xFF00201C)
    val Success = Color(0xFF4ADE80)
    val Warning = Color(0xFFF5B73B)
    val Info = Color(0xFF7AA2F7)
}

private val Sans = FontFamily.SansSerif

private val LigarTypography = Typography(
    headlineLarge = TextStyle(fontFamily = Sans, fontWeight = FontWeight.SemiBold, fontSize = 34.sp, lineHeight = 40.sp, letterSpacing = (-0.4).sp),
    titleLarge = TextStyle(fontFamily = Sans, fontWeight = FontWeight.SemiBold, fontSize = 18.sp),
    titleMedium = TextStyle(fontFamily = Sans, fontWeight = FontWeight.Medium, fontSize = 16.sp),
    bodyLarge = TextStyle(fontFamily = Sans, fontWeight = FontWeight.Normal, fontSize = 16.sp, lineHeight = 23.sp),
    bodyMedium = TextStyle(fontFamily = Sans, fontWeight = FontWeight.Normal, fontSize = 14.sp, lineHeight = 21.sp),
    bodySmall = TextStyle(fontFamily = Sans, fontWeight = FontWeight.Normal, fontSize = 13.sp, lineHeight = 19.sp),
    labelLarge = TextStyle(fontFamily = Sans, fontWeight = FontWeight.Medium, fontSize = 14.sp),
)

private val LigarShapes = Shapes(
    small = RoundedCornerShape(8.dp),
    medium = RoundedCornerShape(10.dp),
    large = RoundedCornerShape(12.dp),
)

@Composable
fun LigarviewTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = darkColorScheme(
            primary = Palette.Accent,
            onPrimary = Palette.OnAccent,
            background = Palette.Background,
            onBackground = Palette.Text,
            surface = Palette.Surface,
            onSurface = Palette.Text,
            onSurfaceVariant = Palette.TextDim,
        ),
        typography = LigarTypography,
        shapes = LigarShapes,
        content = content,
    )
}
