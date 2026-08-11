// Nearby Screen
//
// Shows discovered devices on the local network with real mDNS discovery.
// Primary entry point: "Select device → Select content → Send"

import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:file_picker/file_picker.dart';
import '../../rust/api/init.dart' as rust_api;
import '../../rust/api/engine_api.dart' as engine;
import 'qr_pairing_dialog.dart';
import 'optical_qr_sender_dialog.dart';
import 'active_transfer_dialog.dart';
import 'confirm_send_dialog.dart';
import 'instant_chat_dialog.dart';

// Device model parsed from JSON.
class DeviceInfo {
  final String deviceId;
  final String deviceName;
  final String deviceType;
  final String? address;
  final List<String> capabilities;

  DeviceInfo({
    required this.deviceId,
    required this.deviceName,
    required this.deviceType,
    this.address,
    this.capabilities = const [],
  });

  factory DeviceInfo.fromJson(Map<String, dynamic> json) {
    return DeviceInfo(
      deviceId: json['device_id'] ?? '',
      deviceName: json['device_name'] ?? 'Unknown',
      deviceType: json['device_type'] ?? 'Unknown',
      address: json['address'],
      capabilities: (json['capabilities'] as List<dynamic>?)
              ?.map((e) => e.toString())
              .toList() ??
          [],
    );
  }

  IconData get icon {
    switch (deviceType) {
      case 'Phone':
        return Icons.phone_android_rounded;
      case 'Tablet':
        return Icons.tablet_rounded;
      case 'Laptop':
        return Icons.laptop_rounded;
      case 'Desktop':
        return Icons.desktop_windows_rounded;
      case 'Tv':
        return Icons.tv_rounded;
      default:
        return Icons.devices_rounded;
    }
  }
}

// Nearby devices discovery screen.
class NearbyScreen extends StatefulWidget {
  const NearbyScreen({super.key});

  @override
  State<NearbyScreen> createState() => _NearbyScreenState();
}

