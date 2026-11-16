package com.prismfiles.app

import android.app.Notification
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.media.MediaScannerConnection
import android.os.IBinder
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.*
import org.json.JSONObject
import java.io.File

class TaskForegroundService : Service() {

    companion object {
        const val ACTION_START = "com.prismfiles.app.ACTION_START_TASK"
        const val ACTION_PAUSE = "com.prismfiles.app.ACTION_PAUSE_TASK"
        const val ACTION_RESUME = "com.prismfiles.app.ACTION_RESUME_TASK"
        const val ACTION_CANCEL = "com.prismfiles.app.ACTION_CANCEL_TASK"

        const val EXTRA_TASK_ID = "extra_task_id"
        const val EXTRA_TASK_TITLE = "extra_task_title"
        const val EXTRA_SCAN_PATH = "extra_scan_path"

        private const val NOTIFICATION_ID = 2001

        var activeTaskId: Long? = null
            private set
        var activeSnapshot: TaskSnapshotInfo? = null
            private set

        fun startTask(context: Context, taskId: Long, title: String, scanPath: String? = null) {
            val intent = Intent(context, TaskForegroundService::class.java).apply {
                action = ACTION_START
                putExtra(EXTRA_TASK_ID, taskId)
                putExtra(EXTRA_TASK_TITLE, title)
                if (scanPath != null) putExtra(EXTRA_SCAN_PATH, scanPath)
            }
            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }
    }

    private val serviceScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var pollJob: Job? = null
    private var scanPathAfterComplete: String? = null

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action ?: return START_NOT_STICKY
        val taskId = intent.getLongExtra(EXTRA_TASK_ID, -1L)

        when (action) {
            ACTION_START -> {
                val title = intent.getStringExtra(EXTRA_TASK_TITLE) ?: "PrismFiles Operation"
                scanPathAfterComplete = intent.getStringExtra(EXTRA_SCAN_PATH)
                activeTaskId = taskId
                val notif = buildNotification(title, 0, "Starting...", false)
                if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.Q) {
                    startForeground(NOTIFICATION_ID, notif, android.content.pm.ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC)
                } else {
                    startForeground(NOTIFICATION_ID, notif)
                }
                startPolling(taskId, title)
            }
            ACTION_PAUSE -> {
                if (taskId != -1L) {
                    RustCore.pauseTask(taskId)
                }
            }
            ACTION_RESUME -> {
                if (taskId != -1L) {
                    RustCore.resumeTask(taskId)
                }
            }
            ACTION_CANCEL -> {
                if (taskId != -1L) {
                    RustCore.cancelTask(taskId)
                    RustCore.dropTask(taskId)
                }
                stopSelf()
            }
        }
        return START_NOT_STICKY
    }

    private fun startPolling(taskId: Long, title: String) {
        pollJob?.cancel()
        pollJob = serviceScope.launch {
            var missingCount = 0
            while (isActive) {
                delay(300)
                val snapJson = RustCore.getTaskSnapshot(taskId)
                val snapshot = parseSnapshot(snapJson)
                if (snapshot != null) {
                    missingCount = 0
                    activeSnapshot = snapshot
                    val isPaused = snapshot.state == "Paused"
                    val isCompleted = snapshot.state == "Completed"
                    val isFailed = snapshot.state == "Failed"
                    val isCanceled = snapshot.state == "Canceled"

                    if (isCompleted || isFailed || isCanceled) {
                        val pathToScan = scanPathAfterComplete
                        if (isCompleted && pathToScan != null) {
                            scanExportedFile(pathToScan)
                        }
                        RustCore.dropTask(taskId)
                        activeTaskId = null
                        activeSnapshot = null
                        stopForeground(STOP_FOREGROUND_REMOVE)
                        stopSelf()
                        break
                    } else {
                        val progressInt = (snapshot.percent * 100f).toInt().coerceIn(0, 100)
                        val text = "${snapshot.itemsProcessed} / ${snapshot.totalItems} items (${snapshot.currentFile})"
                        val notif = buildNotification(title, progressInt, text, isPaused)
                        val manager = getSystemService(Context.NOTIFICATION_SERVICE) as android.app.NotificationManager
                        manager.notify(NOTIFICATION_ID, notif)
                    }
                } else {
                    missingCount++
                    if (missingCount >= 3) {
                        activeTaskId = null
                        activeSnapshot = null
                        stopForeground(STOP_FOREGROUND_REMOVE)
                        stopSelf()
                        break
                    }
                }
            }
        }
    }

    private fun parseSnapshot(json: String): TaskSnapshotInfo? {
        return try {
            val root = JSONObject(json)
            if (!root.optBoolean("ok", false)) return null
            val d = root.optJSONObject("data") ?: return null
            TaskSnapshotInfo(
                taskId = d.optLong("task_id"),
                title = d.optString("title"),
                percent = d.optDouble("percent", 0.0).toFloat(),
                itemsProcessed = d.optLong("items_processed"),
                totalItems = d.optLong("total_items"),
                bytesProcessed = d.optLong("bytes_processed"),
                totalBytes = d.optLong("total_bytes"),
                state = d.optString("state"),
                currentFile = d.optString("current_file"),
                errorMessage = if (d.has("error_message") && !d.isNull("error_message")) d.optString("error_message") else null,
                resultJson = if (d.has("result_json") && !d.isNull("result_json")) d.optString("result_json") else null
            )
        } catch (e: Exception) {
            null
        }
    }

    private fun buildNotification(title: String, progress: Int, content: String, isPaused: Boolean): Notification {
        val cancelIntent = PendingIntent.getService(
            this, 1,
            Intent(this, TaskForegroundService::class.java).apply {
                action = ACTION_CANCEL
                putExtra(EXTRA_TASK_ID, activeTaskId ?: -1L)
            },
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val pauseResumeAction = if (isPaused) {
            val resumeIntent = PendingIntent.getService(
                this, 2,
                Intent(this, TaskForegroundService::class.java).apply {
                    action = ACTION_RESUME
                    putExtra(EXTRA_TASK_ID, activeTaskId ?: -1L)
                },
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )
            NotificationCompat.Action.Builder(0, "Resume", resumeIntent).build()
        } else {
            val pauseIntent = PendingIntent.getService(
                this, 3,
                Intent(this, TaskForegroundService::class.java).apply {
                    action = ACTION_PAUSE
                    putExtra(EXTRA_TASK_ID, activeTaskId ?: -1L)
                },
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )
            NotificationCompat.Action.Builder(0, "Pause", pauseIntent).build()
        }

        val openAppIntent = PendingIntent.getActivity(
            this, 0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        return NotificationCompat.Builder(this, PrismFilesApplication.TASK_CHANNEL_ID)
            .setContentTitle(title)
            .setContentText(content)
            .setSmallIcon(R.drawable.ic_file)
            .setContentIntent(openAppIntent)
            .setProgress(100, progress, progress == 0)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setColor(0xFFFFFFFF.toInt())
            .addAction(pauseResumeAction)
            .addAction(NotificationCompat.Action.Builder(0, "Cancel", cancelIntent).build())
            .build()
    }

    private fun scanExportedFile(path: String) {
        val f = File(path)
        if (f.exists()) {
            MediaScannerConnection.scanFile(
                applicationContext,
                arrayOf(f.absolutePath),
                null
            ) { _, _ -> }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        serviceScope.cancel()
    }
}
