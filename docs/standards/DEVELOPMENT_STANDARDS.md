# ShrivenQ Development Standards

## Executive Summary

This document establishes comprehensive development standards for ShrivenQ, an ultra-low latency quantitative trading platform. These standards are derived from battle-tested experience, critical lessons learned from the clippy configuration crisis, analysis of ShrivenQuant's production codebase, and industry best practices for high-frequency trading systems.

**Critical Principle:** Every microsecond matters in HFT. These standards prioritize performance, safety, and maintainability in that order, while ensuring institutional-grade reliability.

---

## 📋 Table of Contents

1. [Architecture Principles](#architecture-principles)
2. [Safe vs Unsafe System Standards](#safe-vs-unsafe-system-standards)
3. [Code Quality Standards](#code-quality-standards)
4. [Build System Standards](#build-system-standards)
5. [Performance Standards](#performance-standards)
6. [Safety and Testing Standards](#safety-and-testing-standards)
7. [Naming Conventions](#naming-conventions)
8. [Documentation Standards](#documentation-standards)
9. [Error Handling Standards](#error-handling-standards)
10. [Review and Compliance](#review-and-compliance)

---

## 🏗️ Architecture Principles

### Core Design Philosophy

1. **Latency First** - Every design decision must consider latency impact
2. **Safe by Default** - Unsafe code is opt-in only through feature flags
3. **Pre-allocate Everything** - Zero allocations during trading hours
4. **Measure Everything** - Performance regression detection is mandatory
5. **Build → Measure → Optimize** - Evidence-based performance optimization

### Feature-Gated Architecture

```rust
// ✅ MANDATORY: All unsafe code must be feature-gated
#[cfg(feature = "hft-unsafe")]
pub mod lock_free_pool;

// ✅ REQUIRED: Safe alternatives for all functionality  
pub mod safe_pool;  // Always available

// ✅ MANDATORY: Clear feature documentation
[features]
default = ["zerodha-integration"]  # Safe by default
hft-unsafe = []                   # Opt-in for performance
```

### Memory Architecture Standards

**MANDATORY Requirements:**
- Pre-allocate all memory at startup
- Use lock-free data structures for hot paths
- NUMA-aware allocation for multi-socket systems
- Memory pools sized for peak trading volume + 50% buffer

**FORBIDDEN Practices:**
- Runtime allocation during trading hours
- Garbage collection or automatic memory management
- Memory fragmentation through frequent alloc/dealloc cycles
- Using standard allocators for performance-critical paths

---

## 🛡️ Safe vs Unsafe System Standards

### Safe System Standards (Default Mode)

**MANDATORY for Production Trading:**
- All code compiles with `-D unsafe-code`
- Zero unsafe blocks in safe mode
- Comprehensive error handling with `Result<T, E>`
- Automatic memory management through safe abstractions
- Performance targets: < 1ms latency (acceptable for most retail trading)

```rust
// ✅ REQUIRED: Safe implementation example
pub struct SafeOrderBook {
    bids: BTreeMap<Price, Quantity>,  // Safe, predictable performance
    asks: BTreeMap<Price, Quantity>,
    // No unsafe code anywhere
}

impl SafeOrderBook {
    pub fn add_order(&mut self, order: Order) -> Result<OrderId, OrderError> {
        // Proper error handling, no panics
        self.validate_order(&order)?;
        // Safe insertion with predictable performance
        Ok(self.insert_order(order))
    }
}
```

### Unsafe System Standards (HFT Mode)

**MANDATORY Safety Requirements:**
- Every unsafe function MUST have comprehensive safety documentation
- Unsafe code isolated to specific modules with clear boundaries  
- Hazard pointer protocols for safe memory reclamation
- Comprehensive testing with Miri, Loom, and AddressSanitizer

**Performance Targets:**
- Order-to-exchange latency: < 100μs
- Market data processing: < 10μs
- Risk calculations: < 50μs

```rust
// ✅ REQUIRED: Unsafe implementation example
#[cfg(feature = "hft-unsafe")]
pub struct LockFreeOrderBook {
    // Implementation details...
}

#[cfg(feature = "hft-unsafe")]
impl LockFreeOrderBook {
    /// Add order to the book with lock-free performance
    /// 
    /// # Safety
    /// 
    /// This function is safe to call provided:
    /// - The order has been validated for correct price/quantity ranges
    /// - The order ID is unique and has not been used before
    /// - The caller has exclusive access to modify this book
    /// 
    /// # Performance
    /// 
    /// Guaranteed O(1) insertion with < 1μs latency on modern hardware
    pub unsafe fn add_order_unchecked(&self, order: Order) -> OrderId {
        // SAFETY: Caller guarantees order validity and exclusive access
        unsafe {
            self.insert_lock_free(order)
        }
    }
}
```

### Dual-Mode Architecture Requirements

**MANDATORY Implementation Pattern:**
```rust
// ✅ REQUIRED: Unified interface for both modes
pub enum OrderBookBackend {
    Safe(SafeOrderBook),
    #[cfg(feature = "hft-unsafe")]
    LockFree(LockFreeOrderBook),
}

impl OrderBookBackend {
    pub fn add_order(&mut self, order: Order) -> Result<OrderId, OrderError> {
        match self {
            OrderBookBackend::Safe(book) => book.add_order(order),
            #[cfg(feature = "hft-unsafe")]
            OrderBookBackend::LockFree(book) => {
                // Safe wrapper around unsafe implementation
                order.validate()?;
                unsafe { Ok(book.add_order_unchecked(order)) }
            }
        }
    }
}
```

---

## 📐 Code Quality Standards

### Lint Configuration Standards

**CRITICAL LESSON: The Clippy Configuration Crisis**

On 2025-08-20, ShrivenQ experienced a critical build failure due to improper clippy configuration that generated 1000+ contradictory lint errors. This teaches us:

**❌ FORBIDDEN: Never Use These Patterns**
```bash
# NEVER DO THIS - Causes 1000+ contradictory errors
export CLIPPY_FLAGS="-W clippy::restriction -D warnings"

# NEVER enable restriction group globally
clippy = { level = "warn", restriction = true }
```

**✅ MANDATORY: Workspace Lint Tables**
```toml
# Cargo.toml - REQUIRED lint configuration
[lints.rust]
unsafe_code = "deny"        # Default to safe Rust
unused = { level = "warn", priority = -1 }

[lints.clippy]
# Hierarchical lint policy with clear priorities
all = { level = "warn", priority = -1 }                # Baseline quality
pedantic = { level = "allow", priority = -1 }          # Opt-in per module
nursery = { level = "allow", priority = -1 }           # Experimental only
restriction = { level = "allow", priority = -1 }       # NEVER global

# Cherry-picked individual restriction lints
expect_used = "deny"        # HFT requires proper error handling
unwrap_used = "deny"        # No panics in trading code
missing_safety_doc = "deny" # Document every unsafe operation
```

**MANDATORY Clippy Group Usage Rules:**
- `clippy::all` - ✅ Always enable globally
- `clippy::pedantic` - ⚠️ Enable per-module only  
- `clippy::nursery` - ⚠️ Experimental, careful selection
- `clippy::restriction` - ❌ **NEVER enable globally, cherry-pick only**

### Module-Level Lint Overrides

**MANDATORY Override Patterns:**
```rust
// ✅ CORRECT: Per-module customization when needed
#![warn(clippy::pedantic)]          // Enable strict style for this module
#![allow(clippy::module_name_repetitions)]  // But allow domain modeling patterns

// ✅ CORRECT: Feature-gated unsafe architecture
#![cfg_attr(feature = "hft-unsafe", allow(unsafe_code))]
#![deny(unsafe_op_in_unsafe_fn)]       // Force explicit unsafe blocks
#![deny(clippy::missing_safety_doc)]   // Require safety documentation
```

### Code Organization Standards

**MANDATORY Module Structure:**
```rust
// ✅ REQUIRED: Feature-gated unsafe modules
#![cfg_attr(feature = "hft-unsafe", allow(unsafe_code))]
#![deny(unsafe_op_in_unsafe_fn)]       // Force explicit safety boundaries
#![deny(clippy::missing_safety_doc)]   // Require safety documentation

// ✅ REQUIRED: Module organization
src/
├── core/
│   ├── memory/           # Memory management
│   ├── execution/        # Order execution
│   └── data/            # Market data
├── engines/
│   ├── crypto/          # Cryptocurrency trading
│   ├── equity/          # Stock trading
│   └── options/         # Options trading
├── infrastructure/
│   ├── monitoring/      # Performance monitoring
│   ├── config/         # Configuration management
│   └── logging/        # High-performance logging
└── interfaces/
    ├── rest_api/       # REST interfaces
    └── websocket/      # WebSocket streaming
```

### Error Handling Standards

**MANDATORY Error Patterns:**
```rust
// ✅ REQUIRED: Structured error types with thiserror
#[derive(Debug, thiserror::Error)]
pub enum TradingError {
    #[error("Insufficient margin: required {required}, available {available}")]
    InsufficientMargin { required: f64, available: f64 },
    
    #[error("Invalid order price {price}: must be between {min} and {max}")]
    InvalidPrice { price: f64, min: f64, max: f64 },
    
    #[error("Market data unavailable for symbol {symbol}")]
    MarketDataUnavailable { symbol: String },
    
    #[error("Risk limit exceeded: {limit_type} = {current} > {maximum}")]
    RiskLimitExceeded { limit_type: String, current: f64, maximum: f64 },
}

// ✅ REQUIRED: Context-rich error handling
impl OrderExecutor {
    pub fn execute_order(&self, order: Order) -> Result<Execution, TradingError> {
        let available_margin = self.get_available_margin()
            .map_err(|e| TradingError::MarketDataUnavailable { 
                symbol: order.symbol.clone() 
            })?;
        
        let required_margin = self.calculate_margin_requirement(&order)?;
        
        if required_margin > available_margin {
            return Err(TradingError::InsufficientMargin {
                required: required_margin,
                available: available_margin,
            });
        }
        
        self.submit_to_exchange(order)
    }
}
```

**FORBIDDEN Error Patterns:**
```rust
// ❌ FORBIDDEN: Generic error messages
return Err("Something went wrong".into());

// ❌ FORBIDDEN: expect() in production code
let price = order.price.expect("Price should be set");

// ❌ FORBIDDEN: unwrap() in trading paths
let quantity = order.quantity.unwrap();

// ❌ FORBIDDEN: Panic in production
panic!("This should never happen");
```

---

## 🔧 Build System Standards

### Build System Principles

**Philosophy:** Centralize lint and build configuration in `Cargo.toml` rather than scattered CLI flags.

```toml
# Cargo.toml - Single source of truth
[lints.rust]
unsafe_code = "deny"
unused = { level = "warn", priority = -1 }

[lints.clippy]
all = { level = "warn", priority = -1 }
restriction = { level = "allow", priority = -1 }  # Never enable globally
```

**Benefits:**
- Version-controlled configuration
- Consistent across all build environments
- No CLI flag proliferation
- Team-wide standardization

### Build Script Architecture

**CRITICAL LESSON: Sequential Validation Gates**

Based on the clippy crisis resolution, our build system uses sequential validation where each stage gates the next:

```bash
# ✅ REQUIRED: Sequential build validation
#!/bin/bash
set -euo pipefail  # Fail fast on any error

echo "🚀 ShrivenQ Strict Build - Sequential Validation"

# Stage 1: Formatting (must pass before proceeding)
cargo fmt --all -- --check || {
    echo "❌ Code formatting failed"
    exit 1
}

# Stage 2: Compilation (must pass before linting)
cargo build --workspace --all-features || {
    echo "❌ Compilation failed" 
    exit 1
}

# Stage 3: Linting (uses workspace configuration)
cargo clippy --workspace --all-features -- -D warnings || {
    echo "❌ Linting failed"
    exit 1
}

# Stage 4: Testing (only after all above pass)
cargo test --workspace --all-features || {
    echo "❌ Tests failed"
    exit 1
}

echo "✅ All build stages passed"
```

**MANDATORY Build Variants:**
- `build_strict_sequential.sh` - Comprehensive validation
- `build_development_quick.sh` - Fast iteration for developers
- `build_release_optimized.sh` - Production builds with full optimization
- `validate_documentation.sh` - Documentation consistency checking

### Build Script Standards

#### Error Handling and Exit Codes

```bash
#!/bin/bash
set -euo pipefail  # Fail fast on any error

# Color-coded output for clarity
RED='\033[0;31m'
GREEN='\033[0;32m' 
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function for consistent error reporting
build_stage() {
    local stage_name="$1"
    local command="$2"
    
    echo -e "${YELLOW}${stage_name}...${NC}"
    
    if eval "$command"; then
        echo -e "${GREEN}✓ ${stage_name} completed${NC}"
    else
        echo -e "${RED}✗ ${stage_name} failed${NC}"
        exit 1
    fi
}
```

#### Environment Validation

```bash
# Validate Rust toolchain
check_environment() {
    if ! command -v cargo &> /dev/null; then
        echo "Error: Cargo not found. Please install Rust toolchain."
        exit 1
    fi
    
    local rust_version
    rust_version=$(rustc --version | cut -d' ' -f2)
    local required_version="1.85.0"
    
    if ! cargo --version | grep -q "cargo 1.85"; then
        echo "Warning: Rust version $rust_version may not match required $required_version"
    fi
}
```

### Build Performance Monitoring

```bash
# Time each build stage
time_stage() {
    local stage_name="$1"
    local start_time
    local end_time
    local duration
    
    start_time=$(date +%s)
    build_stage "$stage_name" "$2"
    end_time=$(date +%s)
    duration=$((end_time - start_time))
    
    echo "⏱️ ${stage_name}: ${duration}s"
}

# Monitor peak memory usage during builds
monitor_build_memory() {
    local build_cmd="$1"
    
    # Start memory monitoring in background
    (
        while sleep 1; do
            ps aux | grep -E "(rustc|cargo)" | awk '{sum+=$6} END {print sum/1024 " MB"}'
        done
    ) &
    local monitor_pid=$!
    
    # Run the actual build
    eval "$build_cmd"
    local build_exit=$?
    
    # Stop monitoring
    kill $monitor_pid 2>/dev/null
    
    return $build_exit
}

# Build with comprehensive timing metrics
build_with_timing() {
    local total_start=$(date +%s.%N)
    
    # Track each phase
    time_phase "Format Check" "cargo fmt --all -- --check"
    time_phase "Compilation" "cargo build --workspace --all-features"  
    time_phase "Clippy Analysis" "cargo clippy --workspace --all-features -- -D warnings"
    time_phase "Test Execution" "cargo test --workspace --all-features"
    
    local total_end=$(date +%s.%N)
    local total_time=$(echo "$total_end - $total_start" | bc)
    
    echo "🏁 Total build time: ${total_time}s"
    
    # Log metrics for trend analysis
    echo "$(date +%Y-%m-%d,%H:%M:%S),$total_time" >> build_times.csv
}
```

### Feature-Gated Build Paths

```bash
# Safe builds (default)
cargo build --workspace

# High-performance builds (opt-in)  
cargo build --workspace --features hft-unsafe,high-performance

# GPU-accelerated builds
cargo build --workspace --features gpu-acceleration
```

### Profile-Based Build Configuration

#### Development Profile

```toml
[profile.dev]
opt-level = 0               # Fast compilation
debug = true                # Full debug info
overflow-checks = true      # Catch arithmetic errors
```

#### Release Profile (Production Trading)

```toml
[profile.release]
opt-level = 3               # Maximum optimization
debug = false               # No debug overhead
strip = true                # Remove debug symbols
lto = "fat"                 # Link-time optimization
codegen-units = 1           # Single codegen unit for maximum optimization
panic = "abort"             # Fail fast, no unwinding overhead
```

#### GPU-Optimized Profile

```toml
[profile.gpu]
inherits = "release"
opt-level = 3               # Maximum optimization for GPU kernels
debug = false
lto = "fat"
codegen-units = 1
```

#### Benchmark Profile

```toml
[profile.bench]
opt-level = 3               # Maximum optimization for accurate benchmarks
debug = false               # No debug overhead in measurements
lto = true                  # Link-time optimization
codegen-units = 1           # Consistent optimization
```

### CI/CD Integration Strategy

**MANDATORY GitHub Actions Configuration:**
```yaml
# .github/workflows/build-and-test.yml
name: ShrivenQ Build and Test

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  build-matrix:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust-version: [1.85.0, stable, nightly]
        features: 
          - ""                                    # Safe build
          - "hft-unsafe"                         # Performance build
          - "hft-unsafe,gpu-acceleration"        # Full performance build
        
    steps:
    - uses: actions/checkout@v4
    
    - name: Setup Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: ${{ matrix.rust-version }}
        components: rustfmt, clippy, miri
        
    - name: Run Build Script
      run: ./scripts/build/build_strict_sequential.sh
      env:
        FEATURES: ${{ matrix.features }}
```

### Profile Configuration Standards

**MANDATORY Cargo Profiles:**
```toml
# Development profile - fast compilation
[profile.dev]
opt-level = 0
debug = true
overflow-checks = true

# Release profile - maximum performance
[profile.release] 
opt-level = 3               # Maximum optimization
debug = false               # No debug overhead
strip = true                # Remove debug symbols  
lto = "fat"                 # Link-time optimization
codegen-units = 1           # Single codegen unit for maximum optimization
panic = "abort"             # Fail fast, no unwinding overhead

# HFT profile - ultra-performance
[profile.hft]
inherits = "release"
opt-level = 3
debug = false
lto = "fat" 
codegen-units = 1
panic = "abort"

# Benchmark profile - accurate measurements
[profile.bench]
opt-level = 3
debug = false
lto = true
codegen-units = 1
```

---

## ⚡ Performance Standards

### Latency Requirements

**MANDATORY Performance Targets:**

| Component | Safe Mode | HFT Unsafe Mode | Measurement Method |
|-----------|-----------|----------------|-------------------|
| Order Entry | < 1ms | < 100μs | Hardware timestamping |
| Market Data Processing | < 5ms | < 10μs | End-to-end latency |
| Risk Calculation | < 10ms | < 50μs | Function-level timing |
| Database Operations | < 50ms | < 1ms | Query performance logs |
| Network Round-trip | < 100ms | < 10ms | Network monitoring |

**MANDATORY Latency Budget Example:**
```
Market Data Reception:          10 μs
Order Book Update:              5 μs  
Signal Generation:              20 μs
Risk Check:                     5 μs
Order Creation:                 2 μs
Order Transmission:             50 μs
----------------------------------------
TOTAL:                         92 μs (< 100 μs target)
```

### Performance Monitoring Standards

**MANDATORY Instrumentation:**
```rust
// ✅ REQUIRED: Performance measurement in hot paths
use std::time::Instant;

impl OrderExecutor {
    pub fn execute_order_with_timing(&self, order: Order) -> Result<Execution, TradingError> {
        let start = Instant::now();
        
        // Critical path execution
        let result = self.execute_order(order)?;
        
        let latency = start.elapsed();
        
        // REQUIRED: Log performance metrics
        if latency > Duration::from_micros(100) {
            warn!(
                latency_us = latency.as_micros(),
                order_id = %order.id,
                "Order execution exceeded 100μs target"
            );
        }
        
        // REQUIRED: Update performance metrics
        self.metrics.record_latency("order_execution", latency);
        
        Ok(result)
    }
}
```

### Memory Performance Standards

**MANDATORY Requirements:**
```rust
// ✅ REQUIRED: Pre-allocated memory pools
#[derive(Debug)]
pub struct TradingEngine {
    // Pre-allocated order pool (no runtime allocation)
    order_pool: MemoryPool<Order>,
    // Pre-allocated message buffers  
    message_buffers: Vec<MessageBuffer>,
    // NUMA-aware allocations for multi-socket systems
    numa_allocator: NumaAllocator,
}

impl TradingEngine {
    pub fn new() -> Result<Self, InitError> {
        // REQUIRED: All memory allocated at startup
        let order_pool = MemoryPool::pre_allocate(10_000)?;
        let message_buffers = (0..1000)
            .map(|_| MessageBuffer::with_capacity(4096))
            .collect();
        
        Ok(Self {
            order_pool,
            message_buffers,
            numa_allocator: NumaAllocator::new()?,
        })
    }
    
    pub fn process_order(&mut self, order_data: &[u8]) -> Result<(), ProcessError> {
        // ✅ REQUIRED: Use pre-allocated memory
        let order = self.order_pool.acquire()?;
        order.deserialize_from(order_data)?;
        
        // Process without additional allocations
        self.execute_order(order)?;
        
        // Return to pool
        self.order_pool.release(order);
        Ok(())
    }
}
```

---

## 🧪 Safety and Testing Standards

### Testing Architecture

**MANDATORY Test Categories:**
1. **Unit Tests** - Component-level functionality
2. **Integration Tests** - Cross-component interactions  
3. **Performance Tests** - Latency and throughput validation
4. **Property Tests** - Mathematical invariant verification
5. **Chaos Tests** - System resilience under failure conditions
6. **Unsafe Code Tests** - Memory safety verification

**REQUIRED Test Implementation:**
```rust
// ✅ REQUIRED: Property-based testing for financial logic
#[cfg(test)]
mod tests {
    use quickcheck::{quickcheck, TestResult};
    use super::*;
    
    #[quickcheck]
    fn order_book_invariants_hold(orders: Vec<Order>) -> TestResult {
        let mut book = OrderBook::new("BTCUSDT");
        
        for order in orders {
            if let Err(_) = book.add_order(order) {
                return TestResult::discard();
            }
        }
        
        // REQUIRED: Test financial invariants
        let (best_bid, best_ask) = book.get_bbo();
        match (best_bid, best_ask) {
            (Some(bid), Some(ask)) => TestResult::from_bool(bid.price <= ask.price),
            _ => TestResult::passed(),
        }
    }
    
    #[test]
    fn test_order_execution_latency() {
        let executor = OrderExecutor::new();
        let order = create_test_order();
        
        let start = Instant::now();
        let result = executor.execute_order(order);
        let latency = start.elapsed();
        
        // REQUIRED: Performance assertions
        assert!(result.is_ok());
        assert!(latency < Duration::from_micros(100), 
                "Order execution took {:?}, exceeds 100μs limit", latency);
    }
}
```

### Unsafe Code Testing Standards

**MANDATORY for Unsafe Code:**
```bash
# REQUIRED: Miri testing for memory safety
cargo +nightly miri test --features hft-unsafe --package shriven-q

# REQUIRED: Loom testing for concurrency
cargo test --features hft-unsafe,loom-testing  

# REQUIRED: AddressSanitizer builds
RUSTFLAGS="-Z sanitizer=address" cargo build --features hft-unsafe
RUSTFLAGS="-Z sanitizer=address" cargo test --features hft-unsafe

# REQUIRED: Thread sanitizer for data races
RUSTFLAGS="-Z sanitizer=thread" cargo test --features hft-unsafe

# REQUIRED: Memory leak detection
RUSTFLAGS="-Z sanitizer=leak" cargo test --features hft-unsafe
```

#### Unsafe Code Review Checklist

**Mandatory Requirements:**
- [ ] **Feature gating**: Code is behind `hft-unsafe` feature flag
- [ ] **Safety documentation**: Every `unsafe fn` has comprehensive safety docs
- [ ] **Explicit unsafe blocks**: No `unsafe fn` without inner `unsafe` blocks
- [ ] **Justification**: Performance requirement justifies unsafe usage
- [ ] **Alternative evaluation**: Safe alternatives considered and documented
- [ ] **Testing coverage**: Miri and Loom tests included where applicable

**Architecture Validation:**
- [ ] **Memory safety**: No possible use-after-free or double-free
- [ ] **Data race freedom**: Proper synchronization for shared data
- [ ] **ABA prevention**: Generation counters or hazard pointers used
- [ ] **Alignment requirements**: All pointer operations respect alignment
- [ ] **Error handling**: Unsafe operations have proper error paths

**MANDATORY Unsafe Test Pattern:**
```rust
#[cfg(all(test, feature = "hft-unsafe", loom))]
mod loom_tests {
    use loom::sync::Arc;
    use loom::thread;
    
    #[test]
    fn test_lock_free_order_book_concurrent_access() {
        loom::model(|| {
            let book = Arc::new(LockFreeOrderBook::new());
            let book1 = Arc::clone(&book);
            let book2 = Arc::clone(&book);
            
            let t1 = thread::spawn(move || {
                let order = Order::new(Price(50000), Quantity(100), Side::Bid);
                unsafe { book1.add_order_unchecked(order) }
            });
            
            let t2 = thread::spawn(move || {
                let order = Order::new(Price(50001), Quantity(50), Side::Ask);  
                unsafe { book2.add_order_unchecked(order) }
            });
            
            let order_id1 = t1.join().unwrap();
            let order_id2 = t2.join().unwrap();
            
            // Verify both orders were added successfully
            assert_ne!(order_id1, order_id2);
        });
    }
}
```

### Performance Justification Framework

**MANDATORY Performance Validation for Unsafe Code:**

| Operation | Safe Implementation | Unsafe Implementation | Required Justification |
|-----------|-------------------|----------------------|----------------------|
| Memory allocation | `Vec::with_capacity()` ~500ns | Lock-free pool ~50ns | 10x improvement critical for HFT |
| NUMA allocation | Standard allocator | NUMA-aware ~100ns | Memory locality for sub-microsecond latency |
| Object pooling | `VecDeque` with mutex | Slab allocator ~20ns | Zero allocation during trading hours |

**REQUIRED Benchmark Pattern:**
```rust
#[bench]
fn bench_safe_vs_unsafe_allocation(b: &mut Bencher) {
    // Benchmark safe implementation
    let safe_time = b.iter(|| {
        let _order = Box::new(Order::default());
    });
    
    // Benchmark unsafe implementation
    let unsafe_time = b.iter(|| {
        unsafe { pool.allocate::<Order>() }
    });
    
    // REQUIRED: Document performance improvement
    assert!(unsafe_time < safe_time / 5, 
            "Unsafe implementation must be 5x faster to justify complexity");
}
            // Verify concurrent operations maintain invariants
            assert_ne!(order_id1, order_id2);
        });
    }
}
```

---

## 🏷️ Naming Conventions

### Project Structure Naming

**MANDATORY Directory Structure:**
```
shriven_q/                          # Root project: snake_case
├── src/
│   ├── core/                       # Major modules: snake_case
│   │   ├── execution/              # Features: snake_case
│   │   ├── memory/                 # Systems: snake_case
│   │   └── networking/             # Capabilities: snake_case
│   ├── engines/                    # Major subsystems: snake_case
│   │   ├── crypto/                 # Asset classes: snake_case
│   │   ├── equity/                 # Markets: snake_case
│   │   └── options/                # Instruments: snake_case
│   └── infrastructure/             # System layers: snake_case
├── scripts/
│   ├── build/                     # Build automation
│   └── deployment/                # Deployment automation
└── docs/
    ├── standards/                 # Project standards
    ├── architecture/              # Design documents
    └── development/               # Developer guides
```

### Rust Code Naming Standards

**MANDATORY Rust Naming:**
```rust
// Types: PascalCase with domain clarity
pub struct OrderBook { }             // Core entities
pub struct PriceLevel { }            // Components  
pub struct MarketDataFeed { }        // Systems
pub struct CudaKernelManager { }     // Hardware managers

// Enums: PascalCase with descriptive variants
pub enum ExecutionMode {
    Backtest,                        // Simple variants: PascalCase
    Simulation,
    Paper,
    Live,
}

pub enum AssetClass {
    Crypto,                         // Market categories: established terms
    Equity, 
    Options,
    Futures,
}

// Functions: snake_case with action verbs
pub fn add_order() -> OrderId { }           // Actions: verb + object
pub fn calculate_pnl() -> f64 { }           // Calculations: calculate_
pub fn get_best_bid() -> Option<Price> { }  // Getters: get_
pub fn is_market_open() -> bool { }         // Predicates: is_/has_/can_

// Performance-critical functions: hint at characteristics
pub fn fast_order_insert() -> u64 { }      // fast_ prefix for hot paths
pub fn lock_free_price_read() -> Price { } // lock_free_ for atomic ops
pub fn gpu_accelerated_pnl() -> f64 { }    // gpu_ prefix for GPU functions

// Constants: SCREAMING_SNAKE_CASE with descriptive context
pub const MAX_ORDERS_PER_SECOND: u32 = 100_000;   // Limits: MAX_
pub const MIN_ORDER_SIZE: f64 = 0.01;             // Limits: MIN_
pub const DEFAULT_TIMEOUT_MS: u64 = 5_000;        // Defaults: DEFAULT_

// Financial constants: include market context  
pub const NSE_TICK_SIZE: f64 = 0.05;             // Exchange specific
pub const BINANCE_MAX_LEVERAGE: f64 = 125.0;     // Platform limits
```

### Configuration and Environment Naming

**MANDATORY Environment Variables:**
```bash
# Environment: SCREAMING_SNAKE_CASE with namespace prefix
SHRIVEN_Q_EXECUTION_MODE=live              # Core configuration
SHRIVEN_Q_LOG_LEVEL=info                   # System settings
SHRIVEN_Q_CUDA_DEVICES=0,1,2               # Hardware configuration

# Exchange credentials: EXCHANGE_PURPOSE pattern
BINANCE_SPOT_API_KEY=xxx                   # Exchange API keys
BINANCE_FUTURES_API_SECRET=xxx             # Credential types
ZERODHA_API_KEY=xxx                        # Platform credentials

# Performance settings: SHRIVEN_Q_PERF prefix
SHRIVEN_Q_PERF_MAX_LATENCY_US=100         # Performance limits
SHRIVEN_Q_PERF_THREAD_COUNT=16            # Resource allocation
```

### Database and Storage Naming

**MANDATORY Database Naming:**
```sql
-- Tables: snake_case with descriptive names
CREATE TABLE market_data_ticks (
    id BIGSERIAL PRIMARY KEY,
    symbol VARCHAR(20) NOT NULL,
    timestamp_nanos BIGINT NOT NULL,
    price_ticks BIGINT NOT NULL,
    quantity_lots INTEGER NOT NULL
);

-- Indexes: idx_ prefix with table_column pattern  
CREATE INDEX idx_market_data_symbol_time ON market_data_ticks(symbol, timestamp_nanos);
CREATE INDEX idx_orders_status_time ON orders(status, created_at);
```

---

## 🔍 Troubleshooting and Debugging

### Common Build Issues and Solutions

#### Clippy Restriction Group Errors
```bash
# Symptom: 1000+ contradictory clippy errors
error: this lint expectation is unfulfilled
 --> src/core/memory/mod.rs:1:1
  |
1 | #![expect(clippy::restriction)]
  |   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

# Solution: Remove restriction group, use workspace lint tables
```

#### Dependency Version Conflicts
```bash
# Symptom: Version resolution failures
error: failed to select a version for `tower`
  required by package `tonic v0.14.0`

# Solution: Use unified version resolution
[workspace.dependencies]
tower = "0.5.1"
```

#### Feature Flag Combinations
```bash
# Symptom: Feature combinations causing build failures
error[E0432]: unresolved import `crate::core::memory::lock_free_pool`

# Solution: Proper feature gating
#[cfg(feature = "hft-unsafe")]
pub mod lock_free_pool;
```

### Debug Build Scripts
```bash
# Enable verbose output for debugging
export RUST_LOG=debug
export CARGO_LOG=cargo::core::compiler::fingerprint=info

# Run with maximum verbosity
cargo build --verbose --verbose
```

---

## 📚 Documentation Standards

### Code Documentation Requirements

**MANDATORY Documentation Patterns:**
```rust
/// High-performance order book implementation optimized for HFT
/// 
/// This order book supports both safe and unsafe operation modes:
/// - Safe mode: Uses BTreeMap with ~1ms latency, suitable for retail trading
/// - Unsafe mode: Uses lock-free structures with <10μs latency for HFT
/// 
/// # Performance Characteristics
/// 
/// - Order insertion: O(log n) safe mode, O(1) unsafe mode
/// - Best bid/ask query: O(log n) safe mode, O(1) unsafe mode  
/// - Memory usage: 64 bytes per price level + order overhead
/// 
/// # Safety Requirements (Unsafe Mode)
/// 
/// When using unsafe mode (`hft-unsafe` feature):
/// - Orders must be validated before insertion
/// - Thread affinity should be set for optimal NUMA performance
/// - Memory barriers are handled automatically
/// 
/// # Examples
/// 
/// ```rust
/// // Safe mode (default)
/// let mut book = OrderBook::new_safe("BTCUSDT");
/// let order_id = book.add_order(Order::new(50000.0, 1.0, Side::Bid))?;
/// 
/// // Unsafe mode (requires feature flag)
/// #[cfg(feature = "hft-unsafe")]
/// {
///     let book = OrderBook::new_lockfree("BTCUSDT");
///     // Validation must be done externally in unsafe mode
///     if order.is_valid() {
///         let order_id = unsafe { book.add_order_unchecked(order) };
///     }
/// }
/// ```
pub struct OrderBook {
    mode: OrderBookMode,
    // Implementation details...
}
```

**MANDATORY Safety Documentation:**
```rust
/// Add an order to the book without validation checks
/// 
/// # Safety
/// 
/// This function is unsafe because it bypasses all validation checks
/// for maximum performance. The caller MUST ensure:
/// 
/// 1. **Price Validity**: Order price is within valid range (0 < price < MAX_PRICE)
/// 2. **Quantity Validity**: Order quantity is positive and within limits
/// 3. **Symbol Matching**: Order symbol matches the book's symbol
/// 4. **Unique Order ID**: Order ID has not been used before
/// 5. **Thread Safety**: Only one thread modifies this book at a time
/// 6. **Memory Ordering**: All previous writes to the order are visible
/// 
/// Violating any of these conditions results in undefined behavior,
/// potentially including:
/// - Data corruption in the order book
/// - Inconsistent market data
/// - Financial losses due to incorrect pricing
/// - System crashes or memory corruption
/// 
/// # Performance Guarantees
/// 
/// When safety requirements are met:
/// - Insertion time: < 1μs on modern hardware (Intel Xeon 3.2GHz)
/// - Memory overhead: Exactly 64 bytes per order
/// - Lock-free: Never blocks other threads
/// 
/// # Implementation Notes
/// 
/// Uses hazard pointers for safe memory reclamation and atomic operations
/// with acquire-release ordering to maintain consistency across cores.
/// 
/// # Examples
/// 
/// ```rust
/// # #[cfg(feature = "hft-unsafe")]
/// # {
/// let order = Order::new(50000.0, 1.0, Side::Bid);
/// // REQUIRED: Validate before unsafe call
/// assert!(order.price > 0.0 && order.price < MAX_PRICE);
/// assert!(order.quantity > 0.0);
/// assert!(order.symbol == book.symbol());
/// 
/// let order_id = unsafe { book.add_order_unchecked(order) };
/// # }
/// ```
pub unsafe fn add_order_unchecked(&self, order: Order) -> OrderId {
    // Implementation...
}
```

### Architecture Documentation Standards

**MANDATORY Architecture Documentation:**
Each major system must have comprehensive architecture documentation including:

1. **System Overview** - High-level purpose and goals
2. **Performance Characteristics** - Latency, throughput, resource usage
3. **Safety Analysis** - Risk assessment and mitigation strategies
4. **Integration Points** - How it connects to other systems
5. **Failure Modes** - What can go wrong and how it's handled
6. **Monitoring and Observability** - How to track system health
7. **Scaling Characteristics** - How performance changes with load

### Documentation Change Tracking

**MANDATORY Documentation Validation:**
```bash
# Run documentation validation before commits
./scripts/build/validate_documentation.sh

# Manual validation commands
cargo run --bin doc-tracker validate --docs-path docs --verbose
cargo run --bin doc-tracker metrics --docs-path docs --format markdown
```

**REQUIRED Git Hooks:**
```bash
# .githooks/pre-commit - Automatic documentation validation
if git diff --cached --name-only | grep -E '\.(md|rst)$' > /dev/null; then
    echo "📚 Documentation changes detected, running validation..."
    ./scripts/build/validate_documentation.sh || {
        echo "❌ Documentation validation failed"
        exit 1
    }
fi
```

**Documentation Health Metrics:**
- Broken references must be 0
- All files must have proper headings
- No orphaned files (unreferenced documentation)
- Performance metrics must be consistent across documents

---

## 🔍 Error Handling Standards

### Structured Error Hierarchy

**MANDATORY Error Architecture:**
```rust
// ✅ REQUIRED: Top-level error enum for the entire trading system
#[derive(Debug, thiserror::Error)]
pub enum TradingSystemError {
    #[error("Market data error: {0}")]
    MarketData(#[from] MarketDataError),
    
    #[error("Order execution error: {0}")]
    Execution(#[from] ExecutionError),
    
    #[error("Risk management error: {0}")]
    Risk(#[from] RiskError),
    
    #[error("Infrastructure error: {0}")]
    Infrastructure(#[from] InfrastructureError),
}

// ✅ REQUIRED: Detailed error types for each subsystem
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Order validation failed: {reason}")]
    ValidationFailed { reason: String },
    
    #[error("Insufficient funds: need {required}, have {available}")]
    InsufficientFunds { required: f64, available: f64 },
    
    #[error("Market closed for symbol {symbol}")]
    MarketClosed { symbol: String },
    
    #[error("Exchange connection lost: {exchange}")]
    ConnectionLost { exchange: String },
    
    #[error("Order size {size} exceeds maximum {max} for symbol {symbol}")]
    OrderSizeExceeded { size: f64, max: f64, symbol: String },
}
```

### Error Recovery Strategies

**MANDATORY Recovery Patterns:**
```rust
impl TradingEngine {
    pub fn execute_order_with_retry(&self, order: Order) -> Result<Execution, TradingSystemError> {
        const MAX_RETRIES: usize = 3;
        const RETRY_DELAY: Duration = Duration::from_millis(100);
        
        for attempt in 1..=MAX_RETRIES {
            match self.execute_order(&order) {
                Ok(execution) => return Ok(execution),
                
                Err(TradingSystemError::Infrastructure(InfrastructureError::NetworkTimeout)) => {
                    // REQUIRED: Log retry attempts with context
                    warn!(
                        attempt = attempt,
                        max_retries = MAX_RETRIES,
                        order_id = %order.id,
                        "Network timeout, retrying order execution"
                    );
                    
                    if attempt < MAX_RETRIES {
                        std::thread::sleep(RETRY_DELAY);
                        continue;
                    }
                }
                
                Err(TradingSystemError::Risk(risk_error)) => {
                    // REQUIRED: No retry for risk violations
                    error!(
                        order_id = %order.id,
                        risk_error = %risk_error,
                        "Order rejected due to risk limits, no retry"
                    );
                    return Err(TradingSystemError::Risk(risk_error));
                }
                
                Err(other_error) => {
                    // REQUIRED: Immediate failure for other errors
                    return Err(other_error);
                }
            }
        }
        
        Err(TradingSystemError::Infrastructure(
            InfrastructureError::MaxRetriesExceeded { 
                operation: "execute_order".to_string(),
                attempts: MAX_RETRIES 
            }
        ))
    }
}
```

---

## 📊 Monitoring and Observability Standards

### Performance Metrics Standards

**MANDATORY Metrics Collection:**
```rust
use prometheus::{Counter, Histogram, Gauge};

// ✅ REQUIRED: Comprehensive metrics for all trading operations
pub struct TradingMetrics {
    // Counters for event tracking
    orders_submitted: Counter,
    orders_filled: Counter,
    orders_rejected: Counter,
    
    // Histograms for latency tracking  
    order_latency: Histogram,
    market_data_latency: Histogram,
    risk_check_latency: Histogram,
    
    // Gauges for current state
    open_positions: Gauge,
    available_margin: Gauge,
    connection_status: Gauge,
}

impl TradingMetrics {
    pub fn record_order_execution(&self, latency: Duration, success: bool) {
        // REQUIRED: Record both latency and success/failure
        self.order_latency.observe(latency.as_secs_f64());
        
        if success {
            self.orders_filled.inc();
        } else {
            self.orders_rejected.inc();
        }
    }
    
    pub fn update_position_metrics(&self, positions: &[Position]) {
        // REQUIRED: Update current system state
        self.open_positions.set(positions.len() as f64);
        
        let total_exposure = positions.iter()
            .map(|p| p.market_value.abs())
            .sum::<f64>();
        self.available_margin.set(self.calculate_available_margin(total_exposure));
    }
}
```

### Structured Logging Standards

**MANDATORY Logging Patterns:**
```rust
// ✅ REQUIRED: Structured logging with consistent field names
impl OrderExecutor {
    pub fn process_order(&self, order: Order) -> Result<Execution, ExecutionError> {
        let order_start = Instant::now();
        
        // REQUIRED: Log order start with full context
        info!(
            order_id = %order.id,
            symbol = %order.symbol,
            side = ?order.side,
            price = %order.price,
            quantity = %order.quantity,
            order_type = ?order.order_type,
            client_id = %order.client_id,
            timestamp = %order.timestamp.to_rfc3339(),
            "Order processing started"
        );
        
        // Execute order logic...
        let result = self.execute_internal(&order);
        
        let execution_time = order_start.elapsed();
        
        match &result {
            Ok(execution) => {
                // REQUIRED: Log successful execution with timing
                info!(
                    order_id = %order.id,
                    execution_id = %execution.id,
                    executed_price = %execution.price,
                    executed_quantity = %execution.quantity,
                    execution_time_us = execution_time.as_micros(),
                    fees = %execution.fees,
                    "Order executed successfully"
                );
            }
            
            Err(error) => {
                // REQUIRED: Log errors with full diagnostic context
                error!(
                    order_id = %order.id,
                    error_type = %error,
                    execution_time_us = execution_time.as_micros(),
                    symbol = %order.symbol,
                    attempted_price = %order.price,
                    attempted_quantity = %order.quantity,
                    market_conditions = ?self.get_market_snapshot(&order.symbol),
                    "Order execution failed"
                );
            }
        }
        
        result
    }
}
```

---

## 👥 Review and Compliance Standards

### Code Review Requirements

**MANDATORY Review Checklist:**

#### All Code Reviews Must Verify:
- [ ] **Performance Impact**: No performance regressions introduced
- [ ] **Memory Safety**: No memory leaks or unsafe memory access
- [ ] **Error Handling**: All error paths properly handled with context
- [ ] **Documentation**: Public APIs fully documented with examples
- [ ] **Testing**: Adequate test coverage including edge cases
- [ ] **Naming Conventions**: Consistent with established standards
- [ ] **Logging**: Appropriate structured logging for debugging
- [ ] **Metrics**: Performance metrics collection where appropriate

#### Unsafe Code Reviews Must Additionally Verify:
- [ ] **Safety Documentation**: Comprehensive safety invariants documented
- [ ] **Unsafe Justification**: Performance benefit justifies unsafe usage
- [ ] **Alternative Analysis**: Safe alternatives evaluated and documented
- [ ] **Hazard Analysis**: Potential failure modes identified and mitigated
- [ ] **Test Coverage**: Miri, Loom, and sanitizer tests included
- [ ] **Scope Minimization**: Unsafe code isolated to minimal surface area

### Compliance Verification

**MANDATORY Pre-Release Checklist:**
```bash
# ✅ REQUIRED: Complete validation pipeline
#!/bin/bash

echo "🔍 ShrivenQ Release Compliance Validation"

# Performance regression testing
cargo bench --features hft-unsafe > current_benchmarks.txt
if ! ./scripts/compare_benchmarks.sh baseline.txt current_benchmarks.txt; then
    echo "❌ Performance regression detected"
    exit 1
fi

# Memory safety validation for unsafe code
if ! cargo +nightly miri test --features hft-unsafe; then
    echo "❌ Miri memory safety validation failed"
    exit 1
fi

# Concurrency validation
if ! cargo test --features hft-unsafe,loom-testing; then
    echo "❌ Loom concurrency validation failed"  
    exit 1
fi

# Documentation completeness
if ! cargo doc --features hft-unsafe --no-deps; then
    echo "❌ Documentation generation failed"
    exit 1
fi

# Security audit
if ! cargo audit; then
    echo "❌ Security audit failed"
    exit 1
fi

echo "✅ All compliance checks passed"
```

### Performance Regression Prevention

**MANDATORY Performance Gates:**
```rust
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::{Duration, Instant};
    
    #[test]
    fn test_order_execution_latency_regression() {
        let executor = OrderExecutor::new();
        let test_orders: Vec<Order> = create_test_orders(1000);
        
        let start = Instant::now();
        
        for order in test_orders {
            executor.execute_order(order).expect("Order execution failed");
        }
        
        let total_time = start.elapsed();
        let avg_latency = total_time / 1000;
        
        // REQUIRED: Hard performance limits
        assert!(
            avg_latency < Duration::from_micros(100),
            "Average order execution latency {} exceeds 100μs limit",
            avg_latency.as_micros()
        );
        
        // REQUIRED: Log performance for trending
        println!(
            "PERFORMANCE_METRIC: order_execution_avg_latency_us={}",
            avg_latency.as_micros()
        );
    }
}
```

---

## 📈 Continuous Improvement

### Migration Guide from Legacy Practices

#### Step 1: Remove CLI Lint Overrides
```diff
# scripts/build/build_strict_sequential.sh
- export CLIPPY_FLAGS="-W clippy::restriction -W clippy::nursery -D warnings"
+ export CLIPPY_FLAGS="-D warnings"
```

#### Step 2: Add Workspace Lint Tables
```diff
# Cargo.toml
+ [lints.rust]
+ unsafe_code = "deny"
+ unused = { level = "warn", priority = -1 }
+
+ [lints.clippy]
+ all = { level = "warn", priority = -1 }
+ restriction = { level = "allow", priority = -1 }
```

#### Step 3: Implement Feature-Gated Architecture
```diff
# src/core/memory/mod.rs
+ #![cfg_attr(feature = "hft-unsafe", allow(unsafe_code))]
+ #![deny(unsafe_op_in_unsafe_fn)]
+ #![deny(clippy::missing_safety_doc)]
```

### Standards Evolution Process

**MANDATORY Review Cycle:**
1. **Monthly**: Review performance metrics and identify bottlenecks
2. **Quarterly**: Update standards based on new Rust features and industry practices  
3. **Post-Incident**: Update standards based on production issues or near-misses
4. **Annual**: Comprehensive review of all standards with external audit

### Learning Integration

**MANDATORY Lessons Learned Integration:**
- All production incidents must result in updated standards or tooling
- Performance optimizations must be codified as standard practices
- Security vulnerabilities must result in mandatory security reviews
- Developer pain points must result in tooling or process improvements

### Quarterly Lint Review Process

1. **Audit new clippy lints** in Rust releases
2. **Evaluate restriction group additions** for cherry-picking
3. **Review module-level overrides** for consistency
4. **Update documentation** with new patterns

### Monitoring and Metrics

- Track clippy warning counts in CI
- Monitor build time impacts of lint changes
- Measure developer productivity metrics
- Review lint violation patterns

---

## 🔧 Troubleshooting and Debugging

### Common Build Issues and Solutions

#### 1. Clippy Restriction Group Errors

```bash
# Symptom: 1000+ contradictory clippy errors
error: this lint expectation is unfulfilled
 --> src/core/memory/mod.rs:1:1
  |
1 | #![expect(clippy::restriction)]
  |   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

# Solution: Remove restriction group, use workspace lint tables
```

#### 2. Dependency Version Conflicts

```bash
# Symptom: Version resolution failures
error: failed to select a version for `tower`
  required by package `tonic v0.14.0`
  versions that meet the requirements `^0.5.0` are: 0.5.1, 0.5.0
  all possible versions conflict with previously selected packages

# Solution: Use unified version resolution
[workspace.dependencies]
tower = "0.5.1"
```

#### 3. Feature Flag Combinations

```bash
# Symptom: Feature combinations causing build failures
error[E0432]: unresolved import `crate::core::memory::lock_free_pool`
  --> src/core/memory/mod.rs:19:9

# Solution: Proper feature gating
#[cfg(feature = "hft-unsafe")]
pub mod lock_free_pool;
```

### Debug Build Scripts

```bash
# Enable verbose output for debugging
export RUST_LOG=debug
export CARGO_LOG=cargo::core::compiler::fingerprint=info

# Run with maximum verbosity
cargo build --verbose --verbose
```

---

## 📝 Documentation Change Tracking

### Documentation Reference Management

**MANDATORY Documentation Standards:**

#### Reference Tracking System

```rust
// ✅ REQUIRED: Track all documentation references
pub struct DocumentationReference {
    source_file: PathBuf,
    target_file: PathBuf,
    reference_type: ReferenceType,
    line_number: usize,
}

pub enum ReferenceType {
    DirectLink,           // markdown links
    CodeReference,        // See src/core/memory.rs
    ConfigValue,          // References to config values
    FunctionName,         // Function/struct/enum references
    PerformanceMetric,    // Performance targets/metrics
    BuildScript,          // Build script references
    FeatureFlag,          // Feature flag documentation
}
```

#### Documentation Validation Requirements

```bash
# ✅ REQUIRED: Validate documentation on every commit
#!/bin/bash

# Build documentation reference graph
cargo run --bin doc-tracker --features development-tools -- \
    scan --docs-path docs --output docs/.metadata/doc-graph.json --include-source

# Validate all references are correct
cargo run --bin doc-tracker --features development-tools -- \
    validate --docs-path docs --verbose

# Check for orphaned documentation
cargo run --bin doc-tracker --features development-tools -- \
    check-orphaned --docs-path docs
```

#### Documentation Update Propagation

**MANDATORY Process for Documentation Changes:**

1. **Before Changing**: Run doc-tracker to identify all references
2. **During Change**: Update all identified references
3. **After Change**: Validate no broken references exist
4. **Pre-Commit**: Automated validation via git hooks

```bash
# .git/hooks/pre-commit
#!/bin/bash

# Validate documentation consistency
if ! ./scripts/build/validate_documentation.sh; then
    echo "❌ Documentation validation failed"
    echo "Run './scripts/build/validate_documentation.sh' for details"
    exit 1
fi
```

---

## 🎯 Summary and Enforcement

These development standards are **MANDATORY** for all ShrivenQ development. Non-compliance will result in:

1. **Automatic build failures** for lint violations
2. **Code review rejection** for standards violations  
3. **Performance gate failures** for regression introduction
4. **Documentation requirements** for unsafe code usage

**Remember the Core Principles:**
- **Latency First** - Every microsecond matters in HFT
- **Safe by Default** - Unsafe is opt-in only  
- **Measure Everything** - Data-driven optimization
- **Learn from Mistakes** - Continuous improvement based on real experience

The clippy configuration crisis taught us that **tooling must serve the developer, not the other way around**. These standards prioritize developer productivity while maintaining institutional-grade code quality and performance.

---

**Document Version:** 1.0  
**Last Updated:** 2025-08-20  
**Authority:** ShrivenQ Development Team  
**Review Schedule:** Quarterly with Rust ecosystem updates  
**Compliance:** Mandatory for all production code