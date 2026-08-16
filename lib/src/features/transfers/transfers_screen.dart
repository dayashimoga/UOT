// Transfers Screen
//
// Shows active transfer queue with real-time progress and transfer history.

import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../../rust/api/engine_api.dart' as engine;

// Transfer model.
class TransferInfo {
  final String id;
  final String fileName;
  final String remoteName;
  final String direction; // 'Send' or 'Receive'
  final String status;
  final double progress;
  final int totalBytes;
  final int transferredBytes;
  final String? speed;
  final String? eta;
  final String? savedPath;

  TransferInfo({
    required this.id,
    required this.fileName,
    required this.remoteName,
    required this.direction,
    required this.status,
    required this.progress,
    required this.totalBytes,
    required this.transferredBytes,
    this.speed,
    this.eta,
    this.savedPath,
  });

  bool get isActive =>
      status == 'Transferring' || status == 'Pending' || status == 'Queued';

  IconData get statusIcon {
    switch (status) {
      case 'Completed':
        return Icons.check_circle_rounded;
      case 'Failed':
        return Icons.error_rounded;
      case 'Cancelled':
        return Icons.cancel_rounded;
      case 'Transferring':
        return Icons.sync_rounded;
      case 'Paused':
        return Icons.pause_circle_rounded;
      default:
        return Icons.schedule_rounded;
    }
  }

  Color statusColor(ColorScheme colorScheme) {
    switch (status) {
      case 'Completed':
        return const Color(0xFF4ADE80);
      case 'Failed':
        return colorScheme.error;
      case 'Cancelled':
        return colorScheme.onSurfaceVariant;
      case 'Transferring':
        return colorScheme.primary;
      case 'Paused':
        return const Color(0xFFFBBF24);
      default:
        return colorScheme.onSurfaceVariant;
    }
  }
}

// Format bytes to human-readable string.
String _formatBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  if (bytes < 1024 * 1024 * 1024) {
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
  return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
}

class TransfersScreen extends StatefulWidget {
  const TransfersScreen({super.key});

  @override
  State<TransfersScreen> createState() => _TransfersScreenState();
}

