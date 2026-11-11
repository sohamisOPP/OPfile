import 'dart:io';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/services.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';
import '../models/file_item.dart';

class FileService {
  static const MethodChannel _channel = MethodChannel('com.opfile/storage_permission');
  static Directory? _currentDir;
  static String? _rootStoragePath;

  /// Check whether MANAGE_EXTERNAL_STORAGE permission is granted (Android 11+ / API 30+)
  static Future<bool> isStoragePermissionGranted() async {
    try {
      if (Platform.isAndroid) {
        final bool? granted = await _channel.invokeMethod<bool>('isStoragePermissionGranted');
        return granted ?? false;
      }
      return true;
    } catch (_) {
      return false;
    }
  }

  /// Request MANAGE_EXTERNAL_STORAGE permission by opening Android Settings
  static Future<bool> requestStoragePermission() async {
    try {
      if (Platform.isAndroid) {
        final bool? res = await _channel.invokeMethod<bool>('requestStoragePermission');
        return res ?? false;
      }
      return true;
    } catch (_) {
      return false;
    }
  }

  /// Get the root storage directory (/storage/emulated/0)
  static Future<String> getRootStoragePath() async {
    if (_rootStoragePath != null) return _rootStoragePath!;
    try {
      if (Platform.isAndroid) {
        final String? path = await _channel.invokeMethod<String>('getRootStoragePath');
        if (path != null && Directory(path).existsSync()) {
          _rootStoragePath = path;
          return path;
        }
      }
    } catch (_) {}
    return '/storage/emulated/0';
  }

  /// Get the active browsing directory
  static Future<Directory> getCurrentDirectory() async {
    if (_currentDir != null && _currentDir!.existsSync()) {
      return _currentDir!;
    }
    final hasPermission = await isStoragePermissionGranted();
    if (hasPermission) {
      final rootPath = await getRootStoragePath();
      final rootDir = Directory(rootPath);
      if (rootDir.existsSync()) {
        _currentDir = rootDir;
        return rootDir;
      }
    }
    // Fallback to app documents if permission not granted
    final baseDir = await getApplicationDocumentsDirectory();
    final opDir = Directory(p.join(baseDir.path, 'opfile_files'));
    if (!opDir.existsSync()) {
      opDir.createSync(recursive: true);
    }
    _currentDir = opDir;
    return opDir;
  }

  static void setCurrentDirectory(Directory dir) {
    _currentDir = dir;
  }

  static Future<Directory> getStorageDirectory() async {
    return getCurrentDirectory();
  }

  /// Lists all folders and files in the target or current directory.
  static Future<List<FileItem>> listFiles({
    Directory? directory,
    FileCategory category = FileCategory.all,
    String searchQuery = '',
  }) async {
    final dir = directory ?? await getCurrentDirectory();
    if (!dir.existsSync()) return [];

    final entities = <FileSystemEntity>[];
    try {
      entities.addAll(dir.listSync(followLinks: false));
    } catch (_) {
      return [];
    }

    final items = <FileItem>[];

    for (final entity in entities) {
      final name = p.basename(entity.path);
      // Skip hidden system files (.thumb, .android, etc.) unless searched
      if (name.startsWith('.') && searchQuery.isEmpty) {
        continue;
      }

      final item = FileItem.fromEntity(entity);

      if (!item.isFolder) {
        // Filter files by category
        if (category != FileCategory.all && item.category != category) {
          continue;
        }
      }

      // Filter by search query
      if (searchQuery.isNotEmpty &&
          !item.name.toLowerCase().contains(searchQuery.toLowerCase())) {
        continue;
      }

      items.add(item);
    }

    // Sort: Folders first, then Files
    items.sort((a, b) {
      if (a.isFolder && !b.isFolder) return -1;
      if (!a.isFolder && b.isFolder) return 1;
      return a.name.toLowerCase().compareTo(b.name.toLowerCase());
    });

    return items;
  }

  /// Optional native Android file picker fallback
  static Future<List<FileItem>> importFromDevice() async {
    final result = await FilePicker.pickFiles(
      allowMultiple: true,
      type: FileType.any,
    );

    if (result == null || result.files.isEmpty) {
      return [];
    }

    final targetDir = await getCurrentDirectory();
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

  /// Renames an existing file or directory.
  static Future<FileItem?> renameFile(FileItem item, String newName) async {
    final entity = item.isFolder ? Directory(item.path) : File(item.path);
    if (!entity.existsSync()) return null;

    final parent = p.dirname(item.path);
    final newPath = p.join(parent, newName);

    try {
      final renamed = await entity.rename(newPath);
      return FileItem.fromEntity(renamed);
    } catch (_) {
      return null;
    }
  }

  /// Deletes a file or directory.
  static Future<bool> deleteFile(FileItem item) async {
    final entity = item.isFolder ? Directory(item.path) : File(item.path);
    if (!entity.existsSync()) return false;

    try {
      await entity.delete(recursive: true);
      return true;
    } catch (_) {
      return false;
    }
  }

  /// Batch deletes multiple files or folders.
  static Future<int> deleteMultiple(List<FileItem> items) async {
    int deleted = 0;
    for (final it in items) {
      if (await deleteFile(it)) {
        deleted++;
      }
    }
    return deleted;
  }

  /// Exports a file directly to the system Downloads folder.
  static Future<String?> exportToDownloads(FileItem item) async {
    try {
      final downloadDir = Directory('/storage/emulated/0/Download');
      if (downloadDir.existsSync()) {
        final dest = p.join(downloadDir.path, item.name);
        await File(item.path).copy(dest);
        return dest;
      }
    } catch (_) {}
    return null;
  }

  /// Creates a new subfolder in the current directory.
  static Future<Directory?> createFolder(String folderName) async {
    final current = await getCurrentDirectory();
    final newFld = Directory(p.join(current.path, folderName));
    if (!newFld.existsSync()) {
      await newFld.create(recursive: true);
      return newFld;
    }
    return null;
  }
}
