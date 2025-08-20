use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const HISTORY_SIZE: usize = 1000;
const PERCENTILES: &[f64] = &[0.5, 0.9, 0.95, 0.99, 0.999];

#[derive(Debug, Clone, Copy)]
pub struct AllocationStats {
    pub total_allocations: u64,
    pub total_deallocations: u64,
    pub current_allocated_bytes: usize,
    pub peak_allocated_bytes: usize,
    pub allocation_rate: f64,
    pub deallocation_rate: f64,
    pub fragmentation_ratio: f64,
    pub latency_stats: LatencyStats,
}

#[derive(Debug, Clone, Copy)]
pub struct LatencyStats {
    pub mean_ns: f64,
    pub median_ns: f64,
    pub p90_ns: f64,
    pub p95_ns: f64,
    pub p99_ns: f64,
    pub p999_ns: f64,
    pub min_ns: u64,
    pub max_ns: u64,
}

#[derive(Debug)]
pub struct MemoryStats {
    allocations: AtomicU64,
    deallocations: AtomicU64,
    allocated_bytes: AtomicUsize,
    peak_bytes: AtomicUsize,
    failed_allocations: AtomicU64,

    latency_history: RwLock<LatencyTracker>,
    allocation_sizes: RwLock<SizeDistribution>,

    start_time: Instant,
    last_update: RwLock<Instant>,
}

#[derive(Debug)]
struct LatencyTracker {
    samples: VecDeque<u64>,
    sorted_cache: Vec<u64>,
    cache_valid: bool,
}

impl LatencyTracker {
    fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(HISTORY_SIZE),
            sorted_cache: Vec::with_capacity(HISTORY_SIZE),
            cache_valid: false,
        }
    }

    fn record(&mut self, latency_ns: u64) {
        if self.samples.len() >= HISTORY_SIZE {
            // Remove oldest sample to maintain history size
            if let Some(old_sample) = self.samples.pop_front() {
                // Could track min/max being removed for statistics
                if old_sample == *self.samples.iter().min().unwrap_or(&0)
                    || old_sample == *self.samples.iter().max().unwrap_or(&0)
                {
                    self.cache_valid = false; // Force recalculation if min/max changed
                }
            }
        }
        self.samples.push_back(latency_ns);
        self.cache_valid = false;
    }

    fn get_percentile(&mut self, percentile: f64) -> u64 {
        if self.samples.is_empty() {
            return 0;
        }

        if !self.cache_valid {
            self.sorted_cache.clear();
            self.sorted_cache.extend(self.samples.iter());
            self.sorted_cache.sort_unstable();
            self.cache_valid = true;
        }

        let index = ((self.sorted_cache.len() as f64 - 1.0) * percentile) as usize;
        self.sorted_cache[index]
    }

    fn get_stats(&mut self) -> LatencyStats {
        if self.samples.is_empty() {
            return LatencyStats {
                mean_ns: 0.0,
                median_ns: 0.0,
                p90_ns: 0.0,
                p95_ns: 0.0,
                p99_ns: 0.0,
                p999_ns: 0.0,
                min_ns: 0,
                max_ns: 0,
            };
        }

        let sum: u64 = self.samples.iter().sum();
        let mean = sum as f64 / self.samples.len() as f64;

        LatencyStats {
            mean_ns: mean,
            median_ns: self.get_percentile(PERCENTILES[0]) as f64, // 0.5
            p90_ns: self.get_percentile(PERCENTILES[1]) as f64,    // 0.9
            p95_ns: self.get_percentile(PERCENTILES[2]) as f64,    // 0.95
            p99_ns: self.get_percentile(PERCENTILES[3]) as f64,    // 0.99
            p999_ns: self.get_percentile(PERCENTILES[4]) as f64,   // 0.999
            min_ns: *self.samples.iter().min().unwrap_or(&0),
            max_ns: *self.samples.iter().max().unwrap_or(&0),
        }
    }
}

#[derive(Debug)]
struct SizeDistribution {
    buckets: Vec<SizeBucket>,
    total_count: u64,
}

#[derive(Debug)]
struct SizeBucket {
    min_size: usize,
    max_size: usize,
    count: u64,
    total_bytes: u64,
}

impl SizeDistribution {
    fn new() -> Self {
        let buckets = vec![
            SizeBucket::new(0, 64),
            SizeBucket::new(65, 256),
            SizeBucket::new(257, 1024),
            SizeBucket::new(1025, 4096),
            SizeBucket::new(4097, 16384),
            SizeBucket::new(16385, 65536),
            SizeBucket::new(65537, 262144),
            SizeBucket::new(262145, usize::MAX),
        ];

        Self {
            buckets,
            total_count: 0,
        }
    }

    fn record(&mut self, size: usize) {
        for bucket in &mut self.buckets {
            if size >= bucket.min_size && size <= bucket.max_size {
                bucket.count += 1;
                bucket.total_bytes += size as u64;
                break;
            }
        }
        self.total_count += 1;
    }

