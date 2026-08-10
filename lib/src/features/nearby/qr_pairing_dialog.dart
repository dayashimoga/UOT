// Optical QR Pairing & Direct IP Connect Modal Dialog
//
// Provides dual-tab interface:
// 1. "My QR Code & IP": Displays device QR Code, 6-digit PIN, and local IPv4 address with copy button.
// 2. "Direct IP / Connect": Text input to connect directly to peer IP address (e.g. 192.168.1.50).

import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:qr_flutter/qr_flutter.dart';
import '../../rust/api/engine_api.dart' as engine;
import 'qr_scanner_dialog.dart';

class QrPairingDialog extends StatefulWidget {
  const QrPairingDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog<void>(
      context: context,
      builder: (context) => const QrPairingDialog(),
    );
  }

  @override
  State<QrPairingDialog> createState() => _QrPairingDialogState();
}

class _QrPairingDialogState extends State<QrPairingDialog>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;
  final TextEditingController _ipController = TextEditingController();

  String _localIp = 'Loading...';
  String _pin = '------';
  String _qrPayload = '';
  bool _isConnecting = false;
  String? _connectError;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    _loadDeviceInfo();
  }

  void _loadDeviceInfo() {
    try {
      final pin = engine.engineGeneratePin(ttlSecs: BigInt.from(300));
      final devNameJson = engine.engineLoadSettings();
      String name = 'Device';
      try {
        final settingsMap = jsonDecode(devNameJson) as Map<String, dynamic>;
        name = settingsMap['device_name']?.toString() ?? 'UOT Device';
      } catch (_) {}

      final ipsJson = engine.engineGetLocalIps();
      List<dynamic> ips = [];
      try {
        ips = jsonDecode(ipsJson) as List<dynamic>;
      } catch (_) {}

      final mainIp = ips.isNotEmpty ? ips.first.toString() : '127.0.0.1';

      final qrData =
          'uot://pair?ip=$mainIp&port=42000&pin=$pin&name=${Uri.encodeComponent(name)}';

      setState(() {
        _pin = pin;
        _localIp = mainIp;
        _qrPayload = qrData;
      });
    } catch (e) {
      setState(() {
        _localIp = '127.0.0.1';
        _qrPayload = 'uot://pair?ip=127.0.0.1&port=42000';
      });
    }
  }

  Future<void> _connectDirectIp() async {
    final rawInput = _ipController.text.trim();
    if (rawInput.isEmpty) {
      setState(() => _connectError = 'Please enter an IP address');
      return;
    }

    setState(() {
      _isConnecting = true;
      _connectError = null;
    });

    try {
      final res = await engine.engineConnectPeer(address: rawInput);
      if (res.startsWith('error:')) {
        setState(() {
          _isConnecting = false;
          _connectError = res.replaceFirst('error:', '');
        });
      } else {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Connected to $rawInput'),
              backgroundColor: Colors.green,
            ),
          );
          Navigator.of(context).pop();
        }
      }
    } catch (e) {
      setState(() {
        _isConnecting = false;
        _connectError = e.toString();
      });
    }
  }

  @override
  void dispose() {
    _tabController.dispose();
    _ipController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Dialog(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 440, maxHeight: 560),
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(20, 16, 8, 8),
              child: Row(
                children: [
                  Icon(Icons.qr_code_2_rounded, color: colorScheme.primary),
                  const SizedBox(width: 10),
                  Text(
                    'Device Pairing & QR',
                    style: theme.textTheme.titleMedium?.copyWith(
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  const Spacer(),
                  IconButton(
                    icon: const Icon(Icons.close_rounded),
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
            ),
            TabBar(
              controller: _tabController,
              tabs: const [
                Tab(
                  icon: Icon(Icons.qr_code_rounded, size: 20),
                  text: 'My QR & IP',
                ),
                Tab(
                  icon: Icon(Icons.lan_rounded, size: 20),
                  text: 'Direct IP Connect',
                ),
              ],
            ),
            Expanded(
              child: TabBarView(
                controller: _tabController,
                children: [
                  // Tab 1: Show My QR Code
                  SingleChildScrollView(
                    padding: const EdgeInsets.all(20),
                    child: Column(
                      children: [
                        Container(
                          padding: const EdgeInsets.all(12),
                          decoration: BoxDecoration(
                            color: Colors.white,
                            borderRadius: BorderRadius.circular(16),
                            border: Border.all(
                              color: colorScheme.outlineVariant,
                            ),
                          ),
                          child: QrImageView(
                            data: _qrPayload.isNotEmpty
                                ? _qrPayload
                                : 'uot://pair',
                            version: QrVersions.auto,
                            size: 180.0,
                            backgroundColor: Colors.white,
                          ),
                        ),
                        const SizedBox(height: 16),
                        Row(
                          mainAxisAlignment: MainAxisAlignment.center,
                          children: [
                            Text(
                              'PIN: ',
                              style: theme.textTheme.bodyMedium?.copyWith(
                                color: colorScheme.onSurfaceVariant,
                              ),
                            ),
                            Container(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 12,
                                vertical: 4,
                              ),
                              decoration: BoxDecoration(
                                color: colorScheme.primaryContainer,
                                borderRadius: BorderRadius.circular(8),
                              ),
                              child: Text(
                                _pin,
                                style: theme.textTheme.titleMedium?.copyWith(
                                  fontWeight: FontWeight.bold,
                                  letterSpacing: 2,
                                  color: colorScheme.onPrimaryContainer,
                                ),
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 12),
                        Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 14,
                            vertical: 10,
                          ),
                          decoration: BoxDecoration(
                            color: colorScheme.surfaceContainerHighest,
                            borderRadius: BorderRadius.circular(12),
                          ),
                          child: Row(
                            children: [
                              Icon(
                                Icons.wifi_rounded,
                                size: 18,
                                color: colorScheme.primary,
                              ),
                              const SizedBox(width: 8),
                              Expanded(
                                child: Text(
                                  'My IP: $_localIp:42000',
                                  style: theme.textTheme.bodyMedium?.copyWith(
                                    fontWeight: FontWeight.w600,
                                  ),
                                ),
                              ),
                              IconButton(
                                icon: const Icon(Icons.copy_rounded, size: 18),
                                tooltip: 'Copy IP',
                                onPressed: () {
                                  Clipboard.setData(
                                    ClipboardData(text: '$_localIp:42000'),
                                  );
                                  ScaffoldMessenger.of(context).showSnackBar(
                                    const SnackBar(
                                      content: Text('IP copied to clipboard!'),
                                      duration: Duration(seconds: 2),
                                    ),
                                  );
                                },
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),

                  // Tab 2: Connect directly via IP
                  SingleChildScrollView(
                    padding: const EdgeInsets.all(20),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Connect via LAN IP Address',
                          style: theme.textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                        const SizedBox(height: 6),
                        Text(
                          'Enter the IP address of another UOT device on your Wi-Fi network.',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                        const SizedBox(height: 16),
                        TextField(
                          controller: _ipController,
                          keyboardType: TextInputType.text,
                          decoration: InputDecoration(
                            labelText: 'Peer IP Address or Address:Port',
                            hintText: 'e.g. 192.168.1.50 or 192.168.1.50:42000',
                            prefixIcon: const Icon(Icons.computer_rounded),
                            border: OutlineInputBorder(
                              borderRadius: BorderRadius.circular(12),
                            ),
                          ),
                        ),
                        if (_connectError != null) ...[
                          const SizedBox(height: 10),
                          Text(
                            _connectError!,
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: colorScheme.error,
                            ),
                          ),
                        ],
                        const SizedBox(height: 20),
                        SizedBox(
                          width: double.infinity,
                          height: 48,
                          child: FilledButton.icon(
                            onPressed: _isConnecting ? null : _connectDirectIp,
                            icon: _isConnecting
                                ? const SizedBox(
                                    width: 18,
                                    height: 18,
                                    child: CircularProgressIndicator(
                                      strokeWidth: 2,
                                      color: Colors.white,
                                    ),
                                  )
                                : const Icon(Icons.link_rounded),
                            label: Text(
                              _isConnecting
                                  ? 'Connecting...'
                                  : 'Connect to Device',
                            ),
                          ),
                        ),
                        const SizedBox(height: 16),
                        const Divider(),
                        const SizedBox(height: 12),
                        SizedBox(
                          width: double.infinity,
                          child: OutlinedButton.icon(
                            onPressed: () async {
                              Navigator.of(context).pop();
                              await QrScannerDialog.show(context);
                            },
                            icon: const Icon(Icons.camera_alt_outlined),
                            label: const Text('Open Camera QR Scanner'),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
