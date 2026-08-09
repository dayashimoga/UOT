// Wi-Fi Direct P2P Platform Adapter — Production Implementation
//
// Uses Flutter MethodChannel to bridge to native Wi-Fi Direct APIs:
// - Android: android.net.wifi.p2p.WifiP2pManager (Group Owner + Service Discovery)
// - iOS: Uses Multipeer Connectivity framework (MCNearbyServiceBrowser + MCSession)
//
// Falls back to stub mode on unsupported platforms.

import 'dart:async';
import 'dart:convert';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// Wi-Fi Direct group state.
enum WifiDirectGroupState {
  idle,
  creatingGroup,
  groupCreated,
  discoveringPeers,
  connecting,
  connected,
  error,
}

/// Wi-Fi Direct group connection info.
class WifiDirectGroupInfo {
  final String ssid;
  final String passphrase;
  final int frequencyMhz;
  final String groupOwnerIp;
  final int port;
  final bool isGroupOwner;

  const WifiDirectGroupInfo({
    required this.ssid,
    required this.passphrase,
    required this.frequencyMhz,
    required this.groupOwnerIp,
    required this.port,
    this.isGroupOwner = true,
  });

  Map<String, dynamic> toJson() => {
    'ssid': ssid,
    'passphrase': passphrase,
    'frequency_mhz': frequencyMhz,
    'group_owner_ip': groupOwnerIp,
    'port': port,
    'is_group_owner': isGroupOwner,
  };

  factory WifiDirectGroupInfo.fromJson(Map<String, dynamic> json) =>
      WifiDirectGroupInfo(
        ssid: json['ssid'] as String,
        passphrase: json['passphrase'] as String,
        frequencyMhz: json['frequency_mhz'] as int? ?? 5180,
        groupOwnerIp: json['group_owner_ip'] as String,
        port: json['port'] as int,
        isGroupOwner: json['is_group_owner'] as bool? ?? true,
      );
}

/// Discovered Wi-Fi Direct peer.
class WifiDirectPeer {
  final String deviceId;
  final String deviceName;
  final String deviceAddress;
  final bool isGroupOwner;

  const WifiDirectPeer({
    required this.deviceId,
    required this.deviceName,
    required this.deviceAddress,
    this.isGroupOwner = false,
  });

  factory WifiDirectPeer.fromJson(Map<String, dynamic> json) => WifiDirectPeer(
    deviceId: json['device_id'] as String,
    deviceName: json['device_name'] as String? ?? 'Unknown',
    deviceAddress: json['device_address'] as String,
    isGroupOwner: json['is_group_owner'] as bool? ?? false,
  );
}

/// Production Wi-Fi Direct adapter with native platform channel bridge.
class WifiDirectAdapter {
  static const MethodChannel _channel = MethodChannel(
    'com.uot.wifidirect/adapter',
  );
  static const EventChannel _stateChannel = EventChannel(
    'com.uot.wifidirect/state_stream',
  );
  static const EventChannel _peerChannel = EventChannel(
    'com.uot.wifidirect/peer_stream',
  );

  final _stateController = StreamController<WifiDirectGroupState>.broadcast();
  final _peerController = StreamController<WifiDirectPeer>.broadcast();
  WifiDirectGroupState _currentState = WifiDirectGroupState.idle;
  WifiDirectGroupInfo? _activeGroup;
  bool _isSupported = false;

  Stream<WifiDirectGroupState> get stateStream => _stateController.stream;
  Stream<WifiDirectPeer> get discoveredPeers => _peerController.stream;
  WifiDirectGroupState get currentState => _currentState;
  WifiDirectGroupInfo? get activeGroup => _activeGroup;
  bool get isSupported => _isSupported;

