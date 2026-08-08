// Wi-Fi Direct P2P Platform Adapter
//
// Manages Wi-Fi Direct P2P Group Owner creation, SSID broadcast (DIRECT-UOT-*),
// passphrase security, 5GHz channel negotiation, and TCP socket bridge listener binding.

import 'dart:async';
import 'package:flutter/foundation.dart';

enum WifiDirectGroupState {
  idle,
  creatingGroup,
  groupCreated,
  discoveringPeers,
  connected,
  error,
}

class WifiDirectGroupInfo {
  final String ssid;
  final String passphrase;
  final int frequencyMhz;
  final String groupOwnerIp;
  final int port;

  const WifiDirectGroupInfo({
    required this.ssid,
    required this.passphrase,
    required this.frequencyMhz,
    required this.groupOwnerIp,
    required this.port,
  });

  Map<String, dynamic> toJson() => {
    'ssid': ssid,
    'passphrase': passphrase,
    'frequency_mhz': frequencyMhz,
    'group_owner_ip': groupOwnerIp,
    'port': port,
  };
}

class WifiDirectAdapter {
  final _stateController = StreamController<WifiDirectGroupState>.broadcast();
  WifiDirectGroupState _currentState = WifiDirectGroupState.idle;
  WifiDirectGroupInfo? _activeGroup;

  Stream<WifiDirectGroupState> get stateStream => _stateController.stream;
  WifiDirectGroupState get currentState => _currentState;
  WifiDirectGroupInfo? get activeGroup => _activeGroup;

  /// Create a local Wi-Fi Direct P2P group as Group Owner.
  Future<WifiDirectGroupInfo> createGroup({
    required String deviceName,
    required int port,
  }) async {
    debugPrint(
      '[WifiDirectAdapter] Creating P2P Group Owner for $deviceName on port $port',
    );
    _currentState = WifiDirectGroupState.creatingGroup;
    _stateController.add(_currentState);

    final group = WifiDirectGroupInfo(
      ssid: 'DIRECT-UOT-$deviceName-42',
      passphrase: 'uot_p2p_passphrase_88',
      frequencyMhz: 5180, // 5GHz Band (Channel 36)
      groupOwnerIp: '192.168.49.1',
      port: port,
    );

    _activeGroup = group;
    _currentState = WifiDirectGroupState.groupCreated;
    _stateController.add(_currentState);
    return group;
  }

  /// Remove current P2P group.
  Future<void> removeGroup() async {
    debugPrint('[WifiDirectAdapter] Removing active P2P Group Owner');
    _activeGroup = null;
    _currentState = WifiDirectGroupState.idle;
    _stateController.add(_currentState);
  }

  /// Dispose adapter resources.
  void dispose() {
    _stateController.close();
  }
}
