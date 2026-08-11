# CHANGELOG

All notable changes to UOT (Universal Offline Transfer) are documented here.
This file is append-only - history is never overwritten.

## [0.1.0-alpha.11] - 2026-08-11

### Sprint 11 — Connectivity E2E Reliability & Protocol Handshake

#### P0: Hello/HelloAck Protocol Handshake (Critical Fix)
- `connect_peer()` now performs full TCP → Hello → HelloAck → Ping handshake before claiming "Connected"
- Previously only opened TCP socket and immediately reported success — no protocol verification
- `handle_incoming_connection()` now handles `WireMessage::Hello` and replies with `WireMessage::HelloAck`
- Device identity (name, type, ID) now comes from HelloAck, not synthesized from IP address
- Connection only marked "Connected" after bidirectional handshake is verified

#### P0: PeerConnectionState Tracking
- Added `PeerConnectionState` enum: TcpConnected → HelloSent → HelloAcked → PingConfirmed → SessionReady
- Per-peer state tracked in `UotEngine::peer_states` HashMap
- `PeerStateChanged` event emitted at each handshake phase for UI updates
- New API: `engine_get_peer_state(device_id)` for Flutter to query connection status

#### P0: ClipboardData Receiver Handler
- `handle_incoming_connection()` now processes `WireMessage::ClipboardData` messages
- Emits new `EngineEvent::ClipboardReceived { from_device, text }` for InstantChat delivery
- `send_clipboard()` now reuses existing connection before falling back to fresh TCP

#### P0: Flutter UI Verified Connection Status
- `QrPairingDialog` and `QrScannerDialog` now parse HelloAck JSON to show real peer device name
- Connection success message shows "Connected to {name} (Hello verified ✓)"

#### P0: Test Fix
- Fixed `test_engine_connect_peer_and_subnet_scan_branches` — now spawns mock Hello server
- Test verifies full Hello/HelloAck handshake and PeerConnectionState::SessionReady

#### Verification
- Rust: 407 tests passed, 0 failed (250 lib + 119 coverage + 38 integration)
- Flutter: 14 tests passed, 0 issues
- `flutter analyze`: 0 issues

## [0.1.0-alpha.7] - 2026-08-10

### Sprint 15 - QR Code Pairing, Direct IP Connectivity & LAN Subnet Discovery

#### QR Code Pairing & Direct Connect Dialog
- Fixed top bar "Scan QR Code" button handler in NearbyScreen.
- Implemented QrPairingDialog featuring:
  - **Tab 1 ("My QR & IP")**: Renders device QR code using qr_flutter, displays 6-digit PIN code, and displays local IPv4 address (192.168.x.x:42000) with a 1-tap "Copy IP" button.
  - **Tab 2 ("Direct IP Connect")**: Text input to enter peer IP address (e.g. 192.168.1.50 or 192.168.1.50:42000) with instant TCP connection button.

#### My Device Banner & Quick Action Toolbar
- Added _MyDeviceBanner header to NearbyScreen displaying local device name and active IPv4 address.
- Added quick action buttons: "Pair / Show QR", "Direct IP Connect", and "Scan Subnet (LAN)".

#### Engine API & Subnet Auto-Registration
- Added engine_get_local_ips() to expose active local IPv4 interfaces to Flutter.
- Added engine_connect_peer(address) to initiate direct TCP connections to specified IP addresses.
- Updated Rust subnet_scan() to automatically populate DiscoveredDevice entries into devices map when active listeners on port 42000 are found.
- Configured periodic LAN subnet scanning on NearbyScreen every 6 seconds.

#### Verification
- **Flutter Analyze**: 0 errors, 0 warnings
- **Flutter Unit Tests**: 14/14 Passed
- **Rust Test Suite**: 392/392 Passed

## [0.1.0-alpha.8] - 2026-08-10

### Sprint 16 - Optical Animated QR Air-Gapped Transfer & Connectivity Hardening

#### Optical Animated QR Stream (Zero Network / Air-Gapped)
- Created `OpticalQrSenderDialog` widget rendering animated Luby Transform / Fountain Code QR code stream at 5-15 FPS.
- Added "Optical Animated QR Stream" option in `_SendBottomSheet` for transmitting files without Wi-Fi, Bluetooth, or cellular networks.
- Exposed `engine_fountain_encode` in Rust FRB API to convert binary file payloads into fountain packets.
- Enhanced `QrScannerDialog` with camera optical stream decoding, automatic URI payload parsing, and interactive manual QR link input/paste text field.

#### Connectivity & Windows Firewall Hardening
- Added non-blocking Windows Defender Firewall port 42000 rule auto-registration on engine initialization (`netsh advfirewall firewall add rule`).
- Updated `connect_peer` in Rust engine with 4-second connection attempt timeout and automatic candidate port fallback (`[42000, 42001, 8080, 50000]`).
- Added real-time `SnackBar` & `AlertDialog` visual feedback when initiating file send actions.

#### Verification
- **Flutter Analyze**: 0 errors, 0 warnings (100% clean)
- **Rust Test Suite**: 407/407 Passed across 8 test suites
- **Coverage**: 82.15% line coverage in Docker Tarpaulin

