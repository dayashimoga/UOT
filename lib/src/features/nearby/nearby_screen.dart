// Nearby Screen
//
// Shows discovered devices on the local network.
// Primary entry point: "Select device → Select content → Send"

import 'package:flutter/material.dart';
import '../../rust/api/init.dart' as rust_api;

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

  @override
  void initState() {
    super.initState();
    _pulseController = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 2),
    )..repeat();
    _loadCoreInfo();
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

  @override
  void dispose() {
    _pulseController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return SafeArea(
      child: CustomScrollView(
        slivers: [
          SliverAppBar(
            floating: true,
            title: const Text('Nearby'),
            actions: [
              IconButton(
                icon: const Icon(Icons.qr_code_scanner_rounded),
                onPressed: () {},
                tooltip: 'Scan QR Code',
              ),
            ],
          ),
          SliverPadding(
            padding: const EdgeInsets.all(16),
            sliver: SliverList(
              delegate: SliverChildListDelegate([
                // Connection Status Card
                _StatusCard(
                  colorScheme: colorScheme,
                  pulseController: _pulseController,
                ),
                const SizedBox(height: 16),
                // Core Engine Info
                _CoreInfoCard(
                  version: _coreVersion,
                  healthStatus: _healthStatus,
                ),
                const SizedBox(height: 24),
                // Scanning indicator
                Center(
                  child: Text(
                    'Scanning for nearby devices…',
                    style: theme.textTheme.bodyMedium,
                  ),
                ),
                const SizedBox(height: 16),
                Center(
                  child: AnimatedBuilder(
                    animation: _pulseController,
                    builder: (context, child) {
                      return Opacity(
                        opacity: 0.3 + (_pulseController.value * 0.7),
                        child: Icon(
                          Icons.radar_rounded,
                          size: 64,
                          color: colorScheme.primary,
                        ),
                      );
                    },
                  ),
                ),
                const SizedBox(height: 32),
                // Sprint info
                _SprintInfoCard(theme: theme),
              ]),
            ),
          ),
        ],
      ),
    );
  }
}

class _StatusCard extends StatelessWidget {
  const _StatusCard({
    required this.colorScheme,
    required this.pulseController,
  });

  final ColorScheme colorScheme;
  final AnimationController pulseController;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            AnimatedBuilder(
              animation: pulseController,
              builder: (context, child) {
                return Container(
                  width: 12,
                  height: 12,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: colorScheme.primary
                        .withValues(alpha: 0.5 + (pulseController.value * 0.5)),
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
                    'Ready to discover',
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 2),
                  Text(
                    'Listening for nearby devices',
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
              ),
            ),
            Icon(
              Icons.wifi_rounded,
              color: colorScheme.primary,
            ),
          ],
        ),
      ),
    );
  }
}

class _CoreInfoCard extends StatelessWidget {
  const _CoreInfoCard({
    required this.version,
    required this.healthStatus,
  });

  final String version;
  final String healthStatus;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.memory_rounded,
                    size: 18, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text('Rust Core Engine',
                    style: theme.textTheme.titleMedium),
              ],
            ),
            const SizedBox(height: 12),
            if (version.isNotEmpty)
              _InfoRow(label: 'Version', value: 'v$version'),
            if (healthStatus.isNotEmpty)
              _InfoRow(label: 'Status', value: '✓ Healthy'),
          ],
        ),
      ),
    );
  }
}

class _InfoRow extends StatelessWidget {
  const _InfoRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        children: [
          SizedBox(
            width: 80,
            child: Text(label, style: theme.textTheme.bodySmall),
          ),
          Expanded(
            child: Text(
              value,
              style: theme.textTheme.bodyMedium?.copyWith(
                color: theme.colorScheme.onSurface,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _SprintInfoCard extends StatelessWidget {
  const _SprintInfoCard({required this.theme});

  final ThemeData theme;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Sprint 0 — Foundation',
                style: theme.textTheme.titleMedium),
            const SizedBox(height: 8),
            Text(
              'Architecture, theme, navigation, Rust core engine, '
              'testing framework, CI/CD, and documentation are established. '
              'Device discovery and file transfer coming in Sprint 1.',
              style: theme.textTheme.bodyMedium,
            ),
          ],
        ),
      ),
    );
  }
}
