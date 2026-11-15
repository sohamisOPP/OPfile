package com.opfile.app

import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.os.Build
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.ProcessLifecycleOwner

class OpfileApplication : Application(), DefaultLifecycleObserver {

    companion object {
        const val TASK_CHANNEL_ID = "opfile_tasks_channel"
        lateinit var instance: OpfileApplication
            private set
    }

    override fun onCreate() {
        super<Application>.onCreate()
        instance = this
        ProcessLifecycleOwner.get().lifecycle.addObserver(this)
        createNotificationChannel()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                TASK_CHANNEL_ID,
                "OPfile Tasks",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Shows real-time progress for file copying, moving, archiving, and format conversion"
                enableVibration(false)
                setSound(null, null)
            }
            val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
            manager.createNotificationChannel(channel)
        }
    }

    override fun onResume(owner: LifecycleOwner) {
        // App resumed - re-verify storage access state
        StorageManager.refreshStorageState(this)
    }
}
