plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.sqyre.app"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.sqyre.app"
        minSdk = 29
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    // libsqyre_app.so comes from cargo-ndk (scripts/android/build-apk.sh).
    sourceSets["main"].jniLibs.srcDirs("src/main/jniLibs")
}
