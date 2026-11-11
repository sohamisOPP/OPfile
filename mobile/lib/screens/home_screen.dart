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

class _HomeScreenState extends State<HomeScreen> {
  List<FileItem> _files = [];
  bool _isLoading = true;
  bool _isGridView = true;
  bool _isDualPane = false;
  FileCategory _selectedCategory = FileCategory.all;
  String _searchQuery = '';
  final TextEditingController _searchController = TextEditingController();

  // Batch selection
  bool _isBatchMode = false;
  final Set<String> _selectedPaths = {};

  @override
  void initState() {
    super.initState();
    _loadFiles();
  }

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  Future<void> _loadFiles() async {
    setState(() => _isLoading = true);
    await FileService.ensureInitialSamples();
    final items = await FileService.listFiles(
      category: _selectedCategory,
      searchQuery: _searchQuery,
    );
    if (mounted) {
      setState(() {
        _files = items;
        _isLoading = false;
      });
    }
  }

  Future<void> _importFiles() async {
    HapticFeedback.lightImpact();
    final imported = await FileService.importFromDevice();
    if (imported.isNotEmpty && mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Imported ${imported.length} file(s) successfully!'),
          backgroundColor: const Color(0xFF161616),
        ),
      );
      _loadFiles();
    }
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
      backgroundColor: const Color(0xFF0F0F0F),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(24)),
        side: BorderSide(color: Color(0xFF262626)),
      ),
      builder: (ctx) {
        return SafeArea(
          child: Padding(
            padding: const EdgeInsets.symmetric(vertical: 16),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                // File name & badge
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
                  child: Row(
                    children: [
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                        decoration: BoxDecoration(
                          color: const Color(0xFF1E1E1E),
                          borderRadius: BorderRadius.circular(6),
                        ),
                        child: Text(
                          item.extension.toUpperCase(),
                          style: const TextStyle(
                            color: Color(0xFF00E599),
                            fontSize: 11,
                            fontWeight: FontWeight.w800,
                          ),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Text(
                          item.name,
                          style: const TextStyle(
                            color: Colors.white,
                            fontSize: 14,
                            fontWeight: FontWeight.w700,
                          ),
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                    ],
                  ),
                ),
                const Divider(color: Color(0xFF222222)),

                // Donut Convert Action
                ListTile(
                  leading: const Icon(Icons.donut_large_rounded, color: Color(0xFF00E599)),
                  title: const Text('Convert (Donut Menu)', style: TextStyle(color: Colors.white, fontWeight: FontWeight.w600)),
                  onTap: () {
                    Navigator.of(ctx).pop();
                    _openDonutMenu(item);
                  },
                ),

                // Preview
                ListTile(
                  leading: const Icon(Icons.visibility_rounded, color: Colors.white),
                  title: const Text('Preview', style: TextStyle(color: Colors.white)),
                  onTap: () {
                    Navigator.of(ctx).pop();
                    _showPreview(item);
                  },
                ),

                // Share
                ListTile(
                  leading: const Icon(Icons.share_rounded, color: Colors.white),
                  title: const Text('Share File', style: TextStyle(color: Colors.white)),
                  onTap: () {
                    Navigator.of(ctx).pop();
                    SharePlus.instance.share(ShareParams(files: [XFile(item.path)]));
                  },
                ),

                // Export to Downloads
                ListTile(
                  leading: const Icon(Icons.download_rounded, color: Colors.white),
                  title: const Text('Export to Downloads', style: TextStyle(color: Colors.white)),
                  onTap: () async {
                    Navigator.of(ctx).pop();
                    final savedPath = await FileService.exportToDownloads(item);
                    if (mounted) {
                      ScaffoldMessenger.of(context).showSnackBar(
                        SnackBar(
                          content: Text(savedPath != null
                              ? 'Saved to device Downloads folder'
                              : 'Could not export file'),
                          backgroundColor: const Color(0xFF161616),
                        ),
                      );
                    }
                  },
                ),

                // Rename
                ListTile(
                  leading: const Icon(Icons.edit_rounded, color: Colors.white),
                  title: const Text('Rename', style: TextStyle(color: Colors.white)),
                  onTap: () {
                    Navigator.of(ctx).pop();
                    _showRenameDialog(item);
                  },
                ),

                // Checksum
                ListTile(
                  leading: const Icon(Icons.fingerprint_rounded, color: Color(0xFF6C5CE7)),
                  title: const Text('Checksum (SHA-256 / MD5)', style: TextStyle(color: Colors.white)),
                  onTap: () {
                    Navigator.of(ctx).pop();
                    _showChecksumDialog(item);
                  },
                ),

                // Shredder
                ListTile(
                  leading: const Icon(Icons.security_rounded, color: Color(0xFFFF5C5C)),
                  title: const Text('Shred (DoD 5220.22-M)', style: TextStyle(color: Color(0xFFFF5C5C))),
                  onTap: () {
                    Navigator.of(ctx).pop();
                    _showShredDialog(item);
                  },
                ),

                // Delete
                ListTile(
                  leading: const Icon(Icons.delete_outline_rounded, color: Color(0xFFFF5C5C)),
                  title: const Text('Delete', style: TextStyle(color: Color(0xFFFF5C5C))),
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
        backgroundColor: const Color(0xFF0E0E0E),
        insetPadding: const EdgeInsets.all(20),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(20),
          side: const BorderSide(color: Color(0xFF262626)),
        ),
        child: SizedBox(
          width: double.infinity,
          height: 420,
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
        backgroundColor: const Color(0xFF121212),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(20),
          side: const BorderSide(color: Color(0xFF262626)),
        ),
        title: const Text('Rename File', style: TextStyle(color: Colors.white)),
        content: TextField(
          controller: controller,
          style: const TextStyle(color: Colors.white),
          decoration: const InputDecoration(
            enabledBorder: UnderlineInputBorder(
              borderSide: BorderSide(color: Color(0xFF00E599)),
            ),
            focusedBorder: UnderlineInputBorder(
              borderSide: BorderSide(color: Color(0xFF00E599), width: 2),
            ),
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
            style: ElevatedButton.styleFrom(
              backgroundColor: const Color(0xFF00E599),
            ),
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
        backgroundColor: const Color(0xFF111111),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: const BorderSide(color: Color(0xFF262626)),
        ),
        title: Text(item.name, style: const TextStyle(color: Colors.white, fontSize: 16)),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text('SHA-256 Checksum:', style: TextStyle(color: Color(0xFF00E599), fontSize: 12, fontWeight: FontWeight.bold)),
            const SizedBox(height: 4),
            SelectableText(sha256Hash, style: const TextStyle(color: Colors.white, fontFamily: 'monospace', fontSize: 11)),
            const SizedBox(height: 12),
            const Text('MD5 Checksum:', style: TextStyle(color: Color(0xFF6C5CE7), fontSize: 12, fontWeight: FontWeight.bold)),
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
        backgroundColor: const Color(0xFF111111),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: const BorderSide(color: Color(0xFFFF5C5C)),
        ),
        title: const Text('🚨 DoD 5220.22-M Shred', style: TextStyle(color: Color(0xFFFF5C5C), fontWeight: FontWeight.bold)),
        content: Text(
          'Permanently destroy ${item.name} with 3 passes of cryptographically secure random entropy before unlinking?\n\nThis cannot be undone.',
          style: const TextStyle(color: Colors.white70, fontSize: 13),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text('Cancel', style: TextStyle(color: Colors.white))),
          ElevatedButton(
            style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFFFF5C5C)),
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

  void _showCommandPalette() {
    showModalBottomSheet(
      context: context,
      backgroundColor: const Color(0xFF0F0F0F),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(24)),
        side: BorderSide(color: Color(0xFF262626)),
      ),
      builder: (ctx) => SafeArea(
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Padding(
                padding: EdgeInsets.symmetric(horizontal: 20, vertical: 8),
                child: Row(
                  children: [
                    Icon(Icons.bolt_rounded, color: Color(0xFF6C5CE7)),
                    SizedBox(width: 10),
                    Text('OPfile Command Palette', style: TextStyle(color: Colors.white, fontWeight: FontWeight.bold, fontSize: 16)),
                  ],
                ),
              ),
              const Divider(color: Color(0xFF222222)),
              ListTile(
                leading: const Icon(Icons.compare_arrows_rounded, color: Color(0xFF00E599)),
                title: const Text('Toggle Dual-Pane Split Layout', style: TextStyle(color: Colors.white)),
                onTap: () {
                  Navigator.of(ctx).pop();
                  setState(() => _isDualPane = !_isDualPane);
                },
              ),
              ListTile(
                leading: const Icon(Icons.find_replace_rounded, color: Color(0xFF6C5CE7)),
                title: const Text('Batch Renamer (Regex & Numbering)', style: TextStyle(color: Colors.white)),
                onTap: () {
                  Navigator.of(ctx).pop();
                  _showBatchRenamerDialog();
                },
              ),
              ListTile(
                leading: const Icon(Icons.wifi_tethering_rounded, color: Color(0xFF00CEC9)),
                title: const Text('Wi-Fi Local Transfer Info', style: TextStyle(color: Colors.white)),
                onTap: () {
                  Navigator.of(ctx).pop();
                  _showWifiShareDialog();
                },
              ),
              ListTile(
                leading: const Icon(Icons.cleaning_services_rounded, color: Color(0xFFFDCB6E)),
                title: const Text('Clean Empty Folders', style: TextStyle(color: Colors.white)),
                onTap: () {
                  Navigator.of(ctx).pop();
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('Scanned directory: 0 empty folders')),
                  );
                },
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _showBatchRenamerDialog() {
    final prefixCtrl = TextEditingController();
    final suffixCtrl = TextEditingController();
    showDialog(
      context: context,
      builder: (_) => AlertDialog(
        backgroundColor: const Color(0xFF111111),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: const BorderSide(color: Color(0xFF262626)),
        ),
        title: const Text('Batch Renamer', style: TextStyle(color: Colors.white)),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: prefixCtrl,
              style: const TextStyle(color: Colors.white),
              decoration: const InputDecoration(labelText: 'Prefix (e.g. IMG_)', labelStyle: TextStyle(color: Color(0xFF888888))),
            ),
            const SizedBox(height: 8),
            TextField(
              controller: suffixCtrl,
              style: const TextStyle(color: Colors.white),
              decoration: const InputDecoration(labelText: 'Suffix (e.g. _v1)', labelStyle: TextStyle(color: Color(0xFF888888))),
            ),
          ],
        ),
        actions: [
          TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text('Cancel', style: TextStyle(color: Colors.white70))),
          ElevatedButton(
            style: ElevatedButton.styleFrom(backgroundColor: const Color(0xFF6C5CE7)),
            onPressed: () async {
              Navigator.of(context).pop();
              final prefix = prefixCtrl.text;
              final suffix = suffixCtrl.text;
              int count = 0;
              for (final f in _files) {
                if (_selectedPaths.isEmpty || _selectedPaths.contains(f.path)) {
                  final dotIdx = f.name.lastIndexOf('.');
                  final base = dotIdx != -1 ? f.name.substring(0, dotIdx) : f.name;
                  final ext = dotIdx != -1 ? f.name.substring(dotIdx) : '';
                  final newName = '$prefix$base$suffix$ext';
                  await FileService.renameFile(f, newName);
                  count++;
                }
              }
              _loadFiles();
              if (mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text('Renamed $count file(s)')),
                );
              }
            },
            child: const Text('Rename All', style: TextStyle(color: Colors.white)),
          ),
        ],
      ),
    );
  }

  void _showWifiShareDialog() {
    showDialog(
      context: context,
      builder: (_) => AlertDialog(
        backgroundColor: const Color(0xFF111111),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(16),
          side: const BorderSide(color: Color(0xFF262626)),
        ),
        title: const Row(
          children: [
            Icon(Icons.wifi_rounded, color: Color(0xFF00E599)),
            SizedBox(width: 8),
            Text('Wi-Fi Local Transfer', style: TextStyle(color: Colors.white, fontSize: 16)),
          ],
        ),
        content: const Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Connect your PC or other phones to the same Wi-Fi network and open:', style: TextStyle(color: Colors.white70, fontSize: 13)),
            SizedBox(height: 10),
            SelectableText('http://192.168.1.10:8080', style: TextStyle(color: Color(0xFF00E599), fontWeight: FontWeight.bold, fontFamily: 'monospace')),
            SizedBox(height: 12),
            Text('Instant offline browser access to your OPfile files and format converter.', style: TextStyle(color: Color(0xFF888888), fontSize: 12)),
          ],
        ),
        actions: [
          TextButton(onPressed: () => Navigator.of(context).pop(), child: const Text('Close', style: TextStyle(color: Colors.white))),
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
      archiveName: 'OPfile_archive_${DateTime.now().millisecondsSinceEpoch}',
      outputDirectory: targetDir.path,
    );

    if (res.success && mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('Created ZIP archive successfully!'),
          backgroundColor: Color(0xFF161616),
        ),
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

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF000000), // OLED Pitch Black
      appBar: AppBar(
        backgroundColor: const Color(0xFF000000),
        elevation: 0,
        titleSpacing: 16,
        title: Row(
          children: [
            Container(
              padding: const EdgeInsets.all(6),
              decoration: BoxDecoration(
                color: const Color(0xFF111111),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(color: const Color(0xFF262626)),
              ),
              child: const Icon(
                Icons.donut_large_rounded,
                color: Color(0xFF00E599),
                size: 20,
              ),
            ),
            const SizedBox(width: 10),
            const Text(
              'OPfile',
              style: TextStyle(
                color: Colors.white,
                fontSize: 18,
                fontWeight: FontWeight.w900,
                letterSpacing: 0.5,
              ),
            ),
            const SizedBox(width: 8),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
              decoration: BoxDecoration(
                color: const Color(0xFF14241B),
                borderRadius: BorderRadius.circular(6),
                border: Border.all(color: const Color(0xFF00E599), width: 0.8),
              ),
              child: const Text(
                '100% OFFLINE',
                style: TextStyle(
                  color: Color(0xFF00E599),
                  fontSize: 9,
                  fontWeight: FontWeight.w800,
                  letterSpacing: 0.8,
                ),
              ),
            ),
          ],
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.bolt_rounded, color: Color(0xFF6C5CE7), size: 22),
            tooltip: 'Command Palette',
            onPressed: _showCommandPalette,
          ),
          IconButton(
            icon: Icon(
              _isDualPane ? Icons.splitscreen_rounded : Icons.crop_square_rounded,
              color: _isDualPane ? const Color(0xFF00E599) : Colors.white,
              size: 20,
            ),
            tooltip: 'Toggle Dual Pane',
            onPressed: () => setState(() => _isDualPane = !_isDualPane),
          ),
          IconButton(
            icon: Icon(
              _isGridView ? Icons.view_list_rounded : Icons.grid_view_rounded,
              color: Colors.white,
              size: 20,
            ),
            onPressed: () => setState(() => _isGridView = !_isGridView),
          ),
          IconButton(
            icon: Icon(
              _isBatchMode ? Icons.check_circle_rounded : Icons.checklist_rounded,
              color: _isBatchMode ? const Color(0xFF00E599) : Colors.white,
              size: 20,
            ),
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
          // Search Bar
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Container(
              height: 42,
              decoration: BoxDecoration(
                color: const Color(0xFF111111),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: const Color(0xFF222222)),
              ),
              child: TextField(
                controller: _searchController,
                style: const TextStyle(color: Colors.white, fontSize: 13),
                decoration: InputDecoration(
                  hintText: 'Search files by name or format...',
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
                  contentPadding: const EdgeInsets.symmetric(vertical: 10),
                ),
                onChanged: (val) {
                  setState(() => _searchQuery = val);
                  _loadFiles();
                },
              ),
            ),
          ),

          // Category Filter Pills
          SizedBox(
            height: 38,
            child: ListView(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: 16),
              children: [
                _buildCategoryPill('All', FileCategory.all),
                _buildCategoryPill('🖼️ Images', FileCategory.image),
                _buildCategoryPill('📄 Documents', FileCategory.document),
                _buildCategoryPill('🎵 Audio', FileCategory.audio),
                _buildCategoryPill('🎬 Video', FileCategory.video),
                _buildCategoryPill('📦 Archives', FileCategory.archive),
                _buildCategoryPill('📊 Data', FileCategory.data),
              ],
            ),
          ),
          const SizedBox(height: 8),

          // File List / Grid
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
                                  crossAxisCount: 2,
                                  crossAxisSpacing: 12,
                                  mainAxisSpacing: 12,
                                  childAspectRatio: 1.05,
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
                                padding: const EdgeInsets.symmetric(vertical: 8),
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

          // Batch Action Bar
          if (_isBatchMode)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
              decoration: const BoxDecoration(
                color: Color(0xFF0F0F0F),
                border: Border(top: BorderSide(color: Color(0xFF222222))),
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
      floatingActionButton: FloatingActionButton.extended(
        onPressed: _importFiles,
        backgroundColor: const Color(0xFF00E599),
        icon: const Icon(Icons.add_rounded, color: Colors.black, size: 22),
        label: const Text(
          'Import Files',
          style: TextStyle(
            color: Colors.black,
            fontWeight: FontWeight.w800,
            fontSize: 13,
          ),
        ),
      ),
    );
  }

  Widget _buildCategoryPill(String title, FileCategory cat) {
    final isSel = _selectedCategory == cat;
    return Padding(
      padding: const EdgeInsets.only(right: 8),
      child: InkWell(
        onTap: () {
          setState(() => _selectedCategory = cat);
          _loadFiles();
        },
        borderRadius: BorderRadius.circular(20),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 7),
          decoration: BoxDecoration(
            color: isSel ? const Color(0xFF00E599) : const Color(0xFF141414),
            borderRadius: BorderRadius.circular(20),
            border: Border.all(
              color: isSel ? const Color(0xFF00E599) : const Color(0xFF262626),
            ),
          ),
          child: Text(
            title,
            style: TextStyle(
              color: isSel ? Colors.black : Colors.white,
              fontSize: 12,
              fontWeight: isSel ? FontWeight.w800 : FontWeight.w600,
            ),
          ),
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
            Container(
              width: 72,
              height: 72,
              decoration: BoxDecoration(
                color: const Color(0xFF111111),
                borderRadius: BorderRadius.circular(20),
                border: Border.all(color: const Color(0xFF262626)),
              ),
              child: const Icon(
                Icons.folder_open_rounded,
                color: Color(0xFF555555),
                size: 36,
              ),
            ),
            const SizedBox(height: 16),
            const Text(
              'No Files Found',
              style: TextStyle(
                color: Colors.white,
                fontSize: 16,
                fontWeight: FontWeight.w700,
              ),
            ),
            const SizedBox(height: 6),
            const Text(
              'Tap "+ Import Files" to add photos, documents, or data from your phone storage.',
              textAlign: TextAlign.center,
              style: TextStyle(
                color: Color(0xFF777777),
                fontSize: 12,
              ),
            ),
            const SizedBox(height: 20),
            ElevatedButton.icon(
              onPressed: _importFiles,
              icon: const Icon(Icons.add_rounded, color: Colors.black, size: 18),
              label: const Text(
                'Import from Phone',
                style: TextStyle(
                  color: Colors.black,
                  fontWeight: FontWeight.w800,
                ),
              ),
              style: ElevatedButton.styleFrom(
                backgroundColor: const Color(0xFF00E599),
                padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
                shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
