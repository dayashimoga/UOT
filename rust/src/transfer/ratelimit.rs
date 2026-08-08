//! Rate Limiter / Token Bucket Bandwidth Throttler
//!
//! Controls byte transfer rate to prevent network saturation.
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Token bucket rate limiter.
pub struct RateLimiter {
    bytes_per_sec: u64,
    tokens: f64,
    last_update: Instant,
}

impl RateLimiter {
    /// Create a rate limiter with specified max bytes/sec (0 = unlimited).
    pub fn new(bytes_per_sec: u64) -> Self {
        Self {
            bytes_per_sec,
            tokens: bytes_per_sec as f64,
            last_update: Instant::now(),
        }
    }

    /// Consume bytes, sleeping if rate limit exceeded.
    pub async fn consume(&mut self, amount: usize) {
        if self.bytes_per_sec == 0 {
            return;
        }

        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;

        self.tokens += elapsed * (self.bytes_per_sec as f64);
        if self.tokens > (self.bytes_per_sec as f64) {
            self.tokens = self.bytes_per_sec as f64;
        }

        self.tokens -= amount as f64;
        if self.tokens < 0.0 {
            let wait_secs = (-self.tokens) / (self.bytes_per_sec as f64);
            if wait_secs > 0.001 {
                sleep(Duration::from_secs_f64(wait_secs)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_unlimited() {
        let mut limiter = RateLimiter::new(0);
        limiter.consume(1024 * 1024).await;
    }

    #[tokio::test]
    async fn test_rate_limiter_limited() {
        let mut limiter = RateLimiter::new(10_000_000); // 10 MB/s
        limiter.consume(1000).await;
    }
}
