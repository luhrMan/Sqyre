package com.sqyre.app

import android.content.Context
import android.hardware.display.DisplayManager
import android.util.DisplayMetrics
import android.view.Display

/** Physical size of the default display: the space for projection frames and gestures. */
object DisplaySize {
    fun real(context: Context): Pair<Int, Int> {
        val display = context.getSystemService(DisplayManager::class.java)
            .getDisplay(Display.DEFAULT_DISPLAY)
        val metrics = DisplayMetrics()
        // WindowMetrics needs a visual context; this is called from the app context.
        @Suppress("DEPRECATION")
        display.getRealMetrics(metrics)
        return metrics.widthPixels to metrics.heightPixels
    }

    fun densityDpi(context: Context): Int = context.resources.displayMetrics.densityDpi
}