class _NearbyScreenState extends State<NearbyScreen>
    with SingleTickerProviderStateMixin {
  late final AnimationController _pulseController;
  String _coreVersion = '';
  String _healthStatus = '';
  String _engineState = 'Stopped';
  final List<DeviceInfo> _devices = [];
  Timer? _refreshTimer;
  bool _isScanning = true;
  bool _engineInitialized = false;

  @override
  void initState() {
    super.initState();
    _pulseController = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 2),
    )..repeat();
    _loadCoreInfo();
    _initEngine();
  }

  Future<void> _loadCoreInfo() async {
    final version = rust_api.getVersion();
    final health = rust_api.healthCheck();
    if (mounted) {
      setState(() {
        _coreVersion = version;
        _healthStatus = health;
      });
    }
  }

  Future<void> _initEngine() async {
    final result = await engine.engineInit();
    if (mounted) {
      setState(() {
        _engineInitialized = true;
        _engineState = result.startsWith('ok') ? 'Running' : 'Partial';
      });
    }
    _startRefresh();
  }

  int _refreshCount = 0;

  void _startRefresh() {
    _refreshDevices();
    _refreshTimer = Timer.periodic(const Duration(seconds: 2), (_) {
      _refreshCount++;
      _refreshDevices();
      // Periodically trigger subnet scan every 6 seconds if scanning is active
      if (_isScanning && _refreshCount % 3 == 0) {
        engine.engineSubnetScan();
      }
    });
  }

  void _refreshDevices() {
    if (!mounted || !_engineInitialized) return;
    try {
      final devicesJson = engine.engineGetDevices();
      final List<dynamic> parsed = jsonDecode(devicesJson);
      final newDevices = parsed
          .map((d) => DeviceInfo.fromJson(d as Map<String, dynamic>))
          .toList();
      setState(() {
        _devices.clear();
        _devices.addAll(newDevices);
        _engineState = engine.engineState();
      });
    } catch (_) {
      // Silently handle parse errors during polling
    }
  }

  void _triggerSubnetScan() {
    engine.engineSubnetScan();
    _refreshDevices();
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('Scanning local network subnet for UOT devices...'),
        duration: Duration(seconds: 2),
      ),
    );
  }

  @override
  void dispose() {
    _refreshTimer?.cancel();
    _pulseController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    String localIp = '127.0.0.1';
    try {
      final ipsJson = engine.engineGetLocalIps();
      final List<dynamic> ips = jsonDecode(ipsJson);
      if (ips.isNotEmpty) localIp = ips.first.toString();
    } catch (_) {}

    String devName = 'My Device';
    try {
      final settingsJson = engine.engineLoadSettings();
      final map = jsonDecode(settingsJson) as Map<String, dynamic>;
      devName = map['device_name']?.toString() ?? 'UOT Device';
    } catch (_) {}

    return SafeArea(
      child: CustomScrollView(
        slivers: [
          SliverAppBar(
            floating: true,
            title: const Text('Nearby'),
            actions: [
              IconButton(
                icon: const Icon(Icons.qr_code_scanner_rounded),
                onPressed: () => QrPairingDialog.show(context),
                tooltip: 'Scan QR Code / Direct Connect',
              ),
              IconButton(
                icon: Icon(
                  _isScanning ? Icons.pause_rounded : Icons.play_arrow_rounded,
                ),
                onPressed: () {
                  setState(() => _isScanning = !_isScanning);
                },
                tooltip: _isScanning ? 'Pause Scan' : 'Resume Scan',
              ),
            ],
          ),
          SliverPadding(
            padding: const EdgeInsets.all(16),
            sliver: SliverList(
              delegate: SliverChildListDelegate([
                // Engine Status Card
                _EngineStatusCard(
                  colorScheme: colorScheme,
                  pulseController: _pulseController,
                  engineState: _engineState,
                  version: _coreVersion,
                  healthStatus: _healthStatus,
                  isScanning: _isScanning,
                ),
                const SizedBox(height: 12),

                // My Device & Quick Connect Header
                _MyDeviceBanner(
                  colorScheme: colorScheme,
                  deviceName: devName,
                  localIp: localIp,
                  onOpenQr: () => QrPairingDialog.show(context),
                  onOpenDirectConnect: () => QrPairingDialog.show(context),
                  onScanSubnet: _triggerSubnetScan,
                ),
                const SizedBox(height: 16),

                // Device list or scanning indicator
                if (_devices.isEmpty) ...[
                  _ScanningIndicator(
                    pulseController: _pulseController,
                    colorScheme: colorScheme,
                    isScanning: _isScanning,
                  ),
                ] else ...[
                  Text(
                    '${_devices.length} device${_devices.length == 1 ? '' : 's'} found',
                    style: theme.textTheme.titleSmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 12),
                  ..._devices.map(
                    (device) => Padding(
                      padding: const EdgeInsets.only(bottom: 8),
                      child: _DeviceCard(
                        device: device,
                        onTap: () => _onDeviceTap(device),
                      ),
                    ),
                  ),
                ],
              ]),
            ),
          ),
        ],
      ),
    );
  }

  void _onDeviceTap(DeviceInfo device) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      builder: (ctx) => _SendBottomSheet(device: device),
    );
  }
}

// Engine status + core info combined card.
class _EngineStatusCard extends StatelessWidget {
  const _EngineStatusCard({
    required this.colorScheme,
    required this.pulseController,
    required this.engineState,
    required this.version,
    required this.healthStatus,
    required this.isScanning,
  });

