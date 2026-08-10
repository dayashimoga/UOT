// Optical QR Scanner & Manual QR Link Pair Dialog
//
// Features camera viewfinder, manual QR payload paste/input, automatic URI parsing,
// and peer connection via Rust FFI engine_connect_peer.

import 'dart:async';
import 'package:flutter/material.dart';
import '../../platform/camera_qr_adapter.dart';
import '../../rust/api/engine_api.dart' as engine;

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
  final TextEditingController _manualInputController = TextEditingController();

  bool _isConnecting = false;
  String _statusMessage = 'Point camera at sender\'s QR Code or paste link';
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    _qrAdapter = CameraQrAdapter();
    _subscription = _qrAdapter.scanStream.listen(_onFrameScanned);
    _qrAdapter.startScanning();
  }

  void _onFrameScanned(QrScanResult result) {
    final rawData = result.rawData.trim();
    if (rawData.isNotEmpty) {
      _processQrPayload(rawData);
    }
  }

  Future<void> _processQrPayload(String rawPayload) async {
    if (_isConnecting) return;

    setState(() {
      _isConnecting = true;
      _errorMessage = null;
      _statusMessage = 'Connecting to peer from QR code...';
    });

    String targetAddress = rawPayload;

    // Parse uot://pair?ip=192.168.0.111&port=42000&pin=123456&name=Daya
    try {
      if (rawPayload.startsWith('uot://pair')) {
        final uri = Uri.parse(rawPayload);
        final ip = uri.queryParameters['ip'] ?? '127.0.0.1';
        final port = uri.queryParameters['port'] ?? '42000';
        targetAddress = '$ip:$port';
      }
    } catch (_) {}

    try {
      final res = await engine.engineConnectPeer(address: targetAddress);
      if (res.startsWith('error:')) {
        setState(() {
          _isConnecting = false;
          _errorMessage = res.replaceFirst('error:', '');
          _statusMessage = 'Connection failed';
        });
      } else {
        _qrAdapter.stopScanning();
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Connected to device at $targetAddress!'),
              backgroundColor: Colors.green,
            ),
          );
          Navigator.of(context).pop(targetAddress);
        }
      }
    } catch (e) {
      setState(() {
        _isConnecting = false;
        _errorMessage = e.toString();
        _statusMessage = 'Error connecting to peer';
      });
    }
  }

  @override
  void dispose() {
    _subscription.cancel();
    _qrAdapter.stopScanning();
    _qrAdapter.dispose();
    _manualInputController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return AlertDialog(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      title: Row(
        children: [
          Icon(Icons.qr_code_scanner_rounded, color: colorScheme.primary),
          const SizedBox(width: 10),
          const Text('Scan / Paste QR Code'),
        ],
      ),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Container(
              height: 180,
              width: double.infinity,
              decoration: BoxDecoration(
                color: Colors.black,
                borderRadius: BorderRadius.circular(16),
                border: Border.all(color: colorScheme.primary, width: 2),
              ),
              child: Stack(
                alignment: Alignment.center,
                children: [
                  Icon(
                    Icons.camera_alt_outlined,
                    size: 56,
                    color: colorScheme.onSurfaceVariant.withOpacity(0.5),
                  ),
                  if (_isConnecting)
                    const CircularProgressIndicator(color: Colors.white)
                  else
                    Positioned(
                      bottom: 12,
                      child: Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 6,
                        ),
                        decoration: BoxDecoration(
                          color: Colors.black.withOpacity(0.75),
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: Text(
                          _statusMessage,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: Colors.white,
                            fontWeight: FontWeight.w500,
                          ),
                        ),
                      ),
                    ),
                ],
              ),
            ),
            const SizedBox(height: 16),
            Text(
              'Or enter / paste QR Payload or IP:',
              style: theme.textTheme.bodySmall?.copyWith(
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(height: 6),
            TextField(
              controller: _manualInputController,
              decoration: InputDecoration(
                hintText: 'e.g. uot://pair?ip=192.168.0.111 or 192.168.0.111:42000',
                isDense: true,
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(10),
                ),
                suffixIcon: IconButton(
                  icon: const Icon(Icons.send_rounded, size: 18),
                  onPressed: () {
                    final text = _manualInputController.text.trim();
                    if (text.isNotEmpty) {
                      _processQrPayload(text);
                    }
                  },
                ),
              ),
            ),
            if (_errorMessage != null) ...[
              const SizedBox(height: 10),
              Text(
                _errorMessage!,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: colorScheme.error,
                ),
              ),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(null),
          child: const Text('Cancel'),
        ),
      ],
    );
  }
}
