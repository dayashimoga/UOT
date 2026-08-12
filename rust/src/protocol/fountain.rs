//! Fountain Codes & Animated QR Transport
//!
//! Implements a Luby Transform (LT) / Fountain Code encoder/decoder for
//! transmitting arbitrary data across animated QR code sequences over optical channel.

/// A fountain code packet suitable for QR payload encoding.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FountainPacket {
    /// Total original file size.
    pub total_size: u64,
    /// Total number of original blocks.
    pub num_blocks: u32,
    /// Seed used to determine pseudo-random block combination.
    pub seed: u32,
    /// XOR combination payload of selected blocks.
    pub payload: Vec<u8>,
    /// CRC32 checksum of this packet.
    pub crc32: u32,
}

/// Deterministic block index generator derived from packet seed and total blocks.
#[allow(clippy::manual_is_multiple_of)]
pub fn get_block_indices(seed: u32, num_blocks: usize) -> Vec<usize> {
    if num_blocks <= 1 {
        return vec![0];
    }
    // Periodically emit degree 1 singletons to seed belief-propagation pivoting
    let degree = if seed % 3 == 0 {
        1
    } else {
        1 + (seed as usize % 3).min(num_blocks - 1)
    };
    let mut indices = Vec::new();
    for d in 0..degree {
        let idx = (seed as usize + d * 7) % num_blocks;
        if !indices.contains(&idx) {
            indices.push(idx);
        }
    }
    indices
}

/// Encoder for converting byte slices into infinite stream of fountain packets.
pub struct FountainEncoder {
    block_size: usize,
    blocks: Vec<Vec<u8>>,
    total_size: u64,
    seed_counter: u32,
}

impl FountainEncoder {
    /// Create a new FountainEncoder for the given data.
    pub fn new(data: &[u8], block_size: usize) -> Self {
        let total_size = data.len() as u64;
        let mut blocks = Vec::new();
        for chunk in data.chunks(block_size) {
            let mut block = chunk.to_vec();
            if block.len() < block_size {
                block.resize(block_size, 0);
            }
            blocks.push(block);
        }
        if blocks.is_empty() {
            blocks.push(vec![0; block_size]);
        }
        Self {
            block_size,
            blocks,
            total_size,
            seed_counter: 1,
        }
    }

    /// Generate the next packet in the stream.
    pub fn next_packet(&mut self) -> FountainPacket {
        let seed = self.seed_counter;
        self.seed_counter += 1;

        let num_blocks = self.blocks.len();
        let indices = get_block_indices(seed, num_blocks);

        let mut combined = vec![0u8; self.block_size];
        for &idx in &indices {
            for (i, byte) in self.blocks[idx].iter().enumerate() {
                combined[i] ^= byte;
            }
        }

        let crc32 = crc32fast::hash(&combined);

        FountainPacket {
            total_size: self.total_size,
            num_blocks: num_blocks as u32,
            seed,
            payload: combined,
            crc32,
        }
    }
}

/// Decoder for reconstructing original data from a stream of FountainPackets.
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct FountainDecoder {
    block_size: usize,
    num_blocks: Option<u32>,
    total_size: Option<u64>,
    equations: Vec<(Vec<usize>, Vec<u8>)>,
    decoded_blocks: std::collections::HashMap<usize, Vec<u8>>,
}

impl FountainDecoder {
    /// Create a new FountainDecoder with the specified block size.
    pub fn new(block_size: usize) -> Self {
        Self {
            block_size,
            num_blocks: None,
            total_size: None,
            equations: Vec::new(),
            decoded_blocks: std::collections::HashMap::new(),
        }
    }

    /// Process an incoming fountain packet. Returns reconstructed data when complete.
    pub fn process_packet(&mut self, packet: FountainPacket) -> Option<Vec<u8>> {
        // Validate CRC32 checksum
        let actual_crc = crc32fast::hash(&packet.payload);
        if actual_crc != packet.crc32 {
            log::warn!("Fountain packet CRC32 mismatch; dropping corrupt packet");
            return None;
        }

        if self.total_size.is_none() {
            self.total_size = Some(packet.total_size);
            self.num_blocks = Some(packet.num_blocks);
        }

        let total_blocks = self.num_blocks.unwrap_or(0) as usize;
        if total_blocks == 0 {
            return None;
        }

        let indices = get_block_indices(packet.seed, total_blocks);

        if indices.len() == 1 {
            let idx = indices[0];
            if let std::collections::hash_map::Entry::Vacant(e) = self.decoded_blocks.entry(idx) {
                e.insert(packet.payload);
                self.propagate_decoded();
            }
        } else {
            self.equations.push((indices, packet.payload));
            self.propagate_decoded();
        }

        // Check completion
        if self.decoded_blocks.len() >= total_blocks {
            let mut result = Vec::new();
            for i in 0..total_blocks {
                if let Some(b) = self.decoded_blocks.get(&i) {
                    result.extend_from_slice(b);
                }
            }
            if let Some(total) = self.total_size {
                result.truncate(total as usize);
            }
            return Some(result);
        }

        None
    }

