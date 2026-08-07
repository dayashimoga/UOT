// Stream Screen
//
// Media streaming interface for video, audio, camera, and screen sharing.

import 'package:flutter/material.dart';

// Media streaming screen.
class StreamScreen extends StatelessWidget {
  const StreamScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return SafeArea(
      child: CustomScrollView(
        slivers: [
          const SliverAppBar(
            floating: true,
            title: Text('Stream'),
          ),
          SliverPadding(
            padding: const EdgeInsets.all(16),
            sliver: SliverList(
              delegate: SliverChildListDelegate([
                _StreamOption(
                  icon: Icons.videocam_rounded,
                  title: 'Camera',
                  subtitle: 'Stream your camera to a nearby device',
                  available: false,
                  theme: theme,
                ),
                const SizedBox(height: 8),
                _StreamOption(
                  icon: Icons.screen_share_rounded,
                  title: 'Screen',
                  subtitle: 'Share your screen with a nearby device',
                  available: false,
                  theme: theme,
                ),
                const SizedBox(height: 8),
                _StreamOption(
                  icon: Icons.movie_rounded,
                  title: 'Video File',
                  subtitle: 'Stream a video file to a nearby device',
                  available: false,
                  theme: theme,
                ),
                const SizedBox(height: 8),
                _StreamOption(
                  icon: Icons.music_note_rounded,
                  title: 'Audio File',
                  subtitle: 'Stream an audio file to a nearby device',
                  available: false,
                  theme: theme,
                ),
                const SizedBox(height: 16),
                Card(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: Row(
                      children: [
                        Icon(Icons.info_outline_rounded,
                            size: 20, color: theme.colorScheme.primary),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            'Streaming requires a connected device. '
                            'Go to Nearby to discover and connect first.',
                            style: theme.textTheme.bodySmall,
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

class _StreamOption extends StatelessWidget {
  const _StreamOption({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.available,
    required this.theme,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final bool available;
  final ThemeData theme;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: ListTile(
        leading: Icon(icon,
            color: available
                ? theme.colorScheme.primary
                : theme.colorScheme.onSurface.withValues(alpha: 0.3)),
        title: Text(title),
        subtitle: Text(subtitle, style: theme.textTheme.bodySmall),
        trailing: available
            ? Icon(Icons.arrow_forward_ios_rounded,
                size: 16, color: theme.colorScheme.primary)
            : Chip(
                label: Text('Coming Soon',
                    style: theme.textTheme.bodySmall
                        ?.copyWith(fontSize: 10)),
                padding: EdgeInsets.zero,
                visualDensity: VisualDensity.compact,
              ),
        enabled: available,
      ),
    );
  }
}
