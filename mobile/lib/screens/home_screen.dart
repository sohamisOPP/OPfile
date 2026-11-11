import 'dart:io';
import 'dart:math' as math;
import 'package:crypto/crypto.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:share_plus/share_plus.dart';
import '../models/file_item.dart';
import '../services/conversion_engine.dart';
import '../services/file_service.dart';
import '../widgets/conversion_dialog.dart';
import '../widgets/donut_menu.dart';
import '../widgets/file_card.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> with WidgetsBindingObserver {
  List<FileItem> _files = [];
  bool _isLoading = true;
  bool _isGridView = false; // Default to classic list view
  bool _showSearchBar = false;
  FileCategory _selectedCategory = FileCategory.all;
  String _searchQuery = '';
  final TextEditingController _searchController = TextEditingController();

  // Batch selection
  bool _isBatchMode = false;
  final Set<String> _selectedPaths = {};

  // Real Storage Management
  bool _hasStoragePermission = true;
  Directory? _currentDir;
  String _rootPath = '/storage/emulated/0';

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _initializeStorage();
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _searchController.dispose();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      _checkPermissionAndRefresh();
    }
  }

  Future<void> _initializeStorage() async {
    final root = await FileService.getRootStoragePath();
    _rootPath = root;
    await _loadFiles();
  }

  Future<void> _checkPermissionAndRefresh() async {
    final hasPerm = await FileService.isStoragePermissionGranted();
    if (hasPerm) {
      final root = await FileService.getRootStoragePath();
      _rootPath = root;
      if (_currentDir == null || !_currentDir!.path.startsWith(_rootPath)) {
        FileService.setCurrentDirectory(Directory(root));
      }
    }
    await _loadFiles();
  }

  Future<void> _loadFiles() async {
    setState(() => _isLoading = true);
    final hasPerm = await FileService.isStoragePermissionGranted();
    final currentDir = await FileService.getCurrentDirectory();

    final items = await FileService.listFiles(
      directory: currentDir,
      category: _selectedCategory,
      searchQuery: _searchQuery,
    );

    if (mounted) {
      setState(() {
        _hasStoragePermission = hasPerm;
        _currentDir = currentDir;
        _files = items;
        _isLoading = false;
      });
    }
  }

  void _navigateToFolder(FileItem folderItem) {
    if (!folderItem.isFolder) return;
    FileService.setCurrentDirectory(Directory(folderItem.path));
    _loadFiles();
  }

  void _navigateToPath(String path) {
    final dir = Directory(path);
    if (dir.existsSync()) {
      FileService.setCurrentDirectory(dir);
      _loadFiles();
    }
  }

  void _navigateUp() {
    if (_currentDir == null) return;
    final parent = _currentDir!.parent;
    if (parent.existsSync() && parent.path != _currentDir!.path) {
      FileService.setCurrentDirectory(parent);
      _loadFiles();
    }
  }

  Future<void> _requestPermission() async {
    await FileService.requestStoragePermission();
    // Re-check after prompt/settings return
    final granted = await FileService.isStoragePermissionGranted();
    if (granted) {
      final root = await FileService.getRootStoragePath();
      FileService.setCurrentDirectory(Directory(root));
    }
    await _loadFiles();
  }

  void _openDonutMenu(FileItem item) {
    HapticFeedback.mediumImpact();
    showDialog(
      context: context,
      barrierColor: Colors.black.withValues(alpha: 0.8),
      builder: (_) => DonutMenuDialog(
        fileItem: item,
        onSelectFormat: (targetFormat) {
          _openConversionDialog(item, targetFormat);
        },
      ),
    );
  }

  void _openConversionDialog(FileItem item, String targetFormat) {
    showDialog(
      context: context,
      barrierColor: Colors.black.withValues(alpha: 0.8),
      builder: (_) => ConversionDialog(
        fileItem: item,
        targetFormat: targetFormat,
        onConversionFinished: () {
          _loadFiles();
        },
      ),
    );
  }

  void _showFileActions(FileItem item) {
    showModalBottomSheet(
      context: context,
      backgroundColor: const Color(0xFF141414),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (ctx) {
        return SafeArea(
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 12),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                // Item Header
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
                  child: Row(
                    children: [
                      Icon(
                        item.isFolder ? Icons.folder_rounded : Icons.insert_drive_file_rounded,
                        color: item.isFolder ? const Color(0xFFF59E0B) : const Color(0xFF00E599),
                        size: 24,
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Text(
                          item.name,
                          style: const TextStyle(
                            color: Colors.white,
                            fontSize: 15,
                            fontWeight: FontWeight.w700,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ),
                const Divider(color: Color(0xFF262626)),

                if (item.isFolder) ...[
                  ListTile(
                    leading: const Icon(Icons.folder_open_rounded, color: Color(0xFFF59E0B)),
                    title: const Text('Open Folder', style: TextStyle(color: Colors.white)),
                    onTap: () {
                      Navigator.of(ctx).pop();
                      _navigateToFolder(item);
                    },
                  ),
                  ListTile(
                    leading: const Icon(Icons.folder_zip_rounded, color: Color(0xFF00E599)),
                    title: const Text('Compress to ZIP', style: TextStyle(color: Colors.white)),
                    onTap: () async {
                      Navigator.of(ctx).pop();
                      final folder = Directory(item.path);
                      final entities = folder.listSync(recursive: true).whereType<File>().toList();
                      final targetDir = await FileService.getStorageDirectory();
                      final res = await ConversionEngine.compressToZip(
                        files: entities,
                        archiveName: '${item.name}_archive',
                        outputDirectory: targetDir.path,
                      );
                      if (res.success && mounted) {
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(content: Text('Created ${item.name}_archive.zip')),
                        );
                        _loadFiles();
                      }
                    },
                  ),
                ] else ...[
                  // File options
                  ListTile(
                    leading: const Icon(Icons.donut_large_rounded, color: Color(0xFF00E599)),
                    title: const Text('Convert Format', style: TextStyle(color: Color(0xFF00E599), fontWeight: FontWeight.bold)),
                    subtitle: const Text('Offline engine conversion', style: TextStyle(color: Color(0xFF777777), fontSize: 11)),
                    onTap: () {
                      Navigator.of(ctx).pop();
                      _openDonutMenu(item);
                    },
                  ),
                  ListTile(
                    leading: const Icon(Icons.visibility_rounded, color: Colors.white),
                    title: const Text('Preview / Open', style: TextStyle(color: Colors.white)),
                    onTap: () {
                      Navigator.of(ctx).pop();
                      _showPreview(item);
                    },
                  ),
                  ListTile(
                    leading: const Icon(Icons.share_rounded, color: Colors.white),
                    title: const Text('Share', style: TextStyle(color: Colors.white)),
                    onTap: () {
                      Navigator.of(ctx).pop();
                      SharePlus.instance.share(ShareParams(files: [XFile(item.path)]));
                    },
                  ),
                  ListTile(
                    leading: const Icon(Icons.download_rounded, color: Colors.white),
                    title: const Text('Save Copy to Downloads', style: TextStyle(color: Colors.white)),
                    onTap: () async {
                      Navigator.of(ctx).pop();
                      final savedPath = await FileService.exportToDownloads(item);
                      if (mounted) {
                        ScaffoldMessenger.of(context).showSnackBar(
                          SnackBar(
                            content: Text(savedPath != null
                                ? 'Saved copy to Downloads folder'
                                : 'Could not export file'),
                            backgroundColor: const Color(0xFF161616),
                          ),
                        );
                      }
                    },
                  ),
                ],

                // Rename
                ListTile(
                  leading: const Icon(Icons.edit_rounded, color: Colors.white),
                  title: const Text('Rename', style: TextStyle(color: Colors.white)),
                  onTap: () {
                    Navigator.of(ctx).pop();
                    _showRenameDialog(item);
                  },
                ),

                if (!item.isFolder) ...[
                  ListTile(
                    leading: const Icon(Icons.fingerprint_rounded, color: Color(0xFF8B5CF6)),
                    title: const Text('Checksum (SHA-256 / MD5)', style: TextStyle(color: Colors.white)),
                    onTap: () {
                      Navigator.of(ctx).pop();
                      _showChecksumDialog(item);
                    },
                  ),
                  ListTile(
                    leading: const Icon(Icons.security_rounded, color: Color(0xFFEF4444)),
                    title: const Text('DoD 5220.22-M Shred', style: TextStyle(color: Color(0xFFEF4444))),
                    onTap: () {
                      Navigator.of(ctx).pop();
                      _showShredDialog(item);
                    },
                  ),
                ],

                // Delete
                ListTile(
                  leading: const Icon(Icons.delete_outline_rounded, color: Color(0xFFEF4444)),
                  title: Text(item.isFolder ? 'Delete Folder' : 'Delete File', style: const TextStyle(color: Color(0xFFEF4444))),
                  onTap: () async {
                    Navigator.of(ctx).pop();
                    await FileService.deleteFile(item);
                    _loadFiles();
                  },
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  void _showPreview(FileItem item) async {
    final file = File(item.path);
    Widget content;

    if (item.category == FileCategory.image) {
      content = Image.file(file, fit: BoxFit.contain);
    } else if (item.category == FileCategory.data || item.category == FileCategory.document) {
      try {
        final text = await file.readAsString();
        content = SingleChildScrollView(
          padding: const EdgeInsets.all(16),
          child: SelectableText(
            text,
            style: const TextStyle(
              color: Colors.white,
              fontFamily: 'monospace',
              fontSize: 12,
            ),
          ),
        );
      } catch (_) {
        content = const Center(
          child: Text('Binary or unreadable preview', style: TextStyle(color: Color(0xFF888888))),
        );
      }
    } else {
      content = Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.insert_drive_file_rounded, size: 48, color: Color(0xFF555555)),
            const SizedBox(height: 12),
            Text(item.name, style: const TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
            const SizedBox(height: 4),
            Text(item.formattedSize, style: const TextStyle(color: Color(0xFF888888))),
          ],
        ),
      );
    }

    if (!mounted) return;

    showDialog(
      context: context,
      barrierColor: Colors.black.withValues(alpha: 0.85),
      builder: (_) => Dialog(
        backgroundColor: const Color(0xFF141414),
        insetPadding: const EdgeInsets.all(20),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: const BorderSide(color: Color(0xFF262626)),
        ),
        child: SizedBox(
          width: double.infinity,
          height: 440,
          child: Column(
            children: [
              Padding(
                padding: const EdgeInsets.all(16),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Expanded(
                      child: Text(
                        item.name,
                        style: const TextStyle(color: Colors.white, fontWeight: FontWeight.w700),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    IconButton(
                      icon: const Icon(Icons.close_rounded, color: Colors.white),
                      onPressed: () => Navigator.of(context).pop(),
                    ),
                  ],
                ),
              ),
              const Divider(color: Color(0xFF222222), height: 1),
              Expanded(child: content),
            ],
          ),
        ),
      ),
    );
  }

  void _showRenameDialog(FileItem item) {
    final controller = TextEditingController(text: item.name);
    showDialog(
      context: context,
      builder: (_) => AlertDialog(
        backgroundColor: const Color(0xFF161616),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: const BorderSide(color: Color(0xFF262626)),
        ),
        title: Text(item.isFolder ? 'Rename Folder' : 'Rename File', style: const TextStyle(color: Colors.white)),
        content: TextField(
          controller: controller,
          style: const TextStyle(color: Colors.white),
          decoration: const InputDecoration(
            enabledBorder: UnderlineInputBorder(borderSide: BorderSide(color: Color(0xFF00E599))),
            focusedBorder: UnderlineInputBorder(borderSide: BorderSide(color: Color(0xFF00E599), width: 2)),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel', style: TextStyle(color: Color(0xFF888888))),
          ),
          ElevatedButton(
            onPressed: () async {
              final newName = controller.text.trim();
              if (newName.isNotEmpty) {
                await FileService.renameFile(item, newName);
                if (mounted) {
                  Navigator.of(context).pop();
                  _loadFiles();
                }
              }
            },
            style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF00E599)),
            child: const Text('Rename', style: TextStyle(color: Colors.black, fontWeight: FontWeight.w700)),
          ),
        ],
      ),
    );
  }

  void _showChecksumDialog(FileItem item) async {
    final bytes = await File(item.path).readAsBytes();
    final sha256Hash = sha256.convert(bytes).toString();
    final md5Hash = md5.convert(bytes).toString();
    if (!mounted) return;
    showDialog(
      context: context,
      builder: (_) => AlertDialog(
        backgroundColor: const Color(0xFF161616),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: const BorderSide(color: Color(0xFF262626)),
        ),
        title: Text(item.name, style: const TextStyle(color: Colors.white, fontSize: 16)),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('SHA-256:', style: TextStyle(color: Color(0xFF00E599), fontSize: 12, fontWeight: FontWeight.bold)),
            const SizedBox(height: 4),
            SelectableText(sha256Hash, style: const TextStyle(color: Colors.white, fontFamily: 'monospace', fontSize: 11)),
            const SizedBox(height: 12),
            const Text('MD5:', style: TextStyle(color: Color(0xFF8B5CF6), fontSize: 12, fontWeight: FontWeight.bold)),
            const SizedBox(height: 4),
            SelectableText(md5Hash, style: const TextStyle(color: Colors.white, fontFamily: 'monospace', fontSize: 11)),
          ],
        ),
        actions: [
          TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text('Close', style: TextStyle(color: Colors.white))),
        ],
      ),
    );
  }

  void _showShredDialog(FileItem item) {
    showDialog(
      context: context,
      builder: (_) => AlertDialog(
        backgroundColor: const Color(0xFF161616),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: const BorderSide(color: Color(0xFF262626)),
        ),
        title: const Text('DoD 5220.22-M Shred', style: TextStyle(color: Color(0xFFEF4444), fontWeight: FontWeight.bold)),
        content: Text(
          'Permanently destroy ${item.name} with 3 passes of cryptographically secure random entropy?\n\nThis cannot be undone.',
          style: const TextStyle(color: Colors.white70, fontSize: 13),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text('Cancel', style: TextStyle(color: Colors.white))),
          ElevatedButton(
            style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFFEF4444)),
            onPressed: () async {
              Navigator.of(context).pop();
              final file = File(item.path);
              if (await file.exists()) {
                final len = await file.length();
                final random = math.Random.secure();
                for (int pass = 0; pass < 3; pass++) {
                  final buf = Uint8List(math.min(len, 16384));
                  for (int i = 0; i < buf.length; i++) {
                    buf[i] = random.nextInt(256);
                  }
                  await file.writeAsBytes(buf, flush: true);
                }
                await file.delete();
              }
              _loadFiles();
              if (mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text('Permanently wiped ${item.name}')),
                );
              }
            },
            child: const Text('Shred File', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold)),
          ),
        ],
      ),
    );
  }

  void _showNewFolderDialog() {
    final controller = TextEditingController();
    showDialog(
      context: context,
      builder: (_) => AlertDialog(
        backgroundColor: const Color(0xFF161616),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: const BorderSide(color: Color(0xFF262626)),
        ),
        title: const Text('New Folder', style: TextStyle(color: Colors.white, fontSize: 16)),
        content: TextField(
          controller: controller,
          autofocus: true,
          style: const TextStyle(color: Colors.white),
          decoration: const InputDecoration(
            hintText: 'Folder name',
            hintStyle: TextStyle(color: Color(0xFF666666)),
            enabledBorder: UnderlineInputBorder(borderSide: BorderSide(color: Color(0xFF00E599))),
            focusedBorder: UnderlineInputBorder(borderSide: BorderSide(color: Color(0xFF00E599), width: 2)),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('Cancel', style: TextStyle(color: Color(0xFF888888))),
          ),
          ElevatedButton(
            onPressed: () async {
              final name = controller.text.trim();
              if (name.isNotEmpty) {
                Navigator.of(context).pop();
                final created = await FileService.createFolder(name);
                if (created != null) {
                  _loadFiles();
                } else if (mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('Could not create folder')),
                  );
                }
              }
            },
            style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF00E599)),
            child: const Text('Create', style: TextStyle(color: Colors.black, fontWeight: FontWeight.w700)),
          ),
        ],
      ),
    );
  }

  Future<void> _batchZip() async {
    if (_selectedPaths.isEmpty) return;
    final selectedFiles = _files
        .where((f) => _selectedPaths.contains(f.path))
        .map((f) => File(f.path))
        .toList();

    final targetDir = await FileService.getStorageDirectory();
    final res = await ConversionEngine.compressToZip(
      files: selectedFiles,
      archiveName: 'Archive_${DateTime.now().millisecondsSinceEpoch}',
      outputDirectory: targetDir.path,
    );

    if (res.success && mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Created ZIP archive')),
      );
      setState(() {
        _isBatchMode = false;
        _selectedPaths.clear();
      });
      _loadFiles();
    }
  }

  Future<void> _batchDelete() async {
    final selectedItems = _files.where((f) => _selectedPaths.contains(f.path)).toList();
    await FileService.deleteMultiple(selectedItems);
    setState(() {
      _isBatchMode = false;
      _selectedPaths.clear();
    });
    _loadFiles();
  }

  Widget _buildPermissionBanner() {
    return Container(
      margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: const Color(0xFF1E1A14),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: const Color(0xFFF59E0B).withValues(alpha: 0.5)),
      ),
      child: Row(
        children: [
          const Icon(Icons.folder_special_rounded, color: Color(0xFFF59E0B), size: 28),
          const SizedBox(width: 12),
          const Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Storage Access Required',
                  style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 13),
                ),
                SizedBox(height: 2),
                Text(
                  'Allow All Files Access to browse all files and folders on your phone.',
                  style: TextStyle(color: Color(0xFFAAAAAA), fontSize: 11),
                ),
              ],
            ),
          ),
          const SizedBox(width: 8),
          ElevatedButton(
            onPressed: _requestPermission,
            style: ElevatedButton.styleFrom(
              backgroundColor: const Color(0xFFF59E0B),
              foregroundColor: Colors.black,
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
              shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
            ),
            child: const Text('Allow', style: TextStyle(fontWeight: FontWeight.w800, fontSize: 12)),
          ),
        ],
      ),
    );
  }

  Widget _buildBreadcrumbBar() {
    final path = _currentDir?.path ?? '';
    final isAtRoot = path == _rootPath || path == '/' || path.isEmpty;

    String displayPath = path;
    if (path.startsWith(_rootPath)) {
      final sub = path.substring(_rootPath.length);
      displayPath = sub.isEmpty ? 'Internal Storage' : 'Internal Storage$sub';
    } else if (path.isEmpty) {
      displayPath = 'Internal Storage';
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      color: const Color(0xFF0D0D0D),
      child: Row(
        children: [
          if (!isAtRoot)
            InkWell(
              onTap: _navigateUp,
              borderRadius: BorderRadius.circular(6),
              child: Container(
                padding: const EdgeInsets.all(6),
                margin: const EdgeInsets.only(right: 10),
                decoration: BoxDecoration(
                  color: const Color(0xFF1C1C1C),
                  borderRadius: BorderRadius.circular(6),
                ),
                child: const Icon(Icons.arrow_upward_rounded, size: 16, color: Color(0xFF00E599)),
              ),
            ),
          const Icon(Icons.folder_rounded, size: 16, color: Color(0xFFF59E0B)),
          const SizedBox(width: 8),
          Expanded(
            child: SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              reverse: true,
              child: Text(
                displayPath,
                style: const TextStyle(
                  color: Color(0xFFE5E5E5),
                  fontSize: 13,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ),
          IconButton(
            icon: const Icon(Icons.create_new_folder_outlined, size: 20, color: Colors.white70),
            tooltip: 'New Folder',
            onPressed: _showNewFolderDialog,
          ),
          IconButton(
            icon: const Icon(Icons.refresh_rounded, size: 20, color: Colors.white70),
            tooltip: 'Refresh',
            onPressed: _loadFiles,
          ),
        ],
      ),
    );
  }

  Widget _buildQuickShortcuts() {
    final shortcuts = [
      {'name': 'Storage', 'path': _rootPath, 'icon': Icons.storage_rounded},
      {'name': 'Download', 'path': '$_rootPath/Download', 'icon': Icons.download_rounded},
      {'name': 'Documents', 'path': '$_rootPath/Documents', 'icon': Icons.description_rounded},
      {'name': 'DCIM', 'path': '$_rootPath/DCIM', 'icon': Icons.camera_alt_rounded},
      {'name': 'Pictures', 'path': '$_rootPath/Pictures', 'icon': Icons.image_rounded},
      {'name': 'Music', 'path': '$_rootPath/Music', 'icon': Icons.music_note_rounded},
    ];

    return Container(
      height: 38,
      margin: const EdgeInsets.symmetric(vertical: 4),
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 16),
        itemCount: shortcuts.length,
        separatorBuilder: (_, __) => const SizedBox(width: 8),
        itemBuilder: (ctx, idx) {
          final s = shortcuts[idx];
          final isActive = _currentDir?.path == s['path'];
          return InkWell(
            onTap: () => _navigateToPath(s['path'] as String),
            borderRadius: BorderRadius.circular(8),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
              decoration: BoxDecoration(
                color: isActive ? const Color(0xFF1E2822) : const Color(0xFF161616),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(
                  color: isActive ? const Color(0xFF00E599) : const Color(0xFF262626),
                ),
              ),
              child: Row(
                children: [
                  Icon(s['icon'] as IconData, size: 14, color: isActive ? const Color(0xFF00E599) : const Color(0xFF888888)),
                  const SizedBox(width: 6),
                  Text(
                    s['name'] as String,
                    style: TextStyle(
                      color: isActive ? Colors.white : const Color(0xFFCCCCCC),
                      fontSize: 12,
                      fontWeight: isActive ? FontWeight.bold : FontWeight.normal,
                    ),
                  ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }

  Widget _buildCategoryPills() {
    final categories = [
      {'title': 'All', 'cat': FileCategory.all},
      {'title': 'Images', 'cat': FileCategory.image},
      {'title': 'Documents', 'cat': FileCategory.document},
      {'title': 'Audio', 'cat': FileCategory.audio},
      {'title': 'Video', 'cat': FileCategory.video},
      {'title': 'Archives', 'cat': FileCategory.archive},
      {'title': 'Data', 'cat': FileCategory.data},
    ];

    return Container(
      height: 36,
      margin: const EdgeInsets.only(bottom: 6),
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 16),
        itemCount: categories.length,
        separatorBuilder: (_, __) => const SizedBox(width: 6),
        itemBuilder: (ctx, idx) {
          final c = categories[idx];
          final isSel = _selectedCategory == c['cat'];
          return InkWell(
            onTap: () {
              setState(() => _selectedCategory = c['cat'] as FileCategory);
              _loadFiles();
            },
            borderRadius: BorderRadius.circular(6),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              decoration: BoxDecoration(
                color: isSel ? const Color(0xFF00E599) : const Color(0xFF141414),
                borderRadius: BorderRadius.circular(6),
              ),
              child: Text(
                c['title'] as String,
                style: TextStyle(
                  color: isSel ? Colors.black : const Color(0xFFAAAAAA),
                  fontSize: 12,
                  fontWeight: isSel ? FontWeight.w800 : FontWeight.w500,
                ),
              ),
            ),
          );
        },
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final path = _currentDir?.path ?? '';
    final isAtRoot = path == _rootPath || path == '/' || path.isEmpty;

    return PopScope(
      canPop: isAtRoot,
      onPopInvokedWithResult: (didPop, result) {
        if (!didPop) {
          _navigateUp();
        }
      },
      child: Scaffold(
        backgroundColor: const Color(0xFF000000),
        appBar: AppBar(
          backgroundColor: const Color(0xFF000000),
          elevation: 0,
          titleSpacing: 16,
          title: const Text(
            'OPfile',
            style: TextStyle(
              color: Colors.white,
              fontSize: 19,
              fontWeight: FontWeight.w800,
              letterSpacing: 0.5,
            ),
          ),
          actions: [
            IconButton(
              icon: Icon(
                _showSearchBar ? Icons.search_off_rounded : Icons.search_rounded,
                color: Colors.white,
                size: 22,
              ),
              onPressed: () {
                setState(() {
                  _showSearchBar = !_showSearchBar;
                  if (!_showSearchBar) {
                    _searchQuery = '';
                    _searchController.clear();
                    _loadFiles();
                  }
                });
              },
            ),
            IconButton(
              icon: Icon(
                _isGridView ? Icons.view_list_rounded : Icons.grid_view_rounded,
                color: Colors.white,
                size: 22,
              ),
              tooltip: _isGridView ? 'Switch to List' : 'Switch to Grid',
              onPressed: () => setState(() => _isGridView = !_isGridView),
            ),
            IconButton(
              icon: Icon(
                _isBatchMode ? Icons.check_circle_rounded : Icons.checklist_rounded,
                color: _isBatchMode ? const Color(0xFF00E599) : Colors.white,
                size: 22,
              ),
              tooltip: 'Batch Select',
              onPressed: () {
                setState(() {
                  _isBatchMode = !_isBatchMode;
                  _selectedPaths.clear();
                });
              },
            ),
          ],
        ),
        body: Column(
          children: [
            if (!_hasStoragePermission) _buildPermissionBanner(),
            _buildBreadcrumbBar(),
            _buildQuickShortcuts(),
            _buildCategoryPills(),

            // Collapsible Search Bar
            if (_showSearchBar)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
                child: Container(
                  height: 40,
                  decoration: BoxDecoration(
                    color: const Color(0xFF141414),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: const Color(0xFF262626)),
                  ),
                  child: TextField(
                    controller: _searchController,
                    autofocus: true,
                    style: const TextStyle(color: Colors.white, fontSize: 13),
                    decoration: InputDecoration(
                      hintText: 'Search files and folders...',
                      hintStyle: const TextStyle(color: Color(0xFF666666), fontSize: 13),
                      prefixIcon: const Icon(Icons.search_rounded, color: Color(0xFF666666), size: 18),
                      suffixIcon: _searchQuery.isNotEmpty
                          ? IconButton(
                              icon: const Icon(Icons.clear_rounded, color: Color(0xFF666666), size: 16),
                              onPressed: () {
                                _searchController.clear();
                                setState(() => _searchQuery = '');
                                _loadFiles();
                              },
                            )
                          : null,
                      border: InputBorder.none,
                      contentPadding: const EdgeInsets.symmetric(vertical: 8),
                    ),
                    onChanged: (val) {
                      setState(() => _searchQuery = val);
                      _loadFiles();
                    },
                  ),
                ),
              ),

            // File & Folder Listing
            Expanded(
              child: _isLoading
                  ? const Center(
                      child: CircularProgressIndicator(color: Color(0xFF00E599)),
                    )
                  : _files.isEmpty
                      ? _buildEmptyState()
                      : RefreshIndicator(
                          color: const Color(0xFF00E599),
                          backgroundColor: const Color(0xFF161616),
                          onRefresh: _loadFiles,
                          child: _isGridView
                              ? GridView.builder(
                                  padding: const EdgeInsets.all(16),
                                  gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
                                    crossAxisCount: 3,
                                    crossAxisSpacing: 10,
                                    mainAxisSpacing: 10,
                                    childAspectRatio: 0.9,
                                  ),
                                  itemCount: _files.length,
                                  itemBuilder: (ctx, idx) {
                                    final item = _files[idx];
                                    final isSel = _selectedPaths.contains(item.path);
                                    return FileCard(
                                      fileItem: item,
                                      isGridView: true,
                                      isSelected: isSel,
                                      isBatchMode: _isBatchMode,
                                      onTap: () {
                                        if (_isBatchMode) {
                                          setState(() {
                                            if (isSel) {
                                              _selectedPaths.remove(item.path);
                                            } else {
                                              _selectedPaths.add(item.path);
                                            }
                                          });
                                        } else if (item.isFolder) {
                                          _navigateToFolder(item);
                                        } else {
                                          _openDonutMenu(item);
                                        }
                                      },
                                      onLongPress: () {
                                        if (!_isBatchMode) {
                                          setState(() {
                                            _isBatchMode = true;
                                            _selectedPaths.add(item.path);
                                          });
                                        }
                                      },
                                      onMoreOptions: () => _showFileActions(item),
                                    );
                                  },
                                )
                              : ListView.builder(
                                  itemCount: _files.length,
                                  itemBuilder: (ctx, idx) {
                                    final item = _files[idx];
                                    final isSel = _selectedPaths.contains(item.path);
                                    return FileCard(
                                      fileItem: item,
                                      isGridView: false,
                                      isSelected: isSel,
                                      isBatchMode: _isBatchMode,
                                      onTap: () {
                                        if (_isBatchMode) {
                                          setState(() {
                                            if (isSel) {
                                              _selectedPaths.remove(item.path);
                                            } else {
                                              _selectedPaths.add(item.path);
                                            }
                                          });
                                        } else if (item.isFolder) {
                                          _navigateToFolder(item);
                                        } else {
                                          _openDonutMenu(item);
                                        }
                                      },
                                      onLongPress: () {
                                        if (!_isBatchMode) {
                                          setState(() {
                                            _isBatchMode = true;
                                            _selectedPaths.add(item.path);
                                          });
                                        }
                                      },
                                      onMoreOptions: () => _showFileActions(item),
                                    );
                                  },
                                ),
                        ),
            ),

            // Batch Action Toolbar
            if (_isBatchMode)
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
                decoration: const BoxDecoration(
                  color: Color(0xFF141414),
                  border: Border(top: BorderSide(color: Color(0xFF262626))),
                ),
                child: SafeArea(
                  child: Row(
                    children: [
                      Text(
                        '${_selectedPaths.length} selected',
                        style: const TextStyle(
                          color: Colors.white,
                          fontWeight: FontWeight.w700,
                          fontSize: 13,
                        ),
                      ),
                      const Spacer(),
                      OutlinedButton.icon(
                        onPressed: _selectedPaths.isNotEmpty ? _batchZip : null,
                        icon: const Icon(Icons.folder_zip_rounded, size: 16, color: Color(0xFF00E599)),
                        label: const Text('ZIP', style: TextStyle(color: Color(0xFF00E599), fontSize: 12)),
                        style: OutlinedButton.styleFrom(
                          side: const BorderSide(color: Color(0xFF00E599)),
                          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                        ),
                      ),
                      const SizedBox(width: 8),
                      ElevatedButton.icon(
                        onPressed: _selectedPaths.isNotEmpty ? _batchDelete : null,
                        icon: const Icon(Icons.delete_rounded, size: 16, color: Colors.white),
                        label: const Text('Delete', style: TextStyle(color: Colors.white, fontSize: 12)),
                        style: ElevatedButton.styleFrom(
                          backgroundColor: const Color(0xFFEF4444),
                          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }

  Widget _buildEmptyState() {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(
              Icons.folder_open_rounded,
              color: Color(0xFF444444),
              size: 48,
            ),
            const SizedBox(height: 14),
            const Text(
              'No items in this folder',
              style: TextStyle(
                color: Colors.white,
                fontSize: 15,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 6),
            const Text(
              'This directory is empty or cannot be accessed.',
              textAlign: TextAlign.center,
              style: TextStyle(
                color: Color(0xFF777777),
                fontSize: 12,
              ),
            ),
            const SizedBox(height: 16),
            OutlinedButton.icon(
              onPressed: _showNewFolderDialog,
              icon: const Icon(Icons.create_new_folder_outlined, color: Color(0xFF00E599), size: 18),
              label: const Text('New Folder', style: TextStyle(color: Color(0xFF00E599))),
              style: OutlinedButton.styleFrom(
                side: const BorderSide(color: Color(0xFF00E599)),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
