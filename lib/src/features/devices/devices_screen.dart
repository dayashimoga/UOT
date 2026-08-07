// Devices Screen
//
// Manage known, trusted, and blocked devices.

import 'package:flutter/material.dart';

// Device management screen.
class DevicesScreen extends StatelessWidget {
  const DevicesScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return SafeArea(
      child: CustomScrollView(
        slivers: [
          const SliverAppBar(
            floating: true,
            title: Text('Devices'),
          ),
          SliverPadding(
            padding: const EdgeInsets.all(16),
            sliver: SliverList(
              delegate: SliverChildListDelegate([
                Text('Trusted Devices', style: theme.textTheme.titleLarge),
                const SizedBox(height: 8),
                Card(
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                        vertical: 32, horizontal: 16),
                    child: Column(
                      children: [
                        Icon(Icons.devices_rounded,
                            size: 48,
                            color: theme.colorScheme.primary
                                .withValues(alpha: 0.4)),
                        const SizedBox(height: 12),
                        Text('No trusted devices',
                            style: theme.textTheme.titleMedium),
                        const SizedBox(height: 4),
                        Text(
                          'Pair with a device from the Nearby tab '
                          'to add it here',
                          style: theme.textTheme.bodySmall,
                          textAlign: TextAlign.center,
                        ),
                      ],
                    ),
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
