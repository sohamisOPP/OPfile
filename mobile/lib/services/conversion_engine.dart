import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:archive/archive.dart';
import 'package:image/image.dart' as img;
import 'package:path/path.dart' as p;

class ImageConversionOptions {
  final int quality;
  final int? maxWidth;
  final int? maxHeight;
  final bool grayscale;
  final int rotationAngle; // 0, 90, 180, 270

  const ImageConversionOptions({
    this.quality = 90,
    this.maxWidth,
    this.maxHeight,
    this.grayscale = false,
    this.rotationAngle = 0,
  });
}

class ConversionResult {
  final bool success;
  final String outputPath;
  final String? errorMessage;
  final int outputSize;

  ConversionResult({
    required this.success,
    required this.outputPath,
    this.errorMessage,
    required this.outputSize,
  });
}

class ConversionEngine {
  /// Converts an image to the target format (PNG, JPG, BMP, ICO, GIF).
  static Future<ConversionResult> convertImage({
    required File inputFile,
    required String targetFormat,
    required String outputDirectory,
    ImageConversionOptions options = const ImageConversionOptions(),
  }) async {
    try {
      final bytes = await inputFile.readAsBytes();
      img.Image? image = img.decodeImage(bytes);

      if (image == null) {
        return ConversionResult(
          success: false,
          outputPath: '',
          outputSize: 0,
          errorMessage: 'Could not decode image format.',
        );
      }

      // 1. Rotation
      if (options.rotationAngle == 90 ||
          options.rotationAngle == 180 ||
          options.rotationAngle == 270) {
        image = img.copyRotate(image, angle: options.rotationAngle);
      }

      // 2. Grayscale
      if (options.grayscale) {
        image = img.grayscale(image);
      }

      // 3. Resizing
      if (options.maxWidth != null || options.maxHeight != null) {
        final currentW = image.width;
        final currentH = image.height;
        int targetW = options.maxWidth ?? currentW;
        int targetH = options.maxHeight ?? currentH;

        if (currentW > targetW || currentH > targetH) {
          image = img.copyResize(
            image,
            width: options.maxWidth,
            height: options.maxHeight,
            maintainAspect: true,
          );
        }
      }

      // 4. Encode to target format
      Uint8List encodedBytes;
      final fmt = targetFormat.toLowerCase();

      switch (fmt) {
        case 'png':
          encodedBytes = Uint8List.fromList(img.encodePng(image));
          break;
        case 'jpg':
        case 'jpeg':
          encodedBytes = Uint8List.fromList(
            img.encodeJpg(image, quality: options.quality.clamp(1, 100)),
          );
          break;
        case 'bmp':
          encodedBytes = Uint8List.fromList(img.encodeBmp(image));
          break;
        case 'ico':
          encodedBytes = Uint8List.fromList(img.encodeIco(image));
          break;
        case 'gif':
          encodedBytes = Uint8List.fromList(img.encodeGif(image));
          break;
        default:
          encodedBytes = Uint8List.fromList(img.encodePng(image));
      }

      // Write output file
      final baseName = p.basenameWithoutExtension(inputFile.path);
      final outExt = fmt == 'jpeg' ? 'jpg' : fmt;
      final outPath = p.join(outputDirectory, '${baseName}_converted.$outExt');
      final outFile = File(outPath);
      await outFile.writeAsBytes(encodedBytes);

      return ConversionResult(
        success: true,
        outputPath: outPath,
        outputSize: encodedBytes.length,
      );
    } catch (e) {
      return ConversionResult(
        success: false,
        outputPath: '',
        outputSize: 0,
        errorMessage: e.toString(),
      );
    }
  }

  /// Compresses a list of files into a single ZIP archive.
  static Future<ConversionResult> compressToZip({
    required List<File> files,
    required String archiveName,
    required String outputDirectory,
  }) async {
    try {
      final archive = Archive();

      for (final file in files) {
        if (!file.existsSync()) continue;
        final name = p.basename(file.path);
        final bytes = await file.readAsBytes();
        archive.addFile(ArchiveFile(name, bytes.length, bytes));
      }

      final zipData = ZipEncoder().encode(archive);
      final cleanName = archiveName.endsWith('.zip') ? archiveName : '$archiveName.zip';
      final outPath = p.join(outputDirectory, cleanName);
      final outFile = File(outPath);
      await outFile.writeAsBytes(zipData);

      return ConversionResult(
        success: true,
        outputPath: outPath,
        outputSize: zipData.length,
      );
    } catch (e) {
      return ConversionResult(
        success: false,
        outputPath: '',
        outputSize: 0,
        errorMessage: e.toString(),
      );
    }
  }

