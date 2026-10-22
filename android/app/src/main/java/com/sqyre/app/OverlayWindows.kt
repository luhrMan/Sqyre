package com.sqyre.app

import android.accessibilityservice.AccessibilityService
import android.graphics.Bitmap
import android.graphics.PixelFormat
import android.graphics.drawable.BitmapDrawable
import android.os.Handler
import android.os.Looper
import android.util.Log
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.widget.ImageView
import java.nio.ByteBuffer
import kotlin.math.abs
import kotlin.math.roundToInt

/**
 * One small [WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY] per macro button.
 *
 * Touches outside a button pass through (`FLAG_NOT_TOUCH_MODAL`). The windows do not take
 * focus, so the app underneath stays the foreground package. Must be added from the
 * accessibility service; that window type is trusted and can stay fully opaque.
 */
internal object OverlayWindows {
    private const val TAG = "SqyreOverlay"
    private const val TAP_SLOP = 24f

    private class Slot(
        val view: ImageView,
        var relocate: Boolean,
        var clickable: Boolean,
    )

    private class Drag(val downX: Float, val downY: Float, val originX: Int, val originY: Int)

    private val slots = LinkedHashMap<String, Slot>()
    private val drags = HashMap<String, Drag>()
    private val main = Handler(Looper.getMainLooper())
    private var windowManager: WindowManager? = null

    fun upsert(
        service: AccessibilityService,
        id: String,
        x: Int,
        y: Int,
        w: Int,
        h: Int,
        rgba: ByteArray,
        clickable: Boolean,
        relocate: Boolean,
        label: String,
    ): Boolean {
        if (w <= 0 || h <= 0 || rgba.size < w * h * 4) return false
        return onMain {
            val bitmap = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
            bitmap.copyPixelsFromBuffer(ByteBuffer.wrap(rgba, 0, w * h * 4))
            val wm = manager(service)
            val existing = slots[id]
            if (existing == null) {
                val view = ImageView(service).apply {
                    setImageBitmap(bitmap)
                    contentDescription = label
                    setOnTouchListener(touch(id))
                }
                wm.addView(view, layoutParams(x, y, w, h))
                slots[id] = Slot(view, relocate, clickable)
            } else {
                val previous = existing.view.drawable
                existing.view.setImageBitmap(bitmap)
                (previous as? BitmapDrawable)?.bitmap?.recycle()
                existing.view.contentDescription = label
                existing.relocate = relocate
                existing.clickable = clickable
                val lp = existing.view.layoutParams as WindowManager.LayoutParams
                lp.x = x
                lp.y = y
                lp.width = w
                lp.height = h
                wm.updateViewLayout(existing.view, lp)
            }
        }
    }

    fun remove(id: String): Boolean = onMain {
        val slot = slots.remove(id) ?: return@onMain
        drags.remove(id)
        removeView(slot)
    }

    fun clear(service: AccessibilityService?) {
        onMain {
            if (service != null) windowManager = service.getSystemService(WindowManager::class.java)
            val leaving = slots.values.toList()
            slots.clear()
            drags.clear()
            for (slot in leaving) removeView(slot)
            if (service == null) windowManager = null
        }
    }

    private fun manager(service: AccessibilityService): WindowManager {
        val wm = windowManager ?: service.getSystemService(WindowManager::class.java)
        windowManager = wm
        return wm
    }

    private fun removeView(slot: Slot) {
        val wm = windowManager ?: return
        try {
            wm.removeView(slot.view)
        } catch (_: IllegalArgumentException) {
            // Already detached (service teardown).
        }
        (slot.view.drawable as? BitmapDrawable)?.bitmap?.recycle()
    }

    private fun layoutParams(x: Int, y: Int, w: Int, h: Int) = WindowManager.LayoutParams(
        w,
        h,
        WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
        WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
            WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL or
            WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN or
            WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
        PixelFormat.TRANSLUCENT,
    ).apply {
        gravity = Gravity.TOP or Gravity.START
        this.x = x
        this.y = y
    }

    private fun touch(id: String) = View.OnTouchListener { _, event ->
        val slot = slots[id] ?: return@OnTouchListener false
        val lp = slot.view.layoutParams as WindowManager.LayoutParams
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                drags[id] = Drag(event.rawX, event.rawY, lp.x, lp.y)
                true
            }
            MotionEvent.ACTION_MOVE -> {
                val drag = drags[id]
                if (slot.relocate && drag != null) {
                    lp.x = drag.originX + (event.rawX - drag.downX).roundToInt()
                    lp.y = drag.originY + (event.rawY - drag.downY).roundToInt()
                    windowManager?.updateViewLayout(slot.view, lp)
                }
                true
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                val drag = drags.remove(id)
                if (slot.relocate) {
                    SqyreBridge.nativeOnOverlayMoved(id, lp.x, lp.y)
                } else if (
                    event.actionMasked == MotionEvent.ACTION_UP &&
                    slot.clickable &&
                    drag != null &&
                    abs(event.rawX - drag.downX) < TAP_SLOP &&
                    abs(event.rawY - drag.downY) < TAP_SLOP
                ) {
                    SqyreBridge.nativeOnOverlayClick(id)
                }
                true
            }
            else -> false
        }
    }

    /**
     * Run [block] on the main thread without waiting.
     *
     * The overlay host calls this from its own thread and must not block: a waiting host
     * joined from the main thread would deadlock the looper that runs [block]. Posts stay
     * ordered, so a remove cannot pass the upsert that came before it.
     */
    private fun onMain(block: () -> Unit): Boolean {
        if (Looper.myLooper() == Looper.getMainLooper()) {
            return runBlock(block)
        }
        main.post { runBlock(block) }
        return true
    }

    private fun runBlock(block: () -> Unit): Boolean = try {
        block()
        true
    } catch (e: Exception) {
        Log.w(TAG, "overlay window update failed", e)
        false
    }
}
