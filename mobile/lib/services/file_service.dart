import 'dart:io';
import 'package:file_picker/file_picker.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';
import '../models/file_item.dart';

class FileService {
  static Directory? _appDir;

  static Future<Directory> getStorageDirectory() async {
    if (_appDir != null && _appDir!.existsSync()) {
      return _appDir!;
    }
    final baseDir = await getApplicationDocumentsDirectory();
    final opDir = Directory(p.join(baseDir.path, 'opfile_files'));
    if (!opDir.existsSync()) {
      opDir.createSync(recursive: true);
    }
    _appDir = opDir;
    return opDir;
  }

  /// Lists all files in the OPfile directory.
  static Future<List<FileItem>> listFiles({
    FileCategory category = FileCategory.all,
    String searchQuery = '',
  }) async {
    final dir = await getStorageDirectory();
    final entities = dir.listSync();
    final files = <FileItem>[];

    for (final entity in entities) {
      if (entity is File) {
        final item = FileItem.fromFile(entity);

        // Filter by category
        if (category != FileCategory.all && item.category != category) {
          continue;
        }

        // Filter by search query
        if (searchQuery.isNotEmpty &&
            !item.name.toLowerCase().contains(searchQuery.toLowerCase())) {
          continue;
        }

        files.add(item);
      }
    }

    // Sort newest first
    files.sort((a, b) => b.modified.compareTo(a.modified));
    return files;
  }

  /// Opens the native Android file picker to import files from device storage.
  static Future<List<FileItem>> importFromDevice() async {
    final result = await FilePicker.pickFiles(
      allowMultiple: true,
      type: FileType.any,
    );

    if (result == null || result.files.isEmpty) {
      return [];
    }

    final targetDir = await getStorageDirectory();
    final imported = <FileItem>[];

    for (final picked in result.files) {
      if (picked.path != null) {
        final sourceFile = File(picked.path!);
        if (sourceFile.existsSync()) {
          final destPath = p.join(targetDir.path, picked.name);
          final copied = await sourceFile.copy(destPath);
          imported.add(FileItem.fromFile(copied));
        }
      }
    }

    return imported;
  }

  /// Renames a file.
  static Future<FileItem?> renameFile(FileItem item, String newName) async {
    final file = File(item.path);
    if (!file.existsSync()) return null;

    final ext = item.extension.isNotEmpty ? '.${item.extension}' : '';
    final finalName = newName.endsWith(ext) ? newName : '$newName$ext';
    final newPath = p.join(file.parent.path, finalName);

    final renamed = await file.rename(newPath);
    return FileItem.fromFile(renamed);
  }

  /// Deletes a file.
  static Future<bool> deleteFile(FileItem item) async {
    final file = File(item.path);
    if (file.existsSync()) {
      await file.delete();
      return true;
    }
    return false;
  }

  /// Deletes multiple files.
  static Future<int> deleteMultiple(List<FileItem> items) async {
    int deleted = 0;
    for (final item in items) {
      final file = File(item.path);
      if (file.existsSync()) {
        await file.delete();
        deleted++;
      }
    }
    return deleted;
  }

  /// Copies a file to the device's public Downloads directory.
  static Future<String?> exportToDownloads(FileItem item) async {
    try {
      final source = File(item.path);
      if (!source.existsSync()) return null;

      Directory? downloadDir;
      if (Platform.isAndroid) {
        downloadDir = Directory('/storage/emulated/0/Download');
        if (!downloadDir.existsSync()) {
          downloadDir = await getExternalStorageDirectory();
        }
      } else {
        downloadDir = await getDownloadsDirectory();
      }

      if (downloadDir == null || !downloadDir.existsSync()) {
        downloadDir = await getApplicationDocumentsDirectory();
      }

      final destPath = p.join(downloadDir.path, item.name);
      await source.copy(destPath);
      return destPath;
    } catch (_) {
      return null;
    }
  }

  /// Populates initial demo files if directory is empty.
  static Future<void> ensureInitialSamples() async {
    final dir = await getStorageDirectory();
    final existing = dir.listSync();
    if (existing.isNotEmpty) return;

    // 1. Sample document
    final docFile = File(p.join(dir.path, 'Welcome_to_OPfile.md'));
    await docFile.writeAsString('''# Welcome to OPfile 📱
100% Offline Mobile File Manager & Converter.

## Highlights
- **Tap this file** to test the Donut radial menu!
- Tap the **+ Import** button to pick files from your device storage.
- All conversions are completely offline on your device.
''');

    // 2. Sample data JSON
    final jsonFile = File(p.join(dir.path, 'sample_data.json'));
    await jsonFile.writeAsString('''[
  {"id": 1, "format": "PNG", "category": "Image", "status": "Ready"},
  {"id": 2, "format": "JPG", "category": "Image", "status": "Ready"},
  {"id": 3, "format": "ZIP", "category": "Archive", "status": "Ready"},
  {"id": 4, "format": "CSV", "category": "Data", "status": "Ready"}
]''');
  }
}
