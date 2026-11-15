package com.opfile.app

import android.app.Dialog
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.net.Uri
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.view.Window
import android.widget.*
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.core.view.GravityCompat
import androidx.drawerlayout.widget.DrawerLayout
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.google.android.material.bottomsheet.BottomSheetDialog
import com.google.android.material.floatingactionbutton.FloatingActionButton
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.text.SimpleDateFormat
import java.util.*

class MainActivity : AppCompatActivity() {

    private lateinit var prefs: SharedPreferences
    private lateinit var drawerLayout: DrawerLayout
    private lateinit var rvFileList: RecyclerView
    private lateinit var tvActiveFolder: TextView
    private lateinit var headerPathContainer: View
    private lateinit var fabMain: FloatingActionButton
    private lateinit var tvFabBadge: TextView
    private lateinit var searchBarContainer: View
    private lateinit var etSearchQuery: EditText
    private lateinit var selectionActionBar: View
    private lateinit var tvSelectionCount: TextView

    private var currentPath: String = StorageManager.PRIMARY_STORAGE_PATH
    private var isVirtualArchive: Boolean = false
    private var virtualArchivePath: String = ""
    private var virtualInnerDir: String = ""
    private var virtualPassword: String = ""

    private var sortField: String = "name"
    private var sortDescending: Boolean = false
    private var showHidden: Boolean = false
    private var searchQuery: String = ""

    // Multi-select state
    private var isSelectionMode: Boolean = false
    private val selectedItems = mutableSetOf<String>()

    // Clipboard state (for morphing FAB)
    private var clipboardCutMode: Boolean = false
    private val clipboardPaths = mutableListOf<String>()

    // Items list
    private val displayEntries = mutableListOf<RowItem>()
    private lateinit var adapter: FileListAdapter

    // Active task progress dialog
    private var progressDialog: Dialog? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        prefs = getSharedPreferences("opfile_sticky_prefs", Context.MODE_PRIVATE)
        loadStickyPreferences()

        initViews()
        setupDrawer()
        setupRecyclerView()

        // Handle intent if launched from VIEW archive action
        intent?.data?.path?.let { p ->
            val f = File(p)
            if (f.exists()) {
                val name = f.name.lowercase()
                if (name.endsWith(".zip") || name.endsWith(".7z") || name.endsWith(".tar") || name.endsWith(".rar")) {
                    openVirtualArchive(f.absolutePath, "")
                    return
                }
            }
        }

