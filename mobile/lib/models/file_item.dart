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

  FileItem({
    required this.path,
    required this.name,
    required this.size,
    required this.modified,
    required this.extension,
    required this.category,
  });

  factory FileItem.fromFile(File file) {
    final stat = file.statSync();
    final name = file.uri.pathSegments.isNotEmpty
        ? file.uri.pathSegments.lastWhere((s) => s.isNotEmpty, orElse: () => 'file')
        : 'file';
    final dotIdx = name.lastIndexOf('.');
    final ext = dotIdx != -1 ? name.substring(dotIdx + 1).toLowerCase() : '';

    return FileItem(
      path: file.path,
      name: name,
      size: stat.size,
      modified: stat.modified,
      extension: ext,
      category: categoryFromExtension(ext),
    );
  }

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
    if (size < 1024) return '$size B';
    if (size < 1024 * 1024) return '${(size / 1024).toStringAsFixed(1)} KB';
    if (size < 1024 * 1024 * 1024) {
      return '${(size / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(size / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }
}
