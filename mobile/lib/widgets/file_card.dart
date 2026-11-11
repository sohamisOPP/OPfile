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
      return const Color(0xFFF59E0B);
    }
    switch (cat) {
      case FileCategory.image:
        return const Color(0xFF00E599);
      case FileCategory.document:
        return const Color(0xFF3B82F6);
      case FileCategory.audio:
        return const Color(0xFFA855F7);
      case FileCategory.video:
        return const Color(0xFFF59E0B);
      case FileCategory.archive:
        return const Color(0xFFF43F5E);
      case FileCategory.data:
        return const Color(0xFF06B6D4);
      default:
        return const Color(0xFF888888);
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
        return Icons.description_rounded;
      case FileCategory.audio:
        return Icons.music_note_rounded;
      case FileCategory.video:
        return Icons.videocam_rounded;
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
        borderRadius: BorderRadius.circular(16),
        child: Container(
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: isSelected ? const Color(0xFF1E2A24) : const Color(0xFF121212),
            borderRadius: BorderRadius.circular(16),
            border: Border.all(
              color: isSelected ? const Color(0xFF00E599) : const Color(0xFF222222),
              width: isSelected ? 1.5 : 1.0,
            ),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              // Top row: Extension pill + selection/more
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 3),
                    decoration: BoxDecoration(
                      color: catColor.withValues(alpha: 0.15),
                      borderRadius: BorderRadius.circular(6),
                    ),
                    child: Text(
                      fileItem.isFolder
                          ? 'DIR'
                          : (fileItem.extension.isNotEmpty
                              ? fileItem.extension.toUpperCase()
                              : 'FILE'),
                      style: TextStyle(
                        color: catColor,
                        fontSize: 10,
                        fontWeight: FontWeight.w800,
                      ),
                    ),
                  ),
                  if (isBatchMode)
                    Icon(
                      isSelected
                          ? Icons.check_circle_rounded
                          : Icons.radio_button_unchecked_rounded,
                      color: isSelected ? const Color(0xFF00E599) : const Color(0xFF555555),
                      size: 20,
                    )
                  else
                    InkWell(
                      onTap: onMoreOptions,
                      borderRadius: BorderRadius.circular(12),
                      child: const Padding(
                        padding: EdgeInsets.all(4),
                        child: Icon(Icons.more_vert_rounded, color: Color(0xFF666666), size: 16),
                      ),
                    ),
                ],
              ),
              const Spacer(),

              // Icon in center
              Center(
                child: Container(
                  width: 44,
                  height: 44,
                  decoration: BoxDecoration(
                    color: const Color(0xFF1A1A1A),
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(color: const Color(0xFF282828)),
                  ),
                  child: Icon(catIcon, color: catColor, size: 22),
                ),
              ),
              const Spacer(),

              // Name
              Text(
                fileItem.name,
                style: const TextStyle(
                  color: Colors.white,
                  fontSize: 12,
                  fontWeight: FontWeight.w600,
                ),
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
              const SizedBox(height: 2),

              // Size
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

    // List View Mode
    return InkWell(
      onTap: onTap,
      onLongPress: onLongPress,
      borderRadius: BorderRadius.circular(14),
      child: Container(
        margin: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
        decoration: BoxDecoration(
          color: isSelected ? const Color(0xFF1E2A24) : const Color(0xFF121212),
          borderRadius: BorderRadius.circular(14),
          border: Border.all(
            color: isSelected ? const Color(0xFF00E599) : const Color(0xFF222222),
            width: isSelected ? 1.5 : 1.0,
          ),
        ),
        child: Row(
          children: [
            Container(
              width: 40,
              height: 40,
              decoration: BoxDecoration(
                color: const Color(0xFF1A1A1A),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(color: const Color(0xFF282828)),
              ),
              child: Icon(catIcon, color: catColor, size: 20),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    fileItem.name,
                    style: const TextStyle(
                      color: Colors.white,
                      fontSize: 13,
                      fontWeight: FontWeight.w600,
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 2),
                  Row(
                    children: [
                      Container(
                        padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
                        decoration: BoxDecoration(
                          color: catColor.withValues(alpha: 0.15),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          fileItem.isFolder
                              ? 'DIR'
                              : (fileItem.extension.isNotEmpty
                                  ? fileItem.extension.toUpperCase()
                                  : 'FILE'),
                          style: TextStyle(
                            color: catColor,
                            fontSize: 9,
                            fontWeight: FontWeight.w800,
                          ),
                        ),
                      ),
                      const SizedBox(width: 8),
                      Text(
                        fileItem.formattedSize,
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
            if (isBatchMode)
              Icon(
                isSelected
                    ? Icons.check_circle_rounded
                    : Icons.radio_button_unchecked_rounded,
                color: isSelected ? const Color(0xFF00E599) : const Color(0xFF555555),
                size: 20,
              )
            else
              IconButton(
                icon: const Icon(Icons.more_vert_rounded, color: Color(0xFF666666), size: 18),
                onPressed: onMoreOptions,
              ),
          ],
        ),
      ),
    );
  }
}
