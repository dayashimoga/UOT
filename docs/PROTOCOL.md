# UOT Protocol Specification

## Version: 1 (Draft)

## Protocol Flow

```
DISCOVER → PAIR → AUTHENTICATE → NEGOTIATE → CREATE_SESSION →
OFFER → ACCEPT → START → CHUNK → ACK →
[PAUSE → RESUME] | [RECONNECT → RETRY] →
VERIFY → COMPLETE | CANCEL | ERROR
```

## State Machine

15 states with validated transitions. See `rust/src/protocol/state.rs`.

```
                    ┌─────────┐
                    │  IDLE   │
                    └────┬────┘
                         │
                    ┌────▼────┐
                    │DISCOVER │
                    └────┬────┘
                         │
                    ┌────▼────┐
                    │ PAIRING │
                    └────┬────┘
                         │
                    ┌────▼────────┐
                    │AUTHENTICATING│
                    └────┬────────┘
                         │
                    ┌────▼───────┐
                    │NEGOTIATING │
                    └────┬───────┘
                         │
                    ┌────▼──────────┐
           ┌────── │SESSION ACTIVE │◄──────────┐
           │       └────┬──────────┘           │
           │            │                      │
           │       ┌────▼──────────┐           │
           │       │ OFFER PENDING │           │
           │       └────┬──────────┘           │
           │            │                      │
           │       ┌────▼──────────┐           │
           │       │OFFER ACCEPTED │           │
           │       └────┬──────────┘           │
           │            │                      │
           │       ┌────▼──────────┐           │
           │    ┌──│ TRANSFERRING  │──┐        │
           │    │  └───────────────┘  │        │
           │    │         │           │        │
           │ ┌──▼──┐  ┌──▼────┐  ┌──▼───────┐│
           │ │PAUSE│  │VERIFY │  │RECONNECT ││
           │ └──┬──┘  └──┬────┘  └──┬───────┘│
           │    │         │          │        │
           │    └─►RESUME │    RETRY─┘        │
           │              │                   │
           │       ┌──────▼──────┐            │
           └───────│  COMPLETE   │────────────┘
                   ├─────────────┤
                   │  CANCELLED  │
                   ├─────────────┤
                   │    ERROR    │
                   └─────────────┘
```

## Message Format

All messages use JSON serialization with a common header:

```json
{
  "header": {
    "message_id": "uuid-v4",
    "session_id": "uuid-v4 | null",
    "protocol_version": 1,
    "sequence": 0,
    "timestamp": "2026-08-07T12:00:00Z",
    "sender_id": "device-id"
  },
  "payload": { "type": "..." }
}
```

## Message Types

16 message categories defined in `rust/src/protocol/messages.rs`:

| Category | Messages | Purpose |
|----------|----------|---------|
| Discovery | Discover, DiscoverResponse | Find nearby devices |
| Pairing | PairRequest, PairResponse | Establish trust |
| Session | CreateSession, SessionCreated | Create transfer session |
| Transfer | Offer, OfferResponse, Start, Chunk, Ack | Data transfer |
| Control | Pause, Resume, Cancel, Reconnect, Retry | Flow control |
| Verification | Verify, VerifyResult | Integrity check |
| Completion | Complete, Error | Terminal states |
| Heartbeat | Ping, Pong | Keep-alive |

## Integrity

- Per-chunk CRC32 checksum
- Per-file SHA-256 hash verification
- Replay protection via monotonic sequence numbers
- Session expiry with timeout
