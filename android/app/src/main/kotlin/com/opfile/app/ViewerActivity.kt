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
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.File

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

    private fun showImage() {
        hideAllContainers()
        ivImage.visibility = View.VISIBLE
        val bmp = BitmapFactory.decodeFile(filePath)
        if (bmp != null) {
            ivImage.setImageBitmap(bmp)
        } else {
            showText()
        }
    }

    private fun showVideo() {
        hideAllContainers()
        videoView.visibility = View.VISIBLE
        val mc = MediaController(this)
        mc.setAnchorView(videoView)
        videoView.setMediaController(mc)
        videoView.setVideoURI(Uri.fromFile(File(filePath)))
        videoView.start()
    }

    private fun showPdf() {
        hideAllContainers()
        rvPdf.visibility = View.VISIBLE
        rvPdf.layoutManager = LinearLayoutManager(this)
        try {
            val file = File(filePath)
            pdfDescriptor = ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
            pdfRenderer = PdfRenderer(pdfDescriptor!!)
            rvPdf.adapter = PdfPageAdapter(pdfRenderer!!)
        } catch (e: Exception) {
            Toast.makeText(this, "PDF renderer error: ${e.message}", Toast.LENGTH_SHORT).show()
            showHex()
        }
    }

    private fun showText() {
        hideAllContainers()
        textContainer.visibility = View.VISIBLE
        btnSave.visibility = View.VISIBLE
        lifecycleScope.launch(Dispatchers.IO) {
            val resJson = RustCore.readTextBounded(filePath, 2 * 1024 * 1024)
            val parsed = RustCore.parseEnvelope(resJson) { it.optString("content", "") }
            withContext(Dispatchers.Main) {
                parsed.onSuccess { content ->
                    etCode.setText(content)
                }.onFailure {
                    showHex()
                }
            }
        }
    }

    private fun showHex() {
        hideAllContainers()
        hexContainer.visibility = View.VISIBLE
        btnSave.visibility = View.GONE
        lifecycleScope.launch(Dispatchers.IO) {
            val resJson = RustCore.readHexWindow(filePath, 0L, 64 * 1024)
            val parsed = RustCore.parseEnvelope(resJson) { obj ->
                val lines = obj.getJSONArray("lines")
                val sb = StringBuilder()
                for (i in 0 until lines.length()) {
                    val l = lines.getJSONObject(i)
                    sb.append(String.format("%s  %s  |%s|\n", l.optString("offset_hex"), l.optString("hex_bytes"), l.optString("ascii")))
                }
                sb.toString()
            }
            withContext(Dispatchers.Main) {
                parsed.onSuccess { txt ->
                    tvHex.text = txt
                }.onFailure {
                    tvHex.text = "Error reading hex view: ${it.message}"
                }
            }
        }
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
