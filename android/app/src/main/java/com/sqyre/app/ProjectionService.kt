package com.sqyre.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.content.res.Configuration
import android.graphics.PixelFormat
import android.hardware.display.DisplayManager
import android.hardware.display.VirtualDisplay
import android.media.ImageReader
import android.media.projection.MediaProjection
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.os.IBinder
import android.util.Log

/**
 * Foreground service holding the MediaProjection session.
 *
 * Frames go to Rust (`SqyreBridge.nativeOnFrame`), which copies one only while a
 * capture asked recently. The notification's action stops the running macro.
 */
class ProjectionService : Service() {
    companion object {
        private const val TAG = "SqyreProjection"
        private const val CHANNEL_ID = "projection"
        private const val NOTIFICATION_ID = 1
        private const val EXTRA_RESULT_CODE = "resultCode"
        private const val EXTRA_RESULT_DATA = "resultData"
        private const val ACTION_STOP_MACRO = "com.sqyre.app.STOP_MACRO"
        private const val ACTION_CONTINUE_MACRO = "com.sqyre.app.CONTINUE_MACRO"

        @Volatile
        var running = false
            private set

        fun start(context: Context, resultCode: Int, data: Intent) {
            val intent = Intent(context, ProjectionService::class.java)
                .putExtra(EXTRA_RESULT_CODE, resultCode)
                .putExtra(EXTRA_RESULT_DATA, data)
            context.startForegroundService(intent)
        }
    }

    private var thread: HandlerThread? = null
    private var handler: Handler? = null
    private var projection: MediaProjection? = null
    private var display: VirtualDisplay? = null
    private var reader: ImageReader? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP_MACRO -> {
                SqyreBridge.nativeOnStopRequested()
                return START_NOT_STICKY
            }
            ACTION_CONTINUE_MACRO -> {
                SqyreBridge.nativeOnContinueRequested()
                return START_NOT_STICKY
            }
        }
        // Android 14+: the foreground type must be active before getMediaProjection.
        startForeground(NOTIFICATION_ID, notification(), ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION)
        if (projection != null) return START_NOT_STICKY
        val data = intent?.let { resultData(it) }
        val resultCode = intent?.getIntExtra(EXTRA_RESULT_CODE, 0) ?: 0
        if (data == null) {
            stopAndNotify()
            return START_NOT_STICKY
        }
        val worker = HandlerThread("sqyre-projection").also { it.start() }
        val workerHandler = Handler(worker.looper)
        thread = worker
        handler = workerHandler
        val mp = try {
            getSystemService(MediaProjectionManager::class.java).getMediaProjection(resultCode, data)
        } catch (e: RuntimeException) {
            Log.w(TAG, "getMediaProjection failed", e)
            null
        }
        if (mp == null) {
            stopAndNotify()
            return START_NOT_STICKY
        }
        // Android 14+: the callback must be registered before createVirtualDisplay.
        mp.registerCallback(object : MediaProjection.Callback() {
            override fun onStop() {
                stopAndNotify()
            }
        }, workerHandler)
        projection = mp
        createDisplay(mp, workerHandler)
        running = true
        return START_NOT_STICKY
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        val h = handler ?: return
        h.post { resizeDisplay(h) }
    }

    override fun onDestroy() {
        teardown()
        super.onDestroy()
    }

    private fun resultData(intent: Intent): Intent? =
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            intent.getParcelableExtra(EXTRA_RESULT_DATA, Intent::class.java)
        } else {
            @Suppress("DEPRECATION")
            intent.getParcelableExtra(EXTRA_RESULT_DATA)
        }

    private fun newReader(width: Int, height: Int, handler: Handler): ImageReader =
        ImageReader.newInstance(width, height, PixelFormat.RGBA_8888, 2).apply {
            setOnImageAvailableListener({ r ->
                r.acquireLatestImage()?.use { image ->
                    val plane = image.planes[0]
                    SqyreBridge.nativeOnFrame(
                        plane.buffer,
                        image.width,
                        image.height,
                        plane.rowStride,
                        plane.pixelStride,
                    )
                }
            }, handler)
        }

    private fun createDisplay(mp: MediaProjection, handler: Handler) {
        val (width, height) = DisplaySize.real(this)
        val r = newReader(width, height, handler)
        reader = r
        display = mp.createVirtualDisplay(
            "sqyre",
            width,
            height,
            DisplaySize.densityDpi(this),
            DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
            r.surface,
            null,
            handler,
        )
    }

    /** Rotation / resolution change: match the new real size so frames stay 1:1 with gestures. */
    private fun resizeDisplay(handler: Handler) {
        val vd = display ?: return
        val (width, height) = DisplaySize.real(this)
        val old = reader
        if (old != null && old.width == width && old.height == height) return
        val r = newReader(width, height, handler)
        vd.resize(width, height, DisplaySize.densityDpi(this))
        vd.surface = r.surface
        reader = r
        old?.close()
    }

    private fun stopAndNotify() {
        teardown()
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    /** Idempotent; also reports consent tokens that failed before the projection started. */
    private fun teardown() {
        running = false
        display?.release()
        display = null
        reader?.close()
        reader = null
        projection?.stop()
        projection = null
        thread?.quitSafely()
        thread = null
        handler = null
        SqyreBridge.nativeOnProjectionStopped()
    }

    private fun notification(): Notification {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL_ID, getString(R.string.projection_channel), NotificationManager.IMPORTANCE_LOW),
        )
        return Notification.Builder(this, CHANNEL_ID)
            .setContentTitle(getString(R.string.app_name))
            .setContentText(getString(R.string.projection_running))
            .setSmallIcon(R.drawable.ic_notification)
            .setOngoing(true)
            .addAction(action(R.string.projection_continue, ACTION_CONTINUE_MACRO, 1))
            .addAction(action(R.string.projection_stop, ACTION_STOP_MACRO, 0))
            .build()
    }

    /** Distinct [requestCode]s keep the two PendingIntents from replacing each other. */
    private fun action(label: Int, action: String, requestCode: Int): Notification.Action {
        val intent = PendingIntent.getService(
            this,
            requestCode,
            Intent(this, ProjectionService::class.java).setAction(action),
            PendingIntent.FLAG_IMMUTABLE,
        )
        return Notification.Action.Builder(null, getString(label), intent).build()
    }
}
