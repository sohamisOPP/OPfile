import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../models/file_item.dart';

class DonutSlice {
  final String format;
  final String label;
  final IconData icon;
  final Color accentColor;

  const DonutSlice({
    required this.format,
    required this.label,
    required this.icon,
    this.accentColor = const Color(0xFF00E599),
  });
}

class DonutMenuDialog extends StatefulWidget {
  final FileItem fileItem;
  final Function(String targetFormat) onSelectFormat;

  const DonutMenuDialog({
    super.key,
    required this.fileItem,
    required this.onSelectFormat,
  });

  @override
  State<DonutMenuDialog> createState() => _DonutMenuDialogState();
}

class _DonutMenuDialogState extends State<DonutMenuDialog>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _scaleAnimation;
  late final Animation<double> _fadeAnimation;
  int? _hoveredIndex;

  late final List<DonutSlice> _slices;

  @override
  void initState() {
    super.initState();
    _slices = _getSlicesForFile(widget.fileItem);
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 320),
    );
    _scaleAnimation = CurvedAnimation(
      parent: _controller,
      curve: Curves.easeOutBack,
    );
    _fadeAnimation = CurvedAnimation(
      parent: _controller,
      curve: Curves.easeIn,
    );
    _controller.forward();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  List<DonutSlice> _getSlicesForFile(FileItem item) {
    final ext = item.extension.toLowerCase();
    switch (item.category) {
      case FileCategory.image:
        final all = [
          const DonutSlice(format: 'png', label: 'PNG', icon: Icons.image_rounded),
          const DonutSlice(format: 'jpg', label: 'JPG', icon: Icons.photo_rounded),
          const DonutSlice(format: 'webp', label: 'WEBP', icon: Icons.photo_size_select_actual_rounded),
          const DonutSlice(format: 'bmp', label: 'BMP', icon: Icons.crop_original_rounded),
          const DonutSlice(format: 'ico', label: 'ICO', icon: Icons.star_border_rounded),
          const DonutSlice(format: 'zip', label: 'ZIP', icon: Icons.folder_zip_rounded),
        ];
        return all.where((s) => s.format != ext).toList();

      case FileCategory.data:
        if (ext == 'json') {
          return const [
            DonutSlice(format: 'csv', label: 'CSV', icon: Icons.table_chart_rounded),
            DonutSlice(format: 'xml', label: 'XML', icon: Icons.code_rounded),
            DonutSlice(format: 'txt', label: 'TXT', icon: Icons.text_snippet_rounded),
            DonutSlice(format: 'zip', label: 'ZIP', icon: Icons.folder_zip_rounded),
          ];
        } else if (ext == 'csv') {
          return const [
            DonutSlice(format: 'json', label: 'JSON', icon: Icons.data_object_rounded),
            DonutSlice(format: 'html', label: 'HTML', icon: Icons.html_rounded),
            DonutSlice(format: 'txt', label: 'TXT', icon: Icons.text_snippet_rounded),
            DonutSlice(format: 'zip', label: 'ZIP', icon: Icons.folder_zip_rounded),
          ];
        } else if (ext == 'md') {
          return const [
            DonutSlice(format: 'html', label: 'HTML', icon: Icons.html_rounded),
            DonutSlice(format: 'txt', label: 'TXT', icon: Icons.text_snippet_rounded),
            DonutSlice(format: 'zip', label: 'ZIP', icon: Icons.folder_zip_rounded),
          ];
        }
        return const [
          DonutSlice(format: 'txt', label: 'TXT', icon: Icons.text_snippet_rounded),
          DonutSlice(format: 'json', label: 'JSON', icon: Icons.data_object_rounded),
          DonutSlice(format: 'zip', label: 'ZIP', icon: Icons.folder_zip_rounded),
        ];

      case FileCategory.archive:
        return const [
          DonutSlice(format: 'extract', label: 'EXTRACT', icon: Icons.unarchive_rounded, accentColor: Color(0xFF3B82F6)),
          DonutSlice(format: 'tar', label: 'TAR', icon: Icons.inventory_2_rounded),
          DonutSlice(format: 'gz', label: 'GZIP', icon: Icons.archive_rounded),
        ];

      case FileCategory.document:
        return const [
          DonutSlice(format: 'txt', label: 'TXT', icon: Icons.text_snippet_rounded),
          DonutSlice(format: 'zip', label: 'ZIP', icon: Icons.folder_zip_rounded),
        ];

      default:
        return const [
          DonutSlice(format: 'zip', label: 'ZIP', icon: Icons.folder_zip_rounded),
          DonutSlice(format: 'txt', label: 'TXT', icon: Icons.text_snippet_rounded),
        ];
    }
  }

  void _handleTap(Offset localPosition, double size) {
    final center = Offset(size / 2, size / 2);
    final dx = localPosition.dx - center.dx;
    final dy = localPosition.dy - center.dy;
    final distance = math.sqrt(dx * dx + dy * dy);

    final innerRadius = size * 0.22;
    final outerRadius = size * 0.46;

    // Center hole tapped -> cancel/close
    if (distance <= innerRadius) {
      HapticFeedback.lightImpact();
      Navigator.of(context).pop();
      return;
    }

    // Inside donut ring
    if (distance <= outerRadius) {
      // Calculate clockwise angle from 12 o'clock (top)
      double angle = math.atan2(dy, dx) + (math.pi / 2);
      if (angle < 0) angle += 2 * math.pi;

      final sliceCount = _slices.length;
      final sliceAngle = (2 * math.pi) / sliceCount;
      final selectedIndex = (angle / sliceAngle).floor() % sliceCount;

      HapticFeedback.mediumImpact();
      final target = _slices[selectedIndex].format;
      Navigator.of(context).pop();
      widget.onSelectFormat(target);
    }
  }

  @override
  Widget build(BuildContext context) {
    const donutSize = 310.0;

    return Center(
      child: FadeTransition(
        opacity: _fadeAnimation,
        child: ScaleTransition(
          scale: _scaleAnimation,
          child: Material(
            color: Colors.transparent,
            child: Container(
              padding: const EdgeInsets.all(20),
              margin: const EdgeInsets.symmetric(horizontal: 24),
              decoration: BoxDecoration(
                color: const Color(0xFF0A0A0A), // Solid deep black card
                borderRadius: BorderRadius.circular(28),
                border: Border.all(color: const Color(0xFF262626), width: 1.5),
                boxShadow: const [
                  BoxShadow(
                    color: Colors.black,
                    blurRadius: 40,
                    offset: Offset(0, 10),
                  ),
                ],
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  // File title badge
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
                    decoration: BoxDecoration(
                      color: const Color(0xFF161616),
                      borderRadius: BorderRadius.circular(20),
                      border: Border.all(color: const Color(0xFF2A2A2A)),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        const Icon(
                          Icons.donut_large_rounded,
                          color: Color(0xFF00E599),
                          size: 16,
                        ),
                        const SizedBox(width: 8),
                        Flexible(
                          child: Text(
                            widget.fileItem.name,
                            style: const TextStyle(
                              color: Colors.white,
                              fontSize: 13,
                              fontWeight: FontWeight.w600,
                            ),
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 6),
                  const Text(
                    'Tap a slice to convert offline',
                    style: TextStyle(
                      color: Color(0xFF888888),
                      fontSize: 12,
                      fontWeight: FontWeight.w400,
                    ),
                  ),
                  const SizedBox(height: 16),

                  // The Donut Radial Wheel
                  GestureDetector(
                    onTapUp: (details) =>
                        _handleTap(details.localPosition, donutSize),
                    child: SizedBox(
                      width: donutSize,
                      height: donutSize,
                      child: CustomPaint(
                        painter: DonutPainter(
                          slices: _slices,
                          sourceExtension: widget.fileItem.extension.toUpperCase(),
                          selectedIndex: _hoveredIndex,
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(height: 14),

                  // Bottom cancel button
                  TextButton.icon(
                    onPressed: () => Navigator.of(context).pop(),
                    icon: const Icon(Icons.close_rounded, color: Color(0xFF888888), size: 16),
                    label: const Text(
                      'Cancel',
                      style: TextStyle(
                        color: Color(0xFF888888),
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    style: TextButton.styleFrom(
                      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class DonutPainter extends CustomPainter {
  final List<DonutSlice> slices;
  final String sourceExtension;
  final int? selectedIndex;

  DonutPainter({
    required this.slices,
    required this.sourceExtension,
    this.selectedIndex,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final outerRadius = size.width * 0.46;
    final innerRadius = size.width * 0.22;
    final count = slices.length;
    if (count == 0) return;

    final sweepAngle = (2 * math.pi) / count;
    final gapAngle = 0.045; // Separation line between slices

    for (int i = 0; i < count; i++) {
      final slice = slices[i];
      final startAngle = (i * sweepAngle) - (math.pi / 2) + (gapAngle / 2);
      final currentSweep = sweepAngle - gapAngle;

      final isSelected = selectedIndex == i;

      // Draw Slice Sector Path
      final path = Path();
      path.arcTo(
        Rect.fromCircle(center: center, radius: outerRadius),
        startAngle,
        currentSweep,
        false,
      );
      path.arcTo(
        Rect.fromCircle(center: center, radius: innerRadius),
        startAngle + currentSweep,
        -currentSweep,
        false,
      );
      path.close();

      // Slice background paint (solid dark matte)
      final fillPaint = Paint()
        ..color = isSelected ? const Color(0xFF222222) : const Color(0xFF141414)
        ..style = PaintingStyle.fill;
      canvas.drawPath(path, fillPaint);

      // Slice border paint (solid crisp border)
      final borderPaint = Paint()
        ..color = isSelected ? slice.accentColor : const Color(0xFF2B2B2B)
        ..style = PaintingStyle.stroke
        ..strokeWidth = isSelected ? 2.0 : 1.2;
      canvas.drawPath(path, borderPaint);

      // Mid angle for placing icons and text
      final midAngle = startAngle + (currentSweep / 2);
      final midRadius = (innerRadius + outerRadius) / 2;
      final labelPos = Offset(
        center.dx + midRadius * math.cos(midAngle),
        center.dy + midRadius * math.sin(midAngle),
      );

      // Draw Format Text Label
      final textSpan = TextSpan(
        text: slice.label,
        style: TextStyle(
          color: isSelected ? slice.accentColor : Colors.white,
          fontSize: count > 5 ? 12 : 13,
          fontWeight: FontWeight.w800,
          letterSpacing: 0.6,
        ),
      );
      final textPainter = TextPainter(
        text: textSpan,
        textDirection: TextDirection.ltr,
      )..layout();

      textPainter.paint(
        canvas,
        Offset(
          labelPos.dx - (textPainter.width / 2),
          labelPos.dy - (textPainter.height / 2),
        ),
      );
    }

    // Draw Central Hole (Cancel / Source Badge)
    final centerFill = Paint()
      ..color = const Color(0xFF000000)
      ..style = PaintingStyle.fill;
    canvas.drawCircle(center, innerRadius - 2, centerFill);

    final centerBorder = Paint()
      ..color = const Color(0xFF333333)
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.5;
    canvas.drawCircle(center, innerRadius - 2, centerBorder);

    // Source extension text in center
    final centerText = TextSpan(
      text: sourceExtension.isNotEmpty ? sourceExtension : 'FILE',
      style: const TextStyle(
        color: Color(0xFF00E599),
        fontSize: 13,
        fontWeight: FontWeight.w900,
        letterSpacing: 1.0,
      ),
    );
    final centerPainter = TextPainter(
      text: centerText,
      textDirection: TextDirection.ltr,
    )..layout();

    centerPainter.paint(
      canvas,
      Offset(
        center.dx - (centerPainter.width / 2),
        center.dy - (centerPainter.height / 2) - 6,
      ),
    );

    // Central "CONVERT" subtitle
    final subText = TextSpan(
      text: 'TARGET',
      style: TextStyle(
        color: Colors.white.withValues(alpha: 0.4),
        fontSize: 8.5,
        fontWeight: FontWeight.w700,
        letterSpacing: 1.5,
      ),
    );
    final subPainter = TextPainter(
      text: subText,
      textDirection: TextDirection.ltr,
    )..layout();

    subPainter.paint(
      canvas,
      Offset(
        center.dx - (subPainter.width / 2),
        center.dy + 8,
      ),
    );
  }

  @override
  bool shouldRepaint(covariant DonutPainter oldDelegate) {
    return oldDelegate.selectedIndex != selectedIndex ||
        oldDelegate.slices != slices;
  }
}
