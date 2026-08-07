// Settings Screen
//
// Application settings: theme, transfer, discovery, security, about.

import 'package:flutter/material.dart';
import '../../rust/api/init.dart' as rust_api;

// Application settings screen.
class SettingsScreen extends StatelessWidget {
  const SettingsScreen({super.key, required this.onToggleTheme});

  final VoidCallback onToggleTheme;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isDark = theme.brightness == Brightness.dark;

    return SafeArea(
      child: CustomScrollView(
        slivers: [
          const SliverAppBar(
            floating: true,
            title: Text('Settings'),
          ),
          SliverPadding(
            padding: const EdgeInsets.all(16),
            sliver: SliverList(
              delegate: SliverChildListDelegate([
                // Appearance
                Text('Appearance', style: theme.textTheme.titleLarge),
                const SizedBox(height: 8),
                Card(
                  child: ListTile(
                    leading: Icon(isDark
                        ? Icons.dark_mode_rounded
                        : Icons.light_mode_rounded),
                    title: const Text('Theme'),
                    subtitle: Text(isDark ? 'Dark' : 'Light'),
                    trailing: Switch(
                      value: isDark,
                      onChanged: (_) => onToggleTheme(),
                      activeTrackColor: theme.colorScheme.primary,
                    ),
                  ),
                ),
                const SizedBox(height: 24),
                // Transfer
                Text('Transfer', style: theme.textTheme.titleLarge),
                const SizedBox(height: 8),
                Card(
                  child: Column(
                    children: [
                      ListTile(
                        leading: const Icon(Icons.folder_rounded),
                        title: const Text('Save location'),
                        subtitle: const Text('Downloads'),
                        trailing: const Icon(Icons.arrow_forward_ios_rounded,
                            size: 16),
                        onTap: () {},
                      ),
                      const Divider(height: 1, indent: 56),
                      ListTile(
                        leading: const Icon(Icons.speed_rounded),
                        title: const Text('Chunk size'),
                        subtitle: const Text('256 KB'),
                        trailing: const Icon(Icons.arrow_forward_ios_rounded,
                            size: 16),
                        onTap: () {},
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 24),
                // Discovery
                Text('Discovery', style: theme.textTheme.titleLarge),
                const SizedBox(height: 8),
                Card(
                  child: Column(
                    children: [
                      SwitchListTile(
                        secondary: const Icon(Icons.visibility_rounded),
                        title: const Text('Discoverable'),
                        subtitle: const Text(
                            'Allow nearby devices to find this device'),
                        value: true,
                        onChanged: (_) {},
                        activeTrackColor: theme.colorScheme.primary,
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 24),
                // Security
                Text('Security', style: theme.textTheme.titleLarge),
                const SizedBox(height: 8),
                Card(
                  child: Column(
                    children: [
                      SwitchListTile(
                        secondary: const Icon(Icons.pin_rounded),
                        title: const Text('Require PIN'),
                        subtitle:
                            const Text('Require PIN for new device pairing'),
                        value: false,
                        onChanged: (_) {},
                        activeTrackColor: theme.colorScheme.primary,
                      ),
                      const Divider(height: 1, indent: 56),
                      SwitchListTile(
                        secondary: const Icon(Icons.person_add_rounded),
                        title: const Text('Allow unknown devices'),
                        subtitle: const Text(
                            'Accept transfers from non-trusted devices'),
                        value: true,
                        onChanged: (_) {},
                        activeTrackColor: theme.colorScheme.primary,
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 24),
                // About
                Text('About', style: theme.textTheme.titleLarge),
                const SizedBox(height: 8),
                Card(
                  child: Column(
                    children: [
                      ListTile(
                        leading: const Icon(Icons.info_outline_rounded),
                        title: const Text('UOT — Universal Offline Transfer'),
                        subtitle: Text('Core v${rust_api.getVersion()} · '
                            'Protocol v${rust_api.getProtocolVersion()}'),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 16),
              ]),
            ),
          ),
        ],
      ),
    );
  }
}
