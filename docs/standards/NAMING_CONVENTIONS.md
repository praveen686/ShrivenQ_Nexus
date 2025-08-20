# 🎯 **SHRIVENQ NAMING CONVENTION**
*Comprehensive Style Guide for Ultra-Consistent Codebase*

---

## 🎨 **CORE PRINCIPLES**

1. **Consistency Above All**: Same patterns everywhere, no exceptions
2. **Performance Awareness**: Names that hint at performance characteristics  
3. **Domain Clarity**: Financial trading concepts clearly expressed
4. **Tooling Friendly**: Works seamlessly with Rust analyzer, IDEs
5. **Team Scalability**: New developers can predict naming patterns

---

## 📁 **PROJECT STRUCTURE NAMING**

### **Directory Hierarchy**
```
shriven_q/                           # Root project: snake_case
├── core/                           # Major modules: snake_case
│   ├── execution/                  # Features: snake_case
│   ├── memory/                     # Systems: snake_case
│   └── networking/                 # Capabilities: snake_case
├── gpu/                            # Hardware: snake_case
│   ├── kernels/                    # Component groups: snake_case
│   ├── memory/                     # Subsystems: snake_case
│   └── runtime/                    # Operational: snake_case
├── engines/                        # Major subsystems: snake_case
│   ├── crypto/                     # Asset classes: snake_case
│   ├── equity/                     # Markets: snake_case
│   ├── options/                    # Instruments: snake_case
│   └── futures/                    # Products: snake_case
├── exchange_sim/                   # Combined concepts: snake_case
│   ├── nse_simulator/              # Specific implementations: snake_case
│   ├── binance_simulator/          # Exchange_type pattern: snake_case
│   └── market_impact/              # Domain concepts: snake_case
├── analytics/                      # Capabilities: snake_case
└── infrastructure/                 # System layers: snake_case
```

### **Crate Names (Cargo.toml)**
```toml
[package]
name = "shriven-q"                  # Main crate: kebab-case
version = "0.1.0"

[dependencies]
shriven-core = { path = "./core" }   # Sub-crates: kebab-case
shriven-gpu = { path = "./gpu" }     # Hardware modules: kebab-case
shriven-engines = { path = "./engines" } # Feature modules: kebab-case
shriven-analytics = { path = "./analytics" } # Capability modules: kebab-case
```

---

## 🦀 **RUST CODE NAMING**

### **Types (Structs, Enums, Traits)**
```rust
// Structs: PascalCase with domain clarity
pub struct OrderBook { }             // Core entities
pub struct PriceLevel { }            // Components  
pub struct MarketDataFeed { }        // Systems
pub struct CudaKernelManager { }     // Hardware managers
pub struct PortfolioOptimizer { }    // Algorithms

// Enums: PascalCase with descriptive variants
pub enum ExecutionMode {
    Backtest,                        // Simple variants: PascalCase
    Simulation,
    Paper,
    Live,
}

pub enum OrderSide {
    Bid,                            // Financial terms: standard naming
    Ask,
}

pub enum AssetClass {
    Crypto,                         // Market categories: established terms
    Equity, 
    Options,
    Futures,
}

// Traits: PascalCase, often ending in -able or describing capability
pub trait Executable { }            // Behavior: -able suffix
pub trait MarketConnector { }       // Role: descriptive name
pub trait RiskCalculator { }        // Function: -or/-er suffix
pub trait GpuAccelerated { }        # Capability: descriptive
```

