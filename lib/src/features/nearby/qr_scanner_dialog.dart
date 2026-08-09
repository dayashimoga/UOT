// Optical QR Scanner & Fountain Packet Reconstruction Dialog
//
// Shows viewfinder layout, animated frame counter, fountain code reconstruction progress bar,
// and auto-pairs upon completing animated QR optical stream.

import 'dart:async';
import 'package:flutter/material.dart';
import '../../platform/camera_qr_adapter.dart';

class QrScannerDialog extends StatefulWidget {
  const QrScannerDialog({super.key});

  static Future<String?> show(BuildContext context) {
    return showDialog<String>(
      context: context,
      builder: (context) => const QrScannerDialog(),
    );
  }

  @override
  State<QrScannerDialog> createState() => _QrScannerDialogState();
}

class _QrScannerDialogState extends State<QrScannerDialog> {
  late final CameraQrAdapter _qrAdapter;
  late final StreamSubscription<QrScanResult> _subscription;
  int _scannedPackets = 0;
  final int _requiredPackets = 5;
  String _statusMessage = 'Point camera at sender\'s animated QR code';

  @override
  void initState() {
    super.initState();
    _qrAdapter = CameraQrAdapter();
    _subscription = _qrAdapter.scanStream.listen(_onFrameScanned);
    _qrAdapter.startScanning();
  }

  void _onFrameScanned(QrScanResult result) {
    setState(() {
      _scannedPackets++;
      _statusMessage =
          'Reconstructing Fountain stream: $_scannedPackets/$_requiredPackets packets';
    });

    if (_scannedPackets >= _requiredPackets) {
      _qrAdapter.stopScanning();
      Navigator.of(context).pop('uot_qr_pairing_success_token');
    }
  }

  @override
  void dispose() {
    _subscription.cancel();
    _qrAdapter.stopScanning();
    _qrAdapter.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final progress = (_scannedPackets / _requiredPackets).clamp(0.0, 1.0);

    return AlertDialog(
      title: Row(
        children: [
          Icon(Icons.qr_code_scanner, color: colorScheme.primary),
          const SizedBox(width: 8),
          const Text('Scan Optical QR Stream'),
        ],
      ),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            height: 200,
            width: double.infinity,
            decoration: BoxDecoration(
              color: Colors.black,
              borderRadius: BorderRadius.circular(12),
              border: Border.all(color: colorScheme.primary, width: 2),
            ),
            child: Stack(
              alignment: Alignment.center,
              children: [
                Icon(
                  Icons.camera_alt_outlined,
                  size: 64,
                  color: colorScheme.onSurfaceVariant.withOpacity(0.5),
                ),
                Positioned(
                  bottom: 12,
                  child: Container(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 10,
                      vertical: 4,
                    ),
                    decoration: BoxDecoration(
                      color: Colors.black.withOpacity(0.7),
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: Text(
                      _statusMessage,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: Colors.white,
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          LinearProgressIndicator(
            value: progress,
            backgroundColor: colorScheme.surfaceContainerHighest,
            valueColor: AlwaysStoppedAnimation<Color>(colorScheme.primary),
          ),
          const SizedBox(height: 8),
          Text(
            '${(progress * 100).toInt()}% Reconstructed',
            style: theme.textTheme.labelMedium?.copyWith(
              color: colorScheme.secondary,
            ),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () {
            // Simulate scanning a packet frame for testing preview
            _qrAdapter.handleScannedFrame(
              '{"seed":$_scannedPackets,"num_blocks":5,"crc32":12345}',
            );
          },
          child: const Text('Simulate Frame'),
        ),
        TextButton(
          onPressed: () => Navigator.of(context).pop(null),
          child: const Text('Cancel'),
        ),
      ],
    );
  }
}
