// UOT App Router
//
// Adaptive navigation: NavigationBar on mobile, NavigationRail on desktop.
// Supports 6 primary destinations.

import 'package:flutter/material.dart';
import '../../features/nearby/nearby_screen.dart';
import '../../features/transfers/transfers_screen.dart';
import '../../features/receive/receive_screen.dart';
import '../../features/stream/stream_screen.dart';
import '../../features/devices/devices_screen.dart';
import '../../features/settings/settings_screen.dart';

/// Main navigation destinations.
enum NavDestination {
  nearby(icon: Icons.radar_rounded, label: 'Nearby'),
  transfers(icon: Icons.swap_horiz_rounded, label: 'Transfers'),
  receive(icon: Icons.download_rounded, label: 'Receive'),
  stream(icon: Icons.cast_rounded, label: 'Stream'),
  devices(icon: Icons.devices_rounded, label: 'Devices'),
  settings(icon: Icons.settings_rounded, label: 'Settings');

  const NavDestination({required this.icon, required this.label});
  final IconData icon;
  final String label;
}

/// Responsive app router with adaptive navigation.
class AppRouter extends StatefulWidget {
  const AppRouter({super.key, required this.onToggleTheme});

  final VoidCallback onToggleTheme;

  @override
  State<AppRouter> createState() => _AppRouterState();
}

class _AppRouterState extends State<AppRouter> {
  int _selectedIndex = 0;

  /// Build the content for the current destination.
  Widget _buildBody() {
    return switch (NavDestination.values[_selectedIndex]) {
      NavDestination.nearby => const NearbyScreen(),
      NavDestination.transfers => const TransfersScreen(),
      NavDestination.receive => const ReceiveScreen(),
      NavDestination.stream => const StreamScreen(),
      NavDestination.devices => const DevicesScreen(),
      NavDestination.settings =>
        SettingsScreen(onToggleTheme: widget.onToggleTheme),
    };
  }

  @override
  Widget build(BuildContext context) {
    final isWide = MediaQuery.sizeOf(context).width >= 800;

    if (isWide) {
      return _buildDesktopLayout();
    }
    return _buildMobileLayout();
  }

  /// Desktop layout with NavigationRail.
  Widget _buildDesktopLayout() {
    return Scaffold(
      body: Row(
        children: [
          NavigationRail(
            selectedIndex: _selectedIndex,
            onDestinationSelected: (index) {
              setState(() => _selectedIndex = index);
            },
            labelType: NavigationRailLabelType.all,
            leading: Padding(
              padding: const EdgeInsets.symmetric(vertical: 16),
              child: Icon(
                Icons.send_rounded,
                color: Theme.of(context).colorScheme.primary,
                size: 32,
              ),
            ),
            destinations: NavDestination.values
                .map((d) => NavigationRailDestination(
                      icon: Icon(d.icon),
                      selectedIcon: Icon(d.icon),
                      label: Text(d.label),
                    ))
                .toList(),
          ),
          const VerticalDivider(width: 1, thickness: 1),
          Expanded(
            child: AnimatedSwitcher(
              duration: const Duration(milliseconds: 200),
              child: _buildBody(),
            ),
          ),
        ],
      ),
    );
  }

  /// Mobile layout with bottom NavigationBar.
  Widget _buildMobileLayout() {
    return Scaffold(
      body: AnimatedSwitcher(
        duration: const Duration(milliseconds: 200),
        child: _buildBody(),
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _selectedIndex,
        onDestinationSelected: (index) {
          setState(() => _selectedIndex = index);
        },
        height: 65,
        destinations: NavDestination.values
            .map((d) => NavigationDestination(
                  icon: Icon(d.icon),
                  selectedIcon: Icon(d.icon),
                  label: d.label,
                ))
            .toList(),
      ),
    );
  }
}
