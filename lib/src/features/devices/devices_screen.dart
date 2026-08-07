// Devices Screen
//
// Manages trusted/paired devices. Shows device details and trust controls.

import 'package:flutter/material.dart';

class DevicesScreen extends StatefulWidget {
  const DevicesScreen({super.key});

  @override
  State<DevicesScreen> createState() => _DevicesScreenState();
}

class _DevicesScreenState extends State<DevicesScreen> {
  // Paired devices will come from engine
  final List<_PairedDevice> _pairedDevices = [];

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return SafeArea(
      child: CustomScrollView(
        slivers: [
          SliverAppBar(
            floating: true,
            title: const Text('Devices'),
            actions: [
              IconButton(
                icon: const Icon(Icons.qr_code_rounded),
                onPressed: () {},
                tooltip: 'Show my QR code',
              ),
            ],
          ),
          SliverPadding(
            padding: const EdgeInsets.all(16),
            sliver: SliverList(
              delegate: SliverChildListDelegate([
                // This device card
                _ThisDeviceCard(colorScheme: colorScheme, theme: theme),
                const SizedBox(height: 24),

                // Paired devices section
                Text(
                  'TRUSTED DEVICES',
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                    letterSpacing: 1.2,
                  ),
                ),
                const SizedBox(height: 8),

                if (_pairedDevices.isEmpty)
                  Card(
                    child: Padding(
                      padding: const EdgeInsets.symmetric(
                          vertical: 32, horizontal: 16),
                      child: Column(
                        children: [
                          Icon(
                            Icons.devices_rounded,
                            size: 48,
                            color: colorScheme.onSurfaceVariant
                                .withValues(alpha: 0.4),
                          ),
                          const SizedBox(height: 12),
                          Text(
                            'No trusted devices',
                            style: theme.textTheme.titleSmall?.copyWith(
                              color: colorScheme.onSurfaceVariant,
                            ),
                          ),
                          const SizedBox(height: 4),
                          Text(
                            'Devices you pair with will appear here.\nTrusted devices can auto-accept transfers.',
                            style: theme.textTheme.bodySmall?.copyWith(
                              color: colorScheme.onSurfaceVariant
                                  .withValues(alpha: 0.7),
                            ),
                            textAlign: TextAlign.center,
                          ),
                          const SizedBox(height: 16),
                          FilledButton.icon(
                            onPressed: () {},
                            icon: const Icon(Icons.qr_code_scanner_rounded),
                            label: const Text('Pair with QR Code'),
                          ),
                        ],
                      ),
                    ),
                  )
                else
                  ...(_pairedDevices.map((device) => Padding(
                        padding: const EdgeInsets.only(bottom: 8),
                        child:
                            _PairedDeviceCard(device: device, theme: theme),
                      ))),
              ]),
            ),
          ),
        ],
      ),
    );
  }
}

class _ThisDeviceCard extends StatelessWidget {
  const _ThisDeviceCard({
    required this.colorScheme,
    required this.theme,
  });

  final ColorScheme colorScheme;
  final ThemeData theme;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Container(
              width: 56,
              height: 56,
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  colors: [
                    colorScheme.primary,
                    colorScheme.tertiary,
                  ],
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                ),
                borderRadius: BorderRadius.circular(14),
              ),
              child: const Icon(
                Icons.desktop_windows_rounded,
                color: Colors.white,
                size: 28,
              ),
            ),
            const SizedBox(width: 16),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('This Device', style: theme.textTheme.titleMedium),
                  const SizedBox(height: 2),
                  Text(
                    'Desktop • Windows',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                  const SizedBox(height: 4),
                  Container(
                    padding: const EdgeInsets.symmetric(
                        horizontal: 8, vertical: 2),
                    decoration: BoxDecoration(
                      color: colorScheme.primaryContainer,
                      borderRadius: BorderRadius.circular(4),
                    ),
                    child: Text(
                      'Ready',
                      style: theme.textTheme.labelSmall?.copyWith(
                        color: colorScheme.onPrimaryContainer,
                      ),
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

class _PairedDevice {
  final String name;
  final String type;
  final String lastSeen;
  final bool isOnline;

  _PairedDevice({
    required this.name,
    required this.type,
    required this.lastSeen,
    required this.isOnline,
  });
}

class _PairedDeviceCard extends StatelessWidget {
  const _PairedDeviceCard({required this.device, required this.theme});

  final _PairedDevice device;
  final ThemeData theme;

  @override
  Widget build(BuildContext context) {
    final colorScheme = theme.colorScheme;
    return Card(
      child: ListTile(
        leading: Container(
          width: 44,
          height: 44,
          decoration: BoxDecoration(
            color: device.isOnline
                ? colorScheme.primaryContainer
                : colorScheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(10),
          ),
          child: Icon(
            Icons.phone_android_rounded,
            color: device.isOnline
                ? colorScheme.onPrimaryContainer
                : colorScheme.onSurfaceVariant,
          ),
        ),
        title: Text(device.name, style: theme.textTheme.titleSmall),
        subtitle: Text(
          '${device.type} • ${device.lastSeen}',
          style: theme.textTheme.bodySmall,
        ),
        trailing: PopupMenuButton<String>(
          onSelected: (value) {},
          itemBuilder: (context) => [
            const PopupMenuItem(value: 'send', child: Text('Send Files')),
            const PopupMenuItem(value: 'unpair', child: Text('Remove Trust')),
          ],
        ),
      ),
    );
  }
}
