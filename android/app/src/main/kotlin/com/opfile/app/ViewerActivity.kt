package com.opfile.app

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.pdf.PdfRenderer
import android.net.Uri
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File
import java.io.FileInputStream

class ViewerActivity : AppCompatActivity() {

    companion object {
        const val EXTRA_FILE_PATH = "extra_file_path"
        const val EXTRA_FILE_NAME = "extra_file_name"
        const val EXTRA_IS_ARCHIVE_ENTRY = "extra_is_archive_entry"
        const val EXTRA_ARCHIVE_PATH = "extra_archive_path"
        const val EXTRA_ENTRY_PATH = "extra_entry_path"
    }

    private var filePath: String = ""
    private var fileName: String = ""
    private var isArchiveEntry: Boolean = false
    private var archivePath: String = ""
    private var entryPath: String = ""

    private var isHexMode: Boolean = false

    private lateinit var tvTitle: TextView
    private lateinit var btnSave: ImageView
    private lateinit var textContainer: View
    private lateinit var etCode: EditText
    private lateinit var ivImage: ImageView
    private lateinit var videoView: VideoView
    private lateinit var rvPdf: RecyclerView
    private lateinit var hexContainer: View
    private lateinit var tvHex: TextView
    private lateinit var errorContainer: View
    private lateinit var tvError: TextView
    private lateinit var btnErrorHex: Button

