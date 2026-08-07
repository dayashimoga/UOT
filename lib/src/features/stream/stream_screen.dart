// Stream Screen
//
// Media streaming controls: camera, screen, video, audio streaming.

import 'package:flutter/material.dart';

class StreamScreen extends StatefulWidget {
  const StreamScreen({super.key});

  @override
  State<StreamScreen> createState() => _StreamScreenState();
}

class _StreamScreenState extends State<StreamScreen> {
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return SafeArea(
      child: CustomScrollView(
        slivers: [
          SliverAppBar(
            floating: true,
            title: const Text('Stream'),
          ),
          SliverPadding(
            padding: const EdgeInsets.all(16),
            sliver: SliverList(
              delegate: SliverChildListDelegate([
                // Stream types grid
                _StreamTypeCard(
                  icon: Icons.videocam_rounded,
                  title: 'Camera',
                  subtitle: 'Stream camera feed to a device',
                  color: colorScheme.primary,
                  onTap: () {},
                ),
                const SizedBox(height: 8),
                _StreamTypeCard(
                  icon: Icons.screen_share_rounded,
                  title: 'Screen',
                  subtitle: 'Share your screen with a device',
                  color: colorScheme.tertiary,
                  onTap: () {},
                ),
                const SizedBox(height: 8),
                _StreamTypeCard(
                  icon: Icons.video_library_rounded,
                  title: 'Video File',
                  subtitle: 'Stream a local video file',
                  color: colorScheme.secondary,
                  onTap: () {},
                ),
                const SizedBox(height: 8),
                _StreamTypeCard(
                  icon: Icons.music_note_rounded,
                  title: 'Audio File',
                  subtitle: 'Stream a local audio file',
                  color: const Color(0xFFE879F9),
                  onTap: () {},
                ),
                const SizedBox(height: 24),

                // Active streams
                Text(
                  'ACTIVE STREAMS',
                  style: theme.textTheme.labelSmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                    letterSpacing: 1.2,
                  ),
                ),
                const SizedBox(height: 8),
                Card(
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                        vertical: 32, horizontal: 16),
                    child: Column(
                      children: [
                        Icon(
                          Icons.cast_rounded,
                          size: 48,
                          color: colorScheme.onSurfaceVariant
                              .withValues(alpha: 0.4),
                        ),
                        const SizedBox(height: 12),
                        Text(
                          'No active streams',
                          style: theme.textTheme.titleSmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          'Select a stream type above to start',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant
                                .withValues(alpha: 0.7),
                          ),
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

class _StreamTypeCard extends StatelessWidget {
  const _StreamTypeCard({
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
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Container(
                width: 52,
                height: 52,
                decoration: BoxDecoration(
                  color: color.withValues(alpha: 0.15),
                  borderRadius: BorderRadius.circular(14),
                ),
                child: Icon(icon, color: color, size: 28),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(title, style: theme.textTheme.titleMedium),
                    const SizedBox(height: 2),
                    Text(subtitle, style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                    )),
                  ],
                ),
              ),
              Icon(Icons.chevron_right_rounded,
                  color: theme.colorScheme.onSurfaceVariant),
            ],
          ),
        ),
      ),
    );
  }
}
