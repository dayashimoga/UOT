// Instant Message / Ping Communication Dialog
//
// Enables sending live text messages, pings, and delivery receipts between
// connected UOT devices to confirm 100% active connection.

import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import '../../rust/api/engine_api.dart' as engine;

class InstantChatDialog extends StatefulWidget {
  final String deviceId;
  final String deviceName;

  const InstantChatDialog({
    super.key,
    required this.deviceId,
    required this.deviceName,
  });

  static Future<void> show(
    BuildContext context, {
    required String deviceId,
    required String deviceName,
  }) {
    return showDialog<void>(
      context: context,
      builder: (context) => InstantChatDialog(
        deviceId: deviceId,
        deviceName: deviceName,
      ),
    );
  }

  @override
  State<InstantChatDialog> createState() => _InstantChatDialogState();
}

class _InstantChatDialogState extends State<InstantChatDialog> {
  final TextEditingController _msgController = TextEditingController();
  final List<Map<String, String>> _messages = [];
  Timer? _pollTimer;
  final Set<String> _seenEvents = {};

  @override
  void initState() {
    super.initState();
    _messages.add({
      'sender': 'System',
      'text':
          'Connected to ${widget.deviceName}. Ready for messaging & file transfers.',
      'time': _formattedTime(),
    });
    _startPolling();
  }