    private var pdfRenderer: PdfRenderer? = null
    private var pdfDescriptor: ParcelFileDescriptor? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_viewer)

        filePath = intent.getStringExtra(EXTRA_FILE_PATH) ?: ""
        fileName = intent.getStringExtra(EXTRA_FILE_NAME) ?: File(filePath).name
        isArchiveEntry = intent.getBooleanExtra(EXTRA_IS_ARCHIVE_ENTRY, false)
        archivePath = intent.getStringExtra(EXTRA_ARCHIVE_PATH) ?: ""
        entryPath = intent.getStringExtra(EXTRA_ENTRY_PATH) ?: ""

        initViews()
        loadFileContent()
    }

    private fun initViews() {
        tvTitle = findViewById(R.id.viewer_tv_title)
        btnSave = findViewById(R.id.viewer_btn_save)
        textContainer = findViewById(R.id.viewer_text_container)
        etCode = findViewById(R.id.viewer_et_code)
        ivImage = findViewById(R.id.viewer_iv_image)
        videoView = findViewById(R.id.viewer_video_view)
        rvPdf = findViewById(R.id.viewer_rv_pdf)
        hexContainer = findViewById(R.id.viewer_hex_container)
        tvHex = findViewById(R.id.viewer_tv_hex)
        errorContainer = findViewById(R.id.viewer_error_container)
        tvError = findViewById(R.id.viewer_tv_error)
        btnErrorHex = findViewById(R.id.viewer_btn_error_hex)

        tvTitle.text = fileName

        findViewById<View>(R.id.viewer_btn_back).setOnClickListener {
            finish()
        }

        findViewById<View>(R.id.viewer_btn_mode).setOnClickListener {
            toggleHexMode()
        }

        btnSave.setOnClickListener {
            saveCurrentText()
        }

        btnErrorHex.setOnClickListener {
            showHex()
        }
    }

    private suspend fun resolveFileForViewing(): File? {
        if (isArchiveEntry && archivePath.isNotEmpty() && entryPath.isNotEmpty()) {
            val cacheFile = File(cacheDir, "preview_${File(fileName).name}")
            if (cacheFile.exists()) {
                cacheFile.delete()
            }
            val resJson = RustCore.startExtractSingleTask(archivePath, entryPath, cacheFile.absolutePath, "")
            val parsed = RustCore.parseEnvelope(resJson) { it.optLong("task_id") }
            if (parsed.isSuccess) {
                val taskId = parsed.getOrNull() ?: return null
                var done = false
                var attempts = 0
                while (!done && attempts < 60) {
                    delay(50)
                    attempts++
                    val snapJson = RustCore.getTaskSnapshot(taskId)
                    val snap = RustCore.parseEnvelope(snapJson) { it.optString("state", "") }
                    val state = snap.getOrNull() ?: ""
                    if (state == "completed" || state == "done") {
                        done = true
                    } else if (state == "error" || state == "failed" || state == "cancelled") {
                        break
                    }
                }
                if (cacheFile.exists() && cacheFile.length() > 0) {
                    return cacheFile
                }
            }
        }
        val f = File(filePath)
        return if (f.exists()) f else null
    }

    private fun loadFileContent() {
        val ext = File(fileName).extension.lowercase()
        when {
            ext in listOf("png", "jpg", "jpeg", "webp", "bmp", "gif", "ico") -> {
                showImage()
            }
            ext in listOf("mp4", "mkv", "webm", "mov", "3gp", "avi") -> {
                showVideo()
            }
            ext == "pdf" -> {
                showPdf()
            }
            else -> {
                showText()
            }
        }
    }

    private fun decodeSampledBitmap(file: File, reqWidth: Int = 2048, reqHeight: Int = 2048): Bitmap? {
        return try {
            val options = BitmapFactory.Options().apply {
                inJustDecodeBounds = true
            }
            file.inputStream().use { stream ->
                BitmapFactory.decodeStream(stream, null, options)
            }

            if (options.outWidth <= 0 || options.outHeight <= 0) {
                return BitmapFactory.decodeFile(file.absolutePath)
            }

            var inSampleSize = 1
            if (options.outHeight > reqHeight || options.outWidth > reqWidth) {
                val halfHeight = options.outHeight / 2
                val halfWidth = options.outWidth / 2
                while ((halfHeight / inSampleSize) >= reqHeight && (halfWidth / inSampleSize) >= reqWidth) {
                    inSampleSize *= 2
                }
            }

            val decodeOptions = BitmapFactory.Options().apply {
                this.inSampleSize = inSampleSize
                inPreferredConfig = Bitmap.Config.ARGB_8888
            }

            file.inputStream().use { stream ->
                BitmapFactory.decodeStream(stream, null, decodeOptions)
            } ?: BitmapFactory.decodeFile(file.absolutePath, decodeOptions)
        } catch (e: Throwable) {
            null
        }
    }

    private fun showImage() {
        hideAllContainers()
        ivImage.visibility = View.VISIBLE
        lifecycleScope.launch(Dispatchers.IO) {
            val targetFile = resolveFileForViewing()
            val bmp = if (targetFile != null && targetFile.exists()) {
                decodeSampledBitmap(targetFile)
            } else null

            withContext(Dispatchers.Main) {
                if (bmp != null) {
                    ivImage.setImageBitmap(bmp)
                } else {
                    showError("Unable to decode image format: $fileName")
                }
            }
        }
    }

    private fun showVideo() {
        hideAllContainers()
        videoView.visibility = View.VISIBLE
        lifecycleScope.launch(Dispatchers.IO) {
            val targetFile = resolveFileForViewing()
            withContext(Dispatchers.Main) {
                if (targetFile != null && targetFile.exists()) {
                    val mc = MediaController(this@ViewerActivity)
                    mc.setAnchorView(videoView)
                    videoView.setMediaController(mc)
                    videoView.setVideoURI(Uri.fromFile(targetFile))
                    videoView.start()
                } else {
                    showError("Unable to load video stream: $fileName")
                }
            }
        }
    }

    private fun showPdf() {
        hideAllContainers()
        rvPdf.visibility = View.VISIBLE
        rvPdf.layoutManager = LinearLayoutManager(this)
        lifecycleScope.launch(Dispatchers.IO) {
            val targetFile = resolveFileForViewing()
            withContext(Dispatchers.Main) {
                if (targetFile != null && targetFile.exists()) {
                    try {
                        pdfDescriptor = ParcelFileDescriptor.open(targetFile, ParcelFileDescriptor.MODE_READ_ONLY)
                        pdfRenderer = PdfRenderer(pdfDescriptor!!)
                        rvPdf.adapter = PdfPageAdapter(pdfRenderer!!)
                    } catch (e: Exception) {
                        showError("PDF render error: ${e.message}")
                    }
                } else {
                    showError("Unable to open PDF: $fileName")
                }
            }
        }
    }

    private fun showText() {
        hideAllContainers()
        textContainer.visibility = View.VISIBLE
        btnSave.visibility = View.VISIBLE
        lifecycleScope.launch(Dispatchers.IO) {
            val targetFile = resolveFileForViewing()
            val path = targetFile?.absolutePath ?: filePath
            val resJson = RustCore.readTextBounded(path, 2 * 1024 * 1024)
            val parsed = RustCore.parseEnvelope(resJson) { it.optString("content", "") }
            withContext(Dispatchers.Main) {
                parsed.onSuccess { content ->
                    etCode.setText(content)
                }.onFailure {
                    showError("Unable to read as text document: ${it.message}")
                }
            }
        }
    }

    private fun showHex() {
        hideAllContainers()
        hexContainer.visibility = View.VISIBLE
        btnSave.visibility = View.GONE
        lifecycleScope.launch(Dispatchers.IO) {
            val targetFile = resolveFileForViewing()
            val path = targetFile?.absolutePath ?: filePath
            val resJson = RustCore.readHexWindow(path, 0L, 64 * 1024)
            val parsed = RustCore.parseEnvelope(resJson) { obj ->
                val sb = StringBuilder()
                if (obj.has("lines") && !obj.isNull("lines")) {
                    val lines = obj.optJSONArray("lines")
                    if (lines != null) {
                        for (i in 0 until lines.length()) {
                            val l = lines.optJSONObject(i) ?: continue
                            sb.append(String.format("%s  %s  |%s|\n", l.optString("offset_hex"), l.optString("hex_bytes"), l.optString("ascii")))
                        }
                    }
                }
                if (sb.isEmpty() && obj.has("bytes_base64")) {
                    val b64 = obj.optString("bytes_base64", "")
                    val rawBytes = if (b64.isNotEmpty()) {
                        try {
                            android.util.Base64.decode(b64, android.util.Base64.DEFAULT)
                        } catch (_: Exception) {
                            ByteArray(0)
                        }
                    } else {
                        ByteArray(0)
                    }
                    val offset = obj.optLong("offset", 0L)
                    val totalSize = obj.optLong("total_size", rawBytes.size.toLong())
                    sb.append(String.format("File: %s | Size: %d bytes\n\n", File(path).name, totalSize))
                    for (i in rawBytes.indices step 16) {
                        val lineOffset = offset + i
                        val chunkEnd = minOf(i + 16, rawBytes.size)
                        val hexSb = StringBuilder()
                        val asciiSb = StringBuilder()
                        for (j in i until i + 16) {
                            if (j < chunkEnd) {
                                val b = rawBytes[j].toInt() and 0xFF
                                hexSb.append(String.format("%02X ", b))
                                if (b in 32..126) asciiSb.append(b.toChar()) else asciiSb.append('.')
                            } else {
                                hexSb.append("   ")
                            }
                        }
                        sb.append(String.format("%08X  %s |%s|\n", lineOffset, hexSb.toString(), asciiSb.toString()))
                    }
                }
                sb.toString()
            }
            withContext(Dispatchers.Main) {
                parsed.onSuccess { txt ->
                    tvHex.text = if (txt.isNotEmpty()) txt else "Empty file (0 bytes)"
                }.onFailure {
                    tvHex.text = "Error reading hex view: ${it.message}"
                }
            }
        }
    }

    private fun showError(message: String) {
        hideAllContainers()
        errorContainer.visibility = View.VISIBLE
        tvError.text = message
    }

    private fun toggleHexMode() {
        isHexMode = !isHexMode
        if (isHexMode) {
            showHex()
        } else {
            loadFileContent()
        }
    }

    private fun saveCurrentText() {
        val newText = etCode.text.toString()
        lifecycleScope.launch(Dispatchers.IO) {
            if (isArchiveEntry) {
                RustCore.updateArchiveEntry(archivePath, entryPath, newText, "")
            } else {
                RustCore.writeTextAtomic(filePath, newText)
            }
            withContext(Dispatchers.Main) {
                Toast.makeText(this@ViewerActivity, "Saved", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun hideAllContainers() {
        textContainer.visibility = View.GONE
        ivImage.visibility = View.GONE
        videoView.visibility = View.GONE
        rvPdf.visibility = View.GONE
        hexContainer.visibility = View.GONE
        errorContainer.visibility = View.GONE
        btnSave.visibility = View.GONE
    }

    override fun onDestroy() {
        super.onDestroy()
        try {
            pdfRenderer?.close()
            pdfDescriptor?.close()
        } catch (_: Exception) {}
    }
}

class PdfPageAdapter(private val renderer: PdfRenderer) : RecyclerView.Adapter<PdfPageAdapter.PageViewHolder>() {

    class PageViewHolder(view: View) : RecyclerView.ViewHolder(view) {
        val ivPage: ImageView = view as ImageView
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): PageViewHolder {
        val iv = ImageView(parent.context).apply {
            layoutParams = ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            )
            adjustViewBounds = true
            setPadding(0, 8, 0, 8)
        }
        return PageViewHolder(iv)
    }

    override fun onBindViewHolder(holder: PageViewHolder, position: Int) {
        val page = renderer.openPage(position)
        val bitmap = Bitmap.createBitmap(page.width, page.height, Bitmap.Config.ARGB_8888)
        page.render(bitmap, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
        page.close()
        holder.ivPage.setImageBitmap(bitmap)
    }

    override fun getItemCount(): Int = renderer.pageCount
}
