// File Send Confirmation Dialog
//
// Displays target device info, selected file list with individual sizes,
// total batch size summary, and a clear "[🚀 Send Files Now]" action button.

import 'dart:io';
import 'package:flutter/material.dart';

class ConfirmSendDialog extends StatelessWidget {
  final String targetDeviceName;
  final String targetAddress;
  final List<String> filePaths;

  const ConfirmSendDialog({
    super.key,
    required this.targetDeviceName,
    required this.targetAddress,
    required this.filePaths,
  });

  static Future<bool?> show(
    BuildContext context, {
    required String targetDeviceName,
    required String targetAddress,
    required List<String> filePaths,
  }) {
    return showDialog<bool>(
      context: context,
      builder: (context) => ConfirmSendDialog(
        targetDeviceName: targetDeviceName,
        targetAddress: targetAddress,
        filePaths: filePaths,
      ),
    );
  }

  String _formatBytes(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1024 * 1024 * 1024) {
      return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    int totalSize = 0;
    final List<Map<String, dynamic>> items = [];

    for (final path in filePaths) {
      try {
        final file = File(path);
        if (file.existsSync()) {
          final sz = file.lengthSync();
          totalSize += sz;
          items.add({
            'name': path.split(RegExp(r'[/\\]')).last,
            'size': _formatBytes(sz),
          });
        } else {
          items.add({
            'name': path.split(RegExp(r'[/\\]')).last,
            'size': 'Folder / Item',
          });
        }
      } catch (_) {
        items.add({
          'name': path.split(RegExp(r'[/\\]')).last,
          'size': 'File',
        });
      }
    }

    return AlertDialog(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      title: Row(
        children: [
          Icon(Icons.send_rounded, color: colorScheme.primary),
          const SizedBox(width: 10),
          const Text('Confirm File Send'),
        ],
      ),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Receiver badge
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.primaryContainer,
                borderRadius: BorderRadius.circular(12),
              ),
              child: Row(
                children: [
                  Icon(
                    Icons.devices_rounded,
                    color: colorScheme.onPrimaryContainer,
                    size: 20,
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Target: $targetDeviceName',
                          style: theme.textTheme.bodyMedium?.copyWith(
                            fontWeight: FontWeight.bold,
                            color: colorScheme.onPrimaryContainer,
                          ),
                        ),
                        Text(
                          targetAddress,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color:
                                colorScheme.onPrimaryContainer.withOpacity(0.8),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),

            Text(
              'Selected Items (${filePaths.length}):',
              style: theme.textTheme.labelMedium?.copyWith(
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(height: 8),

            // Item list container
            Container(
              constraints: const BoxConstraints(maxHeight: 180),
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: colorScheme.outlineVariant),
              ),
              child: ListView.separated(
                shrinkWrap: true,
                padding: const EdgeInsets.all(8),
                itemCount: items.length,
                separatorBuilder: (ctx, i) => const Divider(height: 1),
                itemBuilder: (ctx, index) {
                  final item = items[index];
                  return Padding(
                    padding: const EdgeInsets.symmetric(
                      vertical: 6,
                      horizontal: 8,
                    ),
                    child: Row(
                      children: [
                        Icon(
                          Icons.insert_drive_file_outlined,
                          size: 18,
                          color: colorScheme.primary,
                        ),
                        const SizedBox(width: 10),
                        Expanded(
                          child: Text(
                            item['name'] as String,
                            style: theme.textTheme.bodySmall?.copyWith(
                              fontWeight: FontWeight.w600,
                            ),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                        Text(
                          item['size'] as String,
                          style: theme.textTheme.labelSmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  );
                },
              ),
            ),
            const SizedBox(height: 12),

            // Total summary
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Text(
                  'Total Batch Size:',
                  style: theme.textTheme.bodySmall?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
                Text(
                  totalSize > 0
                      ? _formatBytes(totalSize)
                      : '${filePaths.length} item(s)',
                  style: theme.textTheme.bodySmall?.copyWith(
                    fontWeight: FontWeight.bold,
                    color: colorScheme.primary,
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text('Cancel'),
        ),
        FilledButton.icon(
          onPressed: () => Navigator.of(context).pop(true),
          icon: const Icon(Icons.rocket_launch_rounded, size: 18),
          label: const Text('Send Files Now'),
        ),
      ],
    );
  }
}
