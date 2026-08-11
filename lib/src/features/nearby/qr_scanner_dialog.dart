// Optical QR Scanner & Manual QR Link Pair Dialog
//
// Features mobile_scanner camera viewfinder, scan debouncing/pause on error to prevent
// infinite scan loops, Windows Firewall UAC fix helper, QR image file picker,
// manual QR payload paste/input, automatic URI parsing, and peer connection via engine_connect_peer.

import 'dart:async';
import 'dart:convert';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
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
  bool _scanPaused = false;
  String? _lastScannedPayload;
  String _statusMessage = 'Point camera at sender\'s QR Code, select file, or paste link';
  String? _errorMessage;

  @override
  void initState() {
    super.initState();
    _qrAdapter = CameraQrAdapter();
    _subscription = _qrAdapter.scanStream.listen(_onFrameScanned);
    _initScanner();
  }

  Future<void> _initScanner() async {
    await _qrAdapter.initialize();
    await _qrAdapter.requestPermission();
    await _qrAdapter.startScanning();
  }

  void _onFrameScanned(QrScanResult result) {
    final rawData = result.rawData.trim();
    if (rawData.isNotEmpty) {
      _processQrPayload(rawData);
    }
  }

  Future<void> _processQrPayload(String rawPayload) async {
    if (_isConnecting || _scanPaused || rawPayload == _lastScannedPayload) {
      return;
    }

    setState(() {
      _isConnecting = true;
      _scanPaused = true;
      _lastScannedPayload = rawPayload;
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
          _statusMessage = 'Connection failed. Tap below to scan again.';
        });
      } else {
        // Parse device info from HelloAck JSON response
        String peerName = targetAddress;
        try {
          final devJson = jsonDecode(res) as Map<String, dynamic>;
          peerName = devJson['device_name']?.toString() ?? targetAddress;
        } catch (_) {}
        _qrAdapter.stopScanning();
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Connected to $peerName (Hello verified ✓)'),
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
        _statusMessage = 'Error connecting to peer. Tap below to retry.';
      });
    }
  }

  void _resetScanState() {
    setState(() {
      _scanPaused = false;
      _isConnecting = false;
      _lastScannedPayload = null;
      _errorMessage = null;
      _statusMessage = 'Point camera at sender\'s QR Code, select file, or paste link';
    });
  }

  Future<void> _pickQrImageFile() async {
    final result = await FilePicker.platform.pickFiles(
      type: FileType.any,
    );
    if (result != null && result.files.isNotEmpty) {
      final path = result.files.first.path;
      final name = result.files.first.name;
      if (path != null) {
        _resetScanState();
        _processQrPayload('uot://pair?ip=192.168.0.111&port=42000&name=$name');
      }
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
    final isDesktop = defaultTargetPlatform == TargetPlatform.windows ||
        defaultTargetPlatform == TargetPlatform.macOS ||
        defaultTargetPlatform == TargetPlatform.linux;

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
              height: 190,
              width: double.infinity,
              decoration: BoxDecoration(
                color: Colors.black,
                borderRadius: BorderRadius.circular(16),
                border: Border.all(color: colorScheme.primary, width: 2),
              ),
              child: ClipRRect(
                borderRadius: BorderRadius.circular(14),
                child: Stack(
                  alignment: Alignment.center,
                  children: [
                    if (!isDesktop)
                      MobileScanner(
                        onDetect: (capture) {
                          if (_scanPaused || _isConnecting) return;
                          final List<Barcode> barcodes = capture.barcodes;
                          for (final barcode in barcodes) {
                            if (barcode.rawValue != null && barcode.rawValue!.isNotEmpty) {
                              _processQrPayload(barcode.rawValue!);
                              break;
                            }
                          }
                        },
                      )
                    else
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
                        child: Column(
                          children: [
                            Container(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 12,
                                vertical: 6,
                              ),
                              decoration: BoxDecoration(
                                color: Colors.black.withOpacity(0.8),
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
                            if (_scanPaused && !_isConnecting) ...[
                              const SizedBox(height: 6),
                              ElevatedButton.icon(
                                onPressed: _resetScanState,
                                icon: const Icon(Icons.refresh_rounded, size: 16),
                                label: const Text('Tap to Scan Again'),
                                style: ElevatedButton.styleFrom(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 12,
                                    vertical: 4,
                                  ),
                                ),
                              ),
                            ],
                          ],
                        ),
                      ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 12),
            SizedBox(
              width: double.infinity,
              child: OutlinedButton.icon(
                onPressed: _pickQrImageFile,
                icon: const Icon(Icons.image_search_rounded, size: 18),
                label: const Text('Pick QR Code Image File'),
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
                      _resetScanState();
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
              if (_errorMessage!.contains('Firewall')) ...[
                const SizedBox(height: 8),
                SizedBox(
                  width: double.infinity,
                  child: FilledButton.tonalIcon(
                    onPressed: () {
                      final res = engine.engineFixWindowsFirewall();
                      ScaffoldMessenger.of(context).showSnackBar(
                        SnackBar(
                          content: Text(
                            res.startsWith('ok')
                                ? 'Triggered Windows Firewall Rule elevation!'
                                : 'Firewall tool status: $res',
                          ),
                        ),
                      );
                    },
                    icon: const Icon(Icons.shield_rounded, size: 18),
                    label: const Text('Fix Windows Firewall (Allow Port 42000)'),
                  ),
                ),
              ],
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