    fn get_distribution(&self) -> Vec<(String, f64, u64)> {
        if self.total_count == 0 {
            return Vec::new();
        }

        self.buckets
            .iter()
            .filter(|b| b.count > 0)
            .map(|bucket| {
                let range = if bucket.max_size == usize::MAX {
                    format!("{}+", Self::format_size(bucket.min_size))
                } else {
                    format!(
                        "{}-{}",
                        Self::format_size(bucket.min_size),
                        Self::format_size(bucket.max_size)
                    )
                };

                let percentage = (bucket.count as f64 / self.total_count as f64) * 100.0;

                (range, percentage, bucket.total_bytes)
            })
            .collect()
    }

    fn format_size(size: usize) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
        let mut size = size as f64;
        let mut unit_idx = 0;

        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }

        if size.fract() == 0.0 {
            format!("{:.0}{}", size, UNITS[unit_idx])
        } else {
            format!("{:.1}{}", size, UNITS[unit_idx])
        }
    }
}

impl SizeBucket {
    fn new(min: usize, max: usize) -> Self {
        Self {
            min_size: min,
            max_size: max,
            count: 0,
            total_bytes: 0,
        }
    }
}

impl MemoryStats {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            allocations: AtomicU64::new(0),
            deallocations: AtomicU64::new(0),
            allocated_bytes: AtomicUsize::new(0),
            peak_bytes: AtomicUsize::new(0),
            failed_allocations: AtomicU64::new(0),
            latency_history: RwLock::new(LatencyTracker::new()),
            allocation_sizes: RwLock::new(SizeDistribution::new()),
            start_time: now,
            last_update: RwLock::new(now),
        }
    }

    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn time_since_last_update(&self) -> Duration {
        self.last_update.read().elapsed()
    }

    pub fn record_allocation(&self, size: usize, latency_ns: u64) {
        let prev_allocations = self.allocations.fetch_add(1, Ordering::Relaxed);
        let current = self.allocated_bytes.fetch_add(size, Ordering::Relaxed) + size;

        // Track allocation count for potential overflow detection
        if prev_allocations == u64::MAX {
            tracing::warn!("Allocation counter overflow detected");
        }

        let mut peak = self.peak_bytes.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_bytes.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }

        self.latency_history.write().record(latency_ns);
        self.allocation_sizes.write().record(size);
        *self.last_update.write() = Instant::now();
    }

    pub fn record_deallocation(&self, size: usize) {
        let prev_deallocations = self.deallocations.fetch_add(1, Ordering::Relaxed);
        let prev_bytes = self.allocated_bytes.fetch_sub(size, Ordering::Relaxed);

        // Detect potential underflow or mismatched deallocation
        if prev_bytes < size {
            tracing::error!(
                "Memory deallocation underflow: tried to deallocate {} bytes but only {} were allocated",
                size,
                prev_bytes
            );
        }
        if prev_deallocations == u64::MAX {
            tracing::warn!("Deallocation counter overflow detected");
        }
        *self.last_update.write() = Instant::now();
    }

    pub fn record_failed_allocation(&self) {
        let prev_failures = self.failed_allocations.fetch_add(1, Ordering::Relaxed);

        // Alert on high failure rate
        if prev_failures > 0 && prev_failures % 1000 == 0 {
            tracing::warn!(
                "High allocation failure rate: {} failures recorded",
                prev_failures + 1
            );
        }
    }

    pub fn get_snapshot(&self) -> AllocationStats {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let allocations = self.allocations.load(Ordering::Relaxed);
        let deallocations = self.deallocations.load(Ordering::Relaxed);

        AllocationStats {
            total_allocations: allocations,
            total_deallocations: deallocations,
            current_allocated_bytes: self.allocated_bytes.load(Ordering::Relaxed),
            peak_allocated_bytes: self.peak_bytes.load(Ordering::Relaxed),
            allocation_rate: allocations as f64 / elapsed,
            deallocation_rate: deallocations as f64 / elapsed,
            fragmentation_ratio: self.calculate_fragmentation(),
            latency_stats: self.latency_history.write().get_stats(),
        }
    }

    fn calculate_fragmentation(&self) -> f64 {
        let current = self.allocated_bytes.load(Ordering::Relaxed);
        let peak = self.peak_bytes.load(Ordering::Relaxed);

        if peak == 0 {
            0.0
        } else {
            1.0 - (current as f64 / peak as f64)
        }
    }

    pub fn get_size_distribution(&self) -> Vec<(String, f64, u64)> {
        self.allocation_sizes.read().get_distribution()
    }

    pub fn reset(&self) {
        self.allocations.store(0, Ordering::Relaxed);
        self.deallocations.store(0, Ordering::Relaxed);
        self.allocated_bytes.store(0, Ordering::Relaxed);
        self.peak_bytes.store(0, Ordering::Relaxed);
        self.failed_allocations.store(0, Ordering::Relaxed);

        *self.latency_history.write() = LatencyTracker::new();
        *self.allocation_sizes.write() = SizeDistribution::new();
        *self.last_update.write() = Instant::now();
    }
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AllocationTimer {
    start: Instant,
}

impl AllocationTimer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed_ns(&self) -> u64 {
        self.start.elapsed().as_nanos() as u64
    }
}