        checkAndRequestStorage()
    }

    override fun onResume() {
        super.onResume()
        if (StorageManager.hasFullStoragePermission(this)) {
            if (currentPath.isEmpty() || !File(currentPath).exists()) {
                currentPath = StorageManager.PRIMARY_STORAGE_PATH
            }
            refreshDirectory()
        }
        updateClipboardFab()
    }

    private fun loadStickyPreferences() {
        sortField = prefs.getString("sort_field", "name") ?: "name"
        sortDescending = prefs.getBoolean("sort_descending", false)
        showHidden = prefs.getBoolean("show_hidden", false)
    }

    private fun saveStickyPreferences() {
        prefs.edit()
            .putString("sort_field", sortField)
            .putBoolean("sort_descending", sortDescending)
            .putBoolean("show_hidden", showHidden)
            .apply()
    }

    private fun checkAndRequestStorage() {
        if (!StorageManager.hasFullStoragePermission(this)) {
            StorageManager.requestFullStoragePermission(this)
        } else {
            currentPath = StorageManager.PRIMARY_STORAGE_PATH
            refreshDirectory()
        }
    }

    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == StorageManager.REQUEST_CODE_MANAGE_STORAGE ||
            requestCode == StorageManager.REQUEST_CODE_LEGACY_STORAGE) {
            if (StorageManager.hasFullStoragePermission(this)) {
                currentPath = StorageManager.PRIMARY_STORAGE_PATH
                refreshDirectory()
            } else {
                Toast.makeText(this, "Storage access is required to manage files", Toast.LENGTH_LONG).show()
            }
        }
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == StorageManager.REQUEST_CODE_LEGACY_STORAGE) {
            if (grantResults.isNotEmpty() && grantResults.all { it == android.content.pm.PackageManager.PERMISSION_GRANTED }) {
                currentPath = StorageManager.PRIMARY_STORAGE_PATH
                refreshDirectory()
            } else {
                Toast.makeText(this, "Storage permission is required to view files", Toast.LENGTH_LONG).show()
            }
        }
    }

    private fun initViews() {
        drawerLayout = findViewById(R.id.drawer_layout)
        rvFileList = findViewById(R.id.rv_file_list)
        tvActiveFolder = findViewById(R.id.tv_active_folder)
        headerPathContainer = findViewById(R.id.header_path_container)
        fabMain = findViewById(R.id.fab_main)
        tvFabBadge = findViewById(R.id.tv_fab_badge)
        searchBarContainer = findViewById(R.id.search_bar_container)
        etSearchQuery = findViewById(R.id.et_search_query)
        selectionActionBar = findViewById(R.id.selection_action_bar)
        tvSelectionCount = findViewById(R.id.tv_selection_count)

        findViewById<View>(R.id.btn_drawer_toggle).setOnClickListener {
            drawerLayout.openDrawer(GravityCompat.START)
        }

        headerPathContainer.setOnClickListener {
            showPathDropdownMenu()
        }

        findViewById<View>(R.id.btn_search).setOnClickListener {
            toggleSearchBar()
        }

        findViewById<View>(R.id.btn_search_close).setOnClickListener {
            searchBarContainer.visibility = View.GONE
            searchQuery = ""
            etSearchQuery.setText("")
            refreshDirectory()
        }

        etSearchQuery.setOnEditorActionListener { _, _, _ ->
            searchQuery = etSearchQuery.text.toString().trim()
            refreshDirectory()
            true
        }

        findViewById<View>(R.id.btn_sort).setOnClickListener {
            showSortDialog()
        }

        findViewById<View>(R.id.btn_tools).setOnClickListener {
            startActivity(Intent(this, ToolsActivity::class.java))
        }

        findViewById<View>(R.id.btn_drawer_tools).setOnClickListener {
            drawerLayout.closeDrawer(GravityCompat.START)
            startActivity(Intent(this, ToolsActivity::class.java))
        }

        findViewById<View>(R.id.btn_select_all).setOnClickListener {
            selectAllItems()
        }

        findViewById<View>(R.id.btn_selection_cancel).setOnClickListener {
            exitSelectionMode()
        }

        fabMain.setOnClickListener {
            handleFabClick()
        }
    }

    private fun setupRecyclerView() {
        rvFileList.layoutManager = LinearLayoutManager(this)
        rvFileList.itemAnimator = null // Instant single-frame rendering
        adapter = FileListAdapter(
            items = displayEntries,
            isSelected = { path -> selectedItems.contains(path) },
            onIconClick = { item ->
                if (item.isParentRow) {
                    navigateUp()
                } else {
                    toggleItemSelection(item.path)
                }
            },
            onRowClick = { item ->
                if (isSelectionMode) {
                    if (!item.isParentRow) toggleItemSelection(item.path)
                } else {
                    if (item.isParentRow) {
                        navigateUp()
                    } else if (item.isDir) {
                        if (isVirtualArchive) {
                            openVirtualArchive(virtualArchivePath, item.virtualSubDir ?: "")
                        } else {
                            navigateToDirectory(item.path)
                        }
                    } else {
                        handleFileTap(item)
                    }
                }
            },
            onRowMoreClick = { item ->
                if (!item.isParentRow) {
                    showFileActionSheet(item)
                }
            }
        )
        rvFileList.adapter = adapter
    }

    private fun setupDrawer() {
        val rvDrawer = findViewById<RecyclerView>(R.id.rv_drawer_items)
        rvDrawer.layoutManager = LinearLayoutManager(this)

        val items = mutableListOf<Pair<String, String>>()
        items.addAll(StorageManager.getQuickJumpShortcuts())
        for (v in StorageManager.getAvailableStorageVolumes(this)) {
            if (!v.isPrimary && !items.any { it.second == v.path }) {
                items.add(v.name to v.path)
            }
        }

        val drawerAdapter = DrawerAdapter(items) { path ->
            drawerLayout.closeDrawer(GravityCompat.START)
            exitVirtualArchive()
            navigateToDirectory(path)
        }
        rvDrawer.adapter = drawerAdapter
    }

    private fun updateHeaderTitle() {
        if (isVirtualArchive) {
            val arcName = File(virtualArchivePath).name
            tvActiveFolder.text = if (virtualInnerDir.isEmpty()) arcName else "$arcName/$virtualInnerDir"
        } else {
            val f = File(currentPath)
            tvActiveFolder.text = if (currentPath == StorageManager.PRIMARY_STORAGE_PATH) "Internal Storage" else f.name
        }
    }

    fun refreshDirectory() {
        if (isVirtualArchive) {
            loadVirtualArchiveDir()
        } else {
            loadPhysicalDirectory()
        }
    }

    private fun loadPhysicalDirectory() {
        if (!File(currentPath).exists()) {
            currentPath = StorageManager.PRIMARY_STORAGE_PATH
        }
        updateHeaderTitle()
        lifecycleScope.launch(Dispatchers.IO) {
            val json = RustCore.listDirectory(
                path = currentPath,
                sortField = sortField,
                descending = sortDescending,
                showHidden = showHidden,
                query = searchQuery
            )
            val parsed = RustCore.parseEnvelope(json) { obj ->
                val entriesArr = obj.optJSONArray("entries") ?: JSONArray()
                val list = mutableListOf<FileEntryItem>()
                for (i in 0 until entriesArr.length()) {
                    val e = entriesArr.optJSONObject(i) ?: continue
                    list.add(
                        FileEntryItem(
                            name = e.optString("name", "Unknown"),
                            path = e.optString("path", ""),
                            isDir = e.optBoolean("is_dir", false),
                            isSymlink = e.optBoolean("is_symlink", false),
                            isHidden = e.optBoolean("is_hidden", false),
                            size = e.optLong("size", 0L),
                            modifiedMs = e.optLong("modified_ms", 0L),
                            permissions = e.optString("permissions", ""),
                            extension = e.optString("extension", ""),
                            category = e.optString("category", "file")
                        )
                    )
                }
                val dirPath = obj.optString("path", obj.optString("current_path", currentPath))
                val parentP = if (obj.has("parent") && !obj.isNull("parent")) {
                    obj.optString("parent")
                } else if (obj.has("parent_path") && !obj.isNull("parent_path")) {
                    obj.optString("parent_path")
                } else {
                    null
                }
                DirectoryListingResult(
                    currentPath = dirPath,
                    parentPath = parentP,
                    entries = list,
                    totalCount = list.size,
                    totalSizeBytes = list.sumOf { it.size },
                    canWrite = obj.optBoolean("can_write", true)
                )
            }

            withContext(Dispatchers.Main) {
                displayEntries.clear()
                // Pinned ".." parent row
                if (currentPath != "/" && currentPath != StorageManager.PRIMARY_STORAGE_PATH) {
                    displayEntries.add(RowItem.parentRow())
                }
                parsed.onSuccess { dir ->
                    for (e in dir.entries) {
                        displayEntries.add(RowItem.fromFileEntry(e))
                    }
                }.onFailure { err ->
                    Toast.makeText(this@MainActivity, "Read error: ${err.message}", Toast.LENGTH_SHORT).show()
                }
                adapter.notifyDataSetChanged()
            }
        }
    }

    private fun loadVirtualArchiveDir() {
        updateHeaderTitle()
        lifecycleScope.launch(Dispatchers.IO) {
            val json = RustCore.listArchiveVirtual(virtualArchivePath, virtualInnerDir, virtualPassword)
            val parsed = RustCore.parseEnvelope(json) { obj ->
                val entriesArr = obj.optJSONArray("entries") ?: JSONArray()
                val list = mutableListOf<VirtualEntryItem>()
                for (i in 0 until entriesArr.length()) {
                    val e = entriesArr.optJSONObject(i) ?: continue
                    list.add(
                        VirtualEntryItem(
                            name = e.optString("name", "Unknown"),
                            fullPath = e.optString("full_path", ""),
                            isDir = e.optBoolean("is_dir", false),
                            size = e.optLong("size", 0L),
                            compressedSize = e.optLong("compressed_size", 0L),
                            method = e.optString("method", ""),
                            encrypted = e.optBoolean("encrypted", false),
                            crc32 = e.optLong("crc32", 0L)
                        )
                    )
                }
                ArchiveVirtualResult(
                    archivePath = obj.optString("archive_path", virtualArchivePath),
                    innerDir = obj.optString("inner_dir", ""),
                    entries = list,
                    totalEntries = obj.optInt("total_entries", list.size)
                )
            }

            withContext(Dispatchers.Main) {
                displayEntries.clear()
                // Pinned parent row
                displayEntries.add(RowItem.parentRow())
                parsed.onSuccess { res ->
                    for (v in res.entries) {
                        displayEntries.add(RowItem.fromVirtualEntry(v, virtualArchivePath, virtualInnerDir))
                    }
                }.onFailure { err ->
                    Toast.makeText(this@MainActivity, "Archive error: ${err.message}", Toast.LENGTH_SHORT).show()
                }
                adapter.notifyDataSetChanged()
            }
        }
    }

    fun navigateToDirectory(path: String) {
        val f = File(path)
        if (f.exists() && f.isDirectory) {
            currentPath = f.absolutePath
            isVirtualArchive = false
            exitSelectionMode()
            refreshDirectory()
        }
    }

    fun openVirtualArchive(archivePath: String, innerDir: String) {
        isVirtualArchive = true
        virtualArchivePath = archivePath
        virtualInnerDir = innerDir
        exitSelectionMode()
        refreshDirectory()
    }

    private fun exitVirtualArchive() {
        isVirtualArchive = false
        virtualArchivePath = ""
        virtualInnerDir = ""
        virtualPassword = ""
    }

    private fun navigateUp() {
        if (isVirtualArchive) {
            if (virtualInnerDir.isEmpty()) {
                exitVirtualArchive()
                refreshDirectory()
            } else {
                val parent = File(virtualInnerDir).parent ?: ""
                virtualInnerDir = parent
                refreshDirectory()
            }
        } else {
            val f = File(currentPath)
            val parent = f.parent
            if (parent != null && f.absolutePath != StorageManager.PRIMARY_STORAGE_PATH && f.absolutePath != "/") {
                navigateToDirectory(parent)
            }
        }
    }

    override fun onBackPressed() {
        if (drawerLayout.isDrawerOpen(GravityCompat.START)) {
            drawerLayout.closeDrawer(GravityCompat.START)
            return
        }
        if (isSelectionMode) {
            exitSelectionMode()
            return
        }
        if (searchBarContainer.visibility == View.VISIBLE) {
            searchBarContainer.visibility = View.GONE
            searchQuery = ""
            refreshDirectory()
            return
        }
        if (isVirtualArchive) {
            navigateUp()
            return
        }
        if (currentPath != StorageManager.PRIMARY_STORAGE_PATH && currentPath != "/") {
            navigateUp()
            return
        }
        super.onBackPressed()
    }

    private fun toggleSearchBar() {
        if (searchBarContainer.visibility == View.VISIBLE) {
            searchBarContainer.visibility = View.GONE
            searchQuery = ""
            refreshDirectory()
        } else {
            searchBarContainer.visibility = View.VISIBLE
            etSearchQuery.requestFocus()
        }
    }

    private fun toggleItemSelection(path: String) {
        if (selectedItems.contains(path)) {
            selectedItems.remove(path)
        } else {
            selectedItems.add(path)
        }
        if (selectedItems.isEmpty()) {
            exitSelectionMode()
        } else {
            isSelectionMode = true
            selectionActionBar.visibility = View.VISIBLE
            tvSelectionCount.text = "${selectedItems.size} selected"
            adapter.notifyDataSetChanged()
        }
    }

    private fun selectAllItems() {
        for (item in displayEntries) {
            if (!item.isParentRow) {
                selectedItems.add(item.path)
            }
        }
        tvSelectionCount.text = "${selectedItems.size} selected"
        adapter.notifyDataSetChanged()
    }

    private fun exitSelectionMode() {
        isSelectionMode = false
        selectedItems.clear()
        selectionActionBar.visibility = View.GONE
        adapter.notifyDataSetChanged()
    }

    private fun updateClipboardFab() {
        if (clipboardPaths.isNotEmpty()) {
            fabMain.setImageResource(R.drawable.ic_paste)
            tvFabBadge.visibility = View.VISIBLE
            tvFabBadge.text = clipboardPaths.size.toString()
        } else {
            fabMain.setImageResource(R.drawable.ic_add)
            tvFabBadge.visibility = View.GONE
        }
    }

    private fun handleFabClick() {
        if (clipboardPaths.isNotEmpty()) {
            // Paste action
            val sources = JSONArray(clipboardPaths).toString()
            val moveMode = clipboardCutMode
            val count = clipboardPaths.size
            val taskJson = RustCore.startCopyMoveTask(sources, currentPath, moveMode)
            val parsed = RustCore.parseEnvelope(taskJson) { it.optLong("task_id") }
            parsed.onSuccess { taskId ->
                TaskForegroundService.startTask(
                    this, taskId,
                    if (moveMode) "Moving $count files" else "Copying $count files",
                    currentPath
                )
                showTaskProgressDialog(taskId, if (moveMode) "Moving files..." else "Copying files...")
                clipboardPaths.clear()
                updateClipboardFab()
            }.onFailure { err ->
                Toast.makeText(this, "Task failed: ${err.message}", Toast.LENGTH_SHORT).show()
            }
        } else {
            // "+" FAB: Create New
            showCreateNewDialog()
        }
    }

    private fun showCreateNewDialog() {
        val options = arrayOf("New Folder", "New Empty File", "Create Archive (.7z / .zip)")
        AlertDialog.Builder(this)
            .setTitle("Create")
            .setItems(options) { _, which ->
                when (which) {
                    0 -> showInputDialog("New Folder", "Folder name") { name ->
                        val target = "$currentPath/$name"
                        RustCore.createFolder(target)
                        refreshDirectory()
                    }
                    1 -> showInputDialog("New File", "File name") { name ->
                        val target = "$currentPath/$name"
                        RustCore.createEmptyFile(target)
                        refreshDirectory()
                    }
                    2 -> {
                        showCompressDialog(listOf(currentPath))
                    }
                }
            }
            .show()
    }

    private fun showInputDialog(title: String, hint: String, onConfirm: (String) -> Unit) {
        val input = EditText(this).apply {
            this.hint = hint
            setTextColor(Color.WHITE)
            setHintTextColor(Color.GRAY)
        }
        AlertDialog.Builder(this)
            .setTitle(title)
            .setView(input)
            .setPositiveButton("Create") { _, _ ->
                val txt = input.text.toString().trim()
                if (txt.isNotEmpty()) onConfirm(txt)
            }
            .setNegativeButton("Cancel", null)
            .show()
    }

    private fun handleFileTap(item: RowItem) {
        val name = item.name.lowercase()
        val isArchive = name.endsWith(".zip") || name.endsWith(".7z") || name.endsWith(".tar") ||
                name.endsWith(".tar.gz") || name.endsWith(".tgz") || name.endsWith(".tar.bz2") ||
                name.endsWith(".rar") || name.endsWith(".apk") || name.endsWith(".xapk") ||
                name.endsWith(".iso") || name.endsWith(".dmg")

        if (isArchive && !isVirtualArchive) {
            // Virtual folder browsing
            openVirtualArchive(item.path, "")
        } else {
            showFileActionSheet(item)
        }
    }

    private fun showFileActionSheet(item: RowItem) {
        val sheet = BottomSheetDialog(this)
        val v = LayoutInflater.from(this).inflate(R.layout.bottom_sheet_file_actions, null)
        sheet.setContentView(v)

        v.findViewById<TextView>(R.id.sheet_file_name).text = item.name
        v.findViewById<TextView>(R.id.sheet_file_meta).text = "${item.meta} | ${item.permissions}"

        val isArchive = item.name.lowercase().let {
            it.endsWith(".zip") || it.endsWith(".7z") || it.endsWith(".tar") || it.endsWith(".tar.gz") ||
            it.endsWith(".rar") || it.endsWith(".iso") || it.endsWith(".apk")
        }

        val extractHereBtn = v.findViewById<View>(R.id.action_extract_here)
        val extractToBtn = v.findViewById<View>(R.id.action_extract_to)
        val testArchiveBtn = v.findViewById<View>(R.id.action_test_archive)

        if (!isArchive) {
            extractHereBtn.visibility = View.GONE
            extractToBtn.visibility = View.GONE
            testArchiveBtn.visibility = View.GONE
        }

        v.findViewById<View>(R.id.action_open_view).setOnClickListener {
            sheet.dismiss()
            openInViewer(item)
        }

        v.findViewById<View>(R.id.action_convert).setOnClickListener {
            sheet.dismiss()
            showDonutWheelDialog(item.path, item.extension)
        }

        extractHereBtn.setOnClickListener {
            sheet.dismiss()
            startExtractJob(item.path, currentPath)
        }

        extractToBtn.setOnClickListener {
            sheet.dismiss()
            val folderName = File(item.path).nameWithoutExtension
            val outDir = "$currentPath/$folderName"
            startExtractJob(item.path, outDir)
        }

        testArchiveBtn.setOnClickListener {
            sheet.dismiss()
            val taskJson = RustCore.startTestArchiveTask(item.path, "")
            val parsed = RustCore.parseEnvelope(taskJson) { it.optLong("task_id") }
            parsed.onSuccess { id ->
                showTaskProgressDialog(id, "Testing Archive Integrity...")
            }
        }

        v.findViewById<View>(R.id.action_compress).setOnClickListener {
            sheet.dismiss()
            showCompressDialog(listOf(item.path))
        }

        v.findViewById<View>(R.id.action_copy).setOnClickListener {
            sheet.dismiss()
            clipboardCutMode = false
            clipboardPaths.clear()
            clipboardPaths.add(item.path)
            updateClipboardFab()
            Toast.makeText(this, "Copied to clipboard", Toast.LENGTH_SHORT).show()
        }

        v.findViewById<View>(R.id.action_cut).setOnClickListener {
            sheet.dismiss()
            clipboardCutMode = true
            clipboardPaths.clear()
            clipboardPaths.add(item.path)
            updateClipboardFab()
            Toast.makeText(this, "Cut to clipboard", Toast.LENGTH_SHORT).show()
        }

        v.findViewById<View>(R.id.action_rename).setOnClickListener {
            sheet.dismiss()
            showInputDialog("Rename", item.name) { newName ->
                RustCore.renameEntry(item.path, newName)
                refreshDirectory()
            }
        }

        v.findViewById<View>(R.id.action_checksum).setOnClickListener {
            sheet.dismiss()
            startChecksumJob(item.path)
        }

        v.findViewById<View>(R.id.action_shred).setOnClickListener {
            sheet.dismiss()
            confirmShred(listOf(item.path))
        }

        v.findViewById<View>(R.id.action_delete).setOnClickListener {
            sheet.dismiss()
            confirmDelete(listOf(item.path))
        }

        sheet.show()
    }

    private fun openInViewer(item: RowItem) {
        val intent = Intent(this, ViewerActivity::class.java).apply {
            putExtra(ViewerActivity.EXTRA_FILE_PATH, item.path)
            putExtra(ViewerActivity.EXTRA_FILE_NAME, item.name)
            putExtra(ViewerActivity.EXTRA_IS_ARCHIVE_ENTRY, isVirtualArchive)
            putExtra(ViewerActivity.EXTRA_ARCHIVE_PATH, virtualArchivePath)
            putExtra(ViewerActivity.EXTRA_ENTRY_PATH, item.virtualFullPath ?: "")
        }
        startActivity(intent)
    }

    private fun showDonutWheelDialog(filePath: String, ext: String) {
        lifecycleScope.launch(Dispatchers.IO) {
            val targetsJson = RustCore.getConversionTargets(ext)
            val parsed = RustCore.parseEnvelope(targetsJson) { obj ->
                val arr = obj.getJSONArray("all_valid_targets")
                val list = mutableListOf<String>()
                for (i in 0 until arr.length()) list.add(arr.getString(i))
                list
            }

            withContext(Dispatchers.Main) {
                parsed.onSuccess { targets ->
                    if (targets.isEmpty()) {
                        Toast.makeText(this@MainActivity, "No conversion targets for .$ext", Toast.LENGTH_SHORT).show()
                        return@onSuccess
                    }
                    val dialog = Dialog(this@MainActivity)
                    dialog.requestWindowFeature(Window.FEATURE_NO_TITLE)
                    val v = LayoutInflater.from(this@MainActivity).inflate(R.layout.dialog_donut_wheel, null)
                    dialog.setContentView(v)
                    dialog.window?.setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))

                    val wheel = v.findViewById<DonutWheelView>(R.id.donut_wheel_view)
                    wheel.setTargetFormats(targets)
                    wheel.onFormatSelectedListener = { targetFmt ->
                        dialog.dismiss()
                        executeFormatConversion(filePath, targetFmt)
                    }

                    v.findViewById<View>(R.id.btn_donut_cancel).setOnClickListener {
                        dialog.dismiss()
                    }
                    dialog.show()
                }.onFailure {
                    Toast.makeText(this@MainActivity, "Targets error: ${it.message}", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    private fun executeFormatConversion(sourcePath: String, targetFormat: String) {
        val req = JSONObject().apply {
            put("source_path", sourcePath)
            put("target_format", targetFormat)
        }
        val taskJson = RustCore.startConvertTask(req.toString())
        val parsed = RustCore.parseEnvelope(taskJson) { it.optLong("task_id") }
        parsed.onSuccess { id ->
            val outName = "${File(sourcePath).nameWithoutExtension}.$targetFormat"
            TaskForegroundService.startTask(this, id, "Converting to ${targetFormat.uppercase()}", "$currentPath/$outName")
            showTaskProgressDialog(id, "Converting to ${targetFormat.uppercase()}...")
        }.onFailure {
            Toast.makeText(this, "Convert error: ${it.message}", Toast.LENGTH_SHORT).show()
        }
    }

    private fun startExtractJob(archivePath: String, destDir: String) {
        val taskJson = RustCore.startExtractTask(archivePath, destDir, "", "")
        val parsed = RustCore.parseEnvelope(taskJson) { it.optLong("task_id") }
        parsed.onSuccess { id ->
            TaskForegroundService.startTask(this, id, "Extracting Archive", destDir)
            showTaskProgressDialog(id, "Extracting Archive...")
        }.onFailure {
            Toast.makeText(this, "Extract error: ${it.message}", Toast.LENGTH_SHORT).show()
        }
    }

    private fun showCompressDialog(paths: List<String>) {
        val dialog = Dialog(this)
        val v = LayoutInflater.from(this).inflate(R.layout.dialog_compress, null)
        dialog.setContentView(v)
        dialog.window?.setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))

        val etName = v.findViewById<EditText>(R.id.et_archive_name)
        val defaultName = if (paths.size == 1) File(paths[0]).nameWithoutExtension else "archive"
        etName.setText(defaultName)

        val spFormat = v.findViewById<Spinner>(R.id.sp_archive_format)
        val formats = arrayOf("7z", "zip", "tar", "tar.gz", "tar.bz2", "tar.xz", "tar.zst")
        spFormat.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_dropdown_item, formats)

        val spLevel = v.findViewById<Spinner>(R.id.sp_compression_level)
        val levels = arrayOf("Normal", "Store", "Fastest", "Fast", "Maximum", "Ultra")
        spLevel.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_dropdown_item, levels)

        val spSplit = v.findViewById<Spinner>(R.id.sp_split_volumes)
        val splitOptions = arrayOf("None", "10 MB", "50 MB", "100 MB", "700 MB (CD)", "2000 MB (FAT32)")
        spSplit.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_dropdown_item, splitOptions)

        v.findViewById<View>(R.id.btn_compress_cancel).setOnClickListener { dialog.dismiss() }
        v.findViewById<View>(R.id.btn_compress_confirm).setOnClickListener {
            val name = etName.text.toString().trim()
            val fmt = spFormat.selectedItem.toString()
            val lvl = spLevel.selectedItem.toString()
            val pw = v.findViewById<EditText>(R.id.et_archive_password).text.toString()

            val splitMb = when (spSplit.selectedItemPosition) {
                1 -> 10
                2 -> 50
                3 -> 100
                4 -> 700
                5 -> 2000
                else -> null
            }

            val req = JSONObject().apply {
                put("archive_path", "$currentPath/$name.$fmt")
                put("format", fmt)
                put("compression_level", lvl)
                if (pw.isNotEmpty()) put("password", pw)
                if (splitMb != null) put("split_volume_size_mb", splitMb)
                put("source_paths", JSONArray(paths))
            }

            dialog.dismiss()
            val taskJson = RustCore.startCreateArchiveTask(req.toString())
            val parsed = RustCore.parseEnvelope(taskJson) { it.optLong("task_id") }
            parsed.onSuccess { id ->
                TaskForegroundService.startTask(this, id, "Compressing $name.$fmt", "$currentPath/$name.$fmt")
                showTaskProgressDialog(id, "Compressing Archive...")
            }.onFailure {
                Toast.makeText(this, "Compress error: ${it.message}", Toast.LENGTH_SHORT).show()
            }
        }
        dialog.show()
    }

    private fun startChecksumJob(filePath: String) {
        val taskJson = RustCore.startChecksumTask(filePath)
        val parsed = RustCore.parseEnvelope(taskJson) { it.optLong("task_id") }
        parsed.onSuccess { id ->
            showTaskProgressDialog(id, "Computing Checksums...")
        }
    }

    private fun confirmShred(paths: List<String>) {
        AlertDialog.Builder(this)
            .setTitle("DoD 5220.22-M Shredder")
            .setMessage("Permanently overwrite ${paths.size} item(s) with 3 DoD sanitization passes? Data cannot be recovered.")
            .setPositiveButton("Shred Permanently") { _, _ ->
                val taskJson = RustCore.startShredTask(JSONArray(paths).toString())
                val parsed = RustCore.parseEnvelope(taskJson) { it.optLong("task_id") }
                parsed.onSuccess { id ->
                    showTaskProgressDialog(id, "Shredding files (DoD 3-pass)...")
                }
            }
            .setNegativeButton("Cancel", null)
            .show()
    }

    private fun confirmDelete(paths: List<String>) {
        AlertDialog.Builder(this)
            .setTitle("Delete")
            .setMessage("Delete ${paths.size} item(s)?")
            .setPositiveButton("Delete") { _, _ ->
                val taskJson = RustCore.startDeleteTask(JSONArray(paths).toString())
                val parsed = RustCore.parseEnvelope(taskJson) { it.optLong("task_id") }
                parsed.onSuccess { id ->
                    showTaskProgressDialog(id, "Deleting items...")
                }
            }
            .setNegativeButton("Cancel", null)
            .show()
    }

    private fun showTaskProgressDialog(taskId: Long, title: String) {
        progressDialog?.dismiss()
        val dialog = Dialog(this)
        dialog.requestWindowFeature(Window.FEATURE_NO_TITLE)
        val v = LayoutInflater.from(this).inflate(R.layout.dialog_progress, null)
        dialog.setContentView(v)
        dialog.window?.setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
        dialog.setCancelable(false)

        val tvTitle = v.findViewById<TextView>(R.id.tv_dialog_task_title)
        val tvItem = v.findViewById<TextView>(R.id.tv_dialog_task_item)
        val pb = v.findViewById<ProgressBar>(R.id.pb_dialog_task)
        val tvStats = v.findViewById<TextView>(R.id.tv_dialog_task_stats)
        val btnPause = v.findViewById<Button>(R.id.btn_dialog_task_pause)
        val btnHide = v.findViewById<Button>(R.id.btn_dialog_task_hide)
        val btnCancel = v.findViewById<Button>(R.id.btn_dialog_task_cancel)

        tvTitle.text = title

        btnHide.setOnClickListener {
            dialog.dismiss()
        }

        var isPaused = false
        btnPause.setOnClickListener {
            if (isPaused) {
                RustCore.resumeTask(taskId)
                btnPause.text = "Pause"
                isPaused = false
            } else {
                RustCore.pauseTask(taskId)
                btnPause.text = "Resume"
                isPaused = true
            }
        }

        btnCancel.setOnClickListener {
            RustCore.cancelTask(taskId)
            dialog.dismiss()
        }

        progressDialog = dialog
        dialog.show()

        lifecycleScope.launch {
            while (dialog.isShowing) {
                delay(250)
                val snapJson = RustCore.getTaskSnapshot(taskId)
                val snap = parseDialogSnapshot(snapJson)
                if (snap != null) {
                    val p = (snap.percent * 100f).toInt().coerceIn(0, 100)
                    pb.progress = p
                    tvItem.text = "Current: ${snap.currentFile}"
                    tvStats.text = "$p% | ${snap.itemsProcessed} / ${snap.totalItems} items | ${formatBytes(snap.bytesProcessed)} / ${formatBytes(snap.totalBytes)}"

                    if (snap.state == "Completed") {
                        dialog.dismiss()
                        refreshDirectory()
                        Toast.makeText(this@MainActivity, "Task Completed", Toast.LENGTH_SHORT).show()
                        break
                    } else if (snap.state == "Failed") {
                        dialog.dismiss()
                        refreshDirectory()
                        Toast.makeText(this@MainActivity, "Task Failed: ${snap.errorMessage}", Toast.LENGTH_LONG).show()
                        break
                    } else if (snap.state == "Canceled") {
                        dialog.dismiss()
                        refreshDirectory()
                        Toast.makeText(this@MainActivity, "Task Canceled", Toast.LENGTH_SHORT).show()
                        break
                    }
                }
            }
        }
    }

    private fun parseDialogSnapshot(json: String): TaskSnapshotInfo? {
        return try {
            val root = JSONObject(json)
            if (!root.optBoolean("ok", false)) return null
            val d = root.getJSONObject("data")
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
                errorMessage = if (d.has("error_message") && !d.isNull("error_message")) d.getString("error_message") else null
            )
        } catch (e: Exception) {
            null
        }
    }

    private fun showSortDialog() {
        val fields = arrayOf("Name", "Size", "Date Modified", "File Type")
        val fieldKeys = arrayOf("name", "size", "date", "type")
        val currentIdx = fieldKeys.indexOf(sortField).coerceAtLeast(0)

        AlertDialog.Builder(this)
            .setTitle("Sort By")
            .setSingleChoiceItems(fields, currentIdx) { d, which ->
                sortField = fieldKeys[which]
                saveStickyPreferences()
                d.dismiss()
                refreshDirectory()
            }
            .setPositiveButton(if (sortDescending) "Order: Descending" else "Order: Ascending") { _, _ ->
                sortDescending = !sortDescending
                saveStickyPreferences()
                refreshDirectory()
            }
            .setNeutralButton(if (showHidden) "Hide Hidden" else "Show Hidden") { _, _ ->
                showHidden = !showHidden
                saveStickyPreferences()
                refreshDirectory()
            }
            .show()
    }

    private fun showPathDropdownMenu() {
        val popup = PopupMenu(this, headerPathContainer)
        popup.menu.add("Copy Path")
        popup.menu.add("Parent Directory")
        popup.menu.add("Jump to Internal Storage")
        popup.setOnMenuItemClickListener { item ->
            when (item.title) {
                "Copy Path" -> {
                    val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as android.content.ClipboardManager
                    clipboard.setPrimaryClip(android.content.ClipData.newPlainText("Path", currentPath))
                    Toast.makeText(this, "Path copied", Toast.LENGTH_SHORT).show()
                }
                "Parent Directory" -> navigateUp()
                "Jump to Internal Storage" -> {
                    exitVirtualArchive()
                    navigateToDirectory(StorageManager.PRIMARY_STORAGE_PATH)
                }
            }
            true
        }
        popup.show()
    }

    private fun formatBytes(bytes: Long): String {
        if (bytes <= 0) return "0 B"
        val units = arrayOf("B", "KB", "MB", "GB", "TB")
        val digitGroups = (Math.log10(bytes.toDouble()) / Math.log10(1024.0)).toInt().coerceIn(0, 4)
        return String.format(Locale.US, "%.1f %s", bytes / Math.pow(1024.0, digitGroups.toDouble()), units[digitGroups])
    }
}

