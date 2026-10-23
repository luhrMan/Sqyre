package com.sqyre.app

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.annotation.SuppressLint
import android.content.Intent
import android.graphics.Bitmap
import android.graphics.Path
import android.os.Build
import android.os.Bundle
import android.os.Looper
import android.view.Display
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import java.nio.ByteBuffer
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.atomic.AtomicReference

/** Dispatches macro gestures and text edits into whatever app is in front. */
class SqyreAccessibilityService : AccessibilityService() {
    companion object {
        @Volatile
        var instance: SqyreAccessibilityService? = null
            private set

        /** Slack past the stroke duration before a gesture counts as lost. */
        private const val GESTURE_SLACK_MS = 5_000L
        private const val SHOT_TIMEOUT_S = 2L
    }

    override fun onServiceConnected() {
        instance = this
    }

    override fun onUnbind(intent: Intent?): Boolean {
        OverlayWindows.clear(this)
        instance = null
        return super.onUnbind(intent)
    }

    override fun onDestroy() {
        OverlayWindows.clear(this)
        instance = null
        super.onDestroy()
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        if (event?.eventType != AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) return
        val pkg = event.packageName?.toString()?.takeIf { it.isNotEmpty() } ?: return
        // Overlay windows belong to this package. Recording them would look like Sqyre
        // took focus and hide the buttons that just appeared.
        if (pkg == packageName) return
        foregroundPackage = pkg
    }

    /** Package of the last window that came to the front (empty before the first event). */
    @Volatile
    var foregroundPackage: String = ""
        private set

    override fun onInterrupt() {}

    /**
     * Dispatch one stroke and block until it completes.
     *
     * Must not run on the main thread: gesture callbacks are delivered there.
     */
    fun stroke(path: Path, durationMs: Long): Int {
        check(Looper.myLooper() != Looper.getMainLooper()) { "stroke() on the main thread" }
        val duration = durationMs.coerceIn(1L, GestureDescription.getMaxGestureDuration())
        val gesture = GestureDescription.Builder()
            .addStroke(GestureDescription.StrokeDescription(path, 0L, duration))
            .build()
        val done = CountDownLatch(1)
        val completed = AtomicBoolean(false)
        val callback = object : GestureResultCallback() {
            override fun onCompleted(gestureDescription: GestureDescription?) {
                completed.set(true)
                done.countDown()
            }

            override fun onCancelled(gestureDescription: GestureDescription?) {
                done.countDown()
            }
        }
        if (!dispatchGesture(gesture, callback, null)) return SqyreBridge.REJECTED
        if (!done.await(duration + GESTURE_SLACK_MS, TimeUnit.MILLISECONDS)) return SqyreBridge.REJECTED
        return if (completed.get()) SqyreBridge.OK else SqyreBridge.REJECTED
    }

    /** Append [text] to the focused editable field (hint text counts as empty). */
    fun appendText(text: String): Int {
        val node = findFocus(AccessibilityNodeInfo.FOCUS_INPUT) ?: return SqyreBridge.NO_FOCUSED_TEXT
        if (!node.isEditable) return SqyreBridge.NO_FOCUSED_TEXT
        val current = if (node.isShowingHintText) "" else node.text?.toString().orEmpty()
        val args = Bundle().apply {
            putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, current + text)
        }
        val ok = node.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args)
        return if (ok) SqyreBridge.OK else SqyreBridge.REJECTED
    }

    /**
     * Capture the default display and publish tightly packed RGBA.
     *
     * API 30+. Must not run on the main thread: the callback is delivered there.
     */
    @SuppressLint("NewApi")
    fun captureDisplay(): Int {
        check(Looper.myLooper() != Looper.getMainLooper()) { "captureDisplay() on the main thread" }
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) return SqyreBridge.SCREENSHOT_UNAVAILABLE
        val done = CountDownLatch(1)
        val status = AtomicInteger(SqyreBridge.REJECTED)
        val shot = AtomicReference<ScreenshotResult?>(null)
        takeScreenshot(
            Display.DEFAULT_DISPLAY,
            mainExecutor,
            object : TakeScreenshotCallback {
                override fun onSuccess(screenshot: ScreenshotResult) {
                    shot.set(screenshot)
                    done.countDown()
                }

                override fun onFailure(errorCode: Int) {
                    status.set(
                        when (errorCode) {
                            ERROR_TAKE_SCREENSHOT_INTERVAL_TIME_SHORT -> SqyreBridge.SCREENSHOT_TOO_SOON
                            ERROR_TAKE_SCREENSHOT_NO_ACCESSIBILITY_ACCESS -> SqyreBridge.SCREENSHOT_UNAVAILABLE
                            else -> SqyreBridge.REJECTED
                        },
                    )
                    done.countDown()
                }
            },
        )
        if (!done.await(SHOT_TIMEOUT_S, TimeUnit.SECONDS)) return SqyreBridge.REJECTED
        val screenshot = shot.get() ?: return status.get()
        // Copy on this worker thread. The callback only hands over the buffer.
        return try {
            publishShot(screenshot)
            SqyreBridge.OK
        } finally {
            screenshot.hardwareBuffer.close()
        }
    }

    /** Copy a hardware screenshot into the RGBA order projection frames already use. */
    private fun publishShot(screenshot: ScreenshotResult) {
        val wrapped = Bitmap.wrapHardwareBuffer(screenshot.hardwareBuffer, screenshot.colorSpace)
            ?: throw IllegalStateException("screenshot buffer")
        val argb = try {
            wrapped.copy(Bitmap.Config.ARGB_8888, false)
                ?: throw IllegalStateException("screenshot copy")
        } finally {
            wrapped.recycle()
        }
        try {
            val width = argb.width
            val height = argb.height
            val pixels = IntArray(width * height)
            argb.getPixels(pixels, 0, width, 0, 0, width, height)
            val rgba = ByteBuffer.allocateDirect(width * height * 4)
            for (pixel in pixels) {
                rgba.put((pixel shr 16 and 0xFF).toByte())
                rgba.put((pixel shr 8 and 0xFF).toByte())
                rgba.put((pixel and 0xFF).toByte())
                rgba.put((pixel ushr 24 and 0xFF).toByte())
            }
            rgba.rewind()
            SqyreBridge.nativeOnShot(rgba, width, height, width * 4, 4)
        } finally {
            argb.recycle()
        }
    }
}
