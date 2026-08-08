// Receive Screen
//
// Controls receiving behavior: visibility, auto-accept, save location.
// Shows incoming transfer requests.

import 'package:flutter/material.dart';
import '../../rust/api/engine_api.dart';
import 'incoming_offer_dialog.dart';

class ReceiveScreen extends StatefulWidget {
  const ReceiveScreen({super.key});

  @override
  State<ReceiveScreen> createState() => _ReceiveScreenState();
}

class _ReceiveScreenState extends State<ReceiveScreen> {
  bool _isVisible = true;
  bool _autoAcceptTrusted = false;
  bool _requirePin = false;
  final String _savePath = 'Downloads/UOT';
  final List<Map<String, dynamic>> _pendingOffers = [];

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return SafeArea(
      child: CustomScrollView(
        slivers: [
          SliverAppBar(floating: true, title: const Text('Receive')),
          SliverPadding(
            padding: const EdgeInsets.all(16),
            sliver: SliverList(
              delegate: SliverChildListDelegate([
                // Visibility Card
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Container(
                              width: 48,
                              height: 48,
                              decoration: BoxDecoration(
                                color: _isVisible
                                    ? colorScheme.primaryContainer
                                    : colorScheme.surfaceContainerHighest,
                                borderRadius: BorderRadius.circular(12),
                              ),
                              child: Icon(
                                _isVisible
                                    ? Icons.visibility_rounded
                                    : Icons.visibility_off_rounded,
                                color: _isVisible
                                    ? colorScheme.onPrimaryContainer
                                    : colorScheme.onSurfaceVariant,
                              ),
                            ),
                            const SizedBox(width: 16),
                            Expanded(
                              child: Column(
                                crossAxisAlignment: CrossAxisAlignment.start,
                                children: [
                                  Text(
                                    _isVisible
                                        ? 'Visible to nearby devices'
                                        : 'Hidden from nearby devices',
                                    style: theme.textTheme.titleMedium,
                                  ),
                                  const SizedBox(height: 2),
                                  Text(
                                    _isVisible
                                        ? 'Other devices can find you and send files'
                                        : 'You won\'t receive incoming transfers',
                                    style: theme.textTheme.bodySmall?.copyWith(
                                      color: colorScheme.onSurfaceVariant,
                                    ),
                                  ),
                                ],
                              ),
                            ),
                            Switch(
                              value: _isVisible,
                              onChanged: (v) => setState(() => _isVisible = v),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                ),
                const SizedBox(height: 16),

                // Settings Section
                Text(
                  'RECEIVE SETTINGS',
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                    letterSpacing: 1.2,
                  ),
                ),
                const SizedBox(height: 8),
                Card(
                  child: Column(
                    children: [
                      SwitchListTile(
                        title: Text(
                          'Auto-accept from trusted',
                          style: theme.textTheme.titleSmall,
                        ),
                        subtitle: Text(
                          'Automatically accept transfers from paired devices',
                          style: theme.textTheme.bodySmall,
                        ),
                        secondary: Icon(
                          Icons.verified_user_rounded,
                          color: colorScheme.primary,
                        ),
                        value: _autoAcceptTrusted,
                        onChanged: (v) =>
                            setState(() => _autoAcceptTrusted = v),
                      ),
                      const Divider(height: 1, indent: 72),
                      SwitchListTile(
                        title: Text(
                          'Require PIN',
                          style: theme.textTheme.titleSmall,
                        ),
                        subtitle: Text(
                          'Require a PIN code for incoming connections',
                          style: theme.textTheme.bodySmall,
                        ),
                        secondary: Icon(
                          Icons.pin_rounded,
                          color: colorScheme.secondary,
                        ),
                        value: _requirePin,
                        onChanged: (v) => setState(() => _requirePin = v),
                      ),
                      const Divider(height: 1, indent: 72),
                      ListTile(
                        leading: Icon(
                          Icons.folder_rounded,
                          color: colorScheme.tertiary,
                        ),
                        title: Text(
                          'Save location',
                          style: theme.textTheme.titleSmall,
                        ),
                        subtitle: Text(
                          _savePath,
                          style: theme.textTheme.bodySmall,
                        ),
                        trailing: Icon(
                          Icons.chevron_right_rounded,
                          color: colorScheme.onSurfaceVariant,
                        ),
                        onTap: () {
                          // TODO: Directory picker
                        },
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 24),

                // Incoming Requests Section
                Text(
                  'INCOMING REQUESTS',
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                    letterSpacing: 1.2,
                  ),
                ),
                const SizedBox(height: 8),
                if (_pendingOffers.isEmpty)
                  _EmptyIncoming(colorScheme: colorScheme, theme: theme)
                else
                  ..._pendingOffers.map(
                    (offer) => _IncomingOfferCard(
                      offer: offer,
                      requirePin: _requirePin,
                      onResponse: (accepted) {
                        setState(() {
                          _pendingOffers.removeWhere(
                            (o) => o['id'] == offer['id'],
                          );
                        });
                      },
                    ),
                  ),
              ]),
            ),
          ),
        ],
      ),
    );
  }
}

class _IncomingOfferCard extends StatelessWidget {
  const _IncomingOfferCard({
    required this.offer,
    required this.requirePin,
    required this.onResponse,
  });

  final Map<String, dynamic> offer;
  final bool requirePin;
  final ValueChanged<bool> onResponse;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final String transferId = offer['id'] ?? '';
    final String fromDevice = offer['device'] ?? 'Unknown Device';
    final List<String> items = List<String>.from(offer['items'] ?? []);
    final int totalSize = offer['size'] ?? 0;

    return Card(
      margin: const EdgeInsets.only(bottom: 8),
      child: ListTile(
        leading: CircleAvatar(
          backgroundColor: colorScheme.primaryContainer,
          child: Icon(
            Icons.move_to_inbox_rounded,
            color: colorScheme.onPrimaryContainer,
          ),
        ),
        title: Text(fromDevice, style: theme.textTheme.titleSmall),
        subtitle: Text(
          '${items.length} file(s) • ${_formatSize(totalSize)}',
          style: theme.textTheme.bodySmall,
        ),
        trailing: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            IconButton(
              icon: Icon(Icons.close_rounded, color: colorScheme.error),
              tooltip: 'Decline',
              onPressed: () {
                try {
                  engineCancelTransfer(transferId: transferId);
                } catch (_) {}
                onResponse(false);
              },
            ),
            IconButton(
              icon: Icon(
                Icons.check_circle_rounded,
                color: colorScheme.primary,
              ),
              tooltip: 'Accept',
              onPressed: () async {
                final result = await IncomingOfferDialog.show(
                  context,
                  transferId: transferId,
                  fromDevice: fromDevice,
                  items: items,
                  totalSize: totalSize,
                  requirePin: requirePin,
                );
                if (result != null) {
                  onResponse(result);
                }
              },
            ),
          ],
        ),
      ),
    );
  }

  String _formatSize(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
}

class _EmptyIncoming extends StatelessWidget {
  const _EmptyIncoming({required this.colorScheme, required this.theme});

  final ColorScheme colorScheme;
  final ThemeData theme;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 32, horizontal: 16),
        child: Column(
          children: [
            Icon(
              Icons.inbox_rounded,
              size: 48,
              color: colorScheme.onSurfaceVariant.withValues(alpha: 0.4),
            ),
            const SizedBox(height: 12),
            Text(
              'No incoming requests',
              style: theme.textTheme.titleSmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              'Incoming transfer requests will appear here',
              style: theme.textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
