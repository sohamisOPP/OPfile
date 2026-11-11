import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:share_plus/share_plus.dart';
import '../models/file_item.dart';
import '../services/conversion_engine.dart';
import '../services/file_service.dart';

class ConversionDialog extends StatefulWidget {
  final FileItem fileItem;
  final String targetFormat;
  final VoidCallback onConversionFinished;

  const ConversionDialog({
    super.key,
    required this.fileItem,
    required this.targetFormat,
    required this.onConversionFinished,
  });

  @override
  State<ConversionDialog> createState() => _ConversionDialogState();
}

class _ConversionDialogState extends State<ConversionDialog> {
  double _quality = 90;
  int? _maxDimension;
  bool _grayscale = false;
  int _rotationAngle = 0;

  bool _isConverting = false;
  ConversionResult? _result;

  bool get _isImageConversion =>
      widget.fileItem.category == FileCategory.image &&
      ['png', 'jpg', 'jpeg', 'webp', 'bmp', 'ico', 'gif']
          .contains(widget.targetFormat.toLowerCase());

  Future<void> _executeConversion() async {
    setState(() {
      _isConverting = true;
      _result = null;
    });

    final targetDir = await FileService.getStorageDirectory();
    final inputFile = File(widget.fileItem.path);

    ConversionResult res;

    if (widget.targetFormat.toLowerCase() == 'zip') {
      res = await ConversionEngine.compressToZip(
        files: [inputFile],
        archiveName: widget.fileItem.name,
        outputDirectory: targetDir.path,
      );
    } else if (widget.targetFormat.toLowerCase() == 'extract') {
      res = await ConversionEngine.extractZip(
        zipFile: inputFile,
        outputDirectory: targetDir.path,
      );
    } else if (_isImageConversion) {
      res = await ConversionEngine.convertImage(
        inputFile: inputFile,
        targetFormat: widget.targetFormat,
        outputDirectory: targetDir.path,
        options: ImageConversionOptions(
          quality: _quality.toInt(),
          maxWidth: _maxDimension,
          maxHeight: _maxDimension,
          grayscale: _grayscale,
          rotationAngle: _rotationAngle,
        ),
      );
    } else {
      res = await ConversionEngine.convertData(
        inputFile: inputFile,
        targetFormat: widget.targetFormat,
        outputDirectory: targetDir.path,
      );
    }

    if (mounted) {
      setState(() {
        _isConverting = false;
        _result = res;
      });
      if (res.success) {
        HapticFeedback.heavyImpact();
        widget.onConversionFinished();
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      backgroundColor: Colors.transparent,
      insetPadding: const EdgeInsets.symmetric(horizontal: 20, vertical: 24),
      child: Container(
        padding: const EdgeInsets.all(24),
        decoration: BoxDecoration(
          color: const Color(0xFF0D0D0D), // Pitch black card
          borderRadius: BorderRadius.circular(24),
          border: Border.all(color: const Color(0xFF262626), width: 1.5),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Header
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.all(8),
                  decoration: BoxDecoration(
                    color: const Color(0xFF1A1A1A),
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(color: const Color(0xFF333333)),
                  ),
                  child: const Icon(
                    Icons.auto_mode_rounded,
                    color: Color(0xFF00E599),
                    size: 20,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Convert to ${widget.targetFormat.toUpperCase()}',
                        style: const TextStyle(
                          color: Colors.white,
                          fontSize: 16,
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      Text(
                        widget.fileItem.name,
                        style: const TextStyle(
                          color: Color(0xFF888888),
                          fontSize: 12,
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.close_rounded, color: Color(0xFF777777)),
                  onPressed: () => Navigator.of(context).pop(),
                ),
              ],
            ),
            const SizedBox(height: 20),

            // Conversion Options (if Image)
            if (_isImageConversion && _result == null) ...[
              // Quality Slider
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  const Text(
                    'Quality',
                    style: TextStyle(color: Colors.white, fontSize: 13, fontWeight: FontWeight.w600),
                  ),
                  Text(
                    '${_quality.toInt()}%',
                    style: const TextStyle(color: Color(0xFF00E599), fontSize: 13, fontWeight: FontWeight.w700),
                  ),
                ],
              ),
              SliderTheme(
                data: SliderTheme.of(context).copyWith(
                  activeTrackColor: const Color(0xFF00E599),
                  inactiveTrackColor: const Color(0xFF262626),
                  thumbColor: const Color(0xFF00E599),
                  overlayColor: const Color(0x3300E599),
                  trackHeight: 4,
                ),
                child: Slider(
                  value: _quality,
                  min: 10,
                  max: 100,
                  onChanged: (v) => setState(() => _quality = v),
                ),
              ),
              const SizedBox(height: 8),

              // Grayscale switch
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  const Text(
                    'Black & White Filter',
                    style: TextStyle(color: Colors.white, fontSize: 13, fontWeight: FontWeight.w600),
                  ),
                  Switch(
                    value: _grayscale,
                    activeColor: const Color(0xFF00E599),
                    activeTrackColor: const Color(0xFF004D33),
                    inactiveThumbColor: const Color(0xFF666666),
                    inactiveTrackColor: const Color(0xFF1E1E1E),
                    onChanged: (v) => setState(() => _grayscale = v),
                  ),
                ],
              ),
              const SizedBox(height: 8),

              // Rotation chips
              Row(
                children: [
                  const Text(
                    'Rotate:',
                    style: TextStyle(color: Colors.white, fontSize: 13, fontWeight: FontWeight.w600),
                  ),
                  const SizedBox(width: 12),
                  ...[0, 90, 180, 270].map((angle) {
                    final isSel = _rotationAngle == angle;
                    return Padding(
                      padding: const EdgeInsets.only(right: 6),
                      child: InkWell(
                        onTap: () => setState(() => _rotationAngle = angle),
                        borderRadius: BorderRadius.circular(8),
                        child: Container(
                          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 5),
                          decoration: BoxDecoration(
                            color: isSel ? const Color(0xFF00E599) : const Color(0xFF1A1A1A),
                            borderRadius: BorderRadius.circular(8),
                            border: Border.all(
                              color: isSel ? const Color(0xFF00E599) : const Color(0xFF333333),
                            ),
                          ),
                          child: Text(
                            '$angle°',
                            style: TextStyle(
                              color: isSel ? Colors.black : Colors.white,
                              fontSize: 11,
                              fontWeight: FontWeight.w700,
                            ),
                          ),
                        ),
                      ),
                    );
                  }),
                ],
              ),
              const SizedBox(height: 16),
            ],