// Data models for the RecyclerView
data class RowItem(
    val name: String,
    val path: String,
    val isParentRow: Boolean,
    val isDir: Boolean,
    val size: Long,
    val modifiedMs: Long,
    val permissions: String,
    val extension: String,
    val category: String,
    val meta: String,
    val virtualFullPath: String? = null,
    val virtualSubDir: String? = null
) {
    companion object {
        fun parentRow(): RowItem {
            return RowItem(
                name = "..",
                path = "..",
                isParentRow = true,
                isDir = true,
                size = 0,
                modifiedMs = 0,
                permissions = "",
                extension = "",
                category = "folder",
                meta = "Parent directory"
            )
        }

        fun fromFileEntry(e: FileEntryItem): RowItem {
            val dateStr = if (e.modifiedMs > 0) {
                SimpleDateFormat("yyyy-MM-dd HH:mm", Locale.US).format(Date(e.modifiedMs))
            } else ""
            val sizeStr = if (e.isDir) "Folder" else formatSize(e.size)
            val metaStr = if (dateStr.isNotEmpty()) "$sizeStr | $dateStr" else sizeStr
            return RowItem(
                name = e.name,
                path = e.path,
                isParentRow = false,
                isDir = e.isDir,
                size = e.size,
                modifiedMs = e.modifiedMs,
                permissions = e.permissions,
                extension = e.extension,
                category = e.category,
                meta = metaStr
            )
        }

        fun fromVirtualEntry(v: VirtualEntryItem, archivePath: String, innerDir: String): RowItem {
            val sizeStr = if (v.isDir) "Virtual Folder" else formatSize(v.size)
            val metaStr = "$sizeStr | ${v.method}"
            val sub = if (innerDir.isEmpty()) v.name else "$innerDir/${v.name}"
            return RowItem(
                name = v.name,
                path = "$archivePath#$sub",
                isParentRow = false,
                isDir = v.isDir,
                size = v.size,
                modifiedMs = 0,
                permissions = if (v.encrypted) "encrypted" else "",
                extension = File(v.name).extension.lowercase(),
                category = if (v.isDir) "folder" else "file",
                meta = metaStr,
                virtualFullPath = v.fullPath,
                virtualSubDir = sub
            )
        }

        private fun formatSize(bytes: Long): String {
            if (bytes <= 0) return "0 B"
            val units = arrayOf("B", "KB", "MB", "GB", "TB")
            val digitGroups = (Math.log10(bytes.toDouble()) / Math.log10(1024.0)).toInt().coerceIn(0, 4)
            return String.format(Locale.US, "%.1f %s", bytes / Math.pow(1024.0, digitGroups.toDouble()), units[digitGroups])
        }
    }
}

