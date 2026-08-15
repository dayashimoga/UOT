import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:open_filex/open_filex.dart';
import 'package:uot_app/src/rust/api/engine_api.dart';
import 'active_session_tracker.dart';

String _formatBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  if (bytes < 1024 * 1024 * 1024) {
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
  return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
}

IconData _getFileIcon(String fileName) {
  final ext = fileName.contains('.')
      ? fileName.split('.').last.toLowerCase()
      : '';
  switch (ext) {
    case 'jpg':
    case 'jpeg':
    case 'png':
    case 'gif':
    case 'webp':
    case 'bmp':
    case 'svg':
      return Icons.image_rounded;
    case 'mp4':
    case 'mkv':
    case 'mov':
    case 'avi':
    case 'webm':
      return Icons.video_file_rounded;
    case 'mp3':
    case 'wav':
    case 'ogg':
    case 'flac':
    case 'm4a':
      return Icons.audio_file_rounded;
    case 'pdf':
      return Icons.picture_as_pdf_rounded;
    case 'zip':
    case 'rar':
    case '7z':
    case 'tar':
    case 'gz':
      return Icons.folder_zip_rounded;
    case 'dart':
    case 'rs':
    case 'py':
    case 'js':
    case 'ts':
    case 'html':
    case 'css':
    case 'json':
    case 'yaml':
      return Icons.code_rounded;
    case 'txt':
    case 'md':
    case 'log':
      return Icons.description_rounded;
    case 'doc':
    case 'docx':
      return Icons.article_rounded;
    case 'xls':
    case 'xlsx':
    case 'csv':
      return Icons.table_chart_rounded;
    case 'ppt':
    case 'pptx':
      return Icons.slideshow_rounded;
    case 'apk':
      return Icons.android_rounded;
    case 'exe':
    case 'msi':
      return Icons.desktop_windows_rounded;
    default:
      return Icons.insert_drive_file_rounded;
  }
}

Color _getFileColor(String fileName) {
  final ext = fileName.contains('.')
      ? fileName.split('.').last.toLowerCase()
      : '';
  switch (ext) {
    case 'jpg':
    case 'jpeg':
    case 'png':
    case 'gif':
    case 'webp':
      return Colors.amber.shade400;
    case 'mp4':
    case 'mkv':
    case 'mov':
      return Colors.purple.shade300;
    case 'mp3':
    case 'wav':
    case 'flac':
      return Colors.pink.shade300;
    case 'pdf':
      return Colors.red.shade400;
    case 'zip':
    case 'rar':
    case '7z':
      return Colors.orange.shade400;
    case 'dart':
    case 'rs':
    case 'py':
    case 'js':
      return Colors.cyan.shade300;
    case 'txt':
    case 'md':
      return Colors.blue.shade300;
    case 'apk':
      return Colors.green.shade400;
    default:
      return Colors.teal.shade300;
  }
}