            // Progress or Result Area
            if (_isConverting) ...[
              const Center(
                child: Padding(
                  padding: EdgeInsets.symmetric(vertical: 24),
                  child: Column(
                    children: [
                      CircularProgressIndicator(
                        color: Color(0xFF00E599),
                        strokeWidth: 3,
                      ),
                      SizedBox(height: 16),
                      Text(
                        'Converting 100% Offline on Device...',
                        style: TextStyle(color: Color(0xFFAAAAAA), fontSize: 13),
                      ),
                    ],
                  ),
                ),
              ),
            ] else if (_result != null) ...[
              Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: _result!.success
                      ? const Color(0xFF0D281E)
                      : const Color(0xFF2D1214),
                  borderRadius: BorderRadius.circular(16),
                  border: Border.all(
                    color: _result!.success
                        ? const Color(0xFF00E599)
                        : const Color(0xFFFF5C5C),
                  ),
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Icon(
                          _result!.success
                              ? Icons.check_circle_rounded
                              : Icons.error_rounded,
                          color: _result!.success
                              ? const Color(0xFF00E599)
                              : const Color(0xFFFF5C5C),
                          size: 20,
                        ),
                        const SizedBox(width: 8),
                        Text(
                          _result!.success
                              ? 'Conversion Complete!'
                              : 'Conversion Failed',
                          style: TextStyle(
                            color: _result!.success
                                ? const Color(0xFF00E599)
                                : const Color(0xFFFF5C5C),
                            fontSize: 14,
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 8),
                    if (_result!.success) ...[
                      Text(
                        'Saved: ${_result!.outputPath.split(Platform.pathSeparator).last}',
                        style: const TextStyle(color: Colors.white, fontSize: 12),
                      ),
                    ] else ...[
                      Text(
                        _result!.errorMessage ?? 'Unknown error occurred.',
                        style: const TextStyle(color: Color(0xFFFFAAAA), fontSize: 12),
                      ),
                    ],
                  ],
                ),
              ),
              const SizedBox(height: 16),
              if (_result!.success)
                Row(
                  children: [
                    Expanded(
                      child: OutlinedButton.icon(
                        onPressed: () {
                          SharePlus.instance.share(ShareParams(files: [XFile(_result!.outputPath)]));
                        },
                        icon: const Icon(Icons.share_rounded, size: 16, color: Colors.white),
                        label: const Text('Share', style: TextStyle(color: Colors.white, fontSize: 12)),
                        style: OutlinedButton.styleFrom(
                          side: const BorderSide(color: Color(0xFF333333)),
                          padding: const EdgeInsets.symmetric(vertical: 12),
                          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                        ),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: ElevatedButton.icon(
                        onPressed: () async {
                          final savedPath = await FileService.exportToDownloads(
                            FileItem.fromFile(File(_result!.outputPath)),
                          );
                          if (context.mounted) {
                            ScaffoldMessenger.of(context).showSnackBar(
                              SnackBar(
                                content: Text(savedPath != null
                                    ? 'Exported to Downloads folder!'
                                    : 'Saved in OPfile storage'),
                                backgroundColor: const Color(0xFF1A1A1A),
                              ),
                            );
                            Navigator.of(context).pop();
                          }
                        },
                        icon: const Icon(Icons.download_rounded, size: 16, color: Colors.black),
                        label: const Text('Save to Phone', style: TextStyle(color: Colors.black, fontSize: 12, fontWeight: FontWeight.w700)),
                        style: ElevatedButton.styleFrom(
                          backgroundColor: const Color(0xFF00E599),
                          padding: const EdgeInsets.symmetric(vertical: 12),
                          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                        ),
                      ),
                    ),
                  ],
                ),
            ] else ...[
              // Action Button to Start Conversion
              ElevatedButton.icon(
                onPressed: _executeConversion,
                icon: const Icon(Icons.bolt_rounded, color: Colors.black, size: 18),
                label: const Text(
                  'Convert Now (Offline)',
                  style: TextStyle(
                    color: Colors.black,
                    fontSize: 14,
                    fontWeight: FontWeight.w800,
                  ),
                ),
                style: ElevatedButton.styleFrom(
                  backgroundColor: const Color(0xFF00E599),
                  padding: const EdgeInsets.symmetric(vertical: 14),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(14),
                  ),
                  elevation: 0,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
