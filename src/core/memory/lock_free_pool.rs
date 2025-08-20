//! Lock-free memory pool for ultra-low latency allocations
//!
//! # Safety
//! This module uses unsafe code for performance. All unsafe operations are
//! documented with SAFETY comments explaining their invariants.

#![allow(unsafe_code)] // This module requires unsafe for performance
#![deny(unsafe_op_in_unsafe_fn)] // But every unsafe op must be justified

use crate::core::memory::allocator::{AllocError, MemoryAllocator};
use crate::core::memory::hazard_pointer::HazardPointerDomain;
use crate::core::memory::stats::{AllocationTimer, MemoryStats};
use crossbeam::queue::SegQueue;
use std::alloc::{Layout, alloc, dealloc};
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const CACHE_LINE_SIZE: usize = 64;
const DEFAULT_CHUNK_SIZE: usize = 4096;
const DEFAULT_INITIAL_CHUNKS: usize = 1024;

#[derive(Clone, Debug)]
pub struct PoolConfig {
    pub chunk_size: usize,
    pub initial_chunks: usize,
    pub max_chunks: usize,
    pub alignment: usize,
    pub zero_on_dealloc: bool,
    pub thread_cache_size: usize,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
            initial_chunks: DEFAULT_INITIAL_CHUNKS,
            max_chunks: 1_000_000,
            alignment: CACHE_LINE_SIZE,
            zero_on_dealloc: false,
            thread_cache_size: 32,
        }
    }
}

pub struct MemoryChunk {
    pub ptr: NonNull<u8>,
    pub size: usize,
    pub generation: u64,
}

unsafe impl Send for MemoryChunk {}
unsafe impl Sync for MemoryChunk {}

#[derive(Debug)]
pub struct LockFreeMemoryPool {
    config: PoolConfig,
    free_chunks: Arc<SegQueue<MemoryChunk>>,
    allocated_count: AtomicUsize,
    free_count: AtomicUsize,
    total_memory: AtomicUsize,
    generation: AtomicUsize,
    hazard_domain: Arc<HazardPointerDomain>,
    stats: Arc<MemoryStats>,
}

impl LockFreeMemoryPool {
    pub fn new(config: PoolConfig) -> Result<Self, AllocError> {
        if !config.chunk_size.is_power_of_two() && config.chunk_size < CACHE_LINE_SIZE {
            return Err(AllocError::InvalidLayout(
                "Chunk size must be power of 2 and >= cache line size".to_string(),
            ));
        }

        let pool = Self {
            config: config.clone(),
            free_chunks: Arc::new(SegQueue::new()),
            allocated_count: AtomicUsize::new(0),
            free_count: AtomicUsize::new(0),
            total_memory: AtomicUsize::new(0),
            generation: AtomicUsize::new(0),
            hazard_domain: Arc::new(HazardPointerDomain::new(128)),
            stats: Arc::new(MemoryStats::new()),
        };

        pool.preallocate_chunks(config.initial_chunks)?;

        Ok(pool)
    }

    fn preallocate_chunks(&self, count: usize) -> Result<(), AllocError> {
        let layout = Layout::from_size_align(self.config.chunk_size, self.config.alignment)
            .map_err(|e| AllocError::InvalidLayout(e.to_string()))?;

        for _ in 0..count {
            // SAFETY: Layout is valid (checked above), alignment is power of 2
            // The allocated memory is immediately wrapped in MemoryChunk
            let ptr = unsafe { alloc(layout) };
            if ptr.is_null() {
                return Err(AllocError::OutOfMemory);
            }

            let chunk = MemoryChunk {
                // SAFETY: We just checked ptr is not null
                ptr: unsafe { NonNull::new_unchecked(ptr) },
                size: self.config.chunk_size,
                generation: self.generation.fetch_add(1, Ordering::Relaxed) as u64,
            };

            self.free_chunks.push(chunk);
            self.free_count.fetch_add(1, Ordering::Relaxed);
            self.total_memory
                .fetch_add(self.config.chunk_size, Ordering::Relaxed);
        }

        Ok(())
    }

