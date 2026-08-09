//! Media Capture & A/V Sync Pipeline
//!
//! Platform-independent capture interfaces and A/V synchronization logic.
//! Real platform capture (Android CameraX, iOS AVFoundation, Windows MediaCapture)
//! plugs in via the trait interfaces. Validated with synthetic sources in software.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Capture device info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureDevice {
    pub device_id: String,
    pub name: String,
    pub device_type: CaptureDeviceType,
    pub capabilities: Vec<String>,
}

/// Type of capture device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureDeviceType {
    Camera,
    Microphone,
    Screen,
}

impl std::fmt::Display for CaptureDeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Camera => write!(f, "Camera"),
            Self::Microphone => write!(f, "Microphone"),
            Self::Screen => write!(f, "Screen"),
        }
    }
}

/// Capture configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    /// Video width (0 = audio only).
    pub width: u32,
    /// Video height.
    pub height: u32,
    /// Target FPS.
    pub fps: u32,
    /// Audio sample rate (0 = video only).
    pub sample_rate: u32,
    /// Audio channels.
    pub channels: u16,
    /// Video bitrate in bps.
    pub video_bitrate: u32,
    /// Audio bitrate in bps.
    pub audio_bitrate: u32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            fps: 30,
            sample_rate: 48000,
            channels: 1,
            video_bitrate: 2_000_000,
            audio_bitrate: 128_000,
        }
    }
}

impl CaptureConfig {
    pub fn audio_only() -> Self {
        Self {
            width: 0,
            height: 0,
            fps: 0,
            sample_rate: 48000,
            channels: 1,
            video_bitrate: 0,
            audio_bitrate: 128_000,
        }
    }

    pub fn video_only_720p() -> Self {
        Self {
            width: 1280,
            height: 720,
            fps: 30,
            sample_rate: 0,
            channels: 0,
            video_bitrate: 2_000_000,
            audio_bitrate: 0,
        }
    }

    pub fn video_only_1080p() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 30,
            sample_rate: 0,
            channels: 0,
            video_bitrate: 4_000_000,
            audio_bitrate: 0,
        }
    }
}

/// Capture state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureState {
    Idle,
    Starting,
    Capturing,
    Paused,
    Stopping,
    Stopped,
    Error,
}

/// A/V synchronization buffer.
///
/// Aligns video and audio frames by presentation timestamp (PTS)
/// to ensure lips match speech, etc.
pub struct AvSyncBuffer {
    /// Video frames indexed by PTS (microseconds).
    video_frames: BTreeMap<u64, SyncFrame>,
    /// Audio frames indexed by PTS (microseconds).
    audio_frames: BTreeMap<u64, SyncFrame>,
    /// Maximum acceptable drift between audio and video in microseconds.
    max_drift_us: u64,
    /// Current playback position (PTS in microseconds).
    playback_pos_us: u64,
    /// Total frames synced.
    frames_synced: u64,
    /// Total frames dropped due to sync issues.
    frames_dropped: u64,
}

/// A synchronized media frame.
#[derive(Debug, Clone)]
pub struct SyncFrame {
    pub pts_us: u64,
    pub is_video: bool,
    pub data: Vec<u8>,
}

/// A synchronized pair of video + audio output.
#[derive(Debug, Clone)]
pub struct SyncedOutput {
    pub video: Option<SyncFrame>,
    pub audio: Option<SyncFrame>,
    pub drift_us: i64,
}

impl AvSyncBuffer {
    /// Create a new A/V sync buffer.
    /// `max_drift_us`: maximum acceptable A/V drift in microseconds (default: 40ms).
    pub fn new(max_drift_us: u64) -> Self {
        Self {
            video_frames: BTreeMap::new(),
            audio_frames: BTreeMap::new(),
            max_drift_us,
            playback_pos_us: 0,
            frames_synced: 0,
            frames_dropped: 0,
        }
    }

