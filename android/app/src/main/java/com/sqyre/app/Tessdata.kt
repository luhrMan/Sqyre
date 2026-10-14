package com.sqyre.app

import android.content.Context
import android.util.Log
import java.io.File
import java.io.FileNotFoundException

/**
 * Copies the APK's `tessdata/eng.traineddata` into `filesDir/tessdata`, where Rust
 * (`sqyre_vision::set_tessdata_dir`) looks first. Tesseract needs a real file path.
 */
object Tessdata {
    private const val TAG = "SqyreTessdata"
    private const val ASSET = "tessdata/eng.traineddata"

    /** Runs before Rust starts, so the first OCR (or the startup probe) finds the model. */
    fun install(context: Context) {
        val dest = File(context.filesDir, ASSET)
        val updated = context.packageManager.getPackageInfo(context.packageName, 0).lastUpdateTime
        if (dest.isFile && dest.lastModified() >= updated) return
        try {
            dest.parentFile?.mkdirs()
            val partial = File(dest.path + ".partial")
            context.assets.open(ASSET).use { input ->
                partial.outputStream().use { input.copyTo(it) }
            }
            if (!partial.renameTo(dest)) {
                partial.delete()
                Log.w(TAG, "could not install $dest")
            }
        } catch (e: FileNotFoundException) {
            // Editor-only APKs ship no tessdata.
        } catch (e: Exception) {
            Log.w(TAG, "extract $ASSET failed", e)
        }
    }
}
