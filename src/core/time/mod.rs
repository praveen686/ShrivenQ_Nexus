// Precision timing for ShrivenQ
// TSC-based timing, hardware timestamps

use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub struct PrecisionTimer {
    start: Instant,
}

impl PrecisionTimer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed_nanos(&self) -> u64 {
        self.start.elapsed().as_nanos() as u64
    }

    pub fn elapsed_micros(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }
}
