//! Throughput & Bandwidth Benchmark Engine
//!
//! Measures real-time transfer speeds, estimates ETA, and logs bandwidth utilization.
use std::time::Instant;

/// Benchmark tracker snapshot.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkSnapshot {
    pub bytes_per_sec: u64,
    pub mbps: f64,
    pub total_bytes: u64,
    pub elapsed_secs: f64,
}

/// Dynamic speed calculator.
pub struct ThroughputBenchmark {
    start_time: Instant,
    last_sample_time: Instant,
    bytes_since_last_sample: u64,
    total_bytes: u64,
    current_speed: u64,
}

impl ThroughputBenchmark {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_sample_time: now,
            bytes_since_last_sample: 0,
            total_bytes: 0,
            current_speed: 0,
        }
    }

    /// Update with newly transferred bytes count.
    pub fn update(&mut self, bytes_added: u64) {
        self.total_bytes += bytes_added;
        self.bytes_since_last_sample += bytes_added;

        let elapsed = self.last_sample_time.elapsed();
        if elapsed.as_millis() >= 500 {
            let secs = elapsed.as_secs_f64();
            if secs > 0.0 {
                self.current_speed = (self.bytes_since_last_sample as f64 / secs) as u64;
            }
            self.bytes_since_last_sample = 0;
            self.last_sample_time = Instant::now();
        }
    }

    /// Snapshot current speed and total metrics.
    pub fn snapshot(&self) -> BenchmarkSnapshot {
        let elapsed_secs = self.start_time.elapsed().as_secs_f64();
        let mbps = (self.current_speed as f64 * 8.0) / 1_000_000.0;
        BenchmarkSnapshot {
            bytes_per_sec: self.current_speed,
            mbps,
            total_bytes: self.total_bytes,
            elapsed_secs,
        }
    }
}

impl Default for ThroughputBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_throughput_benchmark_new_and_snapshot() {
        let mut bench = ThroughputBenchmark::new();
        bench.update(1024 * 1024); // 1 MB
        let snap = bench.snapshot();
        assert_eq!(snap.total_bytes, 1024 * 1024);
        assert!(snap.elapsed_secs >= 0.0);
    }

    #[test]
    fn test_throughput_benchmark_default() {
        let bench = ThroughputBenchmark::default();
        let snap = bench.snapshot();
        assert_eq!(snap.total_bytes, 0);
        assert_eq!(snap.bytes_per_sec, 0);
        assert_eq!(snap.mbps, 0.0);
    }
}
