package com.sqyre.app

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.graphics.Canvas
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

    /** [MainActivity] started or stopped; frames seen while hidden become the editor backdrop. */
    @JvmStatic external fun nativeOnShellVisible(visible: Boolean)

    @JvmStatic external fun nativeOnStopRequested()

    @JvmStatic external fun nativeOnContinueRequested()

    /** Window edges covered by system bars or cutouts, in physical pixels. */
    @JvmStatic external fun nativeOnInsets(left: Int, top: Int, right: Int, bottom: Int)

    /** Answer for [pickDocument] [id]: a readable copy in app cache, or `""` when cancelled. */
    @JvmStatic external fun nativeOnDocumentPicked(id: Int, path: String)

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

    /** Bring [MainActivity] back in front of the app that covers it. */
    @JvmStatic
    fun showShell(): Int = guarded("showShell") {
        val ctx = appContext ?: return@guarded REJECTED
        val intent = Intent(ctx, MainActivity::class.java)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_REORDER_TO_FRONT)
        // The bound accessibility service may start activities from the background.
        (SqyreAccessibilityService.instance ?: ctx).startActivity(intent)
        OK
    }

    /** Send Sqyre's task to the back so the app used before it comes to the front. */
    @JvmStatic
    fun showPrevious(): Int = guarded("showPrevious") {
        val act = activity.get() ?: return@guarded REJECTED
        act.runOnUiThread { act.moveTaskToBack(true) }
        OK
    }

    /**
     * Launchable apps as `package\tlabel` lines (parsed by `sqyre_android::apps`).
     * Empty when the context is gone.
     */
    @JvmStatic
    fun launchableApps(): String = guardedText("launchableApps") {
        val pm = appContext?.packageManager ?: return@guardedText ""
        val launcher = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
        pm.queryIntentActivities(launcher, 0).joinToString("\n") { info ->
            val pkg = info.activityInfo.packageName
            appLine(pkg, info.loadLabel(pm).toString())
        }
    }

    /** Front app as one `package\tlabel` line; empty while the accessibility service is off. */
    @JvmStatic
    fun foregroundApp(): String = guardedText("foregroundApp") {
        val pkg = SqyreAccessibilityService.instance?.foregroundPackage.orEmpty()
        if (pkg.isEmpty()) return@guardedText ""
        val pm = appContext?.packageManager ?: return@guardedText appLine(pkg, "")
        val label = try {
            pm.getApplicationLabel(pm.getApplicationInfo(pkg, 0)).toString()
        } catch (e: PackageManager.NameNotFoundException) {
            ""
        }
        appLine(pkg, label)
    }

    /**
     * [packageName]'s launcher icon as [sizePx]² unpremultiplied RGBA bytes (read by
     * `sqyre_android::AppIcon`). Empty when the app is gone or the context is.
     */
    @JvmStatic
    fun appIcon(packageName: String, sizePx: Int): ByteArray = guardedBytes("appIcon") {
        val pm = appContext?.packageManager ?: return@guardedBytes ByteArray(0)
        if (sizePx <= 0) return@guardedBytes ByteArray(0)
        val drawable = try {
            pm.getApplicationIcon(packageName)
        } catch (e: PackageManager.NameNotFoundException) {
            return@guardedBytes ByteArray(0)
        }
        val bitmap = Bitmap.createBitmap(sizePx, sizePx, Bitmap.Config.ARGB_8888)
        drawable.setBounds(0, 0, sizePx, sizePx)
        drawable.draw(Canvas(bitmap))
        val pixels = IntArray(sizePx * sizePx)
        // getPixels returns unpremultiplied ARGB.
        bitmap.getPixels(pixels, 0, sizePx, 0, 0, sizePx, sizePx)
        bitmap.recycle()
        val rgba = ByteArray(pixels.size * 4)
        for ((i, argb) in pixels.withIndex()) {
            rgba[i * 4] = (argb shr 16).toByte()
            rgba[i * 4 + 1] = (argb shr 8).toByte()
            rgba[i * 4 + 2] = argb.toByte()
            rgba[i * 4 + 3] = (argb ushr 24).toByte()
        }
        rgba
    }

    private fun appLine(pkg: String, label: String): String =
        pkg + "\t" + label.replace('\t', ' ').replace('\n', ' ').replace('\r', ' ')

    /** Open the Storage Access Framework picker; always answers via [nativeOnDocumentPicked]. */
    @JvmStatic
    fun pickDocument(id: Int, mimeTypes: Array<String>) {
        val act = activity.get()
        if (act == null) {
            nativeOnDocumentPicked(id, "")
            return
        }
        act.runOnUiThread { act.pickDocument(id, mimeTypes) }
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

    /** Text-returning variant of [guarded]: failures read as an empty list. */
    private inline fun guardedText(what: String, block: () -> String): String =
        try {
            block()
        } catch (e: Exception) {
            Log.w(TAG, "$what failed", e)
            ""
        }

    /** Byte-returning variant of [guarded]: failures read as no data. */
    private inline fun guardedBytes(what: String, block: () -> ByteArray): ByteArray =
        try {
            block()
        } catch (e: Exception) {
            Log.w(TAG, "$what failed", e)
            ByteArray(0)
        }
}
