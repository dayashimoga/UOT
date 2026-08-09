// BLE GATT Host Platform Adapter — Production Implementation
//
// Uses Flutter MethodChannel to bridge to native BLE APIs:
// - Android: android.bluetooth.BluetoothGattServer + BluetoothLeAdvertiser
// - iOS: CoreBluetooth CBPeripheralManager + CBCentralManager
//
// Falls back to stub mode on unsupported platforms (Windows/Linux/Web).

import 'dart:async';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// BLE adapter state.
enum BleState {
  unknown,
  unsupported,
  unauthorized,
  poweredOff,
  poweredOn,
  advertising,
  connected,
  scanning,
}

/// BLE advertisement payload for UOT discovery.
class BleAdvertisementPayload {
  final String deviceName;
  final String deviceHash;
  final String? wifiIp;
  final int port;

  const BleAdvertisementPayload({
    required this.deviceName,
    required this.deviceHash,
    this.wifiIp,
    required this.port,
  });

  Map<String, dynamic> toJson() => {
    'device_name': deviceName,
    'device_hash': deviceHash,
    'wifi_ip': wifiIp,
    'port': port,
  };

  factory BleAdvertisementPayload.fromJson(Map<String, dynamic> json) =>
      BleAdvertisementPayload(
        deviceName: json['device_name'] as String,
        deviceHash: json['device_hash'] as String,
        wifiIp: json['wifi_ip'] as String?,
        port: json['port'] as int,
      );
}

/// Discovered BLE device.
class BleDiscoveredDevice {
  final String id;
  final String name;
  final int rssi;
  final BleAdvertisementPayload? payload;

  const BleDiscoveredDevice({
    required this.id,
    required this.name,
    required this.rssi,
    this.payload,
  });

  factory BleDiscoveredDevice.fromJson(Map<String, dynamic> json) {
    BleAdvertisementPayload? payload;
    if (json['payload'] != null) {
      payload = BleAdvertisementPayload.fromJson(
        json['payload'] as Map<String, dynamic>,
      );
    }
    return BleDiscoveredDevice(
      id: json['id'] as String,
      name: json['name'] as String? ?? 'Unknown',
      rssi: json['rssi'] as int? ?? -100,
      payload: payload,
    );
  }
}

/// Production BLE GATT adapter with native platform channel bridge.
class BleGattAdapter {
  static const String serviceUuid = '6E400001-B5A3-F393-E0A9-E50E24DCCA9E';
  static const String charControlUuid = '6E400002-B5A3-F393-E0A9-E50E24DCCA9E';
  static const String charDataUuid = '6E400003-B5A3-F393-E0A9-E50E24DCCA9E';

  static const MethodChannel _channel = MethodChannel('com.uot.ble/adapter');
  static const EventChannel _stateChannel = EventChannel(
    'com.uot.ble/state_stream',
  );
  static const EventChannel _scanChannel = EventChannel(
    'com.uot.ble/scan_stream',
  );

  final _stateController = StreamController<BleState>.broadcast();
  final _discoveredController =
      StreamController<BleDiscoveredDevice>.broadcast();
  BleState _currentState = BleState.unknown;
  bool _isSupported = false;

  Stream<BleState> get stateStream => _stateController.stream;
  Stream<BleDiscoveredDevice> get discoveredDevices =>
      _discoveredController.stream;
  BleState get currentState => _currentState;
  bool get isSupported => _isSupported;

