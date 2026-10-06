package com.sqyre.app

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Path
import android.provider.Settings
import android.util.Log
import java.lang.ref.WeakReference
import java.nio.ByteBuffer

/**
 * Static entry points shared with `crates/sqyre-android` (JNI).
 *
 * Rust calls the `@JvmStatic` methods from macro worker threads; status codes must
 * match `sqyre_android::status`. The `native*` functions are implemented in Rust.
 */
object SqyreBridge {
    const val OK = 0
    const val ACCESSIBILITY_OFF = 1
    const val REJECTED = 2
    const val NO_FOCUSED_TEXT = 3
    const val APP_NOT_FOUND = 4
    const val TITLE_MISMATCH = 5

    private const val TAG = "SqyreBridge"

    init {
        // NativeActivity dlopens the same library, but JNI only resolves `external`
        // methods in libraries loaded through System.loadLibrary.
        System.loadLibrary("sqyre_app")
    }

    @Volatile private var appContext: Context? = null
    @Volatile private var activity: WeakReference<MainActivity> = WeakReference(null)

    /** Call from [MainActivity.onCreate] before `super.onCreate` starts the Rust main. */
    fun attach(activity: MainActivity) {
        appContext = activity.applicationContext
        this.activity = WeakReference(activity)
        nativeInit()
    }

    fun detach(activity: MainActivity) {
        if (this.activity.get() === activity) {
            this.activity = WeakReference(null)
        }
    }

    @JvmStatic external fun nativeInit()

    @JvmStatic external fun nativeOnFrame(
        buffer: ByteBuffer,
        width: Int,
        height: Int,
        rowStride: Int,
        pixelStride: Int,
    )

    @JvmStatic external fun nativeOnProjectionStopped()

    @JvmStatic external fun nativeOnStopRequested()

    @JvmStatic
    fun press(x: Int, y: Int, durationMs: Long): Int = guarded("press") {
        val service = SqyreAccessibilityService.instance ?: return@guarded ACCESSIBILITY_OFF
        val path = Path().apply { moveTo(x.toFloat(), y.toFloat()) }
        service.stroke(path, durationMs)
    }

    @JvmStatic
    fun swipe(x1: Int, y1: Int, x2: Int, y2: Int, durationMs: Long): Int = guarded("swipe") {
        val service = SqyreAccessibilityService.instance ?: return@guarded ACCESSIBILITY_OFF
        val path = Path().apply {
            moveTo(x1.toFloat(), y1.toFloat())
            lineTo(x2.toFloat(), y2.toFloat())
        }
        service.stroke(path, durationMs)
    }

    @JvmStatic
    fun appendText(text: String): Int = guarded("appendText") {
        val service = SqyreAccessibilityService.instance ?: return@guarded ACCESSIBILITY_OFF
        service.appendText(text)
    }

    @JvmStatic
    fun setClipboard(text: String): Int = guarded("setClipboard") {
        val ctx = appContext ?: return@guarded REJECTED
        val clipboard = ctx.getSystemService(ClipboardManager::class.java)
        clipboard.setPrimaryClip(ClipData.newPlainText("Sqyre", text))
        OK
    }

    /** Launch [packageName]; a non-empty [label] must equal the app label. */
    @JvmStatic
    fun launch(packageName: String, label: String): Int = guarded("launch") {
        val ctx = appContext ?: return@guarded REJECTED
        val pm = ctx.packageManager
        val intent = pm.getLaunchIntentForPackage(packageName) ?: return@guarded APP_NOT_FOUND
        if (label.isNotEmpty()) {
            val appLabel = try {
                pm.getApplicationLabel(pm.getApplicationInfo(packageName, 0)).toString()
            } catch (e: PackageManager.NameNotFoundException) {
                return@guarded APP_NOT_FOUND
            }
            if (appLabel.trim() != label) return@guarded TITLE_MISMATCH
        }
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED)
        // The bound accessibility service may start activities from the background.
        (SqyreAccessibilityService.instance ?: ctx).startActivity(intent)
        OK
    }

    @JvmStatic
    fun requestProjection() {
        val act = activity.get() ?: return
        act.runOnUiThread { act.requestProjection() }
    }

    @JvmStatic
    fun accessibilityEnabled(): Boolean = SqyreAccessibilityService.instance != null

    @JvmStatic
    fun openAccessibilitySettings() {
        val ctx = appContext ?: return
        ctx.startActivity(
            Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
        )
    }

    @JvmStatic
    fun displayWidth(): Int = appContext?.let { DisplaySize.real(it).first } ?: 0

    @JvmStatic
    fun displayHeight(): Int = appContext?.let { DisplaySize.real(it).second } ?: 0

    /** Exceptions must not cross into Rust; report them as [REJECTED]. */
    private inline fun guarded(what: String, block: () -> Int): Int =
        try {
            block()
        } catch (e: Exception) {
            Log.w(TAG, "$what failed", e)
            REJECTED
        }
}
