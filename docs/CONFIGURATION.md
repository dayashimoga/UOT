# Configuration Reference — Universal Offline Transfer (UOT)

> **Application settings schema, environment variables, and runtime configuration.** Updated: 2026-08-09 (Sprint 11 Audit).

---

## 1. UserSettings Schema (`rust/src/core/settings.rs`)

Settings are saved in JSON format at the platform app data directory:

```json
{
  "device_name": "Desktop-Host",
  "port": 42000,
  "download_dir": "/downloads/uot",
  "max_concurrent_transfers": 4,
  "auto_accept_trusted": true,
  "theme_mode": "dark",
  "bandwidth_limit_mbps": 0
}
```

---

## 2. Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `UOT_PORT` | Port for TCP listener | `42000` |
| `UOT_NODE_ROLE` | Docker node role (`sender` / `receiver`) | `receiver` |
| `FLUTTER_VERSION` | CI Flutter SDK version | `3.24.0` |
