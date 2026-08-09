// Camera QR Scanner Platform Adapter — Production Implementation
//
// Uses Flutter MethodChannel to bridge to native camera APIs:
// - Android: CameraX + ML Kit Barcode Scanner
// - iOS: AVFoundation + Vision framework
//
// Falls back gracefully on unsupported platforms (desktop/web).

import 'dart:async';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// QR scan result containing decoded data.
class QrScanResult {
  final String data;
  final String format;
  final DateTime scannedAt;

  const QrScanResult({
    required this.data,
    required this.format,
    required this.scannedAt,
  });

  factory QrScanResult.fromJson(Map<String, dynamic> json) => QrScanResult(
    data: json['data'] as String,
    format: json['format'] as String? ?? 'QR_CODE',
    scannedAt: DateTime.now(),
  );
}

/// Camera permission state.
enum CameraPermissionState { unknown, granted, denied, restricted }

/// Camera QR scanner adapter with native platform bridge.
class CameraQrAdapter {
  static const MethodChannel _channel = MethodChannel(
    'com.uot.camera/qr_scanner',
  );
  static const EventChannel _scanStream = EventChannel(
    'com.uot.camera/qr_stream',
  );

  final _resultController = StreamController<QrScanResult>.broadcast();
  bool _isSupported = false;
  bool _isScanning = false;

  Stream<QrScanResult> get scanResults => _resultController.stream;
  bool get isSupported => _isSupported;
  bool get isScanning => _isScanning;

  /// Initialize camera QR scanner.
  /// Returns true if camera scanning is available.
  Future<bool> initialize() async {
    try {
      if (!_isPlatformSupported()) {
        debugPrint('[CameraQR] Platform not supported');
        return false;
      }

      final result = await _channel.invokeMethod<Map>('initialize');
      _isSupported = result?['supported'] == true;
      debugPrint('[CameraQR] Initialized: supported=$_isSupported');
      return _isSupported;
    } on PlatformException catch (e) {
      debugPrint('[CameraQR] Init failed: ${e.message}');
      return false;
    } on MissingPluginException {
      debugPrint('[CameraQR] Native plugin not available — stub mode');
      return false;
    }
  }

  /// Request camera permission.
  Future<CameraPermissionState> requestPermission() async {
    if (!_isSupported) return CameraPermissionState.denied;
    try {
      final result = await _channel.invokeMethod<String>('requestPermission');
      switch (result) {
        case 'granted':
          return CameraPermissionState.granted;
        case 'denied':
          return CameraPermissionState.denied;
        case 'restricted':
          return CameraPermissionState.restricted;
        default:
          return CameraPermissionState.unknown;
      }
    } on PlatformException {
      return CameraPermissionState.denied;
    }
  }

  /// Start QR code scanning.
  /// Results are emitted on [scanResults] stream.
  Future<bool> startScanning() async {
    if (!_isSupported || _isScanning) return false;
    try {
      final result = await _channel.invokeMethod<bool>('startScanning');
      if (result == true) {
        _isScanning = true;
        _scanStream.receiveBroadcastStream().listen(
          (event) {
            if (event is Map) {
              final qr = QrScanResult.fromJson(
                Map<String, dynamic>.from(event),
              );
              _resultController.add(qr);
            }
          },
          onError: (error) {
            debugPrint('[CameraQR] Scan error: $error');
          },
        );
      }
      return result == true;
    } on PlatformException catch (e) {
      debugPrint('[CameraQR] startScanning failed: ${e.message}');
      return false;
    }
  }

  /// Stop QR code scanning.
  Future<void> stopScanning() async {
    if (!_isScanning) return;
    try {
      await _channel.invokeMethod('stopScanning');
      _isScanning = false;
    } on PlatformException catch (e) {
      debugPrint('[CameraQR] stopScanning failed: ${e.message}');
    }
  }

  /// Generate a QR code image from data (returns PNG bytes).
  Future<Uint8List?> generateQrImage(String data, {int size = 256}) async {
    if (!_isSupported) return null;
    try {
      final result = await _channel.invokeMethod<Uint8List>('generateQr', {
        'data': data,
        'size': size,
      });
      return result;
    } on PlatformException {
      return null;
    }
  }

  bool _isPlatformSupported() {
    return defaultTargetPlatform == TargetPlatform.android ||
        defaultTargetPlatform == TargetPlatform.iOS;
  }

  void dispose() {
    _resultController.close();
    _isScanning = false;
  }
}
