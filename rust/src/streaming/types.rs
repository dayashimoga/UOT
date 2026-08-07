//! Streaming Types
//!
//! Types for media streaming capabilities, configuration, and status.
use serde::{Deserialize, Serialize};

/// A specific streaming capability.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StreamCapability {
    /// Video file streaming (local playback on remote).
    VideoFile,
    /// Audio file streaming.
    AudioFile,
    /// Live camera streaming.
    Camera,
    /// Screen capture streaming.
    ScreenCapture,
    /// Microphone streaming.
    Microphone,
}

impl std::fmt::Display for StreamCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VideoFile => write!(f, "Video File"),
            Self::AudioFile => write!(f, "Audio File"),
            Self::Camera => write!(f, "Camera"),
            Self::ScreenCapture => write!(f, "Screen Capture"),
            Self::Microphone => write!(f, "Microphone"),
        }
    }
}

/// Configuration for a streaming session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Desired video resolution width.
    pub width: u32,
    /// Desired video resolution height.
    pub height: u32,
    /// Target frames per second.
    pub fps: u32,
    /// Target video bitrate in kbps.
    pub video_bitrate_kbps: u32,
    /// Target audio bitrate in kbps.
    pub audio_bitrate_kbps: u32,
    /// Whether to enable adaptive quality.
    pub adaptive_quality: bool,
    /// Buffer size in milliseconds.
    pub buffer_ms: u32,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            fps: 30,
            video_bitrate_kbps: 2500,
            audio_bitrate_kbps: 128,
            adaptive_quality: true,
            buffer_ms: 500,
        }
    }
}

/// Current status of a stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamStatus {
    /// No stream active.
    Idle,
    /// Buffering data.
    Buffering,
    /// Actively streaming.
    Playing,
    /// Stream paused.
    Paused,
    /// Stream encountered an error.
    Error,
    /// Stream ended.
    Ended,
}

impl std::fmt::Display for StreamStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Buffering => write!(f, "Buffering…"),
            Self::Playing => write!(f, "Playing"),
            Self::Paused => write!(f, "Paused"),
            Self::Error => write!(f, "Error"),
            Self::Ended => write!(f, "Ended"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_capability_display() {
        assert_eq!(StreamCapability::Camera.to_string(), "Camera");
        assert_eq!(
            StreamCapability::ScreenCapture.to_string(),
            "Screen Capture"
        );
        assert_eq!(StreamCapability::VideoFile.to_string(), "Video File");
    }

    #[test]
    fn test_stream_config_default() {
        let config = StreamConfig::default();
        assert_eq!(config.width, 1280);
        assert_eq!(config.height, 720);
        assert_eq!(config.fps, 30);
        assert!(config.adaptive_quality);
    }

    #[test]
    fn test_stream_status_display() {
        assert_eq!(StreamStatus::Playing.to_string(), "Playing");
        assert_eq!(StreamStatus::Buffering.to_string(), "Buffering…");
    }

    #[test]
    fn test_stream_config_serialization() {
        let config = StreamConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: StreamConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.width, deserialized.width);
        assert_eq!(config.fps, deserialized.fps);
    }
}
