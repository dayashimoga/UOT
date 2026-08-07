//! UOT Transfer Protocol
//!
//! Defines the protocol state machine, message types, and session
//! management for the UOT transfer protocol.
//!
//! Protocol flow:
//! DISCOVER → PAIR → AUTHENTICATE → NEGOTIATE → CREATE_SESSION →
//! OFFER → ACCEPT → START → CHUNK → ACK → PAUSE → RESUME →
//! RECONNECT → RETRY → VERIFY → COMPLETE/CANCEL/ERROR
pub mod handler;
pub mod messages;
pub mod state;
