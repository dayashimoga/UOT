// Transfers Screen
//
// Shows active transfers, queue, and transfer history.

import 'package:flutter/material.dart';

// Transfer queue and history screen.
class TransfersScreen extends StatelessWidget {
  const TransfersScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return SafeArea(
      child: CustomScrollView(
        slivers: [
          const SliverAppBar(
            floating: true,
            title: Text('Transfers'),
          ),
          SliverPadding(
            padding: const EdgeInsets.all(16),
            sliver: SliverList(
              delegate: SliverChildListDelegate([
                // Active transfers section
                _SectionHeader(title: 'Active', theme: theme),
                const SizedBox(height: 8),
                _EmptyState(
                  icon: Icons.swap_horiz_rounded,
                  title: 'No active transfers',
                  subtitle: 'Go to Nearby to start sending files',
                  theme: theme,
                ),
                const SizedBox(height: 24),
                // History section
                _SectionHeader(title: 'History', theme: theme),
                const SizedBox(height: 8),
                _EmptyState(
                  icon: Icons.history_rounded,
                  title: 'No transfer history',
                  subtitle: 'Completed transfers will appear here',
                  theme: theme,
                ),
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
    return Text(title, style: theme.textTheme.titleLarge);
  }
}

class _EmptyState extends StatelessWidget {
  const _EmptyState({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.theme,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final ThemeData theme;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 32, horizontal: 16),
        child: Column(
          children: [
            Icon(icon, size: 48, color: theme.colorScheme.primary.withValues(alpha: 0.4)),
            const SizedBox(height: 12),
            Text(title, style: theme.textTheme.titleMedium),
            const SizedBox(height: 4),
            Text(subtitle, style: theme.textTheme.bodySmall),
          ],
        ),
      ),
    );
  }
}