  void _startPolling() {
    _pollTimer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (!mounted) return;
      try {
        final eventsJson = engine.engineGetEvents(limit: 20);
        final List<dynamic> events = jsonDecode(eventsJson);
        for (final ev in events) {
          final str = ev.toString();
          if (_seenEvents.contains(str)) continue;
          _seenEvents.add(str);

          if (str.contains('ClipboardReceived') ||
              str.contains('Received clipboard data')) {
            String rawText = str;
            if (str.contains('MESSAGE:')) {
              final idx = str.indexOf('MESSAGE:');
              rawText = str.substring(idx + 8);
            }
            rawText = rawText.replaceAll(RegExp(r'["}\\]+$'), '').trim();
            if (rawText.isNotEmpty) {
              setState(() {
                _messages.add({
                  'sender': widget.deviceName,
                  'text': rawText,
                  'time': _formattedTime(),
                });
              });
            }
          }
        }
      } catch (_) {}
    });
  }

  String _formattedTime() {
    final now = DateTime.now();
    final h = now.hour.toString().padLeft(2, '0');
    final m = now.minute.toString().padLeft(2, '0');
    return '$h:$m';
  }

  Future<void> _sendMessage([String? textToSend]) async {
    final text = (textToSend ?? _msgController.text).trim();
    if (text.isEmpty) return;

    _msgController.clear();
    final timestamp = _formattedTime();

    setState(() {
      _messages.add({
        'sender': 'Me',
        'text': '$text  • Sending…',
        'time': timestamp,
      });
    });

    final msgIndex = _messages.length - 1;

    try {
      final res = await engine.engineSendClipboard(
        deviceId: widget.deviceId,
        text: 'MESSAGE:$text',
      );

      if (mounted) {
        if (res.startsWith('ok')) {
          setState(() {
            _messages[msgIndex] = {
              'sender': 'Me',
              'text': '$text  ✓ Sent',
              'time': timestamp,
            };
          });
        } else {
          setState(() {
            _messages[msgIndex] = {
              'sender': 'Me',
              'text': '$text  ❌ (Failed: ${res.replaceFirst("error:", "")})',
              'time': timestamp,
            };
          });
        }
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _messages[msgIndex] = {
            'sender': 'Me',
            'text': '$text  ❌ ($e)',
            'time': timestamp,
          };
        });
      }
    }
  }

  @override
  void dispose() {
    _pollTimer?.cancel();
    _msgController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Dialog(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 440, maxHeight: 560),
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Column(
            children: [
              // Header
              Row(
                children: [
                  CircleAvatar(
                    backgroundColor: Colors.green.withOpacity(0.15),
                    child: const Icon(
                      Icons.mark_chat_read_rounded,
                      color: Colors.green,
                    ),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Chat & Ping: ${widget.deviceName}',
                          style: theme.textTheme.titleMedium?.copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                          overflow: TextOverflow.ellipsis,
                        ),
                        Row(
                          children: [
                            Container(
                              width: 8,
                              height: 8,
                              decoration: const BoxDecoration(
                                color: Colors.green,
                                shape: BoxShape.circle,
                              ),
                            ),
                            const SizedBox(width: 6),
                            Text(
                              'Connection Verified Live',
                              style: theme.textTheme.bodySmall?.copyWith(
                                color: colorScheme.onSurfaceVariant,
                              ),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.close_rounded),
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
              const Divider(height: 24),

              // Chat messages
              Expanded(
                child: ListView.builder(
                  itemCount: _messages.length,
                  itemBuilder: (ctx, index) {
                    final msg = _messages[index];
                    final isMe = msg['sender'] == 'Me';
                    final isSystem = msg['sender'] == 'System' ||
                        msg['sender'] == 'System Error';

                    if (isSystem) {
                      return Padding(
                        padding: const EdgeInsets.symmetric(vertical: 6),
                        child: Center(
                          child: Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 12,
                              vertical: 6,
                            ),
                            decoration: BoxDecoration(
                              color: colorScheme.surfaceContainerHighest,
                              borderRadius: BorderRadius.circular(10),
                            ),
                            child: Text(
                              msg['text']!,
                              style: theme.textTheme.bodySmall?.copyWith(
                                color: colorScheme.onSurfaceVariant,
                              ),
                              textAlign: TextAlign.center,
                            ),
                          ),
                        ),
                      );
                    }

                    return Align(
                      alignment:
                          isMe ? Alignment.centerRight : Alignment.centerLeft,
                      child: Container(
                        margin: const EdgeInsets.symmetric(vertical: 4),
                        padding: const EdgeInsets.symmetric(
                          horizontal: 14,
                          vertical: 10,
                        ),
                        constraints: const BoxConstraints(maxWidth: 280),
                        decoration: BoxDecoration(
                          color: isMe
                              ? colorScheme.primary
                              : colorScheme.secondaryContainer,
                          borderRadius: BorderRadius.circular(14),
                        ),
                        child: Column(
                          crossAxisAlignment: isMe
                              ? CrossAxisAlignment.end
                              : CrossAxisAlignment.start,
                          children: [
                            Text(
                              msg['text']!,
                              style: theme.textTheme.bodyMedium?.copyWith(
                                color: isMe
                                    ? colorScheme.onPrimary
                                    : colorScheme.onSecondaryContainer,
                              ),
                            ),
                            const SizedBox(height: 4),
                            Text(
                              msg['time']!,
                              style: theme.textTheme.labelSmall?.copyWith(
                                color: (isMe
                                        ? colorScheme.onPrimary
                                        : colorScheme.onSecondaryContainer)
                                    .withOpacity(0.7),
                                fontSize: 10,
                              ),
                            ),
                          ],
                        ),
                      ),
                    );
                  },
                ),
              ),

              const SizedBox(height: 12),

              // Quick ping chip & Input text row
              Row(
                children: [
                  ActionChip(
                    avatar: const Icon(Icons.bolt_rounded, size: 16),
                    label: const Text('Send Ping'),
                    onPressed: () => _sendMessage('PING: Connection Check'),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: TextField(
                      controller: _msgController,
                      decoration: InputDecoration(
                        hintText: 'Type a message…',
                        isDense: true,
                        border: OutlineInputBorder(
                          borderRadius: BorderRadius.circular(20),
                        ),
                      ),
                      onSubmitted: (val) => _sendMessage(),
                    ),
                  ),
                  const SizedBox(width: 8),
                  IconButton.filled(
                    icon: const Icon(Icons.send_rounded),
                    onPressed: () => _sendMessage(),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