class _TransfersScreenState extends State<TransfersScreen>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;
  final List<TransferInfo> _activeTransfers = [];
  final List<TransferInfo> _historyTransfers = [];
  Timer? _refreshTimer;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    _startRefresh();
  }

  void _startRefresh() {
    _refreshTransfers();
    _refreshTimer = Timer.periodic(const Duration(seconds: 1), (_) {
      _refreshTransfers();
    });
  }

  void _refreshTransfers() {
    if (!mounted) return;
    try {
      final json = engine.engineGetTransfers();
      final List<dynamic> parsed = jsonDecode(json);
      final transfers = parsed.map((t) {
        final items = t['items'] as List<dynamic>?;
        final firstItemName =
            (items != null && items.isNotEmpty && items.first is Map)
                ? items.first['name']?.toString()
                : null;
        final fileName = t['file_name']?.toString() ??
            firstItemName ??
            t['name']?.toString() ??
            'Unknown';
        final totalBytes = (t['total_bytes'] as num?)?.toInt() ??
            (t['total_size'] as num?)?.toInt() ??
            0;
        final rawTransferred = (t['transferred_bytes'] as num?)?.toInt() ?? 0;
        final transferredBytes = totalBytes > 0
            ? rawTransferred.clamp(0, totalBytes)
            : rawTransferred;
        final rawProgress = (t['progress'] as num?)?.toDouble() ??
            (totalBytes > 0 ? transferredBytes / totalBytes : 0.0);
        final progress = rawProgress.clamp(0.0, 1.0);

        return TransferInfo(
          id: t['id']?.toString() ?? t['transfer_id']?.toString() ?? '',
          fileName: fileName,
          remoteName: t['remote_name']?.toString() ??
              t['remote_device']?.toString() ??
              'Unknown',
          direction: t['direction']?.toString() ?? 'Send',
          status: t['status']?.toString() ?? 'Pending',
          progress: progress,
          totalBytes: totalBytes,
          transferredBytes: transferredBytes,
          speed: t['speed']?.toString(),
          eta: t['eta']?.toString(),
          savedPath: t['saved_path']?.toString(),
        );
      }).toList();
      setState(() {
        _activeTransfers.clear();
        _historyTransfers.clear();
        for (final t in transfers) {
          if (t.isActive) {
            _activeTransfers.add(t);
          } else {
            _historyTransfers.add(t);
          }
        }
      });
    } catch (_) {
      // Silently handle parse errors
    }
  }

  @override
  void dispose() {
    _refreshTimer?.cancel();
    _tabController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return SafeArea(
      child: Column(
        children: [
          // App Bar
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            child: Row(
              children: [
                Text('Transfers', style: theme.textTheme.headlineSmall),
                const Spacer(),
                if (_activeTransfers.isNotEmpty)
                  Badge(
                    label: Text('${_activeTransfers.length}'),
                    child: Icon(Icons.sync_rounded, color: colorScheme.primary),
                  ),
              ],
            ),
          ),
          const SizedBox(height: 8),
          // Tab bar
          TabBar(
            controller: _tabController,
            tabs: [
              Tab(
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.sync_rounded, size: 18),
                    const SizedBox(width: 8),
                    Text('Active (${_activeTransfers.length})'),
                  ],
                ),
              ),
              Tab(
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(Icons.history_rounded, size: 18),
                    const SizedBox(width: 8),
                    Text('History (${_historyTransfers.length})'),
                  ],
                ),
              ),
            ],
          ),
          // Tab content
          Expanded(
            child: TabBarView(
              controller: _tabController,
              children: [
                _buildActiveTab(theme, colorScheme),
                _buildHistoryTab(theme, colorScheme),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildActiveTab(ThemeData theme, ColorScheme colorScheme) {
    if (_activeTransfers.isEmpty) {
      return _EmptyState(
        icon: Icons.swap_horiz_rounded,
        title: 'No active transfers',
        subtitle: 'Go to Nearby to discover devices and send files',
        colorScheme: colorScheme,
      );
    }

    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: _activeTransfers.length,
      itemBuilder: (context, index) {
        return Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: _ActiveTransferCard(transfer: _activeTransfers[index]),
        );
      },
    );
  }

  Widget _buildHistoryTab(ThemeData theme, ColorScheme colorScheme) {
    if (_historyTransfers.isEmpty) {
      return _EmptyState(
        icon: Icons.history_rounded,
        title: 'No transfer history',
        subtitle: 'Completed transfers will appear here',
        colorScheme: colorScheme,
      );
    }

    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: _historyTransfers.length,
      itemBuilder: (context, index) {
        return Padding(
          padding: const EdgeInsets.only(bottom: 8),
          child: _HistoryTransferCard(transfer: _historyTransfers[index]),
        );
      },
    );
  }
}

// Empty state widget.
class _EmptyState extends StatelessWidget {
  const _EmptyState({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.colorScheme,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final ColorScheme colorScheme;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 80,
            height: 80,
            decoration: BoxDecoration(
              color: colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(20),
            ),
            child: Icon(
              icon,
              size: 40,
              color: colorScheme.onSurfaceVariant.withOpacity(0.5),
            ),
          ),
          const SizedBox(height: 16),
          Text(title, style: theme.textTheme.titleMedium),
          const SizedBox(height: 4),
          Text(
            subtitle,
            style: theme.textTheme.bodySmall?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
            textAlign: TextAlign.center,
          ),
        ],
      ),
    );
  }
}

// Active transfer card with live progress.
class _ActiveTransferCard extends StatelessWidget {
  const _ActiveTransferCard({required this.transfer});

