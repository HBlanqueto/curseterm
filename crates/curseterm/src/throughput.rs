//! iTermThroughputEstimator.m: bytes per second over a short history of buckets, the newest
//! weighing most.

use std::time::{Duration, Instant};

// TokenExecutor.swift: five buckets of 1/30 s.
const BUCKETS: usize = 5;
const BUCKET: Duration = Duration::from_nanos(1_000_000_000 / 30);

#[derive(Clone, Debug)]
pub struct Throughput {
    start: Instant,
    last_index: u64,
    buckets: [f64; BUCKETS],
}

impl Default for Throughput {
    fn default() -> Self {
        Self {
            start: Instant::now(),
            last_index: 0,
            buckets: [0.0; BUCKETS],
        }
    }
}

impl Throughput {
    pub fn add(&mut self, bytes: usize, now: Instant) {
        self.erase_old(now);
        self.buckets[BUCKETS - 1] += bytes as f64;
    }

    pub fn estimate(&mut self, now: Instant) -> f64 {
        let elapsed = self.erase_old(now);
        let in_current = elapsed.as_secs_f64() % BUCKET.as_secs_f64();
        // The current bucket counts in proportion to the time it has had, capped at ten.
        let current_weight = (BUCKET.as_secs_f64() / in_current).min(10.0);
        let (mut sum, mut weight) = (0.0, 1.0);
        for (index, value) in self.buckets.iter().enumerate() {
            let value = if index == BUCKETS - 1 {
                value * current_weight
            } else {
                *value
            };
            sum += value * weight;
            weight *= 2.0;
        }
        sum / (weight - 1.0) / BUCKET.as_secs_f64()
    }

    fn erase_old(&mut self, now: Instant) -> Duration {
        let elapsed = now.saturating_duration_since(self.start);
        let index = (elapsed.as_nanos() / BUCKET.as_nanos()) as u64;
        let stale = (index.saturating_sub(self.last_index) as usize).min(BUCKETS);
        self.last_index = index;
        self.buckets.rotate_left(stale);
        for bucket in &mut self.buckets[BUCKETS - stale..] {
            *bucket = 0.0;
        }
        elapsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_steady_stream_is_estimated_at_its_rate_and_silence_falls_to_zero() {
        let mut throughput = Throughput::default();
        let start = throughput.start;
        for step in 0..30u32 {
            throughput.add(1_000, start + BUCKET * step + BUCKET / 2);
        }
        let estimate = throughput.estimate(start + BUCKET * 30 - Duration::from_nanos(1));
        assert!((29_000.0..31_000.0).contains(&estimate), "{estimate}");
        assert_eq!(throughput.estimate(start + BUCKET * 40), 0.0);
    }
}
