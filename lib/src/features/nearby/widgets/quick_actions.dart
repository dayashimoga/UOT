// Nearby Screen Quick Actions Bar Widget
import 'package:flutter/material.dart';

class QuickActionsBar extends StatelessWidget {
  const QuickActionsBar({
    super.key,
    required this.onSendFiles,
    required this.onSendClipboard,
    required this.onShowQr,
    required this.onScanSubnet,
  });

  final VoidCallback onSendFiles;
  final VoidCallback onSendClipboard;
  final VoidCallback onShowQr;
  final VoidCallback onScanSubnet;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Row(
        children: [
          ActionChip(
            avatar: Icon(
              Icons.folder_open_rounded,
              size: 18,
              color: colorScheme.primary,
            ),
            label: const Text('Files & Folders'),
            onPressed: onSendFiles,
          ),
          const SizedBox(width: 8),
          ActionChip(
            avatar: Icon(
              Icons.content_paste_rounded,
              size: 18,
              color: colorScheme.secondary,
            ),
            label: const Text('Clipboard Share'),
            onPressed: onSendClipboard,
          ),
          const SizedBox(width: 8),
          ActionChip(
            avatar: Icon(
              Icons.qr_code_rounded,
              size: 18,
              color: colorScheme.tertiary,
            ),
            label: const Text('Pair via QR'),
            onPressed: onShowQr,
          ),
          const SizedBox(width: 8),
          ActionChip(
            avatar: Icon(
              Icons.radar_rounded,
              size: 18,
              color: colorScheme.outline,
            ),
            label: const Text('Subnet Scan'),
            onPressed: onScanSubnet,
          ),
        ],
      ),
    );
  }
}