  final ColorScheme colorScheme;
  final AnimationController pulseController;
  final String engineState;
  final String version;
  final String healthStatus;
  final bool isScanning;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isRunning =
        engineState == 'Running' || engineState == 'Partial' || engineState == 'Starting';

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          children: [
            Row(
              children: [
                AnimatedBuilder(
                  animation: pulseController,
                  builder: (context, child) {
                    return Container(
                      width: 12,
                      height: 12,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: isRunning
                            ? colorScheme.primary.withOpacity(
                                0.5 + (pulseController.value * 0.5),
                              )
                            : colorScheme.error,
                        boxShadow: isRunning && isScanning
                            ? [
                                BoxShadow(
                                  color: colorScheme.primary.withOpacity(0.3),
                                  blurRadius: 8 * pulseController.value,
                                  spreadRadius: 2 * pulseController.value,
                                ),
                              ]
                            : null,
                      ),
                    );
                  },
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        isRunning
                            ? (isScanning
                                ? 'Scanning for devices…'
                                : 'Discovery paused')
                            : 'Engine starting…',
                        style: theme.textTheme.titleMedium,
                      ),
                      const SizedBox(height: 2),
                      Text(
                        isRunning
                            ? 'UOT Core v$version • ${healthStatus.isNotEmpty ? "Healthy" : "Checking…"}'
                            : 'Initializing Rust core engine',
                        style: theme.textTheme.bodySmall,
                      ),
                    ],
                  ),
                ),
                Icon(
                  isRunning ? Icons.wifi_rounded : Icons.wifi_off_rounded,
                  color: isRunning ? colorScheme.primary : colorScheme.error,
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

// Scanning animation + hint.
class _ScanningIndicator extends StatelessWidget {
  const _ScanningIndicator({
    required this.pulseController,
    required this.colorScheme,
    required this.isScanning,
  });

  final AnimationController pulseController;
  final ColorScheme colorScheme;
  final bool isScanning;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Column(
      children: [
        const SizedBox(height: 32),
        AnimatedBuilder(
          animation: pulseController,
          builder: (context, child) {
            return Stack(
              alignment: Alignment.center,
              children: [
                if (isScanning) ...[
                  Container(
                    width: 120 + (40 * pulseController.value),
                    height: 120 + (40 * pulseController.value),
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      border: Border.all(
                        color: colorScheme.primary.withOpacity(
                          0.1 * (1 - pulseController.value),
                        ),
                        width: 2,
                      ),
                    ),
                  ),
                  Container(
                    width: 100 + (20 * pulseController.value),
                    height: 100 + (20 * pulseController.value),
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      border: Border.all(
                        color: colorScheme.primary.withOpacity(
                          0.2 * (1 - pulseController.value),
                        ),
                        width: 2,
                      ),
                    ),
                  ),
                ],
                Icon(
                  Icons.radar_rounded,
                  size: 64,
                  color: colorScheme.primary.withOpacity(
                    isScanning ? 0.3 + (pulseController.value * 0.7) : 0.3,
                  ),
                ),
              ],
            );
          },
        ),
        const SizedBox(height: 24),
        Text(
          isScanning
              ? 'Looking for UOT devices on your network…'
              : 'Scanning paused',
          style: theme.textTheme.bodyMedium?.copyWith(
            color: colorScheme.onSurfaceVariant,
          ),
          textAlign: TextAlign.center,
        ),
        const SizedBox(height: 8),
        Text(
          'Make sure devices are on the same Wi-Fi network',
          style: theme.textTheme.bodySmall?.copyWith(
            color: colorScheme.onSurfaceVariant.withOpacity(0.7),
          ),
          textAlign: TextAlign.center,
        ),
      ],
    );
  }
}

// Device card showing discovered device with connect action.
class _DeviceCard extends StatelessWidget {
  const _DeviceCard({required this.device, required this.onTap});

  final DeviceInfo device;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Container(
                width: 48,
                height: 48,
                decoration: BoxDecoration(
                  color: colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Icon(
                  device.icon,
                  color: colorScheme.onPrimaryContainer,
                  size: 24,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(device.deviceName, style: theme.textTheme.titleMedium),
                    const SizedBox(height: 2),
                    Text(
                      '${device.deviceType} • ${device.capabilities.join(", ")}',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              Icon(Icons.send_rounded, color: colorScheme.primary),
            ],
          ),
        ),
      ),
    );
  }
}

// Bottom sheet for selecting files to send.
class _SendBottomSheet extends StatelessWidget {
  const _SendBottomSheet({required this.device});