### **Functions and Methods**
```rust
// Functions: snake_case with action verbs
pub fn add_order() -> OrderId { }           # Actions: verb + object
pub fn calculate_pnl() -> f64 { }           # Calculations: calculate_
pub fn get_best_bid() -> Option<Price> { }  # Getters: get_
pub fn set_risk_limit() -> Result<()> { }   # Setters: set_
pub fn is_market_open() -> bool { }         # Predicates: is_/has_/can_

// Performance-critical functions: hint at characteristics
pub fn fast_order_insert() -> u64 { }      # fast_ prefix for hot paths
pub fn lock_free_price_read() -> Price { } # lock_free_ for atomic ops
pub fn gpu_accelerated_var() -> f64 { }    # gpu_ prefix for GPU functions

// Async functions: descriptive without async suffix (clear from signature)
pub async fn connect_to_exchange() -> Result<Connection> { }
pub async fn stream_market_data() -> impl Stream<MarketUpdate> { }
```

### **Variables and Fields**
```rust
// Variables: snake_case with context
let order_id = generate_id();               # Clear purpose
let best_bid_price = book.get_best_bid();   # Descriptive
let total_portfolio_value = portfolio.value(); # Complete context

// Financial quantities: include units/type in name when ambiguous
let price_in_ticks = 50000;                # Units specified
let quantity_in_lots = 10;                 # Size units specified
let timestamp_nanos = get_time();          # Precision specified
let margin_requirement_usd = calculate_margin(); # Currency specified

// Performance-sensitive: hint at optimization
let cache_aligned_buffer = Buffer::aligned(); # Memory layout hint
let atomic_counter = AtomicU64::new(0);        # Concurrency hint
let lock_free_queue = SegQueue::new();         # Data structure hint
```

### **Constants and Statics**
```rust
// Constants: SCREAMING_SNAKE_CASE with descriptive context
pub const MAX_ORDERS_PER_SECOND: u32 = 100_000;   # Limits: MAX_
pub const MIN_ORDER_SIZE: f64 = 0.01;             # Limits: MIN_
pub const DEFAULT_TIMEOUT_MS: u64 = 5_000;        # Defaults: DEFAULT_
pub const NANOSECONDS_PER_SECOND: u64 = 1_000_000_000; # Conversions

// Hardware constants: include hardware context
pub const GPU_BLOCK_SIZE: usize = 256;            # CUDA block size
pub const CACHE_LINE_SIZE: usize = 64;            # CPU cache line
pub const NUMA_NODE_COUNT: usize = 2;             # System topology

// Financial constants: include market context  
pub const NSE_TICK_SIZE: f64 = 0.05;             # Exchange specific
pub const BINANCE_MAX_LEVERAGE: f64 = 125.0;     # Platform limits
pub const OPTIONS_EXPIRY_FRIDAY: u8 = 5;         # Market conventions
```

### **Modules**
```rust
// Modules: snake_case organized by domain/function
mod order_book;                    # Core functionality
mod market_data;                   # Data handling
mod execution_engine;              # Processing systems
mod risk_management;               # Business logic
mod gpu_kernels;                   # Hardware acceleration
mod exchange_simulation;           # Testing infrastructure

// Module organization within files
pub mod types {                    # Grouped by purpose
    pub use super::Price;
    pub use super::Quantity;
}

pub mod constants {                # Configuration values
    pub const MAX_PRICE: f64 = 1_000_000.0;
}

pub mod utils {                    # Helper functions
    pub fn format_price(price: Price) -> String { }
}
```

---

## 🌐 **NETWORK & SERVICES NAMING**

### **Service Names**
```rust
// Services: snake_case with clear responsibility
auth_service                       # Authentication & authorization  
market_data_service               # Market data aggregation
order_execution_service           # Order routing & execution
risk_management_service           # Risk monitoring & limits
portfolio_service                 # Portfolio tracking & analytics
gpu_compute_service              # GPU acceleration service

// Service endpoints: descriptive with version
/api/v1/auth/login               # REST endpoints: kebab-case
/api/v1/orders/submit            # Action-oriented
/api/v1/portfolio/positions      # Resource-oriented
/ws/market-data/btcusdt          # WebSocket: resource streams
/grpc/execution.v1.ExecutionService # gRPC: proto namespace
```