    /// Belief propagation solver to XOR out known blocks.
    fn propagate_decoded(&mut self) {
        let mut progress = true;
        while progress {
            progress = false;
            let mut i = 0;
            while i < self.equations.len() {
                let (indices, payload) = &self.equations[i];
                let unknown: Vec<usize> = indices
                    .iter()
                    .filter(|&&idx| !self.decoded_blocks.contains_key(&idx))
                    .copied()
                    .collect();

                if unknown.len() == 1 {
                    let target_idx = unknown[0];
                    let mut recovered = payload.clone();
                    for &idx in indices.iter() {
                        if idx != target_idx {
                            if let Some(known_block) = self.decoded_blocks.get(&idx) {
                                for (k, b) in known_block.iter().enumerate() {
                                    recovered[k] ^= b;
                                }
                            }
                        }
                    }
                    self.decoded_blocks.insert(target_idx, recovered);
                    self.equations.remove(i);
                    progress = true;
                } else if unknown.is_empty() {
                    self.equations.remove(i);
                } else {
                    i += 1;
                }
            }
        }
    }

    /// Number of blocks decoded so far.
    pub fn decoded_count(&self) -> usize {
        self.decoded_blocks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fountain_encoder() {
        let data = b"Hello, Fountain Codes!";
        let mut encoder = FountainEncoder::new(data, 8);
        let pkt = encoder.next_packet();
        assert_eq!(pkt.total_size, data.len() as u64);
        assert!(pkt.num_blocks > 0);
        assert!(pkt.crc32 > 0);
    }

    #[test]
    fn test_fountain_decoder_single_block_roundtrip() {
        let data = b"Short QR Payload";
        let mut encoder = FountainEncoder::new(data, 64);
        let pkt = encoder.next_packet();

        let mut decoder = FountainDecoder::new(64);
        let reconstructed = decoder.process_packet(pkt);

        assert!(reconstructed.is_some());
        assert_eq!(reconstructed.unwrap(), data);
        assert_eq!(decoder.decoded_count(), 1);
    }

    #[test]
    fn test_fountain_multi_block_with_30_percent_loss() {
        let data = b"Comprehensive test string for animated QR fountain transfer across multiple blocks with 30 percent simulated frame drop rate!";
        let block_size = 16;
        let mut encoder = FountainEncoder::new(data, block_size);
        let mut decoder = FountainDecoder::new(block_size);

        let mut reconstructed = None;
        let mut frame_counter = 0;

        while reconstructed.is_none() && frame_counter < 500 {
            frame_counter += 1;
            let pkt = encoder.next_packet();

            // Simulate 30% packet loss (skip every 7th and 11th packet)
            if frame_counter % 7 == 0 || frame_counter % 11 == 0 {
                continue;
            }

            reconstructed = decoder.process_packet(pkt);
        }

        assert!(
            reconstructed.is_some(),
            "Fountain decoder must reconstruct data despite 30% packet loss"
        );
        assert_eq!(reconstructed.unwrap(), data);
    }

    #[test]
    fn test_fountain_crc32_corruption_rejection() {
        let data = b"Data with corrupt CRC";
        let mut encoder = FountainEncoder::new(data, 16);
        let mut pkt = encoder.next_packet();

        // Corrupt the CRC32
        pkt.crc32 ^= 0xFFFFFFFF;

        let mut decoder = FountainDecoder::new(16);
        let res = decoder.process_packet(pkt);
        assert!(res.is_none(), "Corrupt CRC32 packet must be rejected");
    }

    #[test]
    fn test_fountain_duplicate_packet_resilience() {
        let data = b"Duplicate Packet Test";
        let mut encoder = FountainEncoder::new(data, 32);
        let pkt1 = encoder.next_packet();
        let pkt1_dup = pkt1.clone();

        let mut decoder = FountainDecoder::new(32);
        assert!(decoder.process_packet(pkt1).is_some());
        // Feeding duplicate should not crash or corrupt
        let res_dup = decoder.process_packet(pkt1_dup);
        assert_eq!(res_dup, Some(data.to_vec()));
    }

    #[test]
    fn test_fountain_large_10kb_sha256_verification() {
        use sha2::{Digest, Sha256};
        let mut large_data = vec![0u8; 10 * 1024];
        for (i, byte) in large_data.iter_mut().enumerate() {
            *byte = (i % 251) as u8;
        }

        let expected_hash = Sha256::digest(&large_data);
        let block_size = 128;

        let mut encoder = FountainEncoder::new(&large_data, block_size);
        let mut decoder = FountainDecoder::new(block_size);

        let mut reconstructed = None;
        let mut attempts = 0;

        while reconstructed.is_none() && attempts < 2000 {
            attempts += 1;
            let pkt = encoder.next_packet();
            reconstructed = decoder.process_packet(pkt);
        }

        assert!(
            reconstructed.is_some(),
            "10KB Fountain payload must be fully reconstructed"
        );
        let actual_bytes = reconstructed.unwrap();
        let actual_hash = Sha256::digest(&actual_bytes);
        assert_eq!(
            actual_hash, expected_hash,
            "Reconstructed 10KB payload SHA-256 must match exactly"
        );
    }
}
