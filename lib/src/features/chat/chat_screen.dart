import 'dart:async';
import 'dart:convert';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:uot_app/src/rust/api/engine_api.dart';

String _formatBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  if (bytes < 1024 * 1024 * 1024) {
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
  return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
}

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
  List<Map<String, dynamic>> _transfers = [];
  Map<String, dynamic>? _pendingOffer;
  Timer? _refreshTimer;

  @override
  void initState() {
    super.initState();
    _loadData();
    _refreshTimer = Timer.periodic(const Duration(milliseconds: 800), (_) {
      _loadData();
      _pollEvents();
    });
  }

  @override
  void dispose() {
    _refreshTimer?.cancel();
    _messageController.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  void _loadData() {
    try {
      final jsonMsgs = engineGetMessages(peerDeviceId: widget.peerDeviceId);
      final List<dynamic> parsedMsgs = jsonDecode(jsonMsgs);

      final jsonTransfers = engineGetTransfers();
      final List<dynamic> parsedTransfers = jsonDecode(jsonTransfers);

      if (mounted) {
        setState(() {
          _messages = parsedMsgs.cast<Map<String, dynamic>>();
          _transfers = parsedTransfers
              .cast<Map<String, dynamic>>()
              .where((t) =>
                  t['remote_name'] == widget.peerName ||
                  t['remote_device'] == widget.peerDeviceId ||
                  t['remote_device'] == widget.peerName)
              .toList();
        });
      }
    } catch (e) {
      debugPrint('Failed to load data: $e');
    }
  }

  void _pollEvents() {
    if (!mounted) return;
    try {
      final eventsJson = enginePollEvents();
      final List<dynamic> events = jsonDecode(eventsJson);
      for (final event in events) {
        if (event is! Map<String, dynamic>) continue;
        final type = event['type'] as String?;
        if (type == 'IncomingOffer') {
          final fromDevice = event['from_device']?.toString() ?? '';
          if (fromDevice == widget.peerName || fromDevice == widget.peerDeviceId) {
            setState(() {
              _pendingOffer = event;
            });
          }
        } else if (type == 'TransferStatusChanged' || type == 'TransferProgress') {
          _loadData();
        }
      }
    } catch (_) {}
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
      _loadData();
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

  Future<void> _pickAndSendFiles() async {
    try {
      final result = await FilePicker.platform.pickFiles(allowMultiple: true);
      if (result == null || result.paths.isEmpty) return;
      final paths = result.paths.whereType<String>().toList();
      if (paths.isEmpty) return;

      final res = await engineSendFiles(
        deviceId: widget.peerDeviceId,
        filePaths: paths,
      );

      if (mounted) {
        if (res.startsWith('error:')) {
          final err = res.replaceFirst('error:', '');
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('File send error: $err'),
              backgroundColor: Colors.red.shade700,
              duration: const Duration(seconds: 4),
            ),
          );
        } else {
          final tid = res.replaceFirst('ok:', '');
          final shortId = tid.length >= 8 ? tid.substring(0, 8) : tid;
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Sending ${paths.length} file(s)... (ID: $shortId)'),
              duration: const Duration(seconds: 4),
            ),
          );
          _loadData();
        }
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('File send error: $e'),
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
                style:
                    const TextStyle(fontSize: 16, fontWeight: FontWeight.bold)),
            Text(
              'Verified session ready',
              style: TextStyle(fontSize: 12, color: Colors.green.shade300),
            ),
          ],
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.attach_file_rounded),
            tooltip: 'Send Files',
            onPressed: _pickAndSendFiles,
          ),
          IconButton(
            icon: const Icon(Icons.info_outline),
            tooltip: 'Session Info',
            onPressed: () {
              _showSessionInfo(context);
            },
          ),
        ],
      ),
      body: Column(
        children: [
          // Pending Offer Banner
          if (_pendingOffer != null) _buildPendingOfferCard(theme),

          // Messages list
          Expanded(
            child: _messages.isEmpty && _transfers.isEmpty
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
                    itemCount: _messages.length + _transfers.length,
                    itemBuilder: (context, index) {
                      if (index < _messages.length) {
                        final msg = _messages[index];
                        final msgKey = msg['id'] ?? msg['message_id'] ?? 'msg_$index';
                        return KeyedSubtree(
                          key: ValueKey(msgKey),
                          child: _buildMessageBubble(msg, theme),
                        );
                      } else {
                        final t = _transfers[index - _messages.length];
                        final transferKey = t['id'] ?? t['transfer_id'] ?? 'transfer_$index';
                        return KeyedSubtree(
                          key: ValueKey(transferKey),
                          child: _buildTransferCard(t, theme),
                        );
                      }
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
                IconButton(
                  icon: const Icon(Icons.add_circle_outline_rounded),
                  tooltip: 'Send Files',
                  onPressed: _pickAndSendFiles,
                ),
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

  Widget _buildPendingOfferCard(ThemeData theme) {
    final offer = _pendingOffer!;
    final transferId = offer['transfer_id']?.toString() ?? '';
    final items = (offer['items'] as List<dynamic>?)?.map((e) => e.toString()).toList() ?? [];
    final totalSize = (offer['total_size'] as num?)?.toInt() ?? 0;

    return Container(
      margin: const EdgeInsets.all(12),
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: theme.colorScheme.primaryContainer.withOpacity(0.9),
        borderRadius: BorderRadius.circular(16),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withOpacity(0.2),
            blurRadius: 10,
            offset: const Offset(0, 4),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(Icons.file_download_rounded, color: theme.colorScheme.onPrimaryContainer, size: 28),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  'Incoming from ${widget.peerName}',
                  style: theme.textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                    color: theme.colorScheme.onPrimaryContainer,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 8),
          Text(
            '${items.length} file(s) • ${_formatBytes(totalSize)}',
            style: theme.textTheme.bodySmall?.copyWith(
              color: theme.colorScheme.onPrimaryContainer.withOpacity(0.8),
            ),
          ),
          if (items.isNotEmpty) ...[
            const SizedBox(height: 4),
            Text(
              '• ${items.first}${items.length > 1 ? ' (and ${items.length - 1} more)' : ''}',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onPrimaryContainer.withOpacity(0.8),
              ),
            ),
          ],
          const SizedBox(height: 12),
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              TextButton(
                onPressed: () async {
                  setState(() {
                    _pendingOffer = null;
                  });
                  try {
                    await engineCancelTransfer(transferId: transferId);
                  } catch (_) {}
                },
                child: Text('Reject', style: TextStyle(color: theme.colorScheme.error)),
              ),
              const SizedBox(width: 8),
              FilledButton.icon(
                onPressed: () async {
                  setState(() {
                    _pendingOffer = null;
                  });
                  final res = await engineAcceptTransfer(transferId: transferId);
                  if (mounted) {
                    final isOk = res.startsWith('ok');
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(
                        content: Text(isOk
                            ? 'Receiving files...'
                            : 'Failed to accept: ${res.replaceFirst("error:", "")}'),
                        backgroundColor: isOk ? null : Colors.red.shade700,
                      ),
                    );
                    _loadData();
                  }
                },
                icon: const Icon(Icons.check_rounded, size: 18),
                label: const Text('Accept'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildTransferCard(Map<String, dynamic> t, ThemeData theme) {
    final status = t['status']?.toString() ?? 'Pending';
    final items = t['items'] as List<dynamic>?;
    final firstItemName = (items != null && items.isNotEmpty && items.first is Map)
        ? items.first['name']?.toString()
        : null;
    final fileName = t['file_name']?.toString() ??
        firstItemName ??
        t['name']?.toString() ??
        'File transfer';
    final totalBytes = (t['total_bytes'] as num?)?.toInt() ??
        (t['total_size'] as num?)?.toInt() ??
        0;
    final transferredBytes = (t['transferred_bytes'] as num?)?.toInt() ?? 0;
    final progress = (t['progress'] as num?)?.toDouble() ??
        (totalBytes > 0 ? transferredBytes / totalBytes : 0.0);
    final direction = t['direction']?.toString() ?? 'Send';
    final isSend = direction == 'Send';

    return Card(
      margin: const EdgeInsets.symmetric(vertical: 4, horizontal: 4),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  isSend ? Icons.upload_file_rounded : Icons.download_for_offline_rounded,
                  color: isSend ? Colors.blue.shade300 : Colors.green.shade300,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        fileName,
                        style: const TextStyle(fontWeight: FontWeight.bold, fontSize: 14),
                        overflow: TextOverflow.ellipsis,
                      ),
                      Text(
                        '$status • ${_formatBytes(transferredBytes)} / ${_formatBytes(totalBytes)}',
                        style: TextStyle(fontSize: 12, color: theme.colorScheme.onSurfaceVariant),
                      ),
                    ],
                  ),
                ),
                if (status == 'Transferring' || status == 'Pending')
                  const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                else if (status == 'Completed')
                  const Icon(Icons.check_circle_rounded, color: Colors.green, size: 20)
                else if (status == 'Failed')
                  const Icon(Icons.error_rounded, color: Colors.red, size: 20),
              ],
            ),
            if (status == 'Transferring' || status == 'Pending') ...[
              const SizedBox(height: 8),
              LinearProgressIndicator(
                value: progress.clamp(0.0, 1.0),
                borderRadius: BorderRadius.circular(4),
              ),
            ],
          ],
        ),
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
