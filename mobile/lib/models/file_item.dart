import 'dart:io';

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

    final name = entity.uri.pathSegments.isNotEmpty
        ? entity.uri.pathSegments.lastWhere((s) => s.isNotEmpty, orElse: () => 'file')
        : 'file';
    final isDir = entity is Directory;
    final dotIdx = name.lastIndexOf('.');
    final ext = (!isDir && dotIdx != -1) ? name.substring(dotIdx + 1).toLowerCase() : '';

    return FileItem(
      path: entity.path,
      name: name,
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
        return FileCategory.image;
      case 'pdf':
      case 'doc':
      case 'docx':
      case 'txt':
      case 'rtf':
      case 'epub':
        return FileCategory.document;
      case 'mp3':
      case 'wav':
      case 'ogg':
      case 'm4a':
      case 'flac':
      case 'aac':
        return FileCategory.audio;
      case 'mp4':
      case 'mkv':
      case 'webm':
      case 'avi':
      case 'mov':
        return FileCategory.video;
      case 'zip':
      case 'tar':
      case 'gz':
      case 'rar':
      case '7z':
        return FileCategory.archive;
      case 'json':
      case 'csv':
      case 'xml':
      case 'md':
      case 'html':
      case 'yaml':
      case 'yml':
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
}
