import 'package:flutter/material.dart';
import '../models/file_item.dart';

class FileCard extends StatelessWidget {
  final FileItem fileItem;
  final bool isGridView;
  final bool isSelected;
  final bool isBatchMode;
  final VoidCallback onTap;
  final VoidCallback onLongPress;
  final VoidCallback onMoreOptions;

  const FileCard({
    super.key,
    required this.fileItem,
    required this.isGridView,
    required this.isSelected,
    required this.isBatchMode,
    required this.onTap,
    required this.onLongPress,
    required this.onMoreOptions,
  });

  Color _getCategoryColor(FileCategory cat) {
    if (fileItem.isFolder) {
      return const Color(0xFFF59E0B); // Amber folder color
    }
    switch (cat) {
      case FileCategory.image:
        return const Color(0xFF10B981); // Emerald
      case FileCategory.document:
        return const Color(0xFF3B82F6); // Blue
      case FileCategory.audio:
        return const Color(0xFF8B5CF6); // Purple
      case FileCategory.video:
        return const Color(0xFFEC4899); // Pink / Red
      case FileCategory.archive:
        return const Color(0xFFF97316); // Orange
      case FileCategory.data:
        return const Color(0xFF06B6D4); // Cyan
      default:
        return const Color(0xFF9CA3AF); // Muted grey
    }
  }

  IconData _getCategoryIcon(FileCategory cat) {
    if (fileItem.isFolder) {
      return Icons.folder_rounded;
    }
    switch (cat) {
      case FileCategory.image:
        return Icons.image_rounded;
      case FileCategory.document:
        return Icons.article_rounded;
      case FileCategory.audio:
        return Icons.audiotrack_rounded;
      case FileCategory.video:
        return Icons.movie_rounded;
      case FileCategory.archive:
        return Icons.folder_zip_rounded;
      case FileCategory.data:
        return Icons.data_object_rounded;
      default:
        return Icons.insert_drive_file_rounded;
    }
  }

  @override
  Widget build(BuildContext context) {
    final catColor = _getCategoryColor(fileItem.category);
    final catIcon = _getCategoryIcon(fileItem.category);

    if (isGridView) {
      return InkWell(
        onTap: onTap,
        onLongPress: onLongPress,
        borderRadius: BorderRadius.circular(12),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 12),
          decoration: BoxDecoration(
            color: isSelected ? const Color(0xFF1E2822) : const Color(0xFF111111),
            borderRadius: BorderRadius.circular(12),
            border: Border.all(
              color: isSelected ? const Color(0xFF00E599) : const Color(0xFF222222),
              width: isSelected ? 1.5 : 1.0,
            ),
          ),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Stack(
                alignment: Alignment.center,
                children: [
                  Container(
                    width: 50,
                    height: 50,
                    decoration: BoxDecoration(
                      color: catColor.withValues(alpha: 0.12),
                      borderRadius: BorderRadius.circular(14),
                    ),
                    child: Icon(catIcon, color: catColor, size: 28),
                  ),
                  if (isBatchMode)
                    Positioned(
                      top: 0,
                      right: 0,
                      child: Icon(
                        isSelected
                            ? Icons.check_circle_rounded
                            : Icons.radio_button_unchecked_rounded,
                        color: isSelected ? const Color(0xFF00E599) : const Color(0xFF666666),
                        size: 20,
                      ),
                    ),
                ],
              ),
              const SizedBox(height: 10),
              Text(
                fileItem.name,
                style: const TextStyle(
                  color: Colors.white,
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 3),
              Text(
                fileItem.formattedSize,
                style: const TextStyle(
                  color: Color(0xFF777777),
                  fontSize: 10,
                  fontWeight: FontWeight.w500,
                ),
              ),
            ],
          ),
        ),
      );
    }

    // Classic List View
    return InkWell(
      onTap: onTap,
      onLongPress: onLongPress,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 11),
        decoration: BoxDecoration(
          color: isSelected ? const Color(0xFF1A2620) : Colors.transparent,
          border: const Border(
            bottom: BorderSide(color: Color(0xFF161616), width: 1),
          ),
        ),
        child: Row(
          children: [
            // Icon
            Container(
              width: 42,
              height: 42,
              decoration: BoxDecoration(
                color: catColor.withValues(alpha: 0.12),
                borderRadius: BorderRadius.circular(10),
              ),
              child: Icon(catIcon, color: catColor, size: 22),
            ),
            const SizedBox(width: 14),

            // Name & Info
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    fileItem.name,
                    style: const TextStyle(
                      color: Colors.white,
                      fontSize: 13.5,
                      fontWeight: FontWeight.w600,
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 3),
                  Row(
                    children: [
                      Text(
                        fileItem.isFolder ? 'Folder' : fileItem.formattedSize,
                        style: const TextStyle(
                          color: Color(0xFF888888),
                          fontSize: 11,
                          fontWeight: FontWeight.w500,
                        ),
                      ),
                      const SizedBox(width: 6),
                      const Text(
                        '•',
                        style: TextStyle(color: Color(0xFF555555), fontSize: 11),
                      ),
                      const SizedBox(width: 6),
                      Text(
                        fileItem.formattedDate,
                        style: const TextStyle(
                          color: Color(0xFF777777),
                          fontSize: 11,
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),

            // Selection Checkbox or 3-dots Menu
            if (isBatchMode)
              Icon(
                isSelected
                    ? Icons.check_circle_rounded
                    : Icons.radio_button_unchecked_rounded,
                color: isSelected ? const Color(0xFF00E599) : const Color(0xFF555555),
                size: 22,
              )
            else
              IconButton(
                icon: const Icon(Icons.more_vert_rounded, color: Color(0xFF777777), size: 20),
                visualDensity: VisualDensity.compact,
                padding: EdgeInsets.zero,
                constraints: const BoxConstraints(minWidth: 36, minHeight: 36),
                onPressed: onMoreOptions,
              ),
          ],
        ),
      ),
    );
  }
}
