# TODO — Universal Offline Transfer (UOT)

## Active Sprint Tasks (Production Validation Sprint)
- [x] Integrate `TrustManager` & PIN verification into `UotEngine`
- [x] Implement incoming offer transfer consent gating (`accept_transfer` / `cancel_transfer`)
- [x] Add idle connection timeout (60s) to connection receive loop
- [x] Create Rust integration test suite (`rust/tests/integration_transfer.rs`)
- [x] Create Flutter widget tests for `ReceiveScreen` and `IncomingOfferDialog`
- [x] Add Docker container mesh setup (`Dockerfile` & `docker-compose.yml`)
- [x] Update documentation suite (`GAP_ANALYSIS.md`, `PRODUCTION_READINESS.md`, `TESTING.md`)

## Completed Features
- [x] AES-256-GCM + X25519 authenticated encryption
- [x] StrictPathValidator (traversal, null-bytes, symlinks, Windows reserved names)
- [x] mDNS Discovery & Subnet Scanner fallback
- [x] Chunked File Transfer Engine with CRC32 & SHA-256 integrity
- [x] Transfer Queue Manager priority scheduling
- [x] Lifetime Analytics & Persistent Transfer History
- [x] Event Log Ring Buffer
- [x] Real Pause/Resume with tokio watch channels
- [x] StreamManager integrated into UotEngine (`start_stream`, `stop_stream`, `get_streams`)
- [x] 130 Rust Tests (100% Pass) & 10 Flutter Tests (100% Pass)

## Future Roadmap (Platform & Hardware Extensions)
- [ ] Platform-native BLE GATT host adapters (Android/iOS)
- [ ] Platform-native Wi-Fi Direct P2P Group Owner adapters (Android)
- [ ] Camera QR code decoder & Fountain packet reconstruction
- [ ] Real-time video/audio payload byte streaming relay pipeline