  /// Initialize BLE GATT adapter.
  /// Returns true if BLE is available on this platform.
  Future<bool> initialize() async {
    try {
      // Check platform support
      if (!_isPlatformSupported()) {
        debugPrint('[BLEAdapter] Platform not supported for BLE');
        _currentState = BleState.unsupported;
        _stateController.add(_currentState);
        return false;
      }

      final result = await _channel.invokeMethod<Map>('initialize', {
        'serviceUuid': serviceUuid,
        'charControlUuid': charControlUuid,
        'charDataUuid': charDataUuid,
      });

      _isSupported = result?['supported'] == true;
      if (_isSupported) {
        _currentState = BleState.poweredOn;
        _stateController.add(_currentState);

        // Listen for native state changes
        _stateChannel.receiveBroadcastStream().listen(_handleNativeStateChange);
      } else {
        _currentState = BleState.unsupported;
        _stateController.add(_currentState);
      }

      debugPrint('[BLEAdapter] Initialized: supported=$_isSupported');
      return _isSupported;
    } on PlatformException catch (e) {
      debugPrint('[BLEAdapter] Init failed: ${e.message}');
      _currentState = BleState.unsupported;
      _stateController.add(_currentState);
      return false;
    } on MissingPluginException {
      debugPrint('[BLEAdapter] Native plugin not available — stub mode');
      _currentState = BleState.unsupported;
      _stateController.add(_currentState);
      return false;
    }
  }

  /// Start BLE advertising with UOT service UUID.
  Future<bool> startAdvertising(BleAdvertisementPayload payload) async {
    if (!_isSupported) return false;
    try {
      final result = await _channel.invokeMethod<bool>('startAdvertising', {
        'payload': jsonEncode(payload.toJson()),
        'serviceUuid': serviceUuid,
      });
      if (result == true) {
        _currentState = BleState.advertising;
        _stateController.add(_currentState);
      }
      return result == true;
    } on PlatformException catch (e) {
      debugPrint('[BLEAdapter] startAdvertising failed: ${e.message}');
      return false;
    }
  }

  /// Stop BLE advertising.
  Future<void> stopAdvertising() async {
    if (!_isSupported) return;
    try {
      await _channel.invokeMethod('stopAdvertising');
      _currentState = BleState.poweredOn;
      _stateController.add(_currentState);
    } on PlatformException catch (e) {
      debugPrint('[BLEAdapter] stopAdvertising failed: ${e.message}');
    }
  }

  /// Start scanning for nearby UOT BLE devices.
  Future<bool> startScanning({
    Duration timeout = const Duration(seconds: 10),
  }) async {
    if (!_isSupported) return false;
    try {
      _currentState = BleState.scanning;
      _stateController.add(_currentState);

      await _channel.invokeMethod('startScanning', {
        'serviceUuid': serviceUuid,
        'timeoutMs': timeout.inMilliseconds,
      });

      _scanChannel.receiveBroadcastStream().listen((event) {
        if (event is Map) {
          final device = BleDiscoveredDevice.fromJson(
            Map<String, dynamic>.from(event),
          );
          _discoveredController.add(device);
        }
      });

      return true;
    } on PlatformException catch (e) {
      debugPrint('[BLEAdapter] startScanning failed: ${e.message}');
      return false;
    }
  }

  /// Stop scanning.
  Future<void> stopScanning() async {
    if (!_isSupported) return;
    try {
      await _channel.invokeMethod('stopScanning');
      _currentState = BleState.poweredOn;
      _stateController.add(_currentState);
    } on PlatformException catch (e) {
      debugPrint('[BLEAdapter] stopScanning failed: ${e.message}');
    }
  }

  /// Send data to a connected BLE peer via GATT write.
  Future<bool> sendData(String deviceId, List<int> data) async {
    if (!_isSupported) return false;
    try {
      final result = await _channel.invokeMethod<bool>('sendData', {
        'deviceId': deviceId,
        'data': data,
        'characteristicUuid': charDataUuid,
      });
      return result == true;
    } on PlatformException catch (e) {
      debugPrint('[BLEAdapter] sendData failed: ${e.message}');
      return false;
    }
  }

  void _handleNativeStateChange(dynamic event) {
    if (event is String) {
      final state = BleState.values.firstWhere(
        (s) => s.name == event,
        orElse: () => BleState.unknown,
      );
      _currentState = state;
      _stateController.add(_currentState);
    }
  }

  bool _isPlatformSupported() {
    return defaultTargetPlatform == TargetPlatform.android ||
        defaultTargetPlatform == TargetPlatform.iOS;
  }

  void dispose() {
    _stateController.close();
    _discoveredController.close();
  }
}