    /// Push a video frame.
    pub fn push_video(&mut self, pts_us: u64, data: Vec<u8>) {
        self.video_frames.insert(
            pts_us,
            SyncFrame {
                pts_us,
                is_video: true,
                data,
            },
        );
    }

    /// Push an audio frame.
    pub fn push_audio(&mut self, pts_us: u64, data: Vec<u8>) {
        self.audio_frames.insert(
            pts_us,
            SyncFrame {
                pts_us,
                is_video: false,
                data,
            },
        );
    }

    /// Pop the next synchronized output pair.
    ///
    /// Matches the earliest video frame with the nearest audio frame.
    /// Drops frames that are too far out of sync.
    pub fn pop_synced(&mut self) -> Option<SyncedOutput> {
        // Get earliest video frame
        let video_entry = self
            .video_frames
            .iter()
            .next()
            .map(|(&k, v)| (k, v.clone()));
        let audio_entry = self
            .audio_frames
            .iter()
            .next()
            .map(|(&k, v)| (k, v.clone()));

        match (video_entry, audio_entry) {
            (Some((v_pts, video)), Some((a_pts, audio))) => {
                let drift = v_pts as i64 - a_pts as i64;

                if drift.unsigned_abs() <= self.max_drift_us {
                    // In sync — emit both
                    self.video_frames.remove(&v_pts);
                    self.audio_frames.remove(&a_pts);
                    self.frames_synced += 2;
                    self.playback_pos_us = v_pts.max(a_pts);
                    Some(SyncedOutput {
                        video: Some(video),
                        audio: Some(audio),
                        drift_us: drift,
                    })
                } else if drift > 0 {
                    // Audio is ahead — emit audio only, drop video later
                    self.audio_frames.remove(&a_pts);
                    self.frames_synced += 1;
                    Some(SyncedOutput {
                        video: None,
                        audio: Some(audio),
                        drift_us: drift,
                    })
                } else {
                    // Video is ahead — emit video only, drop audio later
                    self.video_frames.remove(&v_pts);
                    self.frames_synced += 1;
                    Some(SyncedOutput {
                        video: Some(video),
                        audio: None,
                        drift_us: drift,
                    })
                }
            }
            (Some((v_pts, video)), None) => {
                self.video_frames.remove(&v_pts);
                self.frames_synced += 1;
                Some(SyncedOutput {
                    video: Some(video),
                    audio: None,
                    drift_us: 0,
                })
            }
            (None, Some((a_pts, audio))) => {
                self.audio_frames.remove(&a_pts);
                self.frames_synced += 1;
                Some(SyncedOutput {
                    video: None,
                    audio: Some(audio),
                    drift_us: 0,
                })
            }
            (None, None) => None,
        }
    }

    /// Drop frames older than the given PTS threshold.
    pub fn drop_old_frames(&mut self, threshold_us: u64) {
        let old_video: Vec<u64> = self
            .video_frames
            .range(..threshold_us)
            .map(|(&k, _)| k)
            .collect();
        for k in old_video {
            self.video_frames.remove(&k);
            self.frames_dropped += 1;
        }
        let old_audio: Vec<u64> = self
            .audio_frames
            .range(..threshold_us)
            .map(|(&k, _)| k)
            .collect();
        for k in old_audio {
            self.audio_frames.remove(&k);
            self.frames_dropped += 1;
        }
    }

    /// Get sync statistics.
    pub fn stats(&self) -> AvSyncStats {
        AvSyncStats {
            video_buffered: self.video_frames.len(),
            audio_buffered: self.audio_frames.len(),
            frames_synced: self.frames_synced,
            frames_dropped: self.frames_dropped,
            playback_pos_us: self.playback_pos_us,
        }
    }

    /// Check if buffers are empty.
    pub fn is_empty(&self) -> bool {
        self.video_frames.is_empty() && self.audio_frames.is_empty()
    }
}

/// A/V sync statistics.
#[derive(Debug, Clone, Default)]
pub struct AvSyncStats {
    pub video_buffered: usize,
    pub audio_buffered: usize,
    pub frames_synced: u64,
    pub frames_dropped: u64,
    pub playback_pos_us: u64,
}

