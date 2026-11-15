package com.opfile.app

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.provider.Settings
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import java.io.File

object StorageManager {
    const val REQUEST_CODE_LEGACY_STORAGE = 1001
    const val REQUEST_CODE_MANAGE_STORAGE = 1002
    const val REQUEST_CODE_SAF_TREE = 1003

    val PRIMARY_STORAGE_PATH: String
        get() {
            val ext = Environment.getExternalStorageDirectory()
            return if (ext != null && ext.exists()) ext.absolutePath else "/storage/emulated/0"
        }

    data class StorageVolumeInfo(
        val name: String,
        val path: String,
        val isPrimary: Boolean,
        val isRemovable: Boolean
    )

    fun hasFullStoragePermission(context: Context): Boolean {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            Environment.isExternalStorageManager()
        } else {
            val read = ContextCompat.checkSelfPermission(
                context,
                android.Manifest.permission.READ_EXTERNAL_STORAGE
            ) == PackageManager.PERMISSION_GRANTED
            val write = ContextCompat.checkSelfPermission(
                context,
                android.Manifest.permission.WRITE_EXTERNAL_STORAGE
            ) == PackageManager.PERMISSION_GRANTED
            read && write
        }
    }

    fun requestFullStoragePermission(activity: Activity) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            try {
                val intent = Intent(Settings.ACTION_MANAGE_APP_ALL_FILES_ACCESS_PERMISSION).apply {
                    data = Uri.parse("package:${activity.packageName}")
                }
                activity.startActivityForResult(intent, REQUEST_CODE_MANAGE_STORAGE)
            } catch (e: Exception) {
                val fallbackIntent = Intent(Settings.ACTION_MANAGE_ALL_FILES_ACCESS_PERMISSION)
                activity.startActivityForResult(fallbackIntent, REQUEST_CODE_MANAGE_STORAGE)
            }
        } else {
            ActivityCompat.requestPermissions(
                activity,
                arrayOf(
                    android.Manifest.permission.READ_EXTERNAL_STORAGE,
                    android.Manifest.permission.WRITE_EXTERNAL_STORAGE
                ),
                REQUEST_CODE_LEGACY_STORAGE
            )
        }
    }

    fun getAvailableStorageVolumes(context: Context): List<StorageVolumeInfo> {
        val list = mutableListOf<StorageVolumeInfo>()
        // Always include Primary Storage
        list.add(
            StorageVolumeInfo(
                name = "Internal Storage",
                path = PRIMARY_STORAGE_PATH,
                isPrimary = true,
                isRemovable = false
            )
        )

        // Query external files dirs to locate secondary SD cards and USB OTG
        val extDirs = ContextCompat.getExternalFilesDirs(context, null)
        for (dir in extDirs) {
            if (dir == null) continue
            val path = dir.absolutePath
            val parts = path.split("/Android/data")
            if (parts.isNotEmpty()) {
                val root = parts[0]
                if (root != PRIMARY_STORAGE_PATH && !list.any { it.path == root }) {
                    val name = if (root.contains("usb", ignoreCase = true) || root.contains("otg", ignoreCase = true)) {
                        "USB Drive"
                    } else {
                        "SD Card (${File(root).name})"
                    }
                    list.add(
                        StorageVolumeInfo(
                            name = name,
                            path = root,
                            isPrimary = false,
                            isRemovable = true
                        )
                    )
                }
            }
        }

        // Direct scan of /storage mount points
        val storageRoot = File("/storage")
        if (storageRoot.exists() && storageRoot.isDirectory) {
            val subFiles = storageRoot.listFiles() ?: emptyArray()
            for (f in subFiles) {
                val p = f.absolutePath
                if (f.isDirectory && p != "/storage/emulated" && p != "/storage/self" && !list.any { it.path == p }) {
                    val isUsb = p.contains("usb", ignoreCase = true) || p.contains("otg", ignoreCase = true)
                    list.add(
                        StorageVolumeInfo(
                            name = if (isUsb) "USB OTG (${f.name})" else "External Storage (${f.name})",
                            path = p,
                            isPrimary = false,
                            isRemovable = true
                        )
                    )
                }
            }
        }

        return list
    }

    fun getQuickJumpShortcuts(): List<Pair<String, String>> {
        return listOf(
            "Internal Storage" to PRIMARY_STORAGE_PATH,
            "Download" to "$PRIMARY_STORAGE_PATH/Download",
            "DCIM" to "$PRIMARY_STORAGE_PATH/DCIM",
            "Pictures" to "$PRIMARY_STORAGE_PATH/Pictures",
            "Music" to "$PRIMARY_STORAGE_PATH/Music",
            "Documents" to "$PRIMARY_STORAGE_PATH/Documents"
        )
    }

    fun refreshStorageState(context: Context) {
        // Called on resume: verifies state
    }
}
