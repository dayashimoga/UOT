// Optical Animated QR Stream Sender Dialog (Air-Gapped / Zero Network)
//
// Transmits file payload using an animated Luby Transform / Fountain Code QR stream.
// No Wi-Fi, Bluetooth, or cellular network connection is required between devices.

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:qr_flutter/qr_flutter.dart';
import '../../rust/api/engine_api.dart' as engine;

class OpticalQrSenderDialog extends StatefulWidget {
  final String fileName;
  final String payloadText;

  const OpticalQrSenderDialog({
    super.key,
    required this.fileName,
    required this.payloadText,
  });

  static Future<void> show(
    BuildContext context, {
    required String fileName,
    required String payloadText,
  }) {
    return showDialog<void>(
      context: context,
      builder: (context) => OpticalQrSenderDialog(
        fileName: fileName,
        payloadText: payloadText,
      ),
    );
  }

  @override
  State<OpticalQrSenderDialog> createState() => _OpticalQrSenderDialogState();
}

class _OpticalQrSenderDialogState extends State<OpticalQrSenderDialog> {
  List<Map<String, dynamic>> _packets = [];
  int _currentIndex = 0;
  Timer? _animTimer;
  bool _isPlaying = true;
  int _fps = 10; // Frames per second
  bool _isLoading = true;

  @override
  void initState() {
    super.initState();
    _encodeFountainStream();
  }

  void _encodeFountainStream() {
    try {
      final base64Payload = base64Encode(utf8.encode(widget.payloadText));
      final packetsJson = engine.engineFountainEncode(
        dataBase64: base64Payload,
        blockSize: 128,
      );

      final List<dynamic> rawList = jsonDecode(packetsJson);
      final List<Map<String, dynamic>> list =
          rawList.map((e) => Map<String, dynamic>.from(e as Map)).toList();

      if (mounted) {
        setState(() {
          _packets = list;
          _isLoading = false;
        });
        _startAnimation();
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isLoading = false;
        });
      }
    }
  }

  void _startAnimation() {
    _animTimer?.cancel();
    if (_packets.isEmpty) return;

    final intervalMs = (1000 / _fps).round();
    _animTimer = Timer.periodic(Duration(milliseconds: intervalMs), (_) {
      if (_isPlaying && mounted && _packets.isNotEmpty) {
        setState(() {
          _currentIndex = (_currentIndex + 1) % _packets.length;
        });
      }
    });
  }

  @override
  void dispose() {
    _animTimer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    String currentQrData = 'uot://fountain?empty=true';
    if (_packets.isNotEmpty && _currentIndex < _packets.length) {
      currentQrData = 'uot://fountain?${jsonEncode(_packets[_currentIndex])}';
    }

    return Dialog(
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 440, maxHeight: 600),
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Column(
            children: [
              Row(
                children: [
                  Icon(Icons.style_rounded, color: colorScheme.primary),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Optical Animated QR Stream',
                          style: theme.textTheme.titleMedium?.copyWith(
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                        Text(
                          'Zero Network • Air-Gapped Fountain Code',
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
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
              const SizedBox(height: 16),

              // File badge
              Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
                decoration: BoxDecoration(
                  color: colorScheme.primaryContainer,
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(
                      Icons.insert_drive_file_rounded,
                      size: 18,
                      color: colorScheme.onPrimaryContainer,
                    ),
                    const SizedBox(width: 8),
                    Flexible(
                      child: Text(
                        widget.fileName,
                        style: theme.textTheme.bodyMedium?.copyWith(
                          fontWeight: FontWeight.bold,
                          color: colorScheme.onPrimaryContainer,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),

              // QR Stream Viewer
              Expanded(
                child: _isLoading
                    ? const Center(child: CircularProgressIndicator())
                    : Center(
                        child: Container(
                          padding: const EdgeInsets.all(12),
                          decoration: BoxDecoration(
                            color: Colors.white,
                            borderRadius: BorderRadius.circular(16),
                            border: Border.all(
                              color: colorScheme.primary,
                              width: 2,
                            ),
                          ),
                          child: QrImageView(
                            data: currentQrData,
                            version: QrVersions.auto,
                            size: 220.0,
                            backgroundColor: Colors.white,
                          ),
                        ),
                      ),
              ),

              const SizedBox(height: 16),

              // Stream info & FPS controls
              if (_packets.isNotEmpty) ...[
                Text(
                  'Frame ${_currentIndex + 1} / ${_packets.length} • Loop Active',
                  style: theme.textTheme.labelLarge?.copyWith(
                    fontWeight: FontWeight.bold,
                    color: colorScheme.primary,
                  ),
                ),
                const SizedBox(height: 12),
                Row(
                  mainAxisAlignment: MainAxisAlignment.center,
                  children: [
                    IconButton.filledTonal(
                      icon: Icon(
                        _isPlaying
                            ? Icons.pause_rounded
                            : Icons.play_arrow_rounded,
                      ),
                      onPressed: () {
                        setState(() => _isPlaying = !_isPlaying);
                      },
                    ),
                    const SizedBox(width: 16),
                    Text(
                      'Speed: $_fps FPS',
                      style: theme.textTheme.bodySmall,
                    ),
                    const SizedBox(width: 8),
                    SegmentedButton<int>(
                      segments: const [
                        ButtonSegment(value: 5, label: Text('5')),
                        ButtonSegment(value: 10, label: Text('10')),
                        ButtonSegment(value: 15, label: Text('15')),
                      ],
                      selected: {_fps},
                      onSelectionChanged: (set) {
                        setState(() {
                          _fps = set.first;
                        });
                        _startAnimation();
                      },
                    ),
                  ],
                ),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