class FileListAdapter(
    private val items: List<RowItem>,
    private val isSelected: (String) -> Boolean,
    private val onIconClick: (RowItem) -> Unit,
    private val onRowClick: (RowItem) -> Unit,
    private val onRowMoreClick: (RowItem) -> Unit
) : RecyclerView.Adapter<FileListAdapter.ViewHolder>() {

    class ViewHolder(view: View) : RecyclerView.ViewHolder(view) {
        val container: View = view.findViewById(R.id.row_container)
        val iconTouchTarget: View = view.findViewById(R.id.icon_touch_target)
        val fileIcon: ImageView = view.findViewById(R.id.file_icon)
        val selectionIndicator: View = view.findViewById(R.id.selection_indicator)
        val textTouchTarget: View = view.findViewById(R.id.text_touch_target)
        val fileName: TextView = view.findViewById(R.id.file_name)
        val fileMeta: TextView = view.findViewById(R.id.file_meta)
        val btnRowMore: ImageView = view.findViewById(R.id.btn_row_more)
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ViewHolder {
        val view = LayoutInflater.from(parent.context).inflate(R.layout.item_file_row, parent, false)
        return ViewHolder(view)
    }

    override fun onBindViewHolder(holder: ViewHolder, position: Int) {
        val item = items[position]
        val selected = isSelected(item.path)

        holder.container.setBackgroundColor(
            if (selected) ContextCompat.getColor(holder.itemView.context, R.color.selected_row_bg)
            else ContextCompat.getColor(holder.itemView.context, R.color.amoled_black)
        )
        holder.selectionIndicator.visibility = if (selected) View.VISIBLE else View.GONE

        holder.fileName.text = item.name
        holder.fileMeta.text = if (item.permissions.isNotEmpty()) "${item.meta} | ${item.permissions}" else item.meta

        if (item.isParentRow) {
            holder.fileIcon.setImageResource(R.drawable.ic_parent_dir)
            holder.fileIcon.setColorFilter(Color.parseColor("#F39C12"))
            holder.btnRowMore.visibility = View.GONE
        } else if (item.isDir) {
            holder.fileIcon.setImageResource(R.drawable.ic_folder)
            holder.fileIcon.setColorFilter(Color.parseColor("#F39C12"))
            holder.btnRowMore.visibility = View.VISIBLE
        } else {
            holder.fileIcon.setImageResource(R.drawable.ic_file)
            val color = when (item.category) {
                "archive" -> Color.parseColor("#6C5CE7")
                "apk" -> Color.parseColor("#00B894")
                "image" -> Color.parseColor("#00CEC9")
                "audio" -> Color.parseColor("#E84393")
                "video" -> Color.parseColor("#E17055")
                "document" -> Color.parseColor("#0984E3")
                "code" -> Color.parseColor("#A29BFE")
                "specialized" -> Color.parseColor("#FDCB6E")
                else -> Color.parseColor("#7F8C8D")
            }
            holder.fileIcon.setColorFilter(color)
            holder.btnRowMore.visibility = View.VISIBLE
        }

        // Split-tap listeners
        holder.iconTouchTarget.setOnClickListener { onIconClick(item) }
        holder.textTouchTarget.setOnClickListener { onRowClick(item) }
        holder.btnRowMore.setOnClickListener { onRowMoreClick(item) }
    }

    override fun getItemCount(): Int = items.size
}

class DrawerAdapter(
    private val items: List<Pair<String, String>>,
    private val onItemClick: (String) -> Unit
) : RecyclerView.Adapter<DrawerAdapter.ViewHolder>() {

    class ViewHolder(view: View) : RecyclerView.ViewHolder(view) {
        val icon: ImageView = view.findViewById(R.id.iv_drawer_icon)
        val title: TextView = view.findViewById(R.id.tv_drawer_title)
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ViewHolder {
        val v = LayoutInflater.from(parent.context).inflate(R.layout.item_drawer_row, parent, false)
        return ViewHolder(v)
    }

    override fun onBindViewHolder(holder: ViewHolder, position: Int) {
        val (name, path) = items[position]
        holder.title.text = name
        holder.itemView.setOnClickListener { onItemClick(path) }
    }

    override fun getItemCount(): Int = items.size
}
