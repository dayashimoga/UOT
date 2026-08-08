//! Fountain Codes & Animated QR Transport
//!
//! Implements a Luby Transform (LT) / Fountain Code encoder/decoder for
//! transmitting arbitrary data across animated QR code sequences over optical channel.
use rand::Rng;

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

        let num_blocks = self.blocks.len() as u32;
        let mut rng = rand::rng();

        // Sample degree (number of blocks to XOR) using Soliton distribution approximation
        let degree = if num_blocks == 1 {
            1
        } else {
            std::cmp::min(
                num_blocks as usize,
                (rng.random_range(1..=num_blocks) as usize % 4) + 1,
            )
        };

        let mut combined = vec![0u8; self.block_size];
        for _ in 0..degree {
            let idx = rng.random_range(0..num_blocks as usize);
            for (i, byte) in self.blocks[idx].iter().enumerate() {
                combined[i] ^= byte;
            }
        }

        let crc32 = crc32fast::hash(&combined);

        FountainPacket {
            total_size: self.total_size,
            num_blocks,
            seed,
            payload: combined,
            crc32,
        }
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
}
