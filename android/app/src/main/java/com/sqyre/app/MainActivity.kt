package com.sqyre.app

import android.Manifest
import android.app.NativeActivity
import android.content.Intent
import android.content.pm.PackageManager
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.Bundle

/** Hosts the egui UI (Rust `android_main`) and owns the screen-recording consent flow. */
class MainActivity : NativeActivity() {
    private var projectionPending = false

    override fun onCreate(savedInstanceState: Bundle?) {
        SqyreBridge.attach(this)
        super.onCreate(savedInstanceState)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), REQUEST_NOTIFICATIONS)
        }
    }

    override fun onDestroy() {
        SqyreBridge.detach(this)
        super.onDestroy()
    }

    /** Show the system consent dialog unless one is already up or recording is running. */
    fun requestProjection() {
        if (projectionPending || ProjectionService.running) return
        projectionPending = true
        val manager = getSystemService(MediaProjectionManager::class.java)
        @Suppress("DEPRECATION")
        startActivityForResult(manager.createScreenCaptureIntent(), REQUEST_PROJECTION)
    }

    @Deprecated("NativeActivity has no ActivityResult API")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        if (requestCode != REQUEST_PROJECTION) {
            @Suppress("DEPRECATION")
            super.onActivityResult(requestCode, resultCode, data)
            return
        }
        projectionPending = false
        if (resultCode == RESULT_OK && data != null) {
            ProjectionService.start(this, resultCode, data)
        } else {
            SqyreBridge.nativeOnProjectionStopped()
        }
    }

    private companion object {
        const val REQUEST_PROJECTION = 1
        const val REQUEST_NOTIFICATIONS = 2
    }
}
