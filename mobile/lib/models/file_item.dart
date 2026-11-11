import 'dart:io';
import 'package:path/path.dart' as p;

enum FileCategory {
  all,
  image,
  document,
  audio,
  video,
  archive,
  data,
  other,
}

class FileItem {
  final String path;
  final String name;
  final int size;
  final DateTime modified;
  final String extension;
  final FileCategory category;
  final bool isFolder;

  FileItem({
    required this.path,
    required this.name,
    required this.size,
    required this.modified,
    required this.extension,
    required this.category,
    this.isFolder = false,
  });

  factory FileItem.fromEntity(FileSystemEntity entity) {
    FileStat? stat;
    try {
      stat = entity.statSync();
    } catch (_) {}

    final rawName = p.basename(entity.path);
    final isDir = (entity is Directory) || (stat?.type == FileSystemEntityType.directory);
    final dotIdx = rawName.lastIndexOf('.');
    final ext = (!isDir && dotIdx != -1) ? rawName.substring(dotIdx + 1).toLowerCase() : '';

    return FileItem(
      path: entity.path,
      name: rawName.isEmpty ? 'Item' : rawName,
      size: isDir ? 0 : (stat?.size ?? 0),
      modified: stat?.modified ?? DateTime.now(),
      extension: ext,
      category: isDir ? FileCategory.other : categoryFromExtension(ext),
      isFolder: isDir,
    );
  }

  factory FileItem.fromFile(File file) => FileItem.fromEntity(file);

  static FileCategory categoryFromExtension(String ext) {
    switch (ext.toLowerCase()) {
      case 'png':
      case 'jpg':
      case 'jpeg':
      case 'webp':
      case 'bmp':
      case 'gif':
      case 'ico':
      case 'tiff':
      case 'tga':
      case 'svg':
      case 'heic':
      case 'heif':
      case 'raw':
      case 'cr2':
      case 'nef':
        return FileCategory.image;
      case 'pdf':
      case 'doc':
      case 'docx':
      case 'txt':
      case 'rtf':
      case 'epub':
      case 'odt':
      case 'pages':
        return FileCategory.document;
      case 'mp3':
      case 'wav':
      case 'ogg':
      case 'm4a':
      case 'flac':
      case 'aac':
      case 'wma':
      case 'opus':
      case 'mid':
      case 'midi':
        return FileCategory.audio;
      case 'mp4':
      case 'mkv':
      case 'webm':
      case 'avi':
      case 'mov':
      case 'wmv':
      case 'flv':
      case '3gp':
      case 'm4v':
        return FileCategory.video;
      case 'zip':
      case 'tar':
      case 'gz':
      case 'rar':
      case '7z':
      case 'bz2':
      case 'xz':
      case 'zst':
        return FileCategory.archive;
      case 'json':
      case 'csv':
      case 'xml':
      case 'md':
      case 'html':
      case 'yaml':
      case 'yml':
      case 'sql':
      case 'log':
      case 'ini':
      case 'conf':
        return FileCategory.data;
      default:
        return FileCategory.other;
    }
  }

  String get formattedSize {
    if (isFolder) return 'Folder';
    if (size < 1024) return '$size B';
    if (size < 1024 * 1024) return '${(size / 1024).toStringAsFixed(1)} KB';
    if (size < 1024 * 1024 * 1024) {
      return '${(size / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(size / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }

  String get formattedDate {
    return '${modified.year}-${modified.month.toString().padLeft(2, '0')}-${modified.day.toString().padLeft(2, '0')} ${modified.hour.toString().padLeft(2, '0')}:${modified.minute.toString().padLeft(2, '0')}';
  }
}
