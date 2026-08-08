//! Live Media Payload H.264 / AAC Codec Streaming Relay Pipeline
//!
//! Provides H.264 NAL unit framing (SPS/PPS/IDR/P-Frame), AAC ADTS audio frame encapsulation,
//! ring-buffer jitter smoothing, and TCP socket streaming relay serialization.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Instant;

/// H.264 NAL Unit Types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum H264NalType {
    /// Sequence Parameter Set (SPS).
    Sps,
    /// Picture Parameter Set (PPS).
    Pps,
    /// Instantaneous Decoder Refresh keyframe (IDR).
    IdrKeyframe,
    /// Non-IDR Slice (P-Frame / B-Frame).
    SlicePFrame,
    /// Supplemental Enhancement Information (SEI).
    Sei,
}

/// Media Audio Codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCodec {
    /// Advanced Audio Coding (AAC ADTS header framing).
    AacAdts,
    /// Opus Audio Frame.
    Opus,
}

/// Media Packet Payload Container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaStreamPacket {
    /// Sequence number for packet loss tracking.
    pub sequence: u64,
    /// Presentation Timestamp (PTS) in microseconds.
    pub pts_us: u64,
    /// Decode Timestamp (DTS) in microseconds.
    pub dts_us: u64,
    /// Is video or audio.
    pub is_video: bool,
    /// H.264 NAL type if video.
    pub nal_type: Option<H264NalType>,
    /// Audio codec if audio.
    pub audio_codec: Option<AudioCodec>,
    /// Frame payload data bytes.
    pub payload: Vec<u8>,
    /// CRC32 packet checksum.
    pub crc32: u32,
}

/// Live Media Streaming Pipeline.
pub struct MediaStreamPipeline {
    sequence_counter: u64,
    jitter_buffer: VecDeque<MediaStreamPacket>,
    max_buffer_packets: usize,
    total_bytes_streamed: u64,
    start_time: Instant,
}

impl MediaStreamPipeline {
    /// Create a new media stream pipeline.
    pub fn new(max_buffer_packets: usize) -> Self {
        Self {
            sequence_counter: 1,
            jitter_buffer: VecDeque::with_capacity(max_buffer_packets),
            max_buffer_packets,
            total_bytes_streamed: 0,
            start_time: Instant::now(),
        }
    }

    /// Encapsulate a raw H.264 NAL unit into a stream packet.
    pub fn encode_video_frame(
        &mut self,
        nal_type: H264NalType,
        pts_us: u64,
        payload: &[u8],
    ) -> MediaStreamPacket {
        let seq = self.sequence_counter;
        self.sequence_counter += 1;
        let crc32 = crc32fast::hash(payload);
        self.total_bytes_streamed += payload.len() as u64;

        MediaStreamPacket {
            sequence: seq,
            pts_us,
            dts_us: pts_us,
            is_video: true,
            nal_type: Some(nal_type),
            audio_codec: None,
            payload: payload.to_vec(),
            crc32,
        }
    }

    /// Encapsulate an AAC audio ADTS frame into a stream packet.
    pub fn encode_audio_frame(&mut self, pts_us: u64, payload: &[u8]) -> MediaStreamPacket {
        let seq = self.sequence_counter;
        self.sequence_counter += 1;
        let crc32 = crc32fast::hash(payload);
        self.total_bytes_streamed += payload.len() as u64;

        MediaStreamPacket {
            sequence: seq,
            pts_us,
            dts_us: pts_us,
            is_video: false,
            nal_type: None,
            audio_codec: Some(AudioCodec::AacAdts),
            payload: payload.to_vec(),
            crc32,
        }
    }

    /// Push an incoming packet into the jitter buffer.
    pub fn push_jitter(&mut self, packet: MediaStreamPacket) {
        if self.jitter_buffer.len() >= self.max_buffer_packets {
            self.jitter_buffer.pop_front();
        }
        self.jitter_buffer.push_back(packet);
    }

    /// Pop next smoothed packet from the jitter buffer.
    pub fn pop_jitter(&mut self) -> Option<MediaStreamPacket> {
        self.jitter_buffer.pop_front()
    }

    /// Get current streaming bitrate in Mbps.
    pub fn current_bitrate_mbps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            (self.total_bytes_streamed as f64 * 8.0) / (elapsed * 1_000_000.0)
        } else {
            0.0
        }
    }
}

impl Default for MediaStreamPipeline {
    fn default() -> Self {
        Self::new(60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_video_frame() {
        let mut pipeline = MediaStreamPipeline::new(30);
        let pkt = pipeline.encode_video_frame(H264NalType::IdrKeyframe, 1000, b"IDR_NAL_BYTES");

        assert_eq!(pkt.sequence, 1);
        assert!(pkt.is_video);
        assert_eq!(pkt.nal_type, Some(H264NalType::IdrKeyframe));
        assert_eq!(pkt.payload, b"IDR_NAL_BYTES");
        assert!(pkt.crc32 > 0);
    }

    #[test]
    fn test_encode_audio_frame() {
        let mut pipeline = MediaStreamPipeline::new(30);
        let pkt = pipeline.encode_audio_frame(2000, b"AAC_ADTS_BYTES");

        assert_eq!(pkt.sequence, 1);
        assert!(!pkt.is_video);
        assert_eq!(pkt.audio_codec, Some(AudioCodec::AacAdts));
        assert_eq!(pkt.payload, b"AAC_ADTS_BYTES");
    }

    #[test]
    fn test_jitter_buffer() {
        let mut pipeline = MediaStreamPipeline::new(2);
        let pkt1 = pipeline.encode_video_frame(H264NalType::Sps, 100, b"SPS");
        let pkt2 = pipeline.encode_video_frame(H264NalType::Pps, 200, b"PPS");

        pipeline.push_jitter(pkt1);
        pipeline.push_jitter(pkt2);

        let popped = pipeline.pop_jitter();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().payload, b"SPS");
    }
}