/// Unified chat & file transfer session screen.
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
  final FocusNode _inputFocusNode = FocusNode();

  List<Map<String, dynamic>> _messages = [];
  List<Map<String, dynamic>> _transfers = [];
  List<Map<String, dynamic>> _timelineItems = [];
  Map<String, dynamic>? _pendingOffer;
  Timer? _refreshTimer;
  bool _isSending = false;

  @override
  void initState() {
    super.initState();
    ActiveChatSessionTracker.setActiveSession(widget.peerDeviceId);
    _loadData();
    _refreshTimer = Timer.periodic(const Duration(milliseconds: 750), (_) {
      _loadData();
      _pollEvents();
    });
  }

  @override
  void dispose() {
    if (ActiveChatSessionTracker.currentPeerDeviceId == widget.peerDeviceId) {
      ActiveChatSessionTracker.setActiveSession(null);
    }
    _refreshTimer?.cancel();
    _messageController.dispose();
    _scrollController.dispose();
    _inputFocusNode.dispose();
    super.dispose();
  }

  void _loadData() {
    try {
      final jsonMsgs = engineGetMessages(peerDeviceId: widget.peerDeviceId);
      final List<dynamic> parsedMsgs = jsonDecode(jsonMsgs);

      final jsonTransfers = engineGetTransfers();
      final List<dynamic> parsedTransfers = jsonDecode(jsonTransfers);

      final matchedTransfers = parsedTransfers
          .cast<Map<String, dynamic>>()
          .where((t) =>
              t['remote_name'] == widget.peerName ||
              t['remote_device'] == widget.peerDeviceId ||
              t['remote_device'] == widget.peerName)
          .toList();

      final List<Map<String, dynamic>> timeline = [];

      for (final m in parsedMsgs) {
        if (m is Map<String, dynamic>) {
          timeline.add({
            'type': 'message',
            'timestamp': m['timestamp'] ?? '',
            'data': m,
          });
        }
      }

      for (final t in matchedTransfers) {
        timeline.add({
          'type': 'transfer',
          'timestamp': t['created_at'] ?? '',
          'data': t,
        });
      }

      timeline.sort((a, b) {
        final tA = a['timestamp']?.toString() ?? '';
        final tB = b['timestamp']?.toString() ?? '';
        return tA.compareTo(tB);
      });

      if (mounted) {
        setState(() {
          _messages = parsedMsgs.cast<Map<String, dynamic>>();
          _transfers = matchedTransfers;
          _timelineItems = timeline;
        });
      }
    } catch (e) {
      debugPrint('Failed to load chat data: $e');
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
          if (fromDevice == widget.peerName ||
              fromDevice == widget.peerDeviceId) {
            setState(() {
              _pendingOffer = event;
            });
          }
        } else if (type == 'TransferStatusChanged' ||
            type == 'TransferProgress' ||
            type == 'IncomingMessage' ||
            type == 'MessageDelivered') {
          _loadData();
        }
      }
    } catch (_) {}
  }

  Future<void> _sendMessage() async {
    final text = _messageController.text.trim();
    if (text.isEmpty || _isSending) return;

    _messageController.clear();
    setState(() => _isSending = true);

    try {
      final result = await engineSendMessage(
        peerDeviceId: widget.peerDeviceId,
        text: text,
      );
      if (result.startsWith('error:') && mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Send failed: ${result.substring(6)}'),
            backgroundColor: Colors.red.shade700,
          ),
        );
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
    } finally {
      if (mounted) setState(() => _isSending = false);
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
          _loadData();
          _scrollToBottom();
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
          _scrollController.position.maxScrollExtent + 120,
          duration: const Duration(milliseconds: 300),
          curve: Curves.easeOut,
        );
      }
    });
  }

  Future<void> _openFile(String path, String fileName) async {
    if (path.isEmpty) return;
    final file = File(path);
    if (!file.existsSync()) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('File not found at: $path'),
            backgroundColor: Colors.red.shade700,
          ),
        );
      }
      return;
    }

    final ext = fileName.contains('.')
        ? fileName.split('.').last.toLowerCase()
        : '';

    // In-app preview for images
    if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp'].contains(ext)) {
      if (!mounted) return;
      showDialog(
        context: context,
        builder: (ctx) => Dialog(
          backgroundColor: Colors.black.withAlpha(220),
          insetPadding: const EdgeInsets.all(12),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              AppBar(
                backgroundColor: Colors.transparent,
                elevation: 0,
                title: Text(fileName, style: const TextStyle(fontSize: 14)),
                actions: [
                  IconButton(
                    icon: const Icon(Icons.close),
                    onPressed: () => Navigator.pop(ctx),
                  ),
                ],
              ),
              Flexible(
                child: InteractiveViewer(
                  child: Image.file(file, fit: BoxFit.contain),
                ),
              ),
              Padding(
                padding: const EdgeInsets.all(8.0),
                child: Text(
                  path,
                  style: const TextStyle(fontSize: 11, color: Colors.white70),
                  textAlign: TextAlign.center,
                ),
              ),
            ],
          ),
        ),
      );
      return;
    }

    // In-app text viewer for notes, markdown, and code
    if (['txt', 'md', 'json', 'yaml', 'rs', 'dart', 'py', 'log'].contains(ext)) {
      try {
        final content = await file.readAsString();
        if (!mounted) return;
        showDialog(
          context: context,
          builder: (ctx) => AlertDialog(
            title: Text(fileName),
            content: SizedBox(
              width: double.maxFinite,
              height: 400,
              child: SingleChildScrollView(
                child: SelectableText(
                  content,
                  style: const TextStyle(
                    fontFamily: 'monospace',
                    fontSize: 13,
                  ),
                ),
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
        return;
      } catch (_) {}
    }

    // Cross-platform native system viewer (Android, iOS, Windows, macOS, Linux)
    try {
      final result = await OpenFilex.open(path);
      if (result.type == ResultType.done) {
        return;
      }
    } catch (_) {}

    // Fallback platform launcher for desktop OS
    if (Platform.isWindows) {
      try {
        await Process.run('cmd', ['/c', 'start', '', path]);
        return;
      } catch (_) {}
    } else if (Platform.isMacOS) {
      try {
        await Process.run('open', [path]);
        return;
      } catch (_) {}
    } else if (Platform.isLinux) {
      try {
        await Process.run('xdg-open', [path]);
        return;
      } catch (_) {}
    }

    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text('Saved to: $path'),
          action: SnackBarAction(
            label: 'Copy Path',
            onPressed: () => Clipboard.setData(ClipboardData(text: path)),
          ),
        ),
      );
    }
  }

  void _revealInFolder(String path) {
    if (path.isEmpty) return;
    if (Platform.isWindows) {
      Process.run('explorer.exe', ['/select,', path]);
    } else if (Platform.isMacOS) {
      Process.run('open', ['-R', path]);
    } else {
      Clipboard.setData(ClipboardData(text: path));
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Path copied: $path')),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Scaffold(
      backgroundColor: theme.scaffoldBackgroundColor,
      appBar: AppBar(
        elevation: 0,
        backgroundColor: colorScheme.surface,
        titleSpacing: 0,
        title: Row(
          children: [
            Stack(
              children: [
                CircleAvatar(
                  radius: 18,
                  backgroundColor: colorScheme.primaryContainer,
                  child: Icon(
                    Icons.devices_rounded,
                    size: 20,
                    color: colorScheme.onPrimaryContainer,
                  ),
                ),
                Positioned(
                  right: 0,
                  bottom: 0,
                  child: Container(
                    width: 9,
                    height: 9,
                    decoration: BoxDecoration(
                      color: const Color(0xFF22C55E),
                      shape: BoxShape.circle,
                      border: Border.all(color: colorScheme.surface, width: 1.5),
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    widget.peerName,
                    style: const TextStyle(
                      fontSize: 15,
                      fontWeight: FontWeight.w700,
                      letterSpacing: 0.1,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                  Row(
                    children: [
                      Icon(
                        Icons.verified_rounded,
                        size: 12,
                        color: Colors.green.shade400,
                      ),
                      const SizedBox(width: 4),
                      Text(
                        'Verified • Wi-Fi • Ready',
                        style: TextStyle(
                          fontSize: 11,
                          fontWeight: FontWeight.w500,
                          color: colorScheme.onSurfaceVariant.withAlpha(200),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
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
            icon: const Icon(Icons.info_outline_rounded),
            tooltip: 'Session Diagnostics',
            onPressed: () => _showSessionInfo(context),
          ),
          const SizedBox(width: 4),
        ],
      ),
      body: SafeArea(
        child: Column(
          children: [
            // Floating Pending Offer Banner
            if (_pendingOffer != null) _buildPendingOfferCard(theme),

            // Messages & Transfers Timeline
            Expanded(
              child: _timelineItems.isEmpty
                  ? _buildEmptyState(theme)
                  : ListView.builder(
                      controller: _scrollController,
                      padding: const EdgeInsets.symmetric(
                        horizontal: 14,
                        vertical: 10,
                      ),
                      itemCount: _timelineItems.length,
                      itemBuilder: (context, index) {
                        final item = _timelineItems[index];
                        final isMsg = item['type'] == 'message';
                        final data = item['data'] as Map<String, dynamic>;

                        if (isMsg) {
                          final msgKey = data['id'] ??
                              data['message_id'] ??
                              'msg_$index';
                          return KeyedSubtree(
                            key: ValueKey(msgKey),
                            child: _buildMessageBubble(data, theme),
                          );
                        } else {
                          final transferKey = data['id'] ??
                              data['transfer_id'] ??
                              'transfer_$index';
                          return KeyedSubtree(
                            key: ValueKey(transferKey),
                            child: _buildTransferCard(data, theme),
                          );
                        }
                      },
                    ),
            ),

            // Bottom Input Composer Bar
            _buildComposerBar(theme),
          ],
        ),
      ),
    );
  }

  Widget _buildEmptyState(ThemeData theme) {
    final colorScheme = theme.colorScheme;
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32.0),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Container(
              padding: const EdgeInsets.all(20),
              decoration: BoxDecoration(
                color: colorScheme.primaryContainer.withAlpha(80),
                shape: BoxShape.circle,
              ),
              child: Icon(
                Icons.swap_horizontal_circle_outlined,
                size: 48,
                color: colorScheme.primary,
              ),
            ),
            const SizedBox(height: 18),
            Text(
              'Connected to ${widget.peerName}',
              style: const TextStyle(
                fontSize: 17,
                fontWeight: FontWeight.bold,
              ),
            ),
            const SizedBox(height: 6),
            Text(
              'End-to-end encrypted offline session ready.\nSend messages or tap + to transfer files.',
              textAlign: TextAlign.center,
              style: TextStyle(
                fontSize: 13,
                height: 1.4,
                color: colorScheme.onSurfaceVariant.withAlpha(160),
              ),
            ),
            const SizedBox(height: 20),
            FilledButton.icon(
              onPressed: _pickAndSendFiles,
              icon: const Icon(Icons.upload_file_rounded, size: 18),
              label: const Text('Send Files'),
              style: FilledButton.styleFrom(
                padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 10),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildPendingOfferCard(ThemeData theme) {
    final offer = _pendingOffer!;
    final transferId = offer['transfer_id']?.toString() ?? '';
    final items = (offer['items'] as List<dynamic>?)
            ?.map((e) => e.toString())
            .toList() ??
        [];
    final totalSize = (offer['total_size'] as num?)?.toInt() ?? 0;

    return Container(
      margin: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: theme.colorScheme.primaryContainer,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: theme.colorScheme.primary.withAlpha(100),
          width: 1.2,
        ),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withAlpha(25),
            blurRadius: 12,
            offset: const Offset(0, 4),
          ),
        ],
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                padding: const EdgeInsets.all(8),
                decoration: BoxDecoration(
                  color: theme.colorScheme.primary,
                  shape: BoxShape.circle,
                ),
                child: const Icon(
                  Icons.file_download_rounded,
                  color: Colors.white,
                  size: 20,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      'Incoming Transfer Offer',
                      style: TextStyle(
                        fontWeight: FontWeight.bold,
                        fontSize: 14,
                        color: theme.colorScheme.onPrimaryContainer,
                      ),
                    ),
                    Text(
                      '${items.length} file(s) • ${_formatBytes(totalSize)}',
                      style: TextStyle(
                        fontSize: 12,
                        color:
                            theme.colorScheme.onPrimaryContainer.withAlpha(200),
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
          if (items.isNotEmpty) ...[
            const SizedBox(height: 8),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
              decoration: BoxDecoration(
                color: theme.colorScheme.surface.withAlpha(120),
                borderRadius: BorderRadius.circular(8),
              ),
              child: Text(
                items.first +
                    (items.length > 1
                        ? ' + ${items.length - 1} more file(s)'
                        : ''),
                style: const TextStyle(fontSize: 12, fontWeight: FontWeight.w500),
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
          const SizedBox(height: 10),
          Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              OutlinedButton(
                onPressed: () async {
                  setState(() => _pendingOffer = null);
                  try {
                    await engineCancelTransfer(transferId: transferId);
                  } catch (_) {}
                },
                style: OutlinedButton.styleFrom(
                  foregroundColor: theme.colorScheme.error,
                  side: BorderSide(color: theme.colorScheme.error.withAlpha(120)),
                ),
                child: const Text('Reject'),
              ),
              const SizedBox(width: 10),
              FilledButton.icon(
                onPressed: () async {
                  setState(() => _pendingOffer = null);
                  final res =
                      await engineAcceptTransfer(transferId: transferId);
                  if (mounted) {
                    final isOk = res.startsWith('ok');
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(
                        content: Text(isOk
                            ? 'Receiving files...'
                            : 'Accept failed: ${res.replaceFirst("error:", "")}'),
                        backgroundColor: isOk ? null : Colors.red.shade700,
                      ),
                    );
                    _loadData();
                  }
                },
                icon: const Icon(Icons.check_rounded, size: 18),
                label: const Text('Accept & Receive'),
              ),
            ],
          ),
        ],
      ),
    );
  }

  Widget _buildTransferCard(Map<String, dynamic> t, ThemeData theme) {
    final status = t['status']?.toString() ?? 'Pending';
    final items = (t['items'] as List<dynamic>?)
        ?.whereType<Map<String, dynamic>>()
        .toList();
    final direction = t['direction']?.toString() ?? 'Send';
    final isSend = direction == 'Send';

    final totalBytes = (t['total_bytes'] as num?)?.toInt() ??
        (t['total_size'] as num?)?.toInt() ??
        0;
    final transferredBytes = (t['transferred_bytes'] as num?)?.toInt() ?? 0;
    final progress = (t['progress'] as num?)?.toDouble() ??
        (totalBytes > 0 ? transferredBytes / totalBytes : 0.0);

    final speedDisplay = t['speed_display']?.toString();
    final etaDisplay = t['eta_display']?.toString();

    final isTransferring = status == 'InProgress' ||
        status == 'Transferring' ||
        status == 'Verifying';
    final isCompleted = status == 'Completed';
    final isFailed = status == 'Failed';

    // Multi-item batch card
    if (items != null && items.length > 1) {
      String batchStatusSubtitle = '';
      if (isTransferring) {
        final pct = (progress * 100).toStringAsFixed(0);
        batchStatusSubtitle =
            '$pct% • ${_formatBytes(transferredBytes)} / ${_formatBytes(totalBytes)}';
        if (speedDisplay != null && speedDisplay.isNotEmpty) {
          batchStatusSubtitle += ' • $speedDisplay';
        }
        if (etaDisplay != null &&
            etaDisplay.isNotEmpty &&
            etaDisplay != 'calculating…') {
          batchStatusSubtitle += ' • ETA $etaDisplay';
        }
      } else if (isCompleted) {
        batchStatusSubtitle =
            'Batch Completed • ${items.length} files • ${_formatBytes(totalBytes)} • Verified ✓';
      } else if (isFailed) {
        final err = t['error']?.toString() ?? 'Transfer interrupted';
        batchStatusSubtitle = 'Failed: $err';
      } else {
        batchStatusSubtitle = isSend
            ? 'Waiting for receiver • ${items.length} files (${_formatBytes(totalBytes)})'
            : 'Pending acceptance • ${items.length} files (${_formatBytes(totalBytes)})';
      }

      return Container(
        margin: const EdgeInsets.symmetric(vertical: 4),
        decoration: BoxDecoration(
          color: theme.colorScheme.surfaceContainerHighest.withAlpha(120),
          borderRadius: BorderRadius.circular(16),
          border: Border.all(
            color: isCompleted
                ? Colors.green.withAlpha(80)
                : isFailed
                    ? Colors.red.withAlpha(80)
                    : theme.colorScheme.outlineVariant.withAlpha(60),
          ),
        ),
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Container(
                    width: 38,
                    height: 38,
                    decoration: BoxDecoration(
                      color: Colors.blue.withAlpha(40),
                      borderRadius: BorderRadius.circular(10),
                    ),
                    child: const Icon(Icons.folder_copy_rounded,
                        color: Colors.blue, size: 22),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Row(
                          children: [
                            Expanded(
                              child: Text(
                                'Batch Transfer (${items.length} files)',
                                style: const TextStyle(
                                  fontWeight: FontWeight.bold,
                                  fontSize: 14,
                                ),
                                overflow: TextOverflow.ellipsis,
                              ),
                            ),
                            Container(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 6,
                                vertical: 2,
                              ),
                              decoration: BoxDecoration(
                                color: isSend
                                    ? Colors.blue.withAlpha(30)
                                    : Colors.green.withAlpha(30),
                                borderRadius: BorderRadius.circular(6),
                              ),
                              child: Text(
                                isSend ? 'SENT' : 'RECEIVED',
                                style: TextStyle(
                                  fontSize: 9,
                                  fontWeight: FontWeight.bold,
                                  color: isSend
                                      ? Colors.blue.shade300
                                      : Colors.green.shade300,
                                ),
                              ),
                            ),
                          ],
                        ),
                        const SizedBox(height: 2),
                        Text(
                          batchStatusSubtitle,
                          style: TextStyle(
                            fontSize: 11.5,
                            color: isFailed
                                ? Colors.red.shade300
                                : theme.colorScheme.onSurfaceVariant,
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
              if (isTransferring || status == 'Pending') ...[
                const SizedBox(height: 10),
                ClipRRect(
                  borderRadius: BorderRadius.circular(4),
                  child: LinearProgressIndicator(
                    value: isTransferring ? progress.clamp(0.0, 1.0) : null,
                    minHeight: 4,
                  ),
                ),
              ],
              const SizedBox(height: 8),
              const Divider(height: 1),
              const SizedBox(height: 6),
              // Individual file items inside batch
              ...items.map((item) {
                final iName = item['name']?.toString() ?? 'File';
                final iSize = (item['size'] as num?)?.toInt() ?? 0;
                final iStatus = item['status']?.toString() ?? 'Pending';
                final iSavedPath = item['saved_path']?.toString() ?? '';
                final iDone = iStatus == 'Completed';
                final iIcon = _getFileIcon(iName);
                final iColor = _getFileColor(iName);

                return Padding(
                  padding: const EdgeInsets.symmetric(vertical: 4),
                  child: Row(
                    children: [
                      Icon(iIcon, color: iColor, size: 18),
                      const SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          '$iName (${_formatBytes(iSize)})',
                          style: const TextStyle(fontSize: 12.5),
                          overflow: TextOverflow.ellipsis,
                        ),
                      ),
                      if (iDone)
                        Icon(Icons.check_circle_rounded,
                            color: Colors.green.shade400, size: 16)
                      else if (iStatus == 'InProgress')
                        const SizedBox(
                          width: 14,
                          height: 14,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      else
                        Text(
                          iStatus,
                          style: TextStyle(
                            fontSize: 10,
                            color: theme.colorScheme.onSurfaceVariant
                                .withAlpha(150),
                          ),
                        ),
                      // Target-only Open action for individual completed file
                      if (!isSend && iDone && iSavedPath.isNotEmpty) ...[
                        const SizedBox(width: 6),
                        IconButton(
                          icon: const Icon(Icons.open_in_new_rounded, size: 15),
                          tooltip: 'Open $iName',
                          visualDensity: VisualDensity.compact,
                          padding: EdgeInsets.zero,
                          constraints: const BoxConstraints(
                              minWidth: 24, minHeight: 24),
                          onPressed: () => _openFile(iSavedPath, iName),
                        ),
                      ],
                    ],
                  ),
                );
              }),
            ],
          ),
        ),
      );
    }

    // Single item transfer card
    final firstItem =
        (items != null && items.isNotEmpty) ? items.first : null;
    final fileName = t['file_name']?.toString() ??
        firstItem?['name']?.toString() ??
        t['name']?.toString() ??
        'File transfer';
    final savedPath = t['saved_path']?.toString() ??
        firstItem?['saved_path']?.toString() ??
        '';

    final fileIcon = _getFileIcon(fileName);
    final fileColor = _getFileColor(fileName);

    String statusSubtitle = '';
    if (isTransferring) {
      final pct = (progress * 100).toStringAsFixed(0);
      statusSubtitle =
          '$pct% • ${_formatBytes(transferredBytes)} / ${_formatBytes(totalBytes)}';
      if (speedDisplay != null && speedDisplay.isNotEmpty) {
        statusSubtitle += ' • $speedDisplay';
      }
      if (etaDisplay != null &&
          etaDisplay.isNotEmpty &&
          etaDisplay != 'calculating…') {
        statusSubtitle += ' • ETA $etaDisplay';
      }
    } else if (isCompleted) {
      statusSubtitle = 'Completed • ${_formatBytes(totalBytes)} • Verified ✓';
    } else if (isFailed) {
      final err = t['error']?.toString() ?? 'Transfer interrupted';
      statusSubtitle = 'Failed: $err';
    } else {
      statusSubtitle = isSend
          ? 'Waiting for receiver • ${_formatBytes(totalBytes)}'
          : 'Pending acceptance • ${_formatBytes(totalBytes)}';
    }

    return Container(
      margin: const EdgeInsets.symmetric(vertical: 4),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withAlpha(120),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: isCompleted
              ? Colors.green.withAlpha(80)
              : isFailed
                  ? Colors.red.withAlpha(80)
                  : theme.colorScheme.outlineVariant.withAlpha(60),
        ),
      ),
      child: Material(
        color: Colors.transparent,
        borderRadius: BorderRadius.circular(16),
        child: InkWell(
          borderRadius: BorderRadius.circular(16),
          // Target-only tap preview
          onTap: (!isSend && isCompleted && savedPath.isNotEmpty)
              ? () => _openFile(savedPath, fileName)
              : null,
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Container(
                      width: 42,
                      height: 42,
                      decoration: BoxDecoration(
                        color: fileColor.withAlpha(40),
                        borderRadius: BorderRadius.circular(10),
                      ),
                      child: Icon(fileIcon, color: fileColor, size: 24),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Expanded(
                                child: Text(
                                  fileName,
                                  style: const TextStyle(
                                    fontWeight: FontWeight.bold,
                                    fontSize: 14,
                                  ),
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                              Container(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 6,
                                  vertical: 2,
                                ),
                                decoration: BoxDecoration(
                                  color: isSend
                                      ? Colors.blue.withAlpha(30)
                                      : Colors.green.withAlpha(30),
                                  borderRadius: BorderRadius.circular(6),
                                ),
                                child: Text(
                                  isSend ? 'SENT' : 'RECEIVED',
                                  style: TextStyle(
                                    fontSize: 9,
                                    fontWeight: FontWeight.bold,
                                    color: isSend
                                        ? Colors.blue.shade300
                                        : Colors.green.shade300,
                                  ),
                                ),
                              ),
                            ],
                          ),
                          const SizedBox(height: 3),
                          Text(
                            statusSubtitle,
                            style: TextStyle(
                              fontSize: 12,
                              color: isFailed
                                  ? Colors.red.shade300
                                  : theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                        ],
                      ),
                    ),
                    const SizedBox(width: 8),
                    if (isTransferring)
                      const SizedBox(
                        width: 18,
                        height: 18,
                        child: CircularProgressIndicator(strokeWidth: 2.2),
                      )
                    else if (isCompleted)
                      Icon(Icons.check_circle_rounded,
                          color: Colors.green.shade400, size: 22)
                    else if (isFailed)
                      Icon(Icons.error_rounded,
                          color: Colors.red.shade400, size: 22),
                  ],
                ),
                if (isTransferring || status == 'Pending') ...[
                  const SizedBox(height: 10),
                  ClipRRect(
                    borderRadius: BorderRadius.circular(4),
                    child: LinearProgressIndicator(
                      value: isTransferring ? progress.clamp(0.0, 1.0) : null,
                      minHeight: 4,
                    ),
                  ),
                ],
                // Target-only actions: Open and Folder exist strictly on receiver
                if (!isSend && isCompleted && savedPath.isNotEmpty) ...[
                  const SizedBox(height: 8),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.end,
                    children: [
                      TextButton.icon(
                        onPressed: () => _openFile(savedPath, fileName),
                        icon: const Icon(Icons.open_in_new_rounded, size: 15),
                        label:
                            const Text('Open', style: TextStyle(fontSize: 12)),
                        style: TextButton.styleFrom(
                          padding: const EdgeInsets.symmetric(horizontal: 10),
                        ),
                      ),
                      const SizedBox(width: 6),
                      TextButton.icon(
                        onPressed: () => _revealInFolder(savedPath),
                        icon: const Icon(Icons.folder_open_rounded, size: 15),
                        label: const Text('Folder',
                            style: TextStyle(fontSize: 12)),
                        style: TextButton.styleFrom(
                          padding: const EdgeInsets.symmetric(horizontal: 10),
                        ),
                      ),
                    ],
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildMessageBubble(Map<String, dynamic> message, ThemeData theme) {
    final isOutgoing = message['direction'] == 'out';
    final content = message['content']?.toString() ?? '';
    final state = message['state']?.toString() ?? '';
    final timestamp = message['timestamp']?.toString() ?? '';

    String timeStr = '';
    try {
      final dt = DateTime.parse(timestamp);
      timeStr =
          '${dt.hour.toString().padLeft(2, '0')}:${dt.minute.toString().padLeft(2, '0')}';
    } catch (_) {}

    Widget stateIcon = const SizedBox.shrink();
    if (isOutgoing) {
      switch (state) {
        case 'Sending':
          stateIcon = const SizedBox(
            width: 11,
            height: 11,
            child: CircularProgressIndicator(
                strokeWidth: 1.5, color: Colors.white70),
          );
          break;
        case 'Sent':
          stateIcon = const Icon(Icons.check, size: 13, color: Colors.white70);
          break;
        case 'Delivered':
          stateIcon =
              const Icon(Icons.done_all, size: 14, color: Color(0xFF60A5FA));
          break;
        case 'Failed':
          stateIcon =
              const Icon(Icons.error_outline, size: 13, color: Colors.redAccent);
          break;
      }
    }

    return Align(
      alignment: isOutgoing ? Alignment.centerRight : Alignment.centerLeft,
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          borderRadius: BorderRadius.only(
            topLeft: const Radius.circular(16),
            topRight: const Radius.circular(16),
            bottomLeft: Radius.circular(isOutgoing ? 16 : 4),
            bottomRight: Radius.circular(isOutgoing ? 4 : 16),
          ),
          onLongPress: () {
            Clipboard.setData(ClipboardData(text: content));
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(
                content: Text('Message copied to clipboard'),
                duration: Duration(seconds: 1),
              ),
            );
          },
          child: Container(
            margin: const EdgeInsets.symmetric(vertical: 3),
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
            constraints: BoxConstraints(
              maxWidth: MediaQuery.of(context).size.width * 0.78,
            ),
            decoration: BoxDecoration(
              color: isOutgoing
                  ? theme.colorScheme.primary
                  : theme.colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.only(
                topLeft: const Radius.circular(16),
                topRight: const Radius.circular(16),
                bottomLeft: Radius.circular(isOutgoing ? 16 : 4),
                bottomRight: Radius.circular(isOutgoing ? 4 : 16),
              ),
              boxShadow: [
                BoxShadow(
                  color: Colors.black.withAlpha(12),
                  blurRadius: 4,
                  offset: const Offset(0, 1),
                ),
              ],
            ),
            child: Column(
              crossAxisAlignment:
                  isOutgoing ? CrossAxisAlignment.end : CrossAxisAlignment.start,
              children: [
                Text(
                  content,
                  style: TextStyle(
                    color: isOutgoing
                        ? theme.colorScheme.onPrimary
                        : theme.colorScheme.onSurface,
                    fontSize: 15,
                    height: 1.3,
                  ),
                ),
                const SizedBox(height: 3),
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      timeStr,
                      style: TextStyle(
                        fontSize: 10.5,
                        color: isOutgoing
                            ? theme.colorScheme.onPrimary.withAlpha(180)
                            : theme.colorScheme.onSurfaceVariant.withAlpha(150),
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
        ),
      ),
    );
  }

  Widget _buildComposerBar(ThemeData theme) {
    final colorScheme = theme.colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
      decoration: BoxDecoration(
        color: colorScheme.surface,
        border: Border(
          top: BorderSide(color: colorScheme.outlineVariant.withAlpha(50)),
        ),
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          IconButton(
            icon: Icon(
              Icons.add_circle_outline_rounded,
              color: colorScheme.primary,
              size: 26,
            ),
            tooltip: 'Send Files',
            onPressed: _pickAndSendFiles,
          ),
          Expanded(
            child: Container(
              margin: const EdgeInsets.symmetric(horizontal: 4),
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerHighest.withAlpha(150),
                borderRadius: BorderRadius.circular(24),
              ),
              child: TextField(
                controller: _messageController,
                focusNode: _inputFocusNode,
                minLines: 1,
                maxLines: 4,
                textInputAction: TextInputAction.send,
                decoration: InputDecoration(
                  hintText: 'Type a message to ${widget.peerName}...',
                  hintStyle: TextStyle(
                    fontSize: 14,
                    color: colorScheme.onSurfaceVariant.withAlpha(120),
                  ),
                  contentPadding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 10,
                  ),
                  border: InputBorder.none,
                ),
                onSubmitted: (_) => _sendMessage(),
              ),
            ),
          ),
          FloatingActionButton.small(
            onPressed: _sendMessage,
            elevation: 0,
            backgroundColor: colorScheme.primary,
            foregroundColor: colorScheme.onPrimary,
            child: _isSending
                ? const SizedBox(
                    width: 16,
                    height: 16,
                    child: CircularProgressIndicator(
                      strokeWidth: 2,
                      color: Colors.white,
                    ),
                  )
                : const Icon(Icons.send_rounded, size: 18),
          ),
        ],
      ),
    );
  }

  void _showSessionInfo(BuildContext context) {
    String diagnostics = '';
    try {
      diagnostics = engineGetDiagnostics();
    } catch (_) {}

    Map<String, dynamic> diagMap = {};
    try {
      diagMap = jsonDecode(diagnostics);
    } catch (_) {}

    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
      builder: (ctx) => DraggableScrollableSheet(
        initialChildSize: 0.55,
        minChildSize: 0.35,
        maxChildSize: 0.85,
        expand: false,
        builder: (_, scrollCtrl) => Padding(
          padding: const EdgeInsets.all(20),
          child: ListView(
            controller: scrollCtrl,
            children: [
              Center(
                child: Container(
                  width: 40,
                  height: 4,
                  margin: const EdgeInsets.only(bottom: 16),
                  decoration: BoxDecoration(
                    color: Colors.grey.shade400,
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
              ),
              Text(
                'Session: ${widget.peerName}',
                style: const TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
              ),
              const SizedBox(height: 12),
              _buildDiagTile(
                Icons.fingerprint_rounded,
                'Peer Device ID',
                widget.peerDeviceId,
              ),
              _buildDiagTile(
                Icons.shield_outlined,
                'Security',
                'AES-256-GCM • X25519 Verified',
              ),
              _buildDiagTile(
                Icons.wifi_rounded,
                'Transport',
                'Local TCP / Wi-Fi',
              ),
              _buildDiagTile(
                Icons.chat_bubble_outline_rounded,
                'Messages Cached',
                '${_messages.length} messages',
              ),
              _buildDiagTile(
                Icons.swap_vert_rounded,
                'Transfers Tracked',
                '${_transfers.length} transfers',
              ),
              const Divider(height: 24),
              const Text(
                'Raw Diagnostics',
                style: TextStyle(fontWeight: FontWeight.bold, fontSize: 13),
              ),
              const SizedBox(height: 6),
              Container(
                padding: const EdgeInsets.all(10),
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: SelectableText(
                  const JsonEncoder.withIndent('  ').convert(diagMap.isNotEmpty ? diagMap : diagnostics),
                  style: const TextStyle(fontFamily: 'monospace', fontSize: 11),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildDiagTile(IconData icon, String title, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        children: [
          Icon(icon, size: 18, color: Colors.teal.shade300),
          const SizedBox(width: 10),
          Text('$title: ', style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 13)),
          Expanded(
            child: Text(
              value,
              style: const TextStyle(fontSize: 13),
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }
}
