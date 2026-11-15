package com.opfile.app

import android.app.AlertDialog
import android.graphics.Bitmap
import android.graphics.Color
import android.os.Bundle
import android.view.View
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

class ToolsActivity : AppCompatActivity() {

    private lateinit var tvWifiStatus: TextView
    private lateinit var tvWifiUrls: TextView
    private lateinit var ivWifiQr: ImageView
    private lateinit var btnToggleWifi: Button

    private var isWifiRunning = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_tools)

        initViews()
        checkWifiStatus()
    }

    private fun initViews() {
        tvWifiStatus = findViewById(R.id.tv_wifi_status)
        tvWifiUrls = findViewById(R.id.tv_wifi_urls)
        ivWifiQr = findViewById(R.id.iv_wifi_qr)
        btnToggleWifi = findViewById(R.id.btn_toggle_wifi)

        findViewById<View>(R.id.tools_btn_back).setOnClickListener {
            finish()
        }

        btnToggleWifi.setOnClickListener {
            toggleWifiServer()
        }

        findViewById<View>(R.id.btn_run_treemap).setOnClickListener {
            runTreemapAnalysis()
        }

        findViewById<View>(R.id.btn_inspect_apk).setOnClickListener {
            selectAndInspectApk()
        }

        findViewById<View>(R.id.btn_preview_rename).setOnClickListener {
            previewBatchRename()
        }

        findViewById<View>(R.id.btn_apply_rename).setOnClickListener {
            executeBatchRename()
        }
    }

    private fun checkWifiStatus() {
        lifecycleScope.launch(Dispatchers.IO) {
            val statusJson = RustCore.getWifiServerStatus()
            val parsed = RustCore.parseEnvelope(statusJson) { obj ->
                val running = obj.optBoolean("running", false)
                val http = obj.optInt("http_port", 8080)
                val ftp = obj.optInt("ftp_port", 2121)
                val urls = obj.optJSONArray("urls")
                val urlList = mutableListOf<String>()
                if (urls != null) {
                    for (i in 0 until urls.length()) urlList.add(urls.getString(i))
                }
                Triple(running, http to ftp, urlList)
            }
            withContext(Dispatchers.Main) {
                parsed.onSuccess { (running, ports, urls) ->
                    isWifiRunning = running
                    if (running) {
                        tvWifiStatus.text = "Server: RUNNING"
                        tvWifiStatus.setTextColor(Color.parseColor("#00B894"))
                        btnToggleWifi.text = "Stop Wi-Fi Transfer Server"
                        val sb = StringBuilder()
                        for (u in urls) sb.append(u).append("\n")
                        tvWifiUrls.text = sb.toString().trim()

                        if (urls.isNotEmpty()) {
                            renderQrCode(urls[0])
                        }
                    } else {
                        tvWifiStatus.text = "Server: Stopped"
                        tvWifiStatus.setTextColor(Color.WHITE)
                        btnToggleWifi.text = "Start Wi-Fi Transfer Server"
                        tvWifiUrls.text = "HTTP Port: 8080 | FTP Port: 2121"
                        ivWifiQr.visibility = View.GONE
                    }
                }
            }
        }
    }

    private fun toggleWifiServer() {
        lifecycleScope.launch(Dispatchers.IO) {
            if (isWifiRunning) {
                RustCore.stopWifiServer()
            } else {
                RustCore.startWifiServer(StorageManager.PRIMARY_STORAGE_PATH, 8080, 2121)
            }
            withContext(Dispatchers.Main) {
                checkWifiStatus()
            }
        }
    }

    private fun renderQrCode(text: String) {
        lifecycleScope.launch(Dispatchers.IO) {
            val qrJson = RustCore.generateQrMatrix(text)
            val parsed = RustCore.parseEnvelope(qrJson) { obj ->
                val w = obj.getInt("width")
                val h = obj.getInt("height")
                val mat = obj.getJSONArray("matrix")
                val bools = BooleanArray(w * h)
                for (y in 0 until h) {
                    val row = mat.getJSONArray(y)
                    for (x in 0 until w) {
                        bools[y * w + x] = row.getBoolean(x)
                    }
                }
                Triple(w, h, bools)
            }
            withContext(Dispatchers.Main) {
                parsed.onSuccess { (w, h, bools) ->
                    val bmp = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
                    for (y in 0 until h) {
                        for (x in 0 until w) {
                            bmp.setPixel(x, y, if (bools[y * w + x]) Color.BLACK else Color.WHITE)
                        }
                    }
                    ivWifiQr.setImageBitmap(Bitmap.createScaledBitmap(bmp, 320, 320, false))
                    ivWifiQr.visibility = View.VISIBLE
                }
            }
        }
    }

    private fun runTreemapAnalysis() {
        Toast.makeText(this, "Calculating cushion treemap...", Toast.LENGTH_SHORT).show()
        val taskJson = RustCore.startTreemapTask(StorageManager.PRIMARY_STORAGE_PATH, 1080f, 1920f)
        val parsed = RustCore.parseEnvelope(taskJson) { it.optLong("task_id") }
        parsed.onSuccess { taskId ->
            lifecycleScope.launch(Dispatchers.IO) {
                var finished = false
                while (!finished) {
                    kotlinx.coroutines.delay(300)
                    val snap = RustCore.getTaskSnapshot(taskId)
                    val sObj = try { JSONObject(snap).optJSONObject("data") } catch (_: Exception) { null }
                    val st = sObj?.optString("state")
                    if (st == "Completed") {
                        finished = true
                        withContext(Dispatchers.Main) {
                            AlertDialog.Builder(this@ToolsActivity)
                                .setTitle("Storage Analysis Complete")
                                .setMessage("Hierarchical treemap generated successfully across storage.")
                                .setPositiveButton("OK", null)
                                .show()
                        }
                    } else if (st == "Failed") {
                        finished = true
                    }
                }
            }
        }
    }

    private fun selectAndInspectApk() {
        val downloadDir = File("${StorageManager.PRIMARY_STORAGE_PATH}/Download")
        val apks = downloadDir.listFiles { f ->
            f.isFile && (f.name.endsWith(".apk", true) || f.name.endsWith(".xapk", true))
        } ?: emptyArray()

        if (apks.isEmpty()) {
            Toast.makeText(this, "No .apk or .xapk files in Download folder", Toast.LENGTH_SHORT).show()
            return
        }

        val names = apks.map { it.name }.toTypedArray()
        AlertDialog.Builder(this)
            .setTitle("Select Package to Inspect")
            .setItems(names) { _, which ->
                inspectApkFile(apks[which].absolutePath)
            }
            .show()
    }

    private fun inspectApkFile(path: String) {
        lifecycleScope.launch(Dispatchers.IO) {
            val resJson = RustCore.inspectApkManifest(path)
            val parsed = RustCore.parseEnvelope(resJson) { obj ->
                val pkg = obj.optString("package_name")
                val verName = obj.optString("version_name")
                val verCode = obj.optString("version_code")
                val minSdk = obj.optString("min_sdk")
                val targetSdk = obj.optString("target_sdk")
                val perms = obj.optJSONArray("permissions")
                val permsList = mutableListOf<String>()
                if (perms != null) {
                    for (i in 0 until perms.length()) permsList.add(perms.getString(i))
                }
                "Package: $pkg\nVersion: $verName ($verCode)\nMin SDK: $minSdk | Target SDK: $targetSdk\n\nPermissions (${permsList.size}):\n" + permsList.take(15).joinToString("\n")
            }
            withContext(Dispatchers.Main) {
                parsed.onSuccess { info ->
                    AlertDialog.Builder(this@ToolsActivity)
                        .setTitle(File(path).name)
                        .setMessage(info)
                        .setPositiveButton("OK", null)
                        .show()
                }.onFailure {
                    Toast.makeText(this@ToolsActivity, "Inspect failed: ${it.message}", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    private fun previewBatchRename() {
        val pat = findViewById<EditText>(R.id.et_rename_pattern).text.toString().trim()
        val rep = findViewById<EditText>(R.id.et_rename_replace).text.toString().trim()
        if (pat.isEmpty()) {
            Toast.makeText(this, "Enter regex pattern", Toast.LENGTH_SHORT).show()
            return
        }
        val req = JSONObject().apply {
            put("directory", "${StorageManager.PRIMARY_STORAGE_PATH}/Download")
            put("pattern", pat)
            put("replacement", rep)
            put("include_extension", false)
        }
        lifecycleScope.launch(Dispatchers.IO) {
            val json = RustCore.previewBatchRename(req.toString())
            val parsed = RustCore.parseEnvelope(json) { obj ->
                val arr = obj.getJSONArray("operations")
                val sb = StringBuilder()
                for (i in 0 until arr.length()) {
                    val o = arr.getJSONObject(i)
                    sb.append(o.optString("old_name")).append("  ->  ").append(o.optString("new_name")).append("\n")
                }
                sb.toString()
            }
            withContext(Dispatchers.Main) {
                parsed.onSuccess { preview ->
                    AlertDialog.Builder(this@ToolsActivity)
                        .setTitle("Rename Preview")
                        .setMessage(if (preview.isEmpty()) "No matching files found" else preview)
                        .setPositiveButton("OK", null)
                        .show()
                }
            }
        }
    }

    private fun executeBatchRename() {
        val pat = findViewById<EditText>(R.id.et_rename_pattern).text.toString().trim()
        val rep = findViewById<EditText>(R.id.et_rename_replace).text.toString().trim()
        if (pat.isEmpty()) return

        val req = JSONObject().apply {
            put("directory", "${StorageManager.PRIMARY_STORAGE_PATH}/Download")
            put("pattern", pat)
            put("replacement", rep)
            put("include_extension", false)
        }
        lifecycleScope.launch(Dispatchers.IO) {
            val prevJson = RustCore.previewBatchRename(req.toString())
            val opsArr = try {
                JSONObject(prevJson).optJSONObject("data")?.optJSONArray("operations") ?: JSONArray()
            } catch (_: Exception) {
                JSONArray()
            }
            RustCore.executeBatchRename(opsArr.toString())
            withContext(Dispatchers.Main) {
                Toast.makeText(this@ToolsActivity, "Renamed ${opsArr.length()} items", Toast.LENGTH_SHORT).show()
            }
        }
    }
}
