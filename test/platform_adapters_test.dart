// Platform Adapters Unit Test
//
// Validates BleGattAdapter, WifiDirectAdapter, and CameraQrAdapter initialization and event streams.

import 'package:flutter_test/flutter_test.dart';
import 'package:uot_app/src/platform/ble_adapter.dart';
import 'package:uot_app/src/platform/camera_qr_adapter.dart';
import 'package:uot_app/src/platform/wifi_direct_adapter.dart';

void main() {
  group('BleGattAdapter', () {
    test('initializes and advertises payload', () async {
      final adapter = BleGattAdapter();
      expect(adapter.currentState, BleState.poweredOn);

      final ok = await adapter.initialize();
      expect(ok, isTrue);

      const payload = BleAdvertisementPayload(
        deviceName: 'TestDevice',
        deviceHash: 'abc12345',
        port: 42000,
      );
      final advOk = await adapter.startAdvertising(payload);
      expect(advOk, isTrue);
      expect(adapter.currentState, BleState.advertising);

      await adapter.stopAdvertising();
      expect(adapter.currentState, BleState.poweredOn);
      adapter.dispose();
    });
  });

  group('WifiDirectAdapter', () {
    test('creates P2P group as Group Owner', () async {
      final adapter = WifiDirectAdapter();
      expect(adapter.currentState, WifiDirectGroupState.idle);

      final group = await adapter.createGroup(
        deviceName: 'HostNode',
        port: 42000,
      );
      expect(group.ssid, contains('DIRECT-UOT-HostNode'));
      expect(group.groupOwnerIp, '192.168.49.1');
      expect(adapter.currentState, WifiDirectGroupState.groupCreated);

      await adapter.removeGroup();
      expect(adapter.currentState, WifiDirectGroupState.idle);
      adapter.dispose();
    });
  });

  group('CameraQrAdapter', () {
    test('starts scanning and processes simulated barcode frames', () async {
      final adapter = CameraQrAdapter();
      expect(adapter.isScanning, isFalse);

      final ok = await adapter.startScanning();
      expect(ok, isTrue);
      expect(adapter.isScanning, isTrue);

      expectLater(
        adapter.scanStream,
        emits(predicate<QrScanResult>((r) => r.rawData == 'test_qr_data')),
      );

      adapter.handleScannedFrame('test_qr_data');
      await adapter.stopScanning();
      expect(adapter.isScanning, isFalse);
      adapter.dispose();
    });
  });
}
