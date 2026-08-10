//! Streaming Module — Video/Audio/Camera/Screen Streaming
//!
//! Manages local media streaming between devices over TCP.
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Stream types supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamType {
    /// Camera feed.
    Camera,
    /// Screen capture.
    Screen,
    /// Local video file.
    Video,
    /// Local audio file.
    Audio,
}

impl std::fmt::Display for StreamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Camera => write!(f, "Camera"),
            Self::Screen => write!(f, "Screen"),
            Self::Video => write!(f, "Video"),
            Self::Audio => write!(f, "Audio"),
        }
    }
}

/// Stream state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamState {
    Idle,
    Starting,
    Streaming,
    Paused,
    Stopping,
    Error,
}

/// Stream session information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSession {
    /// Session ID.
    pub session_id: String,
    /// Stream type.
    pub stream_type: StreamType,
    /// Current state.
    pub state: StreamState,
    /// Remote device ID.
    pub remote_device_id: String,
    /// Remote device name.
    pub remote_device_name: String,
    /// Port used for streaming.
    pub port: u16,
    /// Whether we are the sender or receiver.
    pub is_sender: bool,
    /// Bytes streamed so far.
    pub bytes_streamed: u64,
    /// Duration in seconds.
    pub duration_secs: f64,
}

/// Stream manager handles active streaming sessions.
pub struct StreamManager {
    /// Active sessions.
    sessions: Arc<RwLock<Vec<StreamSession>>>,
}

impl StreamManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Start a new streaming session.
    pub fn start_session(
        &self,
        stream_type: StreamType,
        remote_device_id: &str,
        remote_device_name: &str,
        port: u16,
        is_sender: bool,
    ) -> String {
        let session = StreamSession {
            session_id: Uuid::new_v4().to_string(),
            stream_type,
            state: StreamState::Starting,
            remote_device_id: remote_device_id.to_string(),
            remote_device_name: remote_device_name.to_string(),
            port,
            is_sender,
            bytes_streamed: 0,
            duration_secs: 0.0,
        };
        let id = session.session_id.clone();
        self.sessions.write().push(session);
        id
    }

    /// Update session state.
    pub fn update_state(&self, session_id: &str, state: StreamState) {
        let mut sessions = self.sessions.write();
        if let Some(s) = sessions.iter_mut().find(|s| s.session_id == session_id) {
            s.state = state;
        }
    }

    /// Update stream stats.
    pub fn update_stats(&self, session_id: &str, bytes: u64, duration: f64) {
        let mut sessions = self.sessions.write();
        if let Some(s) = sessions.iter_mut().find(|s| s.session_id == session_id) {
            s.bytes_streamed = bytes;
            s.duration_secs = duration;
        }
    }

    /// Stop a session.
    pub fn stop_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write();
        if let Some(s) = sessions.iter_mut().find(|s| s.session_id == session_id) {
            s.state = StreamState::Stopping;
        }
    }

    /// Remove a session.
    pub fn remove_session(&self, session_id: &str) {
        self.sessions.write().retain(|s| s.session_id != session_id);
    }

    /// Get all active sessions.
    pub fn active_sessions(&self) -> Vec<StreamSession> {
        self.sessions.read().clone()
    }

    /// Get a specific session.
    pub fn get_session(&self, session_id: &str) -> Option<StreamSession> {
        self.sessions
            .read()
            .iter()
            .find(|s| s.session_id == session_id)
            .cloned()
    }
}

impl Default for StreamManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_manager_inline_lifecycle() {
        let mgr = StreamManager::new();
        let session_id = mgr.start_session(
            StreamType::Camera,
            "remote-dev-123",
            "Remote Camera",
            42000,
            true,
        );

        assert_eq!(mgr.active_sessions().len(), 1);

        mgr.update_state(&session_id, StreamState::Streaming);
        mgr.update_stats(&session_id, 1048576, 30.0);

        let session = mgr.get_session(&session_id).unwrap();
        assert_eq!(session.state, StreamState::Streaming);
        assert_eq!(session.bytes_streamed, 1048576);
        assert_eq!(session.duration_secs, 30.0);

        mgr.stop_session(&session_id);
        let session_stopping = mgr.get_session(&session_id).unwrap();
        assert_eq!(session_stopping.state, StreamState::Stopping);

        mgr.remove_session(&session_id);
        assert!(mgr.active_sessions().is_empty());

        let def_mgr = StreamManager::default();
        assert!(def_mgr.active_sessions().is_empty());
    }
}