### **Environment Variables**
```bash
# Environment: SCREAMING_SNAKE_CASE with namespace prefix
SHRIVEN_Q_EXECUTION_MODE=live              # Core configuration
SHRIVEN_Q_LOG_LEVEL=info                   # System settings
SHRIVEN_Q_CUDA_DEVICES=0,1,2               # Hardware configuration
SHRIVEN_Q_NUMA_POLICY=interleave           # Performance tuning

# Exchange credentials: EXCHANGE_PURPOSE pattern
BINANCE_SPOT_API_KEY=xxx                   # Exchange API keys
BINANCE_FUTURES_API_SECRET=xxx             # Credential types
ZERODHA_API_KEY=xxx                        # Platform credentials
NSE_TRADING_SESSION_TOKEN=xxx              # Session management

# Performance settings: SHRIVEN_Q_PERF prefix
SHRIVEN_Q_PERF_MAX_LATENCY_US=100         # Performance limits
SHRIVEN_Q_PERF_THREAD_COUNT=16            # Resource allocation
SHRIVEN_Q_PERF_MEMORY_POOL_SIZE=1GB       # Memory management
```

---

## 🗃️ **FILE NAMING CONVENTIONS**

### **Source Files**
```
// Rust source: snake_case matching module names
src/
├── lib.rs                        # Library root
├── main.rs                       # Binary entry point
├── order_book.rs                 # Core modules
├── market_connector.rs           # Service modules
├── gpu_kernels.cu               # CUDA files: .cu extension
├── options_pricing.cu           # Specific GPU kernels
└── types/                       # Module directories
    ├── mod.rs                    # Module declarations
    ├── price.rs                  # Type definitions
    ├── order.rs                  # Domain objects
    └── timestamp.rs              # Utility types

// Test files: match source with test suffix
tests/
├── integration_tests.rs         # Integration testing
├── performance_benchmarks.rs    # Performance testing  
├── order_book_tests.rs          # Module-specific tests
└── property_tests.rs            # Property-based testing
```

### **Configuration Files**
```
// Configuration: kebab-case with descriptive purpose
config/
├── execution-modes.toml          # Execution configuration
├── gpu-settings.toml             # Hardware settings
├── network-config.toml           # Networking setup
├── exchange-endpoints.toml       # API endpoints
├── risk-limits.toml             # Risk management
└── performance-tuning.toml      # Optimization settings

// Docker and deployment: kebab-case
deployment/
├── shriven-q.dockerfile         # Main application
├── gpu-runtime.dockerfile       # GPU support
├── docker-compose.yml           # Development setup
└── kubernetes-manifests/        # K8s deployments
    ├── auth-service.yaml         # Service definitions
    ├── market-data-service.yaml  # Individual services
    └── ingress-config.yaml       # Network configuration
```

### **Documentation Files**
```
// Documentation: UPPERCASE or kebab-case as appropriate
docs/
├── README.md                     # Standard names: UPPERCASE
├── CONTRIBUTING.md               # Project standards
├── api-reference.md              # Technical docs: kebab-case
├── performance-guide.md          # User guides
├── deployment-guide.md           # Operational docs
└── architecture-overview.md     # Design documents
```

---

## 💾 **DATABASE & STORAGE NAMING**

### **Database Objects**
```sql
-- Tables: snake_case with descriptive names
CREATE TABLE market_data_ticks (      -- Data storage: singular/plural as appropriate
    id BIGSERIAL PRIMARY KEY,
    symbol VARCHAR(20) NOT NULL,
    timestamp_nanos BIGINT NOT NULL,
    price_ticks BIGINT NOT NULL,
    quantity_lots INTEGER NOT NULL
);

CREATE TABLE order_executions (       -- Events: plural nouns
    execution_id UUID PRIMARY KEY,
    order_id UUID NOT NULL,
    executed_price DECIMAL(18,8),
    executed_quantity DECIMAL(18,8),
    execution_timestamp TIMESTAMPTZ
);

-- Indexes: idx_ prefix with table_column pattern  
CREATE INDEX idx_market_data_symbol_time ON market_data_ticks(symbol, timestamp_nanos);
CREATE INDEX idx_orders_status_time ON orders(status, created_at);
```

