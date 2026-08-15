/// Global tracker for the currently active/visible chat session.
/// Used by background screens (e.g. NearbyScreen) to suppress duplicate modals
/// and SnackBars when the user is already in the conversation.
class ActiveChatSessionTracker {
  ActiveChatSessionTracker._();

  static String? _currentPeerDeviceId;

  /// Get the active peer device ID or name currently in view.
  static String? get currentPeerDeviceId => _currentPeerDeviceId;

  /// Set the active peer device ID when entering a chat.
  static void setActiveSession(String? peerDeviceId) {
    _currentPeerDeviceId = peerDeviceId;
  }

  /// Check if the given peer device ID or name is currently open in active chat.
  static bool isSessionActive(String? peerDeviceId, [String? peerName]) {
    if (_currentPeerDeviceId == null || peerDeviceId == null) return false;
    return _currentPeerDeviceId == peerDeviceId ||
        (_currentPeerDeviceId != null &&
            peerName != null &&
            _currentPeerDeviceId == peerName);
  }
}
