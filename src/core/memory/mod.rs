//! Memory management module for ShrivenQ
//!
//! Provides both safe and high-performance memory allocators.
//! By default, only safe allocators are available.
//! Enable `hft-unsafe` feature for lock-free and NUMA-aware allocators.

#![cfg_attr(feature = "hft-unsafe", allow(unsafe_code))]
#![deny(unsafe_op_in_unsafe_fn)] // Force explicit safety boundaries
#![deny(clippy::missing_safety_doc)] // Every unsafe fn must explain invariants

pub mod allocator;
pub mod safe_pool;
pub mod stats;

// Conditionally compile unsafe modules only with hft-unsafe feature
#[cfg(feature = "hft-unsafe")]
pub mod hazard_pointer;
#[cfg(feature = "hft-unsafe")]
pub mod lock_free_pool;
#[cfg(feature = "hft-unsafe")]
pub mod numa_allocator;
#[cfg(feature = "hft-unsafe")]
pub mod slab_allocator;

// Always export safe interfaces
pub use allocator::{AllocError, MemoryAllocator};
pub use safe_pool::{SafeMemoryPool, SafePoolConfig};
pub use stats::MemoryStats;

// Conditionally export unsafe module interfaces
#[cfg(feature = "hft-unsafe")]
pub use hazard_pointer::HazardPointerDomain;
#[cfg(feature = "hft-unsafe")]
pub use lock_free_pool::{LockFreeMemoryPool, PoolConfig};
#[cfg(feature = "hft-unsafe")]
pub use numa_allocator::{NumaAllocator, NumaConfig};
#[cfg(feature = "hft-unsafe")]
pub use slab_allocator::{SlabAllocator, SlabConfig};

/// Unified memory backend that can switch between safe and high-performance implementations
#[derive(Debug)]
pub enum MemoryBackend {
    /// Safe memory pool (always available)
    Safe(SafeMemoryPool),

    /// Lock-free memory pool (requires hft-unsafe feature)
    #[cfg(feature = "hft-unsafe")]
    LockFree(LockFreeMemoryPool),

    /// NUMA-aware allocator (requires hft-unsafe feature)
    #[cfg(feature = "hft-unsafe")]
    Numa(NumaAllocator),

    /// Slab allocator for fixed-size objects (requires hft-unsafe feature)
    #[cfg(feature = "hft-unsafe")]
    Slab(SlabAllocator),
}

impl MemoryBackend {
    /// Create a safe memory backend (default)
    pub fn safe(config: SafePoolConfig) -> Result<Self, AllocError> {
        Ok(MemoryBackend::Safe(SafeMemoryPool::new(config)?))
    }

    /// Create a lock-free memory backend (requires hft-unsafe feature)
    #[cfg(feature = "hft-unsafe")]
    pub fn lock_free(config: PoolConfig) -> Result<Self, AllocError> {
        Ok(MemoryBackend::LockFree(LockFreeMemoryPool::new(config)?))
    }

    /// Create a NUMA-aware backend (requires hft-unsafe feature)
    #[cfg(feature = "hft-unsafe")]
    pub fn numa(config: NumaConfig) -> Result<Self, AllocError> {
        Ok(MemoryBackend::Numa(NumaAllocator::new(config)?))
    }

    /// Create a slab allocator backend (requires hft-unsafe feature)
    #[cfg(feature = "hft-unsafe")]
    pub fn slab(config: SlabConfig) -> Result<Self, AllocError> {
        Ok(MemoryBackend::Slab(SlabAllocator::new(config)?))
    }

    /// Returns true if this backend uses unsafe code
    pub fn is_unsafe(&self) -> bool {
        match self {
            MemoryBackend::Safe(_) => false,
            #[cfg(feature = "hft-unsafe")]
            _ => true,
        }
    }

    /// Get the backend type as a string for logging
    pub fn backend_type(&self) -> &'static str {
        match self {
            MemoryBackend::Safe(_) => "Safe",
            #[cfg(feature = "hft-unsafe")]
            MemoryBackend::LockFree(_) => "LockFree",
            #[cfg(feature = "hft-unsafe")]
            MemoryBackend::Numa(_) => "NUMA-aware",
            #[cfg(feature = "hft-unsafe")]
            MemoryBackend::Slab(_) => "Slab",
        }
    }
}
