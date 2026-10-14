import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.sqyre.app"
    compileSdk = 35
    // Same NDK cargo-ndk links with; AGP needs it to strip debug info from libsqyre_app.so.
    System.getenv("ANDROID_NDK_HOME")?.let { ndk ->
        ndkPath = ndk
        ndkVersion = Properties()
            .apply { file("$ndk/source.properties").inputStream().use { load(it) } }
            .getProperty("Pkg.Revision")
    }

    // CI signs the release build (SQYRE_ANDROID_* secrets in .github/workflows/main.yml);
    // without these vars `assembleRelease` stays unsigned, as before.
    val releaseKeystore = System.getenv("ANDROID_KEYSTORE_FILE")

    defaultConfig {
        applicationId = "com.sqyre.app"
        minSdk = 29
        targetSdk = 35
        versionCode = System.getenv("RELEASE_VERSION")?.let { v ->
            v.split(".").take(3).map { it.toIntOrNull() ?: 0 }.let { parts ->
                if (parts.size == 3 && parts[0] >= 2000) {
                    parts[0] * 10000 + parts[1] * 100 + parts[2]
                } else {
                    1
                }
            }
        } ?: 1
        versionName = System.getenv("RELEASE_VERSION") ?: "0.1.0"
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    signingConfigs {
        if (releaseKeystore != null) {
            create("release") {
                storeFile = file(releaseKeystore)
                storePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("ANDROID_KEYSTORE_ALIAS")
                keyPassword = System.getenv("ANDROID_KEYSTORE_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            if (releaseKeystore != null) {
                signingConfig = signingConfigs.getByName("release")
            }
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
