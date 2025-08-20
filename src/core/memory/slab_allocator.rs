// Ultra-low latency slab allocator for HFT
// Pre-allocates all memory at startup, uses lock-free structures
// No allocations during trading hours

use crate::core::memory::allocator::{AllocError, MemoryAllocator};
use crossbeam::queue::SegQueue;
use std::alloc::{Layout, alloc};
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const CACHE_LINE_SIZE: usize = 64;

#[derive(Clone, Debug, Copy)]
pub struct SlabConfig {
    pub min_object_size: usize,
    pub max_object_size: usize,
    pub objects_per_slab: usize,
    pub pre_allocate_slabs: usize,
    pub cache_align: bool,
}

impl Default for SlabConfig {
    fn default() -> Self {
        Self {
            min_object_size: 64,
            max_object_size: 8192,
            objects_per_slab: 1024,
            pre_allocate_slabs: 100,
            cache_align: true,
        }
    }
}

// Pre-allocated memory block
struct MemoryBlock {
    ptr: usize,  // Store as usize to avoid Send/Sync issues
    size: usize, // Size of this memory block
}

// Lock-free slab allocator using pre-allocated memory
#[derive(Debug)]
pub struct SlabAllocator {
    config: SlabConfig,
    // Lock-free queues for each size class
    free_blocks: Arc<Vec<Arc<SegQueue<MemoryBlock>>>>,
    size_classes: Vec<usize>,
    allocated_count: AtomicUsize,
    freed_count: AtomicUsize,
    total_memory: AtomicUsize,
}

impl SlabAllocator {
    pub fn new(config: SlabConfig) -> Result<Self, AllocError> {
        // Calculate size classes
        let mut size_classes = Vec::new();
        let mut size = config.min_object_size;
        while size <= config.max_object_size {
            size_classes.push(size);
            size = size * 2; // Double each time for simplicity
        }

        // Pre-allocate all memory blocks
        let mut free_blocks = Vec::new();
        let mut total_memory = 0;

        for &size_class in &size_classes {
            let queue = Arc::new(SegQueue::new());

            // Pre-allocate blocks for this size class
            let layout = if config.cache_align {
                Layout::from_size_align(size_class, CACHE_LINE_SIZE)
                    .map_err(|e| AllocError::InvalidLayout(e.to_string()))?
            } else {
                Layout::from_size_align(size_class, std::mem::align_of::<usize>())
                    .map_err(|e| AllocError::InvalidLayout(e.to_string()))?
            };

            for _ in 0..config.pre_allocate_slabs {
                let ptr = unsafe { alloc(layout) };
                if ptr.is_null() {
                    return Err(AllocError::OutOfMemory);
                }

                queue.push(MemoryBlock {
                    ptr: ptr as usize,
                    size: size_class,
                });

                total_memory += size_class;
            }

            free_blocks.push(queue);
        }

        Ok(Self {
            config,
            free_blocks: Arc::new(free_blocks),
            size_classes,
            allocated_count: AtomicUsize::new(0),
            freed_count: AtomicUsize::new(0),
            total_memory: AtomicUsize::new(total_memory),
        })
    }

    fn get_size_class_index(&self, size: usize) -> Option<usize> {
        self.size_classes
            .iter()
            .position(|&class_size| class_size >= size)
    }

    pub fn allocate_object(&self, size: usize) -> Result<NonNull<u8>, AllocError> {
        let class_idx =
            self.get_size_class_index(size)
                .ok_or_else(|| AllocError::SizeExceeded {
                    size,
                    max: self.config.max_object_size,
                })?;

        if let Some(block) = self.free_blocks[class_idx].pop() {
            let prev_allocated = self.allocated_count.fetch_add(1, Ordering::Relaxed);

            // Log allocation milestones
            if prev_allocated % 100000 == 0 && prev_allocated > 0 {
                tracing::debug!(
                    allocated_count = prev_allocated + 1,
                    "SlabAllocator allocation milestone"
                );
            }

            // Verify block size matches expected size class
            debug_assert_eq!(
                block.size, self.size_classes[class_idx],
                "Block size mismatch: expected {}, got {}",
                self.size_classes[class_idx], block.size
            );

            // Convert back to NonNull
            let ptr = block.ptr as *mut u8;
            NonNull::new(ptr).ok_or(AllocError::InvalidLayout(
                "Invalid pointer in free block".to_string(),
            ))
        } else {
            Err(AllocError::PoolExhausted)
        }
    }

    pub fn deallocate_object(&self, ptr: NonNull<u8>, size: usize) {
        if let Some(class_idx) = self.get_size_class_index(size) {
            let size_class = self.size_classes[class_idx];

            self.free_blocks[class_idx].push(MemoryBlock {
                ptr: ptr.as_ptr() as usize,
                size: size_class,
            });

            let prev_freed = self.freed_count.fetch_add(1, Ordering::Relaxed);

            // Log deallocation milestones
            if prev_freed % 100000 == 0 && prev_freed > 0 {
                tracing::debug!(
                    freed_count = prev_freed + 1,
                    "SlabAllocator deallocation milestone"
                );
            }
        }
    }

    pub fn get_stats(&self) -> SlabStats {
        SlabStats {
            allocated_objects: self.allocated_count.load(Ordering::Relaxed),
            freed_objects: self.freed_count.load(Ordering::Relaxed),
            total_memory: self.total_memory.load(Ordering::Relaxed),
            size_classes: self.size_classes.len(),
        }
    }
}

impl MemoryAllocator for SlabAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        self.allocate_object(layout.size())
    }

    fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        self.deallocate_object(ptr, layout.size());
    }

    fn available_memory(&self) -> usize {
        // Approximate - count free blocks
        self.free_blocks
            .iter()
            .zip(&self.size_classes)
            .map(|(queue, &size)| queue.len() * size)
            .sum()
    }

    fn total_memory(&self) -> usize {
        self.total_memory.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SlabStats {
    pub allocated_objects: usize,
    pub freed_objects: usize,
    pub total_memory: usize,
    pub size_classes: usize,
}

// Safe to send/sync because we only store usize addresses
unsafe impl Send for SlabAllocator {}
unsafe impl Sync for SlabAllocator {}

impl Drop for SlabAllocator {
    fn drop(&mut self) {
        use std::alloc::dealloc;

        // Free all remaining blocks
        for (queue, &size_class) in self.free_blocks.iter().zip(&self.size_classes) {
            let layout = if self.config.cache_align {
                Layout::from_size_align(size_class, CACHE_LINE_SIZE).unwrap()
            } else {
                Layout::from_size_align(size_class, std::mem::align_of::<usize>()).unwrap()
            };

            while let Some(block) = queue.pop() {
                unsafe {
                    dealloc(block.ptr as *mut u8, layout);
                }
            }
        }
    }
}
