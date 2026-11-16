package com.opfile.app

import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.ImageView
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.Locale

class TreemapActivity : AppCompatActivity() {

    private lateinit var targetPath: String
    private lateinit var cushionTreemapView: CushionTreemapView
    private lateinit var loadingContainer: View
    private lateinit var contentContainer: View
    private lateinit var tvSubtitle: TextView
    private lateinit var tvSelectedName: TextView
    private lateinit var tvSelectedMeta: TextView
    private lateinit var ivSelectedIcon: ImageView
    private lateinit var btnOpenInFiles: Button
    private lateinit var rvLargestItems: RecyclerView

    private var parsedNodes: List<TreemapNodeUi> = emptyList()
    private var totalAnalyzedBytes: Long = 1L

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_treemap)

        targetPath = intent.getStringExtra("target_path") ?: StorageManager.PRIMARY_STORAGE_PATH
        if (!File(targetPath).exists()) {
            targetPath = StorageManager.PRIMARY_STORAGE_PATH
        }

        findViewById<View>(R.id.btn_back).setOnClickListener { finish() }
        findViewById<View>(R.id.btn_refresh).setOnClickListener { startAnalysis() }

        tvSubtitle = findViewById(R.id.tv_treemap_subtitle)
        loadingContainer = findViewById(R.id.loading_container)
        contentContainer = findViewById(R.id.content_container)
        cushionTreemapView = findViewById(R.id.cushion_treemap_view)

        tvSelectedName = findViewById(R.id.tv_selected_name)
        tvSelectedMeta = findViewById(R.id.tv_selected_meta)
        ivSelectedIcon = findViewById(R.id.iv_selected_icon)
        btnOpenInFiles = findViewById(R.id.btn_open_in_files)
        rvLargestItems = findViewById(R.id.rv_largest_items)
        rvLargestItems.layoutManager = LinearLayoutManager(this)

        cushionTreemapView.onNodeSelected = { node ->
            updateSelectedCard(node)
        }

        btnOpenInFiles.setOnClickListener {
            val sel = cushionTreemapView.getSelected() ?: return@setOnClickListener
            val targetFolder = if (sel.isDir) sel.path else (File(sel.path).parent ?: sel.path)
            val intent = Intent(this, MainActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
                putExtra("navigate_to_path", targetFolder)
            }
            startActivity(intent)
            finish()
        }

        startAnalysis()
    }

    private fun startAnalysis() {
        loadingContainer.visibility = View.VISIBLE
        contentContainer.visibility = View.GONE
        tvSubtitle.text = "${File(targetPath).name} • Analyzing..."

        lifecycleScope.launch(Dispatchers.IO) {
            val taskJson = RustCore.startTreemapTask(targetPath, 1000f, 1000f)
            val parsed = RustCore.parseEnvelope(taskJson) { it.optLong("task_id") }
            val taskId = parsed.getOrNull()

            if (taskId == null) {
                withContext(Dispatchers.Main) {
                    loadingContainer.visibility = View.GONE
                    contentContainer.visibility = View.VISIBLE
                    Toast.makeText(this@TreemapActivity, "Failed to initiate storage analysis", Toast.LENGTH_SHORT).show()
                }
                return@launch
            }

            var finished = false
            var attempts = 0
            try {
                while (!finished && attempts < 250) {
                    attempts++
                    delay(200)
                    val snap = RustCore.getTaskSnapshot(taskId)
                    val sObj = try { JSONObject(snap).optJSONObject("data") } catch (_: Exception) { null }
                    val state = sObj?.optString("state")

                    if (state == "Completed") {
                        finished = true
                        val resultRaw = sObj.opt("result_json")
                        val arr = when (resultRaw) {
                            is JSONArray -> resultRaw
                            is String -> try { JSONArray(resultRaw) } catch (_: Exception) { JSONArray() }
                            else -> JSONArray()
                        }

                        val list = mutableListOf<TreemapNodeUi>()
                        for (i in 0 until arr.length()) {
                            val o = arr.optJSONObject(i) ?: continue
                            list.add(
                                TreemapNodeUi(
                                    name = o.optString("name", "Unknown"),
                                    path = o.optString("path", ""),
                                    size = o.optLong("size", 0L),
                                    isDir = o.optBoolean("is_dir", true),
                                    x = o.optDouble("x", 0.0).toFloat(),
                                    y = o.optDouble("y", 0.0).toFloat(),
                                    w = o.optDouble("w", 0.0).toFloat(),
                                    h = o.optDouble("h", 0.0).toFloat(),
                                    depth = o.optInt("depth", 1)
                                )
                            )
                        }

                        totalAnalyzedBytes = list.sumOf { it.size }.coerceAtLeast(1L)
                        parsedNodes = list

                        withContext(Dispatchers.Main) {
                            loadingContainer.visibility = View.GONE
                            contentContainer.visibility = View.VISIBLE
                            val rootName = if (targetPath == StorageManager.PRIMARY_STORAGE_PATH) "Internal Storage" else File(targetPath).name
                            tvSubtitle.text = "$rootName • ${formatSize(totalAnalyzedBytes)} across ${list.size} items"

                            cushionTreemapView.setNodes(list, 1000f, 1000f)

                            val adapter = LargestItemsAdapter(list, totalAnalyzedBytes) { selected ->
                                cushionTreemapView.setSelected(selected)
                                updateSelectedCard(selected)
                            }
                            rvLargestItems.adapter = adapter

                            list.firstOrNull()?.let { updateSelectedCard(it) }
                        }
                    } else if (state == "Failed") {
                        finished = true
                        withContext(Dispatchers.Main) {
                            loadingContainer.visibility = View.GONE
                            contentContainer.visibility = View.VISIBLE
                            Toast.makeText(this@TreemapActivity, "Storage analysis encountered an error", Toast.LENGTH_SHORT).show()
                        }
                    }
                }

                if (!finished) {
                    withContext(Dispatchers.Main) {
                        loadingContainer.visibility = View.GONE
                        contentContainer.visibility = View.VISIBLE
                        Toast.makeText(this@TreemapActivity, "Storage analysis timed out", Toast.LENGTH_SHORT).show()
                    }
                }
            } finally {
                RustCore.dropTask(taskId)
            }
        }
    }

    private fun updateSelectedCard(node: TreemapNodeUi) {
        tvSelectedName.text = node.name
        val percent = String.format(Locale.US, "%.1f%%", (node.size.toDouble() / totalAnalyzedBytes.toDouble()) * 100.0)
        tvSelectedMeta.text = "${formatSize(node.size)} • $percent of analyzed space"
        ivSelectedIcon.setImageResource(if (node.isDir) R.drawable.ic_folder else R.drawable.ic_file)
    }

    class LargestItemsAdapter(
        private val items: List<TreemapNodeUi>,
        private val totalSize: Long,
        private val onItemClick: (TreemapNodeUi) -> Unit
    ) : RecyclerView.Adapter<LargestItemsAdapter.ViewHolder>() {

        class ViewHolder(v: View) : RecyclerView.ViewHolder(v) {
            val tvRank: TextView = v.findViewById(R.id.tv_rank)
            val ivIcon: ImageView = v.findViewById(R.id.iv_item_icon)
            val tvName: TextView = v.findViewById(R.id.tv_item_name)
            val tvSize: TextView = v.findViewById(R.id.tv_item_size)
            val tvPercent: TextView = v.findViewById(R.id.tv_item_percent)
            val pbPercent: ProgressBar = v.findViewById(R.id.pb_percent)
        }

        override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ViewHolder {
            val v = LayoutInflater.from(parent.context).inflate(R.layout.item_treemap_list_row, parent, false)
            return ViewHolder(v)
        }

        override fun onBindViewHolder(holder: ViewHolder, position: Int) {
            val item = items[position]
            val rank = "#${position + 1}"
            val pctFloat = (item.size.toDouble() / totalSize.toDouble()).coerceIn(0.0, 1.0) * 100.0

            holder.tvRank.text = rank
            holder.tvName.text = item.name
            holder.tvSize.text = formatSize(item.size)
            holder.tvPercent.text = String.format(Locale.US, "%.1f%%", pctFloat)
            holder.pbPercent.progress = pctFloat.toInt().coerceIn(1, 100)
            holder.ivIcon.setImageResource(if (item.isDir) R.drawable.ic_folder else R.drawable.ic_file)

            holder.itemView.setOnClickListener {
                onItemClick(item)
            }
        }

        override fun getItemCount(): Int = items.size
    }
}
