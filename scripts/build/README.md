# 🔨 ShrivenQ Build Scripts

Comprehensive build system for ShrivenQ with various configurations optimized for different use cases.

## 📋 Quick Start

```bash
# Interactive menu
./scripts/build/build_master.sh

# Direct execution
./scripts/build/build_development_quick.sh  # Quick dev build
./scripts/build/build_strict_all.sh        # Full strict checks
./scripts/build/build_release_optimized.sh  # Production build
```

## 🚀 Available Build Scripts

### build_master.sh
**Purpose:** Master orchestrator with interactive menu  
**Usage:** `./build_master.sh [command]`  
**Features:**
- Interactive menu for all build types
- Parallel execution support
- Custom build configuration
- Timing and progress tracking

### build_development_quick.sh
**Purpose:** Fast development iteration  
**Time:** ~10-30 seconds  
**Checks:**
- Basic compilation
- Format verification
- Core unit tests
- Minimal warnings

### build_parallel_fast.sh
**Purpose:** Parallel builds using all CPU cores  
**Time:** ~1-2 minutes  
**Features:**
- Multi-core compilation
- Multiple feature combinations
- Optimized linker usage
- Concurrent test execution

### build_strict_all.sh
**Purpose:** Most comprehensive validation  
**Time:** ~5-10 minutes  
**Checks:**
- ALL Rust compiler warnings
- ALL Clippy lints (pedantic, nursery, cargo)
- Format validation
- Documentation build
- Security audit
- Dependency analysis
- Unused dependency detection

### build_release_optimized.sh
**Purpose:** Maximum performance production build  
**Time:** ~3-5 minutes  
**Optimizations:**
- Native CPU targeting
- Link-time optimization (LTO)
- Profile-guided optimization (PGO)
- Symbol stripping
- Binary compression

### build_test_comprehensive.sh
**Purpose:** Complete test suite execution  
**Time:** ~5-10 minutes  
**Tests:**
- Unit tests
- Integration tests
- Documentation tests
- Property-based tests
- Thread safety tests
- Memory leak detection
- Feature combination testing
- Coverage reporting

### build_benchmark_performance.sh
**Purpose:** Performance benchmarking  
**Time:** ~10-15 minutes  
**Benchmarks:**
- Memory allocator performance
- Lock-free pool throughput
- NUMA allocator efficiency
- Slab allocator speed
- Hazard pointer overhead
- Latency percentiles
- CPU profiling
- Memory profiling

### build_docker_container.sh
**Purpose:** Container image creation  
**Time:** ~5-10 minutes  
**Images:**
- Production (minimal)
- Development (with tools)
- GPU-enabled (CUDA support)

## 🎯 Build Configurations

### Strictest Flags Applied

```rust
-D warnings                      // All warnings as errors
-D rust-2018-idioms             // Enforce Rust 2018 idioms
-D missing-debug-implementations // Require Debug trait
-D unsafe-code                  // Forbid unsafe code
-D unused-imports               // No unused imports
-D dead-code                    // No dead code
-D clippy::all                  // All Clippy lints
-D clippy::pedantic            // Pedantic lints
-D clippy::nursery             // Experimental lints
-D clippy::cargo               // Cargo-related lints
```

### Performance Optimizations

```rust
-C target-cpu=native    // CPU-specific optimizations
-C opt-level=3         // Maximum optimization
-C lto=fat            // Link-time optimization
-C codegen-units=1    // Single codegen unit
-C panic=abort        // Smaller binaries
-C strip=symbols      // Remove debug symbols
```

## 📊 Parallel Execution

The build system supports parallel execution for:
- Multiple target builds
- Test suites
- Feature combinations
- Benchmark runs

## 🔍 Build Reports

All builds generate detailed reports in:
- `target/build-logs/` - Build output logs
- `target/test-reports/` - Test results
- `target/benchmark-reports/` - Performance data

## ⚡ Performance Tips

1. **Quick iteration:** Use `build_development_quick.sh`
2. **Pre-commit:** Run `build_strict_all.sh`
3. **Release:** Use `build_release_optimized.sh`
4. **CI/CD:** Integrate `build_test_comprehensive.sh`

## 🛠️ Custom Builds

For custom configurations, use the master script:

```bash
./build_master.sh
# Select option 8 for custom build
# Enter your RUSTFLAGS and cargo command
```

## 📈 Continuous Integration

Example GitHub Actions workflow:

```yaml
- name: Strict Build
  run: ./scripts/build/build_strict_all.sh

- name: Comprehensive Tests
  run: ./scripts/build/build_test_comprehensive.sh

- name: Benchmarks
  run: ./scripts/build/build_benchmark_performance.sh
```

## 🔧 Required Tools

**Essential:**
- Rust 1.75+
- Cargo

**Optional (for full features):**
- cargo-audit (security checks)
- cargo-outdated (dependency updates)
- cargo-machete (unused dependencies)
- cargo-tarpaulin (coverage)
- valgrind (memory checks)
- perf (profiling)
- Docker (containers)

## 📝 Naming Convention

All scripts follow the pattern:
`build_<category>_<description>.sh`

Categories:
- `development` - Dev builds
- `release` - Production builds
- `test` - Testing
- `benchmark` - Performance
- `docker` - Containerization
- `parallel` - Multi-core builds
- `strict` - Validation

## 🚦 Exit Codes

- `0` - Success
- `1` - Build/test failure
- `2` - Missing dependencies
- `3` - Invalid configuration

---

*Built for ShrivenQ - Ultra-Low Latency Trading Platform*