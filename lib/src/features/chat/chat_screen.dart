import 'dart:async';
import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:uot_app/src/rust/api/engine_api.dart';

/// Unified chat screen for a single peer session.
/// Shows chronological messages (incoming + outgoing) with delivery states.
class ChatScreen extends StatefulWidget {
  final String peerDeviceId;
  final String peerName;

  const ChatScreen({
    super.key,
    required this.peerDeviceId,
    required this.peerName,
  });

  @override
  State<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends State<ChatScreen> {
  final TextEditingController _messageController = TextEditingController();
  final ScrollController _scrollController = ScrollController();
  List<Map<String, dynamic>> _messages = [];
  Timer? _refreshTimer;

  @override
  void initState() {
    super.initState();
    _loadMessages();
    _refreshTimer = Timer.periodic(const Duration(seconds: 2), (_) {
      _loadMessages();
    });
  }

  @override
  void dispose() {
    _refreshTimer?.cancel();
    _messageController.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  void _loadMessages() {
    try {
      final json = engineGetMessages(peerDeviceId: widget.peerDeviceId);
      final List<dynamic> parsed = jsonDecode(json);
      if (mounted) {
        setState(() {
          _messages = parsed.cast<Map<String, dynamic>>();
        });
      }
    } catch (e) {
      debugPrint('Failed to load messages: $e');
    }
  }

  Future<void> _sendMessage() async {
    final text = _messageController.text.trim();
    if (text.isEmpty) return;

    _messageController.clear();

    try {
      final result = await engineSendMessage(
        peerDeviceId: widget.peerDeviceId,
        text: text,
      );
      if (result.startsWith('error:')) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Send failed: ${result.substring(6)}'),
              backgroundColor: Colors.red.shade700,
            ),
          );
        }
      }
      _loadMessages();
      _scrollToBottom();
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Send error: $e'),
            backgroundColor: Colors.red.shade700,
          ),
        );
      }
    }
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.animateTo(
          _scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 300),
          curve: Curves.easeOut,
        );
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      appBar: AppBar(
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(widget.peerName,
                style: const TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
            Text(
              'Session active',
              style: TextStyle(fontSize: 12, color: Colors.green.shade300),
            ),
          ],
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.info_outline),
            onPressed: () {
              _showSessionInfo(context);
            },
          ),
        ],
      ),
      body: Column(
        children: [
          // Messages list
          Expanded(
            child: _messages.isEmpty
                ? Center(
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Icon(Icons.chat_bubble_outline,
                            size: 64, color: theme.colorScheme.onSurface.withAlpha(60)),
                        const SizedBox(height: 16),
                        Text(
                          'No messages yet',
                          style: TextStyle(
                            color: theme.colorScheme.onSurface.withAlpha(120),
                            fontSize: 16,
                          ),
                        ),
                        const SizedBox(height: 8),
                        Text(
                          'Start a conversation with ${widget.peerName}',
                          style: TextStyle(
                            color: theme.colorScheme.onSurface.withAlpha(80),
                            fontSize: 14,
                          ),
                        ),
                      ],
                    ),
                  )
                : ListView.builder(
                    controller: _scrollController,
                    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                    itemCount: _messages.length,
                    itemBuilder: (context, index) {
                      return _buildMessageBubble(_messages[index], theme);
                    },
                  ),
          ),
          // Input bar
          Container(
            decoration: BoxDecoration(
              color: theme.colorScheme.surface,
              boxShadow: [
                BoxShadow(
                  color: Colors.black.withAlpha(15),
                  blurRadius: 8,
                  offset: const Offset(0, -2),
                ),
              ],
            ),
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _messageController,
                    decoration: InputDecoration(
                      hintText: 'Type a message...',
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(24),
                        borderSide: BorderSide.none,
                      ),
                      filled: true,
                      fillColor: theme.colorScheme.surfaceContainerHighest,
                      contentPadding:
                          const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
                    ),
                    onSubmitted: (_) => _sendMessage(),
                    textInputAction: TextInputAction.send,
                  ),
                ),
                const SizedBox(width: 8),
                FloatingActionButton.small(
                  onPressed: _sendMessage,
                  elevation: 0,
                  child: const Icon(Icons.send),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildMessageBubble(Map<String, dynamic> message, ThemeData theme) {
    final isOutgoing = message['direction'] == 'out';
    final content = message['content'] ?? '';
    final state = message['state'] ?? '';
    final timestamp = message['timestamp'] ?? '';

    // Parse timestamp
    String timeStr = '';
    try {
      final dt = DateTime.parse(timestamp);
      timeStr =
          '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
    } catch (_) {}

    // Delivery state icon
    Widget stateIcon = const SizedBox.shrink();
    if (isOutgoing) {
      switch (state) {
        case 'Sending':
          stateIcon = const SizedBox(
            width: 12,
            height: 12,
            child: CircularProgressIndicator(strokeWidth: 1.5),
          );
          break;
        case 'Sent':
          stateIcon = Icon(Icons.check, size: 14, color: Colors.grey.shade400);
          break;
        case 'Delivered':
          stateIcon = Icon(Icons.done_all, size: 14, color: Colors.blue.shade400);
          break;
        case 'Failed':
          stateIcon = Icon(Icons.error_outline, size: 14, color: Colors.red.shade400);
          break;
      }
    }

    return Align(
      alignment: isOutgoing ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.symmetric(vertical: 3),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
        constraints:
            BoxConstraints(maxWidth: MediaQuery.of(context).size.width * 0.75),
        decoration: BoxDecoration(
          color: isOutgoing
              ? theme.colorScheme.primary.withAlpha(200)
              : theme.colorScheme.surfaceContainerHighest,
          borderRadius: BorderRadius.only(
            topLeft: const Radius.circular(16),
            topRight: const Radius.circular(16),
            bottomLeft: Radius.circular(isOutgoing ? 16 : 4),
            bottomRight: Radius.circular(isOutgoing ? 4 : 16),
          ),
        ),
        child: Column(
          crossAxisAlignment:
              isOutgoing ? CrossAxisAlignment.end : CrossAxisAlignment.start,
          children: [
            Text(
              content,
              style: TextStyle(
                color: isOutgoing
                    ? Colors.white
                    : theme.colorScheme.onSurface,
                fontSize: 15,
              ),
            ),
            const SizedBox(height: 3),
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  timeStr,
                  style: TextStyle(
                    fontSize: 11,
                    color: isOutgoing
                        ? Colors.white.withAlpha(180)
                        : theme.colorScheme.onSurface.withAlpha(100),
                  ),
                ),
                if (isOutgoing) ...[
                  const SizedBox(width: 4),
                  stateIcon,
                ],
              ],
            ),
          ],
        ),
      ),
    );
  }

  void _showSessionInfo(BuildContext context) {
    String diagnostics = '';
    try {
      diagnostics = engineGetDiagnostics();
    } catch (_) {}

    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('Session: ${widget.peerName}'),
        content: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text('Peer ID: ${widget.peerDeviceId}',
                  style: const TextStyle(fontSize: 12)),
              const SizedBox(height: 8),
              Text('Messages: ${_messages.length}',
                  style: const TextStyle(fontSize: 12)),
              const SizedBox(height: 8),
              const Text('Diagnostics:',
                  style: TextStyle(fontWeight: FontWeight.bold, fontSize: 12)),
              const SizedBox(height: 4),
              Text(diagnostics, style: const TextStyle(fontSize: 11)),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Close'),
          ),
        ],
      ),
    );
  }
}
