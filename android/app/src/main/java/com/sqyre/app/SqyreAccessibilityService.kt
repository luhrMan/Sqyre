package com.sqyre.app

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.content.Intent
import android.graphics.Path
import android.os.Bundle
import android.os.Looper
import android.view.accessibility.AccessibilityEvent
import android.view.accessibility.AccessibilityNodeInfo
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

/** Dispatches macro gestures and text edits into whatever app is in front. */
class SqyreAccessibilityService : AccessibilityService() {
    companion object {
        @Volatile
        var instance: SqyreAccessibilityService? = null
            private set

        /** Slack past the stroke duration before a gesture counts as lost. */
        private const val GESTURE_SLACK_MS = 5_000L
    }

    override fun onServiceConnected() {
        instance = this
    }

    override fun onUnbind(intent: Intent?): Boolean {
        instance = null
        return super.onUnbind(intent)
    }

    override fun onDestroy() {
        instance = null
        super.onDestroy()
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        if (event?.eventType != AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) return
        event.packageName?.toString()?.takeIf { it.isNotEmpty() }?.let { foregroundPackage = it }
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
}
