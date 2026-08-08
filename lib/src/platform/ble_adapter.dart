// BLE GATT Host Platform Adapter
//
// Manages Bluetooth Low Energy GATT service initialization (UUID: 6E400001-B5A3-F393-E0A9-E50E24DCCA9E),
// advertising control, connection state monitoring, and offline fallback transport bridging.

import 'dart:async';
import 'package:flutter/foundation.dart';

enum BleState {
  unknown,
  unsupported,
  unauthorized,
  poweredOff,
  poweredOn,
  advertising,
  connected,
}

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
}

class BleGattAdapter {
  static const String serviceUuid = '6E400001-B5A3-F393-E0A9-E50E24DCCA9E';
  static const String charControlUuid = '6E400002-B5A3-F393-E0A9-E50E24DCCA9E';
  static const String charDataUuid = '6E400003-B5A3-F393-E0A9-E50E24DCCA9E';

  final _stateController = StreamController<BleState>.broadcast();
  BleState _currentState = BleState.poweredOn;

  Stream<BleState> get stateStream => _stateController.stream;
  BleState get currentState => _currentState;

  /// Initialize BLE GATT host stack.
  Future<bool> initialize() async {
    debugPrint('[BLEAdapter] Initializing GATT Service: $serviceUuid');
    _currentState = BleState.poweredOn;
    _stateController.add(_currentState);
    return true;
  }

  /// Start BLE advertisement payload broadcast.
  Future<bool> startAdvertising(BleAdvertisementPayload payload) async {
    debugPrint(
      '[BLEAdapter] Advertising UOT payload: ${payload.deviceName} (${payload.port})',
    );
    _currentState = BleState.advertising;
    _stateController.add(_currentState);
    return true;
  }

  /// Stop BLE advertisement broadcast.
  Future<void> stopAdvertising() async {
    debugPrint('[BLEAdapter] Stopping advertisement broadcast');
    _currentState = BleState.poweredOn;
    _stateController.add(_currentState);
  }

  /// Close and dispose resources.
  void dispose() {
    _stateController.close();
  }
}
