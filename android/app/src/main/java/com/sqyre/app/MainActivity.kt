package com.sqyre.app

import android.Manifest
import android.app.NativeActivity
import android.content.Intent
import android.content.pm.PackageManager
import android.media.projection.MediaProjectionManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.OpenableColumns
import android.util.Log
import android.view.View
import android.view.WindowInsets
import java.io.File

/** Hosts the egui UI (Rust `android_main`) and owns the screen-recording and file-picker flows. */
class MainActivity : NativeActivity() {
    private var projectionPending = false
    private var pickId: Int? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        SqyreBridge.attach(this)
        Tessdata.install(this)
        super.onCreate(savedInstanceState)
        reportInsets()
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

    /**
     * Forward the system bar and cutout insets that overlap the native surface, so egui keeps
     * its panels out of them (API 35 draws every target-35 app edge to edge).
     */
    private fun reportInsets() {
        findViewById<View>(android.R.id.content).setOnApplyWindowInsetsListener { view, insets ->
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                val bars = insets.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout())
                SqyreBridge.nativeOnInsets(bars.left, bars.top, bars.right, bars.bottom)
            } else {
                @Suppress("DEPRECATION")
                SqyreBridge.nativeOnInsets(
                    insets.systemWindowInsetLeft,
                    insets.systemWindowInsetTop,
                    insets.systemWindowInsetRight,
                    insets.systemWindowInsetBottom,
                )
            }
            view.onApplyWindowInsets(insets)
        }
    }

    /** Show the system consent dialog unless one is already up or recording is running. */
    fun requestProjection() {
        if (projectionPending || ProjectionService.running) return
        projectionPending = true
        val manager = getSystemService(MediaProjectionManager::class.java)
        @Suppress("DEPRECATION")
        startActivityForResult(manager.createScreenCaptureIntent(), REQUEST_PROJECTION)
    }

    /** Open the document picker; a newer pick cancels the unanswered one. */
    fun pickDocument(id: Int, mimeTypes: Array<String>) {
        pickId?.let { SqyreBridge.nativeOnDocumentPicked(it, "") }
        pickId = id
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType(if (mimeTypes.size == 1) mimeTypes[0] else "*/*")
        if (mimeTypes.size > 1) intent.putExtra(Intent.EXTRA_MIME_TYPES, mimeTypes)
        try {
            @Suppress("DEPRECATION")
            startActivityForResult(intent, REQUEST_PICK_DOCUMENT)
        } catch (e: RuntimeException) {
            Log.w(TAG, "document picker unavailable", e)
            pickId = null
            SqyreBridge.nativeOnDocumentPicked(id, "")
        }
    }

    @Deprecated("NativeActivity has no ActivityResult API")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        when (requestCode) {
            REQUEST_PROJECTION -> {
                projectionPending = false
                if (resultCode == RESULT_OK && data != null) {
                    ProjectionService.start(this, resultCode, data)
                } else {
                    SqyreBridge.nativeOnProjectionStopped()
                }
            }
            REQUEST_PICK_DOCUMENT -> {
                val id = pickId ?: return
                pickId = null
                val uri = data?.data
                if (resultCode != RESULT_OK || uri == null) {
                    SqyreBridge.nativeOnDocumentPicked(id, "")
                    return
                }
                Thread({ SqyreBridge.nativeOnDocumentPicked(id, copyToCache(uri)) }, "sqyre-pick").start()
            }
            else -> {
                @Suppress("DEPRECATION")
                super.onActivityResult(requestCode, resultCode, data)
            }
        }
    }

    /** Copy [uri] into `cache/picked/` so Rust can read it as a file; `""` on failure. */
    private fun copyToCache(uri: Uri): String {
        return try {
            val dir = File(cacheDir, "picked").apply {
                deleteRecursively()
                mkdirs()
            }
            val dest = File(dir, displayName(uri))
            val input = contentResolver.openInputStream(uri) ?: return ""
            input.use { src -> dest.outputStream().use { src.copyTo(it) } }
            dest.absolutePath
        } catch (e: Exception) {
            Log.w(TAG, "copy picked document failed", e)
            ""
        }
    }

    /** Provider display name reduced to a safe file name (keeps the extension). */
    private fun displayName(uri: Uri): String {
        val name = contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
            ?.use { c -> if (c.moveToFirst()) c.getString(0) else null }
        val safe = name?.substringAfterLast('/')?.trim()?.takeIf { it.isNotEmpty() && it != "." && it != ".." }
        return safe ?: "document"
    }

    private companion object {
        const val TAG = "SqyreMain"
        const val REQUEST_PROJECTION = 1
        const val REQUEST_NOTIFICATIONS = 2
        const val REQUEST_PICK_DOCUMENT = 3
    }
}
