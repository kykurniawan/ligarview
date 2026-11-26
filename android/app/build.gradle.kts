plugins {
    id("com.android.application")
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

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
}
