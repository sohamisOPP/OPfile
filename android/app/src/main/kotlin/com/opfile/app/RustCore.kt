package com.opfile.app

import org.json.JSONArray
import org.json.JSONObject

object RustCore {
    init {
        try {
            System.loadLibrary("opfile_core")
        } catch (e: UnsatisfiedLinkError) {
            e.printStackTrace()
        }
    }

    // JNI Native Declarations
    external fun listDirectory(
        path: String,
        sortField: String,
        descending: Boolean,
        showHidden: Boolean,
        query: String
    ): String

    external fun searchRecursive(root: String, query: String, maxResults: Int): String
    external fun createFolder(path: String): String
    external fun createEmptyFile(path: String): String
    external fun renameEntry(oldPath: String, newName: String): String
    external fun readTextBounded(path: String, maxBytes: Int): String
    external fun writeTextAtomic(path: String, content: String): String
    external fun listArchiveVirtual(archivePath: String, innerDir: String, password: String): String
    external fun searchArchive(archivePath: String, query: String, password: String): String
    external fun inspectApkManifest(apkPath: String): String
    external fun updateArchiveEntry(archivePath: String, entryPath: String, content: String, password: String): String
    external fun getConversionTargets(ext: String): String
    external fun readHexWindow(path: String, offset: Long, length: Int): String
    external fun previewBatchRename(reqJson: String): String
    external fun executeBatchRename(opsJson: String): String
    external fun undoBatchRename(opsJson: String): String
    external fun generateQrMatrix(text: String): String
    external fun startWifiServer(rootDir: String, httpPort: Int, ftpPort: Int): String
    external fun stopWifiServer(): String
    external fun getWifiServerStatus(): String

    external fun startCopyMoveTask(sourcesJson: String, destDir: String, moveMode: Boolean): String
    external fun startDeleteTask(pathsJson: String): String
    external fun startShredTask(pathsJson: String): String
    external fun startExtractTask(archivePath: String, destDir: String, password: String, entriesJson: String): String
    external fun startExtractSingleTask(archivePath: String, entryPath: String, destFile: String, password: String): String
    external fun startCreateArchiveTask(reqJson: String): String
    external fun startTestArchiveTask(archivePath: String, password: String): String
    external fun startConvertTask(reqJson: String): String
    external fun startChecksumTask(path: String): String
    external fun startTreemapTask(rootDir: String, width: Float, height: Float): String

    external fun getTaskSnapshot(taskId: Long): String
    external fun pauseTask(taskId: Long): Boolean
    external fun resumeTask(taskId: Long): Boolean
    external fun cancelTask(taskId: Long): Boolean
    external fun dropTask(taskId: Long)

    // Helper to parse ApiEnvelope
    inline fun <reified T> parseEnvelope(jsonStr: String, dataParser: (JSONObject) -> T): Result<T> {
        return try {
            val root = JSONObject(jsonStr)
            val ok = root.optBoolean("ok", false)
            if (ok) {
                val dataObj = root.optJSONObject("data")
                if (dataObj != null) {
                    Result.success(dataParser(dataObj))
                } else if (root.has("data") && !root.isNull("data")) {
                    val synthetic = JSONObject().apply {
                        val v = root.get("data")
                        put("value", v)
                        put("content", v)
                    }
                    Result.success(dataParser(synthetic))
                } else {
                    Result.failure(Exception("Missing data object in successful response"))
                }
            } else {
                val errObj = root.optJSONObject("error")
                val msg = errObj?.optString("message") ?: "Unknown error"
                val code = errObj?.optString("code") ?: "Generic"
                Result.failure(OpfileException(code, msg))
            }
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}

class OpfileException(val code: String, message: String) : Exception("[$code] $message")

data class FileEntryItem(
    val name: String,
    val path: String,
    val isDir: Boolean,
    val isSymlink: Boolean,
    val isHidden: Boolean,
    val size: Long,
    val modifiedMs: Long,
    val permissions: String,
    val extension: String,
    val category: String,
    val childCount: Int? = null
)

data class DirectoryListingResult(
    val currentPath: String,
    val parentPath: String?,
    val entries: List<FileEntryItem>,
    val totalCount: Int,
    val totalSizeBytes: Long,
    val canWrite: Boolean
)

data class VirtualEntryItem(
    val name: String,
    val fullPath: String,
    val isDir: Boolean,
    val size: Long,
    val compressedSize: Long,
    val method: String,
    val encrypted: Boolean,
    val crc32: Long
)

data class ArchiveVirtualResult(
    val archivePath: String,
    val innerDir: String,
    val entries: List<VirtualEntryItem>,
    val totalEntries: Int
)

data class TaskSnapshotInfo(
    val taskId: Long,
    val title: String,
    val percent: Float,
    val itemsProcessed: Long,
    val totalItems: Long,
    val bytesProcessed: Long,
    val totalBytes: Long,
    val state: String,
    val currentFile: String,
    val errorMessage: String?,
    val resultJson: String? = null
)