  final DeviceInfo device;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return DraggableScrollableSheet(
      initialChildSize: 0.5,
      minChildSize: 0.3,
      maxChildSize: 0.85,
      expand: false,
      builder: (context, scrollController) {
        return Container(
          decoration: BoxDecoration(
            color: theme.scaffoldBackgroundColor,
            borderRadius: const BorderRadius.vertical(top: Radius.circular(20)),
          ),
          child: ListView(
            controller: scrollController,
            padding: const EdgeInsets.all(24),
            children: [
              // Handle
              Center(
                child: Container(
                  width: 40,
                  height: 4,
                  decoration: BoxDecoration(
                    color: colorScheme.onSurfaceVariant.withOpacity(0.3),
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
              ),
              const SizedBox(height: 20),
              // Header
              Row(
                children: [
                  Container(
                    width: 40,
                    height: 40,
                    decoration: BoxDecoration(
                      color: colorScheme.primaryContainer,
                      borderRadius: BorderRadius.circular(10),
                    ),
                    child: Icon(
                      device.icon,
                      color: colorScheme.onPrimaryContainer,
                      size: 20,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Send to ${device.deviceName}',
                          style: theme.textTheme.titleLarge,
                        ),
                        Text(
                          device.deviceType,
                          style: theme.textTheme.bodySmall,
                        ),
                      ],
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 24),
              // Content options
              _SendOption(
                icon: Icons.insert_drive_file_rounded,
                title: 'Files',
                subtitle: 'Select one or more files',
                color: colorScheme.primary,
                onTap: () async {
                  Navigator.pop(context);
                  final result = await FilePicker.platform.pickFiles(
                    allowMultiple: true,
                    type: FileType.any,
                  );
                  if (result != null && result.files.isNotEmpty) {
                    final paths = result.files
                        .where((f) => f.path != null)
                        .map((f) => f.path!)
                        .toList();
                    if (paths.isNotEmpty && context.mounted) {
                      final confirm = await ConfirmSendDialog.show(
                        context,
                        targetDeviceName: device.deviceName,
                        targetAddress: device.address ?? 'LAN Node',
                        filePaths: paths,
                      );
                      if (confirm == true && context.mounted) {
                        final sendRes = await engine.engineSendFiles(
                          deviceId: device.deviceId,
                          filePaths: paths,
                        );
                        if (context.mounted) {
                          _showSendFeedback(context, sendRes, device.deviceName, paths.length);
                        }
                      }
                    }
                  }
                },
              ),
              const SizedBox(height: 8),
              _SendOption(
                icon: Icons.folder_rounded,
                title: 'Folder',
                subtitle: 'Send an entire folder',
                color: colorScheme.tertiary,
                onTap: () async {
                  Navigator.pop(context);
                  final dir = await FilePicker.platform.getDirectoryPath();
                  if (dir != null && context.mounted) {
                    final confirm = await ConfirmSendDialog.show(
                      context,
                      targetDeviceName: device.deviceName,
                      targetAddress: device.address ?? 'LAN Node',
                      filePaths: [dir],
                    );
                    if (confirm == true && context.mounted) {
                      final sendRes = await engine.engineSendFiles(
                        deviceId: device.deviceId,
                        filePaths: [dir],
                      );
                      if (context.mounted) {
                        _showSendFeedback(context, sendRes, device.deviceName, 1);
                      }
                    }
                  }
                },
              ),
              const SizedBox(height: 8),
              _SendOption(
                icon: Icons.chat_rounded,
                title: 'Instant Message / Chat',
                subtitle: 'Send text messages & ping peer connection',
                color: Colors.green,
                onTap: () {
                  Navigator.pop(context);
                  InstantChatDialog.show(
                    context,
                    deviceId: device.deviceId,
                    deviceName: device.deviceName,
                  );
                },
              ),
              const SizedBox(height: 8),
              _SendOption(
                icon: Icons.content_paste_rounded,
                title: 'Clipboard',
                subtitle: 'Send text, URL, or image from clipboard',
                color: colorScheme.secondary,
                onTap: () async {
                  Navigator.pop(context);
                  final data = await Clipboard.getData(Clipboard.kTextPlain);
                  if (data?.text != null && data!.text!.isNotEmpty) {
                    final sendRes = await engine.engineSendClipboard(
                      deviceId: device.deviceId,
                      text: data.text!,
                    );
                    if (context.mounted) {
                      _showSendFeedback(context, sendRes, device.deviceName, 1);
                    }
                  } else if (context.mounted) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(
                        content: Text('Clipboard is empty!'),
                        duration: Duration(seconds: 2),
                      ),
                    );
                  }
                },
              ),
              const SizedBox(height: 8),
              _SendOption(
                icon: Icons.qr_code_2_rounded,
                title: 'Optical Animated QR Stream',
                subtitle: 'Zero network required • Air-gapped optical transfer',
                color: colorScheme.tertiaryContainer,
                onTap: () async {
                  Navigator.pop(context);
                  final result = await FilePicker.platform.pickFiles(
                    allowMultiple: false,
                    type: FileType.any,
                  );
                  if (result != null && result.files.isNotEmpty) {
                    final file = result.files.first;
                    final name = file.name;
                    final path = file.path;
                    if (path != null && context.mounted) {
                      await OpticalQrSenderDialog.show(
                        context,
                        fileName: name,
                        payloadText: 'UOT-AIRGAPPED-PAYLOAD:$name',
                      );
                    }
                  }
                },
              ),
            ],
          ),
        );
      },
    );
  }
}