/// Enumerate available capture devices (platform-dependent).
/// Returns empty in software-only mode.
pub fn enumerate_devices() -> Vec<CaptureDevice> {
    log::info!("Enumerating capture devices (software-only: returns empty)");
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_config_defaults() {
        let cfg = CaptureConfig::default();
        assert_eq!(cfg.width, 1280);
        assert_eq!(cfg.fps, 30);
        assert_eq!(cfg.sample_rate, 48000);
    }

    #[test]
    fn test_capture_config_presets() {
        let audio = CaptureConfig::audio_only();
        assert_eq!(audio.width, 0);
        assert!(audio.sample_rate > 0);

        let v720 = CaptureConfig::video_only_720p();
        assert_eq!(v720.width, 1280);
        assert_eq!(v720.height, 720);

        let v1080 = CaptureConfig::video_only_1080p();
        assert_eq!(v1080.width, 1920);
    }

    #[test]
    fn test_capture_device_type_display() {
        assert_eq!(CaptureDeviceType::Camera.to_string(), "Camera");
        assert_eq!(CaptureDeviceType::Microphone.to_string(), "Microphone");
        assert_eq!(CaptureDeviceType::Screen.to_string(), "Screen");
    }

    #[test]
    fn test_av_sync_in_sync() {
        let mut buf = AvSyncBuffer::new(40_000); // 40ms tolerance

        // Push perfectly synced frames
        buf.push_video(0, vec![0x01]);
        buf.push_audio(0, vec![0x02]);

        let out = buf.pop_synced().unwrap();
        assert!(out.video.is_some());
        assert!(out.audio.is_some());
        assert_eq!(out.drift_us, 0);
    }

    #[test]
    fn test_av_sync_small_drift() {
        let mut buf = AvSyncBuffer::new(40_000);

        // 10ms drift — within tolerance
        buf.push_video(100_000, vec![0x01]);
        buf.push_audio(110_000, vec![0x02]);

        let out = buf.pop_synced().unwrap();
        assert!(out.video.is_some());
        assert!(out.audio.is_some());
        assert_eq!(out.drift_us, -10_000); // video behind audio
    }

    #[test]
    fn test_av_sync_large_drift() {
        let mut buf = AvSyncBuffer::new(40_000);

        // 100ms drift — out of tolerance (video@0 is earlier, so video emitted first)
        buf.push_video(0, vec![0x01]);
        buf.push_audio(100_000, vec![0x02]);

        let out = buf.pop_synced().unwrap();
        assert!(out.video.is_some());
        assert!(out.audio.is_none());
    }

    #[test]
    fn test_av_sync_drop_old() {
        let mut buf = AvSyncBuffer::new(40_000);
        buf.push_video(1000, vec![0x01]);
        buf.push_video(2000, vec![0x02]);
        buf.push_audio(1500, vec![0x03]);

        buf.drop_old_frames(1500);
        let stats = buf.stats();
        assert_eq!(stats.frames_dropped, 1); // video@1000 (< 1500) dropped
    }

    #[test]
    fn test_av_sync_sequential_playback() {
        let mut buf = AvSyncBuffer::new(40_000);

        // Simulate 3 frames at 33ms intervals (30fps)
        for i in 0..3 {
            let pts = i * 33_333;
            buf.push_video(pts, vec![i as u8]);
            buf.push_audio(pts + 1000, vec![i as u8 + 10]); // 1ms drift
        }

        let mut synced = 0;
        while let Some(out) = buf.pop_synced() {
            if out.video.is_some() && out.audio.is_some() {
                assert!(out.drift_us.unsigned_abs() < 40_000);
                synced += 1;
            }
        }
        assert_eq!(synced, 3);
    }

    #[test]
    fn test_av_sync_empty() {
        let buf = AvSyncBuffer::new(40_000);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_enumerate_devices_software() {
        let devices = enumerate_devices();
        assert!(devices.is_empty()); // No real hardware
    }
}
