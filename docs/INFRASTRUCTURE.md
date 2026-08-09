# Infrastructure and Docker Setup — Universal Offline Transfer (UOT)

> **Containerized test environments, Docker mesh topology, and local runner specs.** Updated: 2026-08-09 (Sprint 11 Audit).

---

## 1. Multi-Node Docker Mesh (`docker-compose.yml`)

The repository includes a 3-node containerized test network:

```yaml
services:
  sender:
    build: .
    environment:
      - UOT_NODE_ROLE=sender
    networks:
      uot-mesh:
        ipv4_address: 172.28.0.10

  receiver:
    build: .
    environment:
      - UOT_NODE_ROLE=receiver
    networks:
      uot-mesh:
        ipv4_address: 172.28.0.11

  runner:
    build: .
    command: cargo test --manifest-path rust/Cargo.toml
    networks:
      uot-mesh:
        ipv4_address: 172.28.0.12
```

---

## 2. Docker Execution Commands

```bash
# Build and run full test suite in Docker container
docker run --rm -v "h:/UOT:/sd" -w /sd ghcr.io/cirruslabs/flutter:3.24.0 bash -c "flutter pub get && flutter analyze && flutter test --coverage"

# Build release APK in Docker container
docker run --rm -v "h:/UOT:/sd" -w /sd ghcr.io/cirruslabs/flutter:3.24.0 bash -c "flutter pub get && flutter build apk --release"
```