### **Storage Paths**
```
# File storage: snake_case with hierarchy
data/
├── market_data/                   # Data categories
│   ├── binance/                   # Exchange-specific
│   │   ├── spot/                  # Product types
│   │   │   ├── btcusdt/           # Symbols
│   │   │   └── ethusdt/
│   │   └── futures/
│   └── zerodha/
├── backtest_results/              # Generated data
│   ├── strategy_001/              # Numbered strategies
│   └── strategy_002/
└── system_logs/                   # Operational data
    ├── execution_logs/            # Service-specific
    ├── risk_logs/
    └── performance_logs/
```

---

## 🧪 **TESTING NAMING CONVENTIONS**

### **Test Functions**
```rust
// Test functions: test_ prefix with descriptive scenario
#[test]
fn test_order_book_maintains_price_time_priority() { }

#[test] 
fn test_gpu_option_pricing_matches_cpu_calculation() { }

#[test]
fn test_risk_limits_prevent_excessive_leverage() { }

// Performance tests: bench_ prefix
#[bench]
fn bench_order_insertion_latency(b: &mut Bencher) { }

#[bench]
fn bench_portfolio_calculation_throughput(b: &mut Bencher) { }

// Property tests: prop_ prefix  
#[quickcheck]
fn prop_order_book_invariants_always_hold(orders: Vec<Order>) -> bool { }
```

### **Test Data**
```rust
// Test fixtures: descriptive with context
fn create_sample_btc_order() -> Order { }
fn generate_realistic_market_data() -> Vec<MarketTick> { }
fn mock_binance_api_response() -> ApiResponse { }

// Test constants: TEST_ prefix
const TEST_SYMBOL: &str = "BTCUSDT";
const TEST_INITIAL_BALANCE: f64 = 10_000.0;
const TEST_MAX_ITERATIONS: usize = 1_000;
```

---

## 🔧 **BUILD & TOOLING NAMING**

### **Cargo Features**
```toml
# Features: kebab-case with clear purpose
[features]
default = ["gpu-acceleration", "zerodha-integration"]
gpu-acceleration = ["cuda-rs", "cudarc"]           # Hardware features
zerodha-integration = ["kiteconnect"]              # Exchange features  
binance-integration = ["binance-rs"]               # Platform features
high-performance = ["jemalloc", "simd"]            # Performance features
development-tools = ["console-subscriber"]         # Development features
```

### **Workspace Organization**
```toml
# Workspace members: kebab-case matching directories
[workspace]
members = [
    "shriven-core",                 # Core functionality
    "shriven-gpu",                  # Hardware acceleration
    "shriven-engines",              # Trading engines  
    "shriven-analytics",            # Analysis tools
    "shriven-infrastructure",       # System tools
]
```

---

## 🏷️ **TAGGING & VERSIONING**

### **Git Tags**
```bash
# Release tags: semantic versioning with prefix
v1.0.0                             # Major releases
v1.1.0                             # Minor releases  
v1.1.1                             # Patch releases
v2.0.0-beta.1                      # Pre-releases
v2.0.0-rc.1                        # Release candidates

# Feature tags: descriptive with purpose
feature/gpu-acceleration-v1        # Feature branches
hotfix/orderbook-deadlock-fix      # Hotfix branches
release/v1.1.0                     # Release branches
```

### **API Versioning**
```
# API versions: simple incremental
/api/v1/orders                     # First version
/api/v2/orders                     # Breaking changes
/api/v2.1/orders                   # Compatible additions

# Proto versions: namespace with version
shriven.q.trading.v1.OrderService  # gRPC services
shriven.q.market.v1.MarketData     # Proto messages
```

---

## 📊 **MONITORING & METRICS**