  /// Extracts a ZIP archive into a destination folder.
  static Future<ConversionResult> extractZip({
    required File zipFile,
    required String outputDirectory,
  }) async {
    try {
      final bytes = await zipFile.readAsBytes();
      final archive = ZipDecoder().decodeBytes(bytes);
      final baseName = p.basenameWithoutExtension(zipFile.path);
      final targetFolder = Directory(p.join(outputDirectory, '${baseName}_extracted'));
      if (!targetFolder.existsSync()) {
        targetFolder.createSync(recursive: true);
      }

      int extractedCount = 0;
      for (final file in archive) {
        final filename = file.name;
        if (file.isFile) {
          final data = file.content as List<int>;
          final outFile = File(p.join(targetFolder.path, filename));
          outFile.parent.createSync(recursive: true);
          outFile.writeAsBytesSync(data);
          extractedCount++;
        }
      }

      return ConversionResult(
        success: true,
        outputPath: targetFolder.path,
        outputSize: extractedCount,
      );
    } catch (e) {
      return ConversionResult(
        success: false,
        outputPath: '',
        outputSize: 0,
        errorMessage: e.toString(),
      );
    }
  }

  /// Converts Structured Data formats: JSON <-> CSV, JSON <-> XML, Markdown -> HTML, CSV -> HTML table.
  static Future<ConversionResult> convertData({
    required File inputFile,
    required String targetFormat,
    required String outputDirectory,
  }) async {
    try {
      final rawText = await inputFile.readAsString();
      final inExt = p.extension(inputFile.path).replaceFirst('.', '').toLowerCase();
      final targetExt = targetFormat.toLowerCase();
      String outputText = '';

      if (inExt == 'json' && targetExt == 'csv') {
        outputText = _jsonToCsv(rawText);
      } else if (inExt == 'csv' && targetExt == 'json') {
        outputText = _csvToJson(rawText);
      } else if (inExt == 'json' && targetExt == 'xml') {
        outputText = _jsonToXml(rawText);
      } else if (inExt == 'csv' && targetExt == 'html') {
        outputText = _csvToHtml(rawText);
      } else if (inExt == 'md' && targetExt == 'html') {
        outputText = _markdownToHtml(rawText);
      } else if (targetExt == 'txt') {
        outputText = rawText;
      } else {
        return ConversionResult(
          success: false,
          outputPath: '',
          outputSize: 0,
          errorMessage: 'Unsupported data conversion from .$inExt to .$targetExt',
        );
      }

      final baseName = p.basenameWithoutExtension(inputFile.path);
      final outPath = p.join(outputDirectory, '${baseName}_converted.$targetExt');
      final outFile = File(outPath);
      await outFile.writeAsString(outputText);

      return ConversionResult(
        success: true,
        outputPath: outPath,
        outputSize: outputText.length,
      );
    } catch (e) {
      return ConversionResult(
        success: false,
        outputPath: '',
        outputSize: 0,
        errorMessage: e.toString(),
      );
    }
  }

  static String _jsonToCsv(String jsonStr) {
    final parsed = jsonDecode(jsonStr);
    List<dynamic> list;
    if (parsed is List) {
      list = parsed;
    } else if (parsed is Map) {
      list = [parsed];
    } else {
      return 'value\n$parsed';
    }

    final headers = <String>{};
    for (final item in list) {
      if (item is Map) {
        headers.addAll(item.keys.map((k) => k.toString()));
      }
    }
    final headerList = headers.toList();
    final buffer = StringBuffer();
    buffer.writeln(headerList.map((h) => '"$h"').join(','));

    for (final item in list) {
      if (item is Map) {
        final row = headerList.map((h) {
          final val = item[h] ?? '';
          return '"${val.toString().replaceAll('"', '""')}"';
        }).join(',');
        buffer.writeln(row);
      }
    }
    return buffer.toString();
  }

  static String _csvToJson(String csvStr) {
    final lines = csvStr.trim().split(RegExp(r'\r?\n'));
    if (lines.isEmpty) return '[]';

    final headers = lines[0].split(',').map((h) => h.replaceAll('"', '').trim()).toList();
    final result = <Map<String, dynamic>>[];

    for (int i = 1; i < lines.length; i++) {
      final line = lines[i].trim();
      if (line.isEmpty) continue;
      final values = line.split(',').map((v) => v.replaceAll('"', '').trim()).toList();
      final map = <String, dynamic>{};
      for (int j = 0; j < headers.length; j++) {
        map[headers[j]] = j < values.length ? values[j] : '';
      }
      result.add(map);
    }
    return const JsonEncoder.withIndent('  ').convert(result);
  }

  static String _jsonToXml(String jsonStr) {
    final parsed = jsonDecode(jsonStr);
    final buffer = StringBuffer();
    buffer.writeln('<?xml version="1.0" encoding="UTF-8"?>');
    buffer.writeln('<root>');

    void buildXml(dynamic val, String tag, int indent) {
      final pad = '  ' * indent;
      if (val is Map) {
        buffer.writeln('$pad<$tag>');
        val.forEach((k, v) => buildXml(v, k.toString(), indent + 1));
        buffer.writeln('$pad</$tag>');
      } else if (val is List) {
        buffer.writeln('$pad<$tag>');
        for (final item in val) {
          buildXml(item, 'item', indent + 1);
        }
        buffer.writeln('$pad</$tag>');
      } else {
        buffer.writeln('$pad<$tag>$val</$tag>');
      }
    }

    if (parsed is Map) {
      parsed.forEach((k, v) => buildXml(v, k.toString(), 1));
    } else {
      buildXml(parsed, 'data', 1);
    }
    buffer.writeln('</root>');
    return buffer.toString();
  }

  static String _csvToHtml(String csvStr) {
    final lines = csvStr.trim().split(RegExp(r'\r?\n'));
    final buffer = StringBuffer();
    buffer.writeln('<!DOCTYPE html><html><head><meta charset="utf-8">');
    buffer.writeln('<style>body{background:#000;color:#fff;font-family:sans-serif;padding:20px;}table{border-collapse:collapse;width:100%;}th,td{border:1px solid #333;padding:8px 12px;text-align:left;}th{background:#1a1a1a;color:#00e599;}</style>');
    buffer.writeln('</head><body><table>');

    if (lines.isNotEmpty) {
      buffer.writeln('<thead><tr>');
      for (final cell in lines[0].split(',')) {
        buffer.writeln('<th>${cell.replaceAll('"', '').trim()}</th>');
      }
      buffer.writeln('</tr></thead><tbody>');

      for (int i = 1; i < lines.length; i++) {
        if (lines[i].trim().isEmpty) continue;
        buffer.writeln('<tr>');
        for (final cell in lines[i].split(',')) {
          buffer.writeln('<td>${cell.replaceAll('"', '').trim()}</td>');
        }
        buffer.writeln('</tr>');
      }
      buffer.writeln('</tbody>');
    }
    buffer.writeln('</table></body></html>');
    return buffer.toString();
  }

  static String _markdownToHtml(String mdStr) {
    String html = mdStr;
    // Bold & italic
    html = html.replaceAllMapped(RegExp(r'\*\*(.+?)\*\*'), (m) => '<strong>${m[1]}</strong>');
    html = html.replaceAllMapped(RegExp(r'\*(.+?)\*'), (m) => '<em>${m[1]}</em>');
    // Headers
    html = html.replaceAllMapped(RegExp(r'^# (.+)$', multiLine: true), (m) => '<h1>${m[1]}</h1>');
    html = html.replaceAllMapped(RegExp(r'^## (.+)$', multiLine: true), (m) => '<h2>${m[1]}</h2>');
    html = html.replaceAllMapped(RegExp(r'^### (.+)$', multiLine: true), (m) => '<h3>${m[1]}</h3>');
    // Paragraphs
    final paragraphs = html.split(RegExp(r'\n\n+'));
    final body = paragraphs.map((p) => p.startsWith('<h') ? p : '<p>$p</p>').join('\n');

    return '<!DOCTYPE html><html><head><meta charset="utf-8"><style>body{background:#000;color:#fff;font-family:sans-serif;line-height:1.6;padding:24px;max-width:800px;margin:auto;}h1,h2,h3{color:#00e599;}code{background:#1a1a1a;padding:2px 6px;border-radius:4px;}</style></head><body>$body</body></html>';
  }
}
