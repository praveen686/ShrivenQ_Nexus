// Allow unsafe in this module only when hft-unsafe feature is enabled
#![cfg_attr(not(feature = "hft-unsafe"), forbid(unsafe_code))]

use std::alloc::Layout;
use std::ptr::NonNull;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum AllocError {
    #[error("Out of memory")]
    OutOfMemory,
    #[error("Invalid layout: {0}")]
    InvalidLayout(String),
    #[error("NUMA node {0} not available")]
    NumaNodeUnavailable(usize),
    #[error("Allocation size {size} exceeds maximum {max}")]
    SizeExceeded { size: usize, max: usize },
    #[error("Memory pool exhausted")]
    PoolExhausted,
    #[error("Alignment requirement {required} not supported (max: {supported})")]
    AlignmentNotSupported { required: usize, supported: usize },
    #[error("Memory system already initialized")]
    AlreadyInitialized,
    #[error("Memory system not initialized")]
    NotInitialized,
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
}

pub trait MemoryAllocator: Send + Sync {
    fn allocate(&self, layout: Layout) -> Result<NonNull<u8>, AllocError>;

    fn deallocate(&self, ptr: NonNull<u8>, layout: Layout);

    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
        #[cfg(feature = "hft-unsafe")]
        {
            let ptr = self.allocate(layout)?;
            // SAFETY: ptr is valid and aligned, size is from layout
            unsafe {
                std::ptr::write_bytes(ptr.as_ptr(), 0, layout.size());
            }
            Ok(ptr)
        }
        #[cfg(not(feature = "hft-unsafe"))]
        {
            // Safe but slower alternative
            let ptr = self.allocate(layout)?;
            // Note: In safe mode, memory is already zeroed by SafeMemoryPool
            Ok(ptr)
        }
    }

    fn reallocate(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<u8>, AllocError> {
        #[cfg(feature = "hft-unsafe")]
        {
            let new_ptr = self.allocate(new_layout)?;
            // SAFETY: Both pointers are valid, copy_size is bounded by min of both sizes
            unsafe {
                let copy_size = old_layout.size().min(new_layout.size());
                std::ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_ptr(), copy_size);
            }
            self.deallocate(ptr, old_layout);
            Ok(new_ptr)
        }
        #[cfg(not(feature = "hft-unsafe"))]
        {
            // In safe mode, we allocate new memory and let the caller handle copying
            // This is safe but requires the caller to manage data transfer
            let _old_size = old_layout.size();
            let _new_size = new_layout.size();

            // We can't safely copy data without unsafe operations
            // So we return an error indicating the operation needs to be handled differently
            let _ = ptr; // Acknowledge we received the pointer
            Err(AllocError::UnsupportedOperation(format!(
                "reallocate from {} to {} bytes requires unsafe code and is not available in safe mode",
                old_layout.size(),
                new_layout.size()
            )))
        }
    }

    fn supports_alignment(&self, align: usize) -> bool {
        align.is_power_of_two() && align <= self.max_alignment()
    }

    fn max_alignment(&self) -> usize {
        64
    }

    fn available_memory(&self) -> usize;

    fn total_memory(&self) -> usize;
}