### **Metric Names**
```rust
// Metrics: snake_case with component prefix  
shriven_q_orders_per_second        # Rate metrics
shriven_q_latency_percentile_99    # Latency metrics
shriven_q_gpu_utilization_percent  # Hardware metrics
shriven_q_memory_allocated_bytes   # Resource metrics

// Counter naming: noun with action
shriven_q_orders_submitted_total   # Total counters
shriven_q_errors_occurred_total    # Error counters
shriven_q_connections_active_count # Current gauges
```

### **Log Fields**
```rust
// Structured logging: snake_case keys
info!(
    component = "order_execution",   # System component
    symbol = %symbol,               # Financial instrument  
    side = ?order_side,             # Enum values
    price_ticks = price.as_i64(),   # Numeric values
    latency_us = latency.as_micros(), # Performance metrics
    "Order executed successfully"    # Human-readable message
);
```

---

## ✅ **NAMING CONVENTION CHECKLIST**

### **Before Naming Anything**
- [ ] Does it follow the established pattern for its category?
- [ ] Is it immediately clear what this represents?  
- [ ] Does it include necessary context (units, precision, scope)?
- [ ] Will it conflict with Rust keywords or std library names?
- [ ] Is it consistent with similar items in the codebase?
- [ ] Does it hint at performance characteristics when relevant?
- [ ] Will new team members understand it without documentation?

### **Rust-Specific Checks**
- [ ] Types are PascalCase
- [ ] Functions/variables are snake_case  
- [ ] Constants are SCREAMING_SNAKE_CASE
- [ ] Modules are snake_case
- [ ] Crates are kebab-case
- [ ] Features are kebab-case

### **Domain-Specific Checks**
- [ ] Financial terms use standard market conventions
- [ ] Exchange names match their official naming
- [ ] Currency codes follow ISO 4217 when applicable  
- [ ] Time units are clearly specified
- [ ] Price/quantity units are unambiguous

---

## 🎯 **EXAMPLES BY CATEGORY**

### **✅ GOOD Examples**
```rust
// Types: Clear, descriptive, following conventions
pub struct OrderBook { }                    # Core entity
pub struct GpuPricingEngine { }             # Hardware-specific
pub enum ExecutionMode { Live, Paper }     # Clear variants

// Functions: Action-oriented with context
pub fn calculate_portfolio_var() -> f64 { } # Clear calculation  
pub fn stream_market_data() -> Stream { }   # Async operation
pub fn lock_free_price_update() -> bool { } # Performance hint

// Variables: Descriptive with units/context
let order_latency_us = measure_latency();   # Units specified
let best_bid_price = get_price();          # Context clear
let gpu_memory_allocated = alloc_gpu();     # Hardware context

// Constants: Descriptive with scope
const MAX_ORDERS_PER_SYMBOL: u32 = 1000;   # Clear limitation
const CUDA_THREADS_PER_BLOCK: usize = 256; # Hardware specific
```

### **❌ BAD Examples**
```rust
// Types: Ambiguous or inconsistent
pub struct Book { }                         # Too generic
pub struct orderbook { }                   # Wrong case  
pub struct GPU_Engine { }                  # Mixed case

// Functions: Unclear or inconsistent  
pub fn calc() -> f64 { }                   # Abbreviated
pub fn GetPrice() -> Price { }             # Wrong case
pub fn do_stuff() -> Result<()> { }        # Vague

// Variables: Ambiguous or unclear
let p = get_price();                       # Single letter
let orderLatency = measure();              # Wrong case
let thing = calculate();                   # Meaningless

// Constants: Inconsistent or unclear
const max_orders: u32 = 1000;              # Wrong case
const TIMEOUT = 5000;                      # Missing units
const X: f64 = 3.14159;                    # Meaningless name
```

---

This comprehensive naming convention ensures ShrivenQ will have the most consistent, maintainable, and professional codebase in the quantitative trading space. Every developer will instantly understand the patterns and be able to contribute effectively from day one.