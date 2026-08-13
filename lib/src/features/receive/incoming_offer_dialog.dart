// Incoming Offer & PIN Consent Dialog
//
// Prompt user to accept or decline incoming file transfers with optional PIN verification.

import 'package:flutter/material.dart';
import '../../rust/api/engine_api.dart';

class IncomingOfferDialog extends StatefulWidget {
  const IncomingOfferDialog({
    super.key,
    required this.transferId,
    required this.fromDevice,
    required this.items,
    required this.totalSize,
    this.requirePin = false,
  });

  final String transferId;
  final String fromDevice;
  final List<String> items;
  final int totalSize;
  final bool requirePin;

  static Future<bool?> show(
    BuildContext context, {
    required String transferId,
    required String fromDevice,
    required List<String> items,
    required int totalSize,
    bool requirePin = false,
  }) {
    return showDialog<bool>(
      context: context,
      barrierDismissible: false,
      builder: (context) => IncomingOfferDialog(
        transferId: transferId,
        fromDevice: fromDevice,
        items: items,
        totalSize: totalSize,
        requirePin: requirePin,
      ),
    );
  }

  @override
  State<IncomingOfferDialog> createState() => _IncomingOfferDialogState();
}

class _IncomingOfferDialogState extends State<IncomingOfferDialog> {
  final TextEditingController _pinController = TextEditingController();
  String? _errorText;
  bool _isLoading = false;

  String _formatBytes(int bytes) {
    if (bytes < 1024) return '$bytes B';
    if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
    if (bytes < 1024 * 1024 * 1024) {
      return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
    }
    return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }

  Future<void> _onAccept() async {
    if (widget.requirePin && _pinController.text.trim().length < 4) {
      setState(() {
        _errorText = 'Please enter a valid 4-digit or 6-digit PIN';
      });
      return;
    }

    setState(() => _isLoading = true);

    try {
      final result = await engineAcceptTransfer(transferId: widget.transferId);
      if (result.startsWith('error:')) {
        if (mounted) {
          setState(() {
            _errorText = 'Accept error: ${result.replaceFirst('error:', '')}';
            _isLoading = false;
          });
        }
        return;
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _errorText = 'Accept exception: $e';
          _isLoading = false;
        });
      }
      return;
    }

    if (mounted) {
      Navigator.of(context).pop(true);
    }
  }

  Future<void> _onDecline() async {
    setState(() => _isLoading = true);
    try {
      await engineCancelTransfer(transferId: widget.transferId);
    } catch (_) {}

    if (mounted) {
      Navigator.of(context).pop(false);
    }
  }

  @override
  void dispose() {
    _pinController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return AlertDialog(
      icon: Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: colorScheme.primaryContainer,
          shape: BoxShape.circle,
        ),
        child: Icon(
          Icons.move_to_inbox_rounded,
          color: colorScheme.onPrimaryContainer,
          size: 32,
        ),
      ),
      title: Text(
        'Incoming Transfer Offer',
        textAlign: TextAlign.center,
        style: theme.textTheme.titleLarge,
      ),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerHighest.withOpacity(0.5),
                borderRadius: BorderRadius.circular(12),
              ),
              child: Row(
                children: [
                  Icon(
                    Icons.devices_rounded,
                    color: colorScheme.primary,
                    size: 24,
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          widget.fromDevice,
                          style: theme.textTheme.titleMedium?.copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                        Text(
                          '${widget.items.length} file(s) • ${_formatBytes(widget.totalSize)}',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
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
              'FILES INCLUDED',
              style: theme.textTheme.labelSmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
                letterSpacing: 1.1,
              ),
            ),
            const SizedBox(height: 6),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              decoration: BoxDecoration(
                border: Border.all(color: colorScheme.outlineVariant),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: widget.items.map((item) {
                  return Padding(
                    padding: const EdgeInsets.symmetric(vertical: 4),
                    child: Row(
                      children: [
                        Icon(
                          Icons.insert_drive_file_outlined,
                          size: 16,
                          color: colorScheme.secondary,
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            item,
                            style: theme.textTheme.bodyMedium,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                      ],
                    ),
                  );
                }).toList(),
              ),
            ),
            if (widget.requirePin) ...[
              const SizedBox(height: 16),
              Text(
                'SECURITY PIN VERIFICATION',
                style: theme.textTheme.labelSmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                  letterSpacing: 1.1,
                ),
              ),
              const SizedBox(height: 6),
              TextField(
                controller: _pinController,
                keyboardType: TextInputType.number,
                maxLength: 6,
                decoration: InputDecoration(
                  labelText: 'Enter Receiver PIN',
                  errorText: _errorText,
                  prefixIcon: const Icon(Icons.pin_rounded),
                  border: const OutlineInputBorder(),
                  counterText: '',
                ),
              ),
            ],
          ],
        ),
      ),
      actions: [
        OutlinedButton.icon(
          onPressed: _isLoading ? null : _onDecline,
          icon: Icon(Icons.close_rounded, color: colorScheme.error),
          label: Text('Decline', style: TextStyle(color: colorScheme.error)),
        ),
        FilledButton.icon(
          onPressed: _isLoading ? null : _onAccept,
          icon: _isLoading
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2, color: Colors.white),
                )
              : const Icon(Icons.check_rounded),
          label: Text(_isLoading ? 'Accepting...' : 'Accept'),
        ),
      ],
    );
  }
}
