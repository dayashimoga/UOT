# CHANGELOG

All notable changes to UOT (Universal Offline Transfer) are documented here.
This file is append-only - history is never overwritten.

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

---
