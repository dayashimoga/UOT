# Troubleshooting & Diagnostics — Universal Offline Transfer (UOT)

> **Common network, platform, and transfer error diagnostics.** Updated: 2026-08-09 (Sprint 11 Audit).

---

## 1. Common Issues & Resolutions

| Symptom | Probable Cause | Diagnostic / Fix |
|---------|----------------|------------------|
| **Device not discovered** | mDNS blocked by local firewall or router isolation | Click **Subnet Scan** in Quick Actions to active-scan IPv4 `/24` range on port 42000. |
| **Connection refused** | Port 42000 blocked or engine not started | Check Settings to verify port 42000 is open in OS firewall. |
| **Path Traversal Error** | Receiving filename contains invalid characters (`..`, `/`, `\0`) | `StrictPathValidator` automatically sanitizes or rejects unsafe filenames to protect host filesystem. |
| **Transfer Paused** | Transient network drop | Tap **Resume** on the active transfer card in Transfers tab. Engine automatically uses `CheckpointStore` to resume from exact chunk offset. |