// Send option tile.
class _SendOption extends StatelessWidget {
  const _SendOption({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.color,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final Color color;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      clipBehavior: Clip.antiAlias,
      child: ListTile(
        leading: Container(
          width: 44,
          height: 44,
          decoration: BoxDecoration(
            color: color.withOpacity(0.15),
            borderRadius: BorderRadius.circular(10),
          ),
          child: Icon(icon, color: color),
        ),
        title: Text(title, style: theme.textTheme.titleSmall),
        subtitle: Text(subtitle, style: theme.textTheme.bodySmall),
        trailing: Icon(
          Icons.chevron_right_rounded,
          color: theme.colorScheme.onSurfaceVariant,
        ),
        onTap: onTap,
      ),
    );
  }
}

class _MyDeviceBanner extends StatelessWidget {
  final ColorScheme colorScheme;
  final String deviceName;
  final String localIp;
  final VoidCallback onOpenQr;
  final VoidCallback onOpenDirectConnect;
  final VoidCallback onScanSubnet;

  const _MyDeviceBanner({
    required this.colorScheme,
    required this.deviceName,
    required this.localIp,
    required this.onOpenQr,
    required this.onOpenDirectConnect,
    required this.onScanSubnet,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      elevation: 0,
      color: colorScheme.surfaceContainerHigh,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: BorderSide(color: colorScheme.outlineVariant),
      ),
      child: Padding(
        padding: const EdgeInsets.all(14),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  Icons.laptop_chromebook_rounded,
                  color: colorScheme.primary,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        deviceName,
                        style: theme.textTheme.titleSmall?.copyWith(
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      Text(
                        'IP: $localIp:${engine.engineGetListeningPort()}',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                FilledButton.tonalIcon(
                  onPressed: onOpenQr,
                  icon: const Icon(Icons.qr_code_2_rounded, size: 18),
                  label: const Text('Pair / Show QR'),
                ),
                OutlinedButton.icon(
                  onPressed: onOpenDirectConnect,
                  icon: const Icon(Icons.lan_rounded, size: 18),
                  label: const Text('Direct IP Connect'),
                ),
                IconButton.outlined(
                  onPressed: onScanSubnet,
                  tooltip: 'Scan Local Subnet (LAN)',
                  icon: const Icon(Icons.radar_rounded, size: 18),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

void _showSendFeedback(
    BuildContext context, String result, String deviceName, int count) {
  if (result.startsWith('ok')) {
    final transferId = result.replaceFirst('ok:', '');
    ActiveTransferDialog.show(
      context,
      transferId: transferId.isEmpty
          ? 'tx-${DateTime.now().millisecondsSinceEpoch}'
          : transferId,
      targetDeviceName: deviceName,
      fileCount: count,
    );
  } else {
    final errorMsg = result.replaceFirst('error:', '');
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Transfer Error'),
        content: Text('Could not send to $deviceName:\n\n$errorMsg'),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('OK'),
          ),
        ],
      ),
    );
  }
}


