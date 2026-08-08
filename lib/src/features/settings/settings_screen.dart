// Settings Screen
//
// Application settings: theme, transfer, discovery, security, about.

import 'dart:convert';
import 'package:flutter/material.dart';
import '../../rust/api/init.dart' as rust_api;
import '../../rust/api/engine_api.dart' as engine;

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key, required this.onToggleTheme});

  final VoidCallback onToggleTheme;

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  bool _darkMode = true;
  bool _discoverable = true;
  bool _autoAcceptTrusted = false;
  bool _requirePin = false;
  bool _integrity = true;
  double _chunkSize = 256; // KB
  String _version = '';

  @override
  void initState() {
    super.initState();
    _version = rust_api.getVersion();
    _loadSettings();
  }

  void _loadSettings() {
    try {
      final json = engine.engineLoadSettings();
      final s = jsonDecode(json) as Map<String, dynamic>;
      setState(() {
        _darkMode = s['theme_mode'] == 'dark';
        _integrity = s['verify_sha256'] ?? true;
        _autoAcceptTrusted = s['auto_accept_trusted'] ?? false;
        _requirePin = s['require_pin'] ?? false;
        _chunkSize = (s['chunk_size_kb'] ?? 256).toDouble();
      });
    } catch (_) {}
  }

  void _saveSettings() {
    engine.engineSaveSettings(
      json: jsonEncode({
        'device_name': 'UOT Device',
        'theme_mode': _darkMode ? 'dark' : 'light',
        'chunk_size_kb': _chunkSize.toInt(),
        'verify_sha256': _integrity,
        'auto_accept_trusted': _autoAcceptTrusted,
        'require_pin': _requirePin,
        'save_directory': '',
        'network_port': 42000,
        'scan_interval_secs': 5,
        'show_hidden_files': false,
        'max_concurrent_transfers': 3,
      }),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return SafeArea(
      child: CustomScrollView(
        slivers: [
          SliverAppBar(floating: true, title: const Text('Settings')),
          SliverPadding(
            padding: const EdgeInsets.all(16),
            sliver: SliverList(
              delegate: SliverChildListDelegate([
                // Appearance
                _SectionHeader(title: 'APPEARANCE', theme: theme),
                const SizedBox(height: 8),
                Card(
                  child: Column(
                    children: [
                      SwitchListTile(
                        title: Text(
                          'Dark Mode',
                          style: theme.textTheme.titleSmall,
                        ),
                        subtitle: Text(
                          'Use dark color scheme',
                          style: theme.textTheme.bodySmall,
                        ),
                        secondary: Icon(
                          Icons.dark_mode_rounded,
                          color: colorScheme.primary,
                        ),
                        value: _darkMode,
                        onChanged: (v) {
                          setState(() => _darkMode = v);
                          widget.onToggleTheme();
                          _saveSettings();
                        },
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 16),

                // Transfer Settings
                _SectionHeader(title: 'TRANSFER', theme: theme),
                const SizedBox(height: 8),
                Card(
                  child: Column(
                    children: [
                      ListTile(
                        leading: Icon(
                          Icons.speed_rounded,
                          color: colorScheme.primary,
                        ),
                        title: Text(
                          'Chunk Size',
                          style: theme.textTheme.titleSmall,
                        ),
                        subtitle: Text(
                          '${_chunkSize.toInt()} KB per chunk',
                          style: theme.textTheme.bodySmall,
                        ),
                      ),
                      Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 16),
                        child: Slider(
                          value: _chunkSize,
                          min: 64,
                          max: 1024,
                          divisions: 15,
                          label: '${_chunkSize.toInt()} KB',
                          onChanged: (v) {
                            setState(() => _chunkSize = v);
                            _saveSettings();
                          },
                        ),
                      ),
                      const Divider(height: 1, indent: 72),
                      SwitchListTile(
                        title: Text(
                          'SHA-256 Verification',
                          style: theme.textTheme.titleSmall,
                        ),
                        subtitle: Text(
                          'Verify file integrity after transfer',
                          style: theme.textTheme.bodySmall,
                        ),
                        secondary: Icon(
                          Icons.verified_rounded,
                          color: colorScheme.secondary,
                        ),
                        value: _integrity,
                        onChanged: (v) {
                          setState(() => _integrity = v);
                          _saveSettings();
                        },
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 16),

                // Discovery Settings
                _SectionHeader(title: 'DISCOVERY', theme: theme),
                const SizedBox(height: 8),
                Card(
                  child: Column(
                    children: [
                      SwitchListTile(
                        title: Text(
                          'Discoverable',
                          style: theme.textTheme.titleSmall,
                        ),
                        subtitle: Text(
                          'Allow other devices to find you via mDNS',
                          style: theme.textTheme.bodySmall,
                        ),
                        secondary: Icon(
                          Icons.wifi_tethering_rounded,
                          color: colorScheme.primary,
                        ),
                        value: _discoverable,
                        onChanged: (v) => setState(() => _discoverable = v),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 16),

                // Security Settings
                _SectionHeader(title: 'SECURITY', theme: theme),
                const SizedBox(height: 8),
                Card(
                  child: Column(
                    children: [
                      SwitchListTile(
                        title: Text(
                          'Auto-accept trusted',
                          style: theme.textTheme.titleSmall,
                        ),
                        subtitle: Text(
                          'Automatically accept from paired devices',
                          style: theme.textTheme.bodySmall,
                        ),
                        secondary: Icon(
                          Icons.verified_user_rounded,
                          color: colorScheme.primary,
                        ),
                        value: _autoAcceptTrusted,
                        onChanged: (v) {
                          setState(() => _autoAcceptTrusted = v);
                          _saveSettings();
                        },
                      ),
                      const Divider(height: 1, indent: 72),
                      SwitchListTile(
                        title: Text(
                          'Require PIN',
                          style: theme.textTheme.titleSmall,
                        ),
                        subtitle: Text(
                          'PIN required for new connections',
                          style: theme.textTheme.bodySmall,
                        ),
                        secondary: Icon(
                          Icons.pin_rounded,
                          color: colorScheme.secondary,
                        ),
                        value: _requirePin,
                        onChanged: (v) {
                          setState(() => _requirePin = v);
                          _saveSettings();
                        },
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 16),

                // About
                _SectionHeader(title: 'ABOUT', theme: theme),
                const SizedBox(height: 8),
                Card(
                  child: Column(
                    children: [
                      ListTile(
                        leading: Icon(
                          Icons.info_outline_rounded,
                          color: colorScheme.onSurfaceVariant,
                        ),
                        title: Text('UOT', style: theme.textTheme.titleSmall),
                        subtitle: Text(
                          'Universal Offline Transfer v$_version',
                          style: theme.textTheme.bodySmall,
                        ),
                      ),
                      const Divider(height: 1, indent: 72),
                      ListTile(
                        leading: Icon(
                          Icons.memory_rounded,
                          color: colorScheme.onSurfaceVariant,
                        ),
                        title: Text(
                          'Rust Core Engine',
                          style: theme.textTheme.titleSmall,
                        ),
                        subtitle: Text(
                          'flutter_rust_bridge v2.12.0',
                          style: theme.textTheme.bodySmall,
                        ),
                      ),
                      const Divider(height: 1, indent: 72),
                      ListTile(
                        leading: Icon(
                          Icons.code_rounded,
                          color: colorScheme.onSurfaceVariant,
                        ),
                        title: Text(
                          'Architecture',
                          style: theme.textTheme.titleSmall,
                        ),
                        subtitle: Text(
                          'Flutter + Rust • TCP/mDNS • SHA-256',
                          style: theme.textTheme.bodySmall,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 32),
              ]),
            ),
          ),
        ],
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader({required this.title, required this.theme});

  final String title;
  final ThemeData theme;

  @override
  Widget build(BuildContext context) {
    return Text(
      title,
      style: theme.textTheme.labelSmall?.copyWith(
        color: theme.colorScheme.onSurfaceVariant,
        letterSpacing: 1.2,
      ),
    );
  }
}
