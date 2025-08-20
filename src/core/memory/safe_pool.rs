// Safe memory pool implementation using only safe Rust
// No unsafe code - uses Vec for memory management

use crate::core::memory::allocator::AllocError;
use crate::core::memory::stats::{AllocationTimer, MemoryStats};
use crossbeam::queue::SegQueue;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{debug, warn};
const DEFAULT_CHUNK_SIZE: usize = 4096;
const DEFAULT_INITIAL_CHUNKS: usize = 1024;

#[derive(Clone, Copy, Debug)]
pub struct SafePoolConfig {
    pub chunk_size: usize,
    pub initial_chunks: usize,
    pub max_chunks: usize,
    pub zero_on_dealloc: bool,
}

impl Default for SafePoolConfig {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
            initial_chunks: DEFAULT_INITIAL_CHUNKS,
            max_chunks: 1_000_000,
            zero_on_dealloc: false,
        }
    }
}

// Safe memory chunk using Box<[u8]>
#[derive(Debug)]
pub struct SafeMemoryChunk {
    data: Box<[u8]>,
    _generation: u64,
}

impl SafeMemoryChunk {
    fn new(size: usize, generation: u64) -> Self {
        Self {
            data: vec![0u8; size].into_boxed_slice(),
            _generation: generation,
        }
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_mut_ptr()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

// Wrapper to provide NonNull interface while keeping memory safe
#[derive(Debug)]
pub struct SafeMemoryHandle {
    chunk: Arc<parking_lot::Mutex<SafeMemoryChunk>>,
}

impl SafeMemoryHandle {
    pub fn as_ptr(&self) -> *const u8 {
        self.chunk.lock().as_ptr()
    }

    pub fn as_mut_ptr(&self) -> *mut u8 {
        self.chunk.lock().as_mut_ptr()
    }
}

#[derive(Debug)]
pub struct SafeMemoryPool {
    config: SafePoolConfig,
    free_chunks: Arc<SegQueue<Arc<parking_lot::Mutex<SafeMemoryChunk>>>>,
    allocated_chunks: Arc<parking_lot::RwLock<Vec<Arc<parking_lot::Mutex<SafeMemoryChunk>>>>>,
    allocated_count: AtomicUsize,
    free_count: AtomicUsize,
    total_memory: AtomicUsize,
    generation: AtomicUsize,
    stats: Arc<MemoryStats>,
}

impl SafeMemoryPool {
    pub fn new(config: SafePoolConfig) -> Result<Self, AllocError> {
        if config.chunk_size == 0 {
            return Err(AllocError::InvalidLayout(
                "Chunk size must be greater than 0".to_string(),
            ));
        }

        let pool = Self {
            config,
            free_chunks: Arc::new(SegQueue::new()),
            allocated_chunks: Arc::new(parking_lot::RwLock::new(Vec::new())),
            allocated_count: AtomicUsize::new(0),
            free_count: AtomicUsize::new(0),
            total_memory: AtomicUsize::new(0),
            generation: AtomicUsize::new(0),
            stats: Arc::new(MemoryStats::new()),
        };

        pool.preallocate_chunks(config.initial_chunks)?;

        Ok(pool)
    }

    fn preallocate_chunks(&self, count: usize) -> Result<(), AllocError> {
        for _ in 0..count {
            let generation = self.generation.fetch_add(1, Ordering::Relaxed);
            let chunk = SafeMemoryChunk::new(self.config.chunk_size, generation as u64);
            let chunk_arc = Arc::new(parking_lot::Mutex::new(chunk));

            self.free_chunks.push(chunk_arc);
            let free_count = self.free_count.fetch_add(1, Ordering::Relaxed);
            let total_memory = self
                .total_memory
                .fetch_add(self.config.chunk_size, Ordering::Relaxed);

            // Log metrics for monitoring
            if free_count % 1000 == 0 {
                debug!(
                    free_chunks = free_count + 1,
                    total_bytes = total_memory + self.config.chunk_size,
                    "SafeMemoryPool pre-allocation progress"
                );
            }
        }

        Ok(())
    }

    pub fn allocate_chunk(&self) -> Result<SafeMemoryHandle, AllocError> {
        let timer = AllocationTimer::start();

        if let Some(chunk) = self.free_chunks.pop() {
            let prev_free = self.free_count.fetch_sub(1, Ordering::Relaxed);
            let prev_allocated = self.allocated_count.fetch_add(1, Ordering::Relaxed);

            // Validate pool state consistency
            if prev_free == 0 {
                warn!(
                    "SafeMemoryPool free count reached 0, this shouldn't happen during normal allocation"
                );
            }
            if prev_allocated % 10000 == 0 {
                debug!(
                    active_allocations = prev_allocated + 1,
                    "SafeMemoryPool allocation milestone"
                );
            }

            // Track allocated chunk
            self.allocated_chunks.write().push(Arc::clone(&chunk));

            self.stats
                .record_allocation(self.config.chunk_size, timer.elapsed_ns());

            return Ok(SafeMemoryHandle { chunk });
        }

        // Check if we can allocate more
        let current_total =
            self.allocated_count.load(Ordering::Relaxed) + self.free_count.load(Ordering::Relaxed);

        if current_total >= self.config.max_chunks {
            return Err(AllocError::PoolExhausted);
        }

        // Allocate a new chunk
        let generation = self.generation.fetch_add(1, Ordering::Relaxed);
        let chunk = SafeMemoryChunk::new(self.config.chunk_size, generation as u64);
        let chunk_arc = Arc::new(parking_lot::Mutex::new(chunk));

        self.allocated_chunks.write().push(Arc::clone(&chunk_arc));
        let allocated_count = self.allocated_count.fetch_add(1, Ordering::Relaxed);
        let total_memory = self
            .total_memory
            .fetch_add(self.config.chunk_size, Ordering::Relaxed);

        // Track memory growth for performance monitoring
        if allocated_count % 5000 == 0 {
            debug!(
                allocations = allocated_count + 1,
                total_mb = (total_memory + self.config.chunk_size) / (1024 * 1024),
                "SafeMemoryPool expanding - new chunk allocated"
            );
        }
        self.stats
            .record_allocation(self.config.chunk_size, timer.elapsed_ns());

        Ok(SafeMemoryHandle { chunk: chunk_arc })
    }

    pub fn deallocate_chunk(&self, handle: SafeMemoryHandle) {
        if self.config.zero_on_dealloc {
            let mut chunk = handle.chunk.lock();
            for byte in chunk.data.iter_mut() {
                *byte = 0;
            }
        }

        // Remove from allocated list
        self.allocated_chunks
            .write()
            .retain(|c| !Arc::ptr_eq(c, &handle.chunk));

        // Add back to free list
        self.free_chunks.push(handle.chunk);
        let prev_allocated = self.allocated_count.fetch_sub(1, Ordering::Relaxed);
        let prev_free = self.free_count.fetch_add(1, Ordering::Relaxed);

        // Track deallocation patterns for memory leak detection
        if prev_allocated == 1 {
            debug!("SafeMemoryPool: All chunks have been deallocated");
        }
        if (prev_free + 1) % 10000 == 0 {
            debug!(
                returned_chunks = prev_free + 1,
                "SafeMemoryPool deallocation milestone"
            );
        }
        self.stats.record_deallocation(self.config.chunk_size);
    }

    pub fn get_stats(&self) -> SafePoolStats {
        SafePoolStats {
            allocated_chunks: self.allocated_count.load(Ordering::Relaxed),
            free_chunks: self.free_count.load(Ordering::Relaxed),
            total_memory_bytes: self.total_memory.load(Ordering::Relaxed),
            chunk_size: self.config.chunk_size,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SafePoolStats {
    pub allocated_chunks: usize,
    pub free_chunks: usize,
    pub total_memory_bytes: usize,
    pub chunk_size: usize,
}