  final TransferInfo transfer;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final statusColor = transfer.statusColor(colorScheme);

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  transfer.direction == 'Send'
                      ? Icons.arrow_upward_rounded
                      : Icons.arrow_downward_rounded,
                  size: 20,
                  color: statusColor,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    transfer.fileName,
                    style: theme.textTheme.titleSmall,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                // Action buttons
                if (transfer.status == 'Transferring') ...[
                  IconButton(
                    icon: const Icon(Icons.pause_rounded, size: 20),
                    onPressed: () {
                      engine.enginePauseTransfer(transferId: transfer.id);
                    },
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(
                      minWidth: 32,
                      minHeight: 32,
                    ),
                    tooltip: 'Pause',
                  ),
                  IconButton(
                    icon: const Icon(Icons.close_rounded, size: 20),
                    onPressed: () {
                      engine.engineCancelTransfer(transferId: transfer.id);
                    },
                    padding: EdgeInsets.zero,
                    constraints: const BoxConstraints(
                      minWidth: 32,
                      minHeight: 32,
                    ),
                    tooltip: 'Cancel',
                  ),
                ],
              ],
            ),
            const SizedBox(height: 8),
            // Progress bar
            ClipRRect(
              borderRadius: BorderRadius.circular(4),
              child: LinearProgressIndicator(
                value: transfer.progress,
                minHeight: 6,
                backgroundColor: colorScheme.surfaceContainerHighest,
                valueColor: AlwaysStoppedAnimation(statusColor),
              ),
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                Text(
                  '${_formatBytes(transfer.transferredBytes)} / ${_formatBytes(transfer.totalBytes)}',
                  style: theme.textTheme.bodySmall,
                ),
                const Spacer(),
                if (transfer.speed != null)
                  Text(
                    '${transfer.speed} • ',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: colorScheme.primary,
                    ),
                  ),
                Text(
                  '${(transfer.progress * 100).toStringAsFixed(0)}%',
                  style: theme.textTheme.bodySmall?.copyWith(
                    fontWeight: FontWeight.bold,
                    color: statusColor,
                  ),
                ),
                if (transfer.eta != null)
                  Text(
                    ' • ETA ${transfer.eta}',
                    style: theme.textTheme.bodySmall,
                  ),
              ],
            ),
            const SizedBox(height: 4),
            Text(
              '${transfer.direction} → ${transfer.remoteName}',
              style: theme.textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

// History transfer card.
class _HistoryTransferCard extends StatelessWidget {
  const _HistoryTransferCard({required this.transfer});

  final TransferInfo transfer;

  void _handleOpen(BuildContext context) async {
    final messenger = ScaffoldMessenger.of(context);
    final path = transfer.savedPath ?? '';
    if (path.isEmpty) {
      messenger.showSnackBar(
        const SnackBar(content: Text('File location not available')),
      );
      return;
    }

    final file = File(path);
    if (!file.existsSync()) {
      messenger.showSnackBar(
        SnackBar(content: Text('File not found at: $path')),
      );
      return;
    }

    if (Platform.isWindows) {
      try {
        await Process.run('cmd', ['/c', 'start', '', path]);
        return;
      } catch (_) {}
    } else if (Platform.isMacOS) {
      try {
        await Process.run('open', [path]);
        return;
      } catch (_) {}
    } else if (Platform.isLinux) {
      try {
        await Process.run('xdg-open', [path]);
        return;
      } catch (_) {}
    }

    Clipboard.setData(ClipboardData(text: path));
    messenger.showSnackBar(
      SnackBar(content: Text('Saved to: $path (Path copied)')),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final statusColor = transfer.statusColor(colorScheme);

    return Card(
      child: ListTile(
        onTap:
            transfer.status == 'Completed' ? () => _handleOpen(context) : null,
        leading: Container(
          width: 40,
          height: 40,
          decoration: BoxDecoration(
            color: statusColor.withOpacity(0.15),
            borderRadius: BorderRadius.circular(10),
          ),
          child: Icon(transfer.statusIcon, color: statusColor, size: 20),
        ),
        title: Text(
          transfer.fileName,
          style: theme.textTheme.titleSmall,
          overflow: TextOverflow.ellipsis,
        ),
        subtitle: Text(
          '${transfer.direction} • ${_formatBytes(transfer.totalBytes)} • ${transfer.remoteName}',
          style: theme.textTheme.bodySmall,
        ),
        trailing: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              transfer.status,
              style: theme.textTheme.labelSmall?.copyWith(color: statusColor),
            ),
            if (transfer.status == 'Completed' &&
                transfer.savedPath != null &&
                transfer.savedPath!.isNotEmpty) ...[
              const SizedBox(width: 6),
              const Icon(Icons.open_in_new_rounded, size: 16),
            ],
          ],
        ),
      ),
    );
  }
}
