plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "app.ligarview.client"
    compileSdk {
        version = release(36)
    }

    defaultConfig {
        applicationId = "app.ligarview.client"
        minSdk = 30
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }

    // Release key: set LIGARVIEW_KEYSTORE (path), LIGARVIEW_KEYSTORE_PASSWORD, LIGARVIEW_KEY_ALIAS and
    // LIGARVIEW_KEY_PASSWORD. Without them the release build is signed with the debug key so it still installs.
    val keystore = System.getenv("LIGARVIEW_KEYSTORE")
    signingConfigs {
        if (keystore != null) {
            create("release") {
                storeFile = file(keystore)
                storePassword = System.getenv("LIGARVIEW_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("LIGARVIEW_KEY_ALIAS")
                keyPassword = System.getenv("LIGARVIEW_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            signingConfig = signingConfigs.findByName("release") ?: signingConfigs.getByName("debug")
        }
    }
    buildFeatures {
        compose = true
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
}

dependencies {
    implementation(libs.androidx.activity.compose)
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.graphics)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.material.icons.core)
}
