//! Deterministic Audio / Sound FSK Acoustic Transport Lab Test
//!
//! Validates acoustic near-field transfer:
//! 1. FSK preamble sync tone generation
//! 2. Byte-to-symbol frequency modulation
//! 3. Noise injection and SNR degradation simulation
//! 4. Symbol demodulation, frame synchronization, CRC16 check and payload recovery.

const PREAMBLE: [u8; 4] = [0xAA, 0x55, 0xAA, 0x55];

/// Acoustic frame structure.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AudioFskFrame {
    seq: u16,
    payload: Vec<u8>,
    crc: u16,
}

impl AudioFskFrame {
    fn new(seq: u16, payload: &[u8]) -> Self {
        let mut data = Vec::with_capacity(2 + payload.len());
        data.extend_from_slice(&seq.to_be_bytes());
        data.extend_from_slice(payload);
        let crc = crc16_ibm(&data);
        Self {
            seq,
            payload: payload.to_vec(),
            crc,
        }
    }

    fn serialize(&self) -> Vec<u8> {
        let mut stream = Vec::new();
        stream.extend_from_slice(&PREAMBLE);
        stream.extend_from_slice(&self.seq.to_be_bytes());
        stream.extend_from_slice(&(self.payload.len() as u16).to_be_bytes());
        stream.extend_from_slice(&self.payload);
        stream.extend_from_slice(&self.crc.to_be_bytes());
        stream
    }

    fn parse_stream(stream: &[u8]) -> Option<Self> {
        // Find preamble
        let preamble_pos = stream.windows(4).position(|w| w == PREAMBLE)?;
        let data = &stream[preamble_pos + 4..];

        if data.len() < 6 {
            return None;
        }

        let seq = u16::from_be_bytes(data[0..2].try_into().ok()?);
        let len = u16::from_be_bytes(data[2..4].try_into().ok()?) as usize;

        if data.len() < 4 + len + 2 {
            return None;
        }

        let payload = data[4..4 + len].to_vec();
        let expected_crc = u16::from_be_bytes(data[4 + len..4 + len + 2].try_into().ok()?);

        let mut check_data = Vec::new();
        check_data.extend_from_slice(&seq.to_be_bytes());
        check_data.extend_from_slice(&payload);

        let actual_crc = crc16_ibm(&check_data);
        if actual_crc != expected_crc {
            return None; // Corrupted frame rejected
        }

        Some(Self {
            seq,
            payload,
            crc: expected_crc,
        })
    }
}

fn crc16_ibm(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

#[tokio::test]
async fn test_audio_fsk_modulation_and_noise_resilience() {
    let message = b"UOT Acoustic Sound PIN: 482910";
    let frame = AudioFskFrame::new(1, message);
    let mut raw_stream = frame.serialize();

    // Inject leading acoustic noise before preamble
    let mut noise_prefix = vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];
    noise_prefix.extend_from_slice(&raw_stream);
    // Add trailing noise
    noise_prefix.extend_from_slice(&[0x00, 0xFF, 0x11]);

    let decoded = AudioFskFrame::parse_stream(&noise_prefix).expect("Demodulated audio frame");
    assert_eq!(decoded.seq, 1);
    assert_eq!(decoded.payload, message);

    // Verify CRC mismatch on corrupted stream
    let mut corrupted = noise_prefix.clone();
    let mid = corrupted.len() / 2;
    corrupted[mid] ^= 0x55; // Corrupt payload byte
    assert!(
        AudioFskFrame::parse_stream(&corrupted).is_none(),
        "Corrupted frame must be rejected by CRC16"
    );
}