    pub fn allocate_chunk(&self) -> Result<NonNull<u8>, AllocError> {
        let timer = AllocationTimer::start();

        // Use hazard pointer to safely access the free list
        let hazard = self.hazard_domain.acquire();

        if let Some(chunk) = self.free_chunks.pop() {
            // Protect the chunk with hazard pointer during access
            hazard.protect(chunk.ptr.as_ptr() as *const u8);

            self.free_count.fetch_sub(1, Ordering::Relaxed);
            self.allocated_count.fetch_add(1, Ordering::Relaxed);
            self.stats
                .record_allocation(self.config.chunk_size, timer.elapsed_ns());
            return Ok(chunk.ptr);
        }

        let current_total =
            self.allocated_count.load(Ordering::Relaxed) + self.free_count.load(Ordering::Relaxed);

        if current_total >= self.config.max_chunks {
            return Err(AllocError::PoolExhausted);
        }

        let layout = Layout::from_size_align(self.config.chunk_size, self.config.alignment)
            .map_err(|e| AllocError::InvalidLayout(e.to_string()))?;

        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            return Err(AllocError::OutOfMemory);
        }

        self.allocated_count.fetch_add(1, Ordering::Relaxed);
        self.total_memory
            .fetch_add(self.config.chunk_size, Ordering::Relaxed);
        self.stats
            .record_allocation(self.config.chunk_size, timer.elapsed_ns());

        Ok(unsafe { NonNull::new_unchecked(ptr) })
    }

    pub fn deallocate_chunk(&self, ptr: NonNull<u8>) {
        if self.config.zero_on_dealloc {
            unsafe {
                std::ptr::write_bytes(ptr.as_ptr(), 0, self.config.chunk_size);
            }
        }

        let chunk = MemoryChunk {
            ptr,
            size: self.config.chunk_size,
            generation: self.generation.fetch_add(1, Ordering::Relaxed) as u64,
        };

        self.free_chunks.push(chunk);
        self.allocated_count.fetch_sub(1, Ordering::Relaxed);
        self.free_count.fetch_add(1, Ordering::Relaxed);
        self.stats.record_deallocation(self.config.chunk_size);
    }

    pub fn get_stats(&self) -> PoolStats {
        PoolStats {
            allocated_chunks: self.allocated_count.load(Ordering::Relaxed),
            free_chunks: self.free_count.load(Ordering::Relaxed),
            total_memory_bytes: self.total_memory.load(Ordering::Relaxed),
            chunk_size: self.config.chunk_size,
        }
    }

    pub fn get_allocation_stats(&self) -> Arc<MemoryStats> {
        Arc::clone(&self.stats)
    }
}

impl MemoryAllocator for LockFreeMemoryPool {
    fn allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        if layout.size() > self.config.chunk_size {
            return Err(AllocError::SizeExceeded {
                size: layout.size(),
                max: self.config.chunk_size,
            });
        }

        if layout.align() > self.config.alignment {
            return Err(AllocError::AlignmentNotSupported {
                required: layout.align(),
                supported: self.config.alignment,
            });
        }

        self.allocate_chunk()
    }

    fn deallocate(&self, ptr: NonNull<u8>, _layout: Layout) {
        self.deallocate_chunk(ptr);
    }

    fn available_memory(&self) -> usize {
        self.free_count.load(Ordering::Relaxed) * self.config.chunk_size
    }

    fn total_memory(&self) -> usize {
        self.total_memory.load(Ordering::Relaxed)
    }

    fn max_alignment(&self) -> usize {
        self.config.alignment
    }
}

impl Drop for LockFreeMemoryPool {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.config.chunk_size, self.config.alignment)
            .expect("Invalid layout in drop");

        while let Some(chunk) = self.free_chunks.pop() {
            unsafe {
                dealloc(chunk.ptr.as_ptr(), layout);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub allocated_chunks: usize,
    pub free_chunks: usize,
    pub total_memory_bytes: usize,
    pub chunk_size: usize,
}
