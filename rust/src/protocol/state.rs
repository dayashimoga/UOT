//! Protocol State Machine
//!
//! Defines all valid states and transitions for the UOT transfer protocol.
use serde::{Deserialize, Serialize};

/// All possible states in the UOT protocol state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolState {
    /// Initial state — no connection.
    Idle,
    /// Discovering nearby devices.
    Discovering,
    /// Pairing with a discovered device.
    Pairing,
    /// Authenticating the paired device.
    Authenticating,
    /// Negotiating transfer parameters.
    Negotiating,
    /// Session established, ready for transfers.
    SessionActive,
    /// An offer has been sent/received.
    OfferPending,
    /// The offer has been accepted.
    OfferAccepted,
    /// Transfer is in progress.
    Transferring,
    /// Transfer is paused.
    Paused,
    /// Reconnecting after connection loss.
    Reconnecting,
    /// Verifying transfer integrity.
    Verifying,
    /// Transfer completed successfully.
    Completed,
    /// Transfer was cancelled.
    Cancelled,
    /// An error occurred.
    Error,
}

impl ProtocolState {
    /// Check if a transition from this state to `next` is valid.
    pub fn can_transition_to(&self, next: ProtocolState) -> bool {
        use ProtocolState::*;
        matches!(
            (self, next),
            // Discovery flow
            (Idle, Discovering)
                | (Discovering, Pairing)
                | (Discovering, Idle)
                | (Pairing, Authenticating)
                | (Pairing, Idle)
                | (Authenticating, Negotiating)
                | (Authenticating, Error)
                | (Negotiating, SessionActive)
                | (Negotiating, Error)
                // Transfer flow
                | (SessionActive, OfferPending)
                | (SessionActive, Idle)
                | (OfferPending, OfferAccepted)
                | (OfferPending, Cancelled)
                | (OfferPending, SessionActive) // offer rejected
                | (OfferAccepted, Transferring)
                | (Transferring, Paused)
                | (Transferring, Verifying)
                | (Transferring, Cancelled)
                | (Transferring, Error)
                | (Transferring, Reconnecting)
                | (Paused, Transferring) // resume
                | (Paused, Cancelled)
                | (Reconnecting, Transferring) // resume after reconnect
                | (Reconnecting, Error)
                | (Reconnecting, Negotiating) // re-negotiate
                | (Verifying, Completed)
                | (Verifying, Error)
                // Terminal states can reset
                | (Completed, SessionActive)
                | (Completed, Idle)
                | (Cancelled, SessionActive)
                | (Cancelled, Idle)
                | (Error, Idle)
                | (Error, Reconnecting)
        )
    }

    /// Whether this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ProtocolState::Completed | ProtocolState::Cancelled | ProtocolState::Error
        )
    }

    /// Whether transfers are active in this state.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            ProtocolState::Transferring | ProtocolState::Paused | ProtocolState::Reconnecting
        )
    }
}

impl std::fmt::Display for ProtocolState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Discovering => write!(f, "Discovering"),
            Self::Pairing => write!(f, "Pairing"),
            Self::Authenticating => write!(f, "Authenticating"),
            Self::Negotiating => write!(f, "Negotiating"),
            Self::SessionActive => write!(f, "Session Active"),
            Self::OfferPending => write!(f, "Offer Pending"),
            Self::OfferAccepted => write!(f, "Offer Accepted"),
            Self::Transferring => write!(f, "Transferring"),
            Self::Paused => write!(f, "Paused"),
            Self::Reconnecting => write!(f, "Reconnecting"),
            Self::Verifying => write!(f, "Verifying"),
            Self::Completed => write!(f, "Completed"),
            Self::Cancelled => write!(f, "Cancelled"),
            Self::Error => write!(f, "Error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        use ProtocolState::*;
        // Happy path
        assert!(Idle.can_transition_to(Discovering));
        assert!(Discovering.can_transition_to(Pairing));
        assert!(Pairing.can_transition_to(Authenticating));
        assert!(Authenticating.can_transition_to(Negotiating));
        assert!(Negotiating.can_transition_to(SessionActive));
        assert!(SessionActive.can_transition_to(OfferPending));
        assert!(OfferPending.can_transition_to(OfferAccepted));
        assert!(OfferAccepted.can_transition_to(Transferring));
        assert!(Transferring.can_transition_to(Verifying));
        assert!(Verifying.can_transition_to(Completed));
    }

    #[test]
    fn test_invalid_transitions() {
        use ProtocolState::*;
        assert!(!Idle.can_transition_to(Transferring));
        assert!(!Discovering.can_transition_to(Completed));
        assert!(!Pairing.can_transition_to(Transferring));
        assert!(!Completed.can_transition_to(Transferring));
    }

    #[test]
    fn test_pause_resume() {
        use ProtocolState::*;
        assert!(Transferring.can_transition_to(Paused));
        assert!(Paused.can_transition_to(Transferring));
    }

    #[test]
    fn test_reconnection() {
        use ProtocolState::*;
        assert!(Transferring.can_transition_to(Reconnecting));
        assert!(Reconnecting.can_transition_to(Transferring));
        assert!(Reconnecting.can_transition_to(Error));
    }

    #[test]
    fn test_cancellation() {
        use ProtocolState::*;
        assert!(Transferring.can_transition_to(Cancelled));
        assert!(Paused.can_transition_to(Cancelled));
        assert!(OfferPending.can_transition_to(Cancelled));
    }

    #[test]
    fn test_terminal_states() {
        assert!(ProtocolState::Completed.is_terminal());
        assert!(ProtocolState::Cancelled.is_terminal());
        assert!(ProtocolState::Error.is_terminal());
        assert!(!ProtocolState::Transferring.is_terminal());
        assert!(!ProtocolState::Idle.is_terminal());
    }

    #[test]
    fn test_active_states() {
        assert!(ProtocolState::Transferring.is_active());
        assert!(ProtocolState::Paused.is_active());
        assert!(ProtocolState::Reconnecting.is_active());
        assert!(!ProtocolState::Idle.is_active());
        assert!(!ProtocolState::Completed.is_active());
    }

    #[test]
    fn test_display() {
        assert_eq!(ProtocolState::Transferring.to_string(), "Transferring");
        assert_eq!(ProtocolState::SessionActive.to_string(), "Session Active");
    }

    #[test]
    fn test_serialization() {
        let state = ProtocolState::Transferring;
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: ProtocolState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, deserialized);
    }
}