  /// Initialize Wi-Fi Direct adapter.
  Future<bool> initialize() async {
    try {
      if (!_isPlatformSupported()) {
        debugPrint('[WifiDirect] Platform not supported');
        _currentState = WifiDirectGroupState.error;
        _stateController.add(_currentState);
        return false;
      }

      final result = await _channel.invokeMethod<Map>('initialize');
      _isSupported = result?['supported'] == true;

      if (_isSupported) {
        _stateChannel.receiveBroadcastStream().listen(_handleNativeStateChange);
      }

      debugPrint('[WifiDirect] Initialized: supported=$_isSupported');
      return _isSupported;
    } on PlatformException catch (e) {
      debugPrint('[WifiDirect] Init failed: ${e.message}');
      return false;
    } on MissingPluginException {
      debugPrint('[WifiDirect] Native plugin not available — stub mode');
      return false;
    }
  }

  /// Create a P2P Group as Group Owner.
  Future<WifiDirectGroupInfo?> createGroup({
    required String deviceName,
    required int port,
  }) async {
    if (!_isSupported) return null;
    try {
      _currentState = WifiDirectGroupState.creatingGroup;
      _stateController.add(_currentState);

      final result = await _channel.invokeMethod<Map>('createGroup', {
        'deviceName': deviceName,
        'port': port,
      });

      if (result != null) {
        _activeGroup = WifiDirectGroupInfo.fromJson(
          Map<String, dynamic>.from(result),
        );
        _currentState = WifiDirectGroupState.groupCreated;
        _stateController.add(_currentState);
        return _activeGroup;
      }
      return null;
    } on PlatformException catch (e) {
      debugPrint('[WifiDirect] createGroup failed: ${e.message}');
      _currentState = WifiDirectGroupState.error;
      _stateController.add(_currentState);
      return null;
    }
  }

  /// Discover nearby Wi-Fi Direct peers.
  Future<bool> discoverPeers({
    Duration timeout = const Duration(seconds: 15),
  }) async {
    if (!_isSupported) return false;
    try {
      _currentState = WifiDirectGroupState.discoveringPeers;
      _stateController.add(_currentState);

      await _channel.invokeMethod('discoverPeers', {
        'timeoutMs': timeout.inMilliseconds,
      });

      _peerChannel.receiveBroadcastStream().listen((event) {
        if (event is Map) {
          final peer = WifiDirectPeer.fromJson(
            Map<String, dynamic>.from(event),
          );
          _peerController.add(peer);
        }
      });

      return true;
    } on PlatformException catch (e) {
      debugPrint('[WifiDirect] discoverPeers failed: ${e.message}');
      return false;
    }
  }

  /// Connect to a discovered peer.
  Future<bool> connectToPeer(String deviceAddress) async {
    if (!_isSupported) return false;
    try {
      _currentState = WifiDirectGroupState.connecting;
      _stateController.add(_currentState);

      final result = await _channel.invokeMethod<bool>('connectToPeer', {
        'deviceAddress': deviceAddress,
      });

      if (result == true) {
        _currentState = WifiDirectGroupState.connected;
        _stateController.add(_currentState);
      }
      return result == true;
    } on PlatformException catch (e) {
      debugPrint('[WifiDirect] connectToPeer failed: ${e.message}');
      return false;
    }
  }

  /// Remove current P2P group.
  Future<void> removeGroup() async {
    if (!_isSupported) return;
    try {
      await _channel.invokeMethod('removeGroup');
      _activeGroup = null;
      _currentState = WifiDirectGroupState.idle;
      _stateController.add(_currentState);
    } on PlatformException catch (e) {
      debugPrint('[WifiDirect] removeGroup failed: ${e.message}');
    }
  }

  void _handleNativeStateChange(dynamic event) {
    if (event is String) {
      final state = WifiDirectGroupState.values.firstWhere(
        (s) => s.name == event,
        orElse: () => WifiDirectGroupState.idle,
      );
      _currentState = state;
      _stateController.add(_currentState);
    }
  }

  bool _isPlatformSupported() {
    // Wi-Fi Direct: Android only. iOS uses Multipeer Connectivity instead.
    return defaultTargetPlatform == TargetPlatform.android;
  }

  void dispose() {
    _stateController.close();
    _peerController.close();
  }
}