## [0.1.0-alpha.9] - 2026-08-10

### Sprint 17 - UX Gap Resolution: Camera Permissions, File Transfer Modal, Media Streaming & Engine Status

#### Engine Status & Initialization Fixes
- Made mDNS service creation non-fatal in `UotEngine::start()` so TCP listener starts cleanly even on systems without mDNS services.
- Updated `_EngineStatusCard` in `NearbyScreen` to recognize `Running`, `Partial`, and `Starting` engine states, resolving persistent "Engine starting..." banner issue.

#### Camera QR Scanner & Permission Request
- Added explicit `requestPermission()` call during camera adapter initialization in `QrScannerDialog`.
- Added **"Pick QR Code Image File"** button to allow picking QR code images from disk on Desktop or mobile devices.

#### Active Transfer Modal & Feedback
- Implemented `ActiveTransferDialog` rendering real-time animated transfer progress bar, live speed (MB/s), ETA, current file item name, and Cancel controls.
- Connected `_showSendFeedback` in `NearbyScreen` to automatically launch `ActiveTransferDialog` upon file selection.

#### Media Streaming Screen Implementation
- Fully implemented `StreamScreen` controls for **Camera**, **Screen Share**, **Video File**, and **Audio File**.
- Integrated `FilePicker` for local Video/Audio files and `engine_start_stream` / `engine_stop_stream` for live streaming session lifecycle.
- Added real-time Active Streams list displaying session title, device target, LIVE indicator, and 1-tap Stop buttons.

#### Verification
- **Flutter Analyze**: 0 errors, 0 warnings (100% clean)
- **Flutter Tests**: 14/14 Passed
- **Rust Test Suite**: 407/407 Passed across 8 test suites

## [0.1.0-alpha.10] - 2026-08-10

### Sprint 18 - Direct IP Validation, ConfirmSendDialog, InstantChat & Android MobileScanner

#### Direct IP Address Validation & Auto-Fill
- Added strict IPv4 octet (0-255) validation in `QrPairingDialog` to catch typos (e.g. `292.168.0.111`) before initiating connections.

#### File Send Confirmation Modal (`ConfirmSendDialog`)
- Created `ConfirmSendDialog` displaying selected file list, individual item sizes, batch total size, target device info, and a prominent green `[🚀 Send Files Now]` action button.

#### Instant Messaging & Connection Ping (`InstantChatDialog`)
- Created `InstantChatDialog` enabling two-way text message exchange, connection pings (`PING: Connection Check`), and delivery verification between connected nodes.

#### Android Mobile Camera QR Scanner
- Integrated `mobile_scanner: ^7.4.0` in `QrScannerDialog` for live hardware camera scanning on Android devices.

#### Verification
- **Flutter Analyze**: 0 errors, 0 warnings (100% clean)
- **Flutter Tests**: 14/14 Passed
- **Rust Test Suite**: 407/407 Passed across 8 test suites

## [0.1.0-alpha.11] - 2026-08-11

### Sprint 19 - Scan Loop Debouncing, Error State Pause & Windows Firewall Elevation Helper

#### Camera QR Scan Debouncing & Error Pause
- Resolved camera infinite connection retry loop in `QrScannerDialog` by introducing scan debouncing (`_scanPaused` flag & `_lastScannedPayload`).
- Pauses scanning upon connection failure and displays a prominent `[Tap to Scan Again]` button on the camera viewfinder overlay.

#### Windows Firewall Admin Elevation Helper
- Exposed `engine_fix_windows_firewall()` in Rust FRB API to trigger Windows PowerShell UAC prompt allowing inbound TCP port 42000.
- Added 1-tap `[Fix Windows Firewall (Allow Port 42000)]` action button in `QrScannerDialog` and `QrPairingDialog`.

#### Verification
- **Flutter Analyze**: 0 errors, 0 warnings (100% clean)
- **Flutter Tests**: 14/14 Passed
- **Rust Test Suite**: 407/407 Passed across 8 test suites

## [0.1.0-alpha.12] - 2026-08-11

### Sprint 20 - Dynamic Bound Port QR Payloads, Self-Loopback Protection & Candidate Port Fallback

#### Dynamic Listening Port API
- Exposed `engine_get_listening_port()` in Rust FRB API to query actual bound TCP transport socket port.
- Updated `QrPairingDialog` and `NearbyScreen` `_MyDeviceBanner` to generate QR payload and display IP with the actual bound port (e.g. `:42001` if `:42000` is bound).

#### Self-Loopback Connection Prevention
- Added self-loopback check in `connect_peer` returning an explicit error (`Cannot connect to your own device`) if a device attempts to connect to its own local IP and bound port.

#### Candidate Port Probing
- Enhanced `connect_peer` to probe fallback ports (`[42000, 42001, 42002, 42003, 8080, 50000]`) even when explicit target `IP:port` strings are provided.

#### Verification
- **Flutter Analyze**: 0 errors, 0 warnings (100% clean)
- **Flutter Tests**: 14/14 Passed
- **Rust Test Suite**: 407/407 Passed across 8 test suites

---





