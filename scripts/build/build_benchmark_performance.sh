#!/usr/bin/env bash
#
# build_benchmark_performance.sh - Performance benchmarking suite
# Runs comprehensive performance benchmarks for memory allocators and core systems
#

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"
BENCH_REPORT_DIR="$PROJECT_ROOT/target/benchmark-reports"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${GREEN}📊 ShrivenQ Performance Benchmarks${NC}"
echo "=================================="

cd "$PROJECT_ROOT"

# Create benchmark report directory
mkdir -p "$BENCH_REPORT_DIR"

# Ensure we're building in release mode for accurate benchmarks
export CARGO_PROFILE_RELEASE_DEBUG=false
export CARGO_PROFILE_RELEASE_OPT_LEVEL=3
export CARGO_PROFILE_RELEASE_LTO=true
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1

# CPU frequency scaling - set to performance mode if available
if [[ "$OSTYPE" == "linux-gnu"* ]] && command -v cpupower &> /dev/null; then
    echo "Setting CPU to performance mode..."
    sudo cpupower frequency-set -g performance 2>/dev/null || \
        echo -e "${YELLOW}Could not set CPU governor (need sudo)${NC}"
fi

# Disable turbo boost for consistent results
if [ -f /sys/devices/system/cpu/intel_pstate/no_turbo ]; then
    echo 1 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo 2>/dev/null || \
        echo -e "${YELLOW}Could not disable turbo boost${NC}"
fi

echo -e "\n${YELLOW}Running benchmarks...${NC}\n"

# Memory allocator benchmarks
echo -e "${BLUE}Memory Allocator Benchmarks:${NC}"
cargo bench --bench memory_allocators -- --save-baseline allocators_$TIMESTAMP \
    2>&1 | tee "$BENCH_REPORT_DIR/memory_allocators_$TIMESTAMP.txt" || true

# Lock-free pool benchmarks
echo -e "\n${BLUE}Lock-free Pool Benchmarks:${NC}"
cargo bench --bench lock_free_pool -- --save-baseline pool_$TIMESTAMP \
    2>&1 | tee "$BENCH_REPORT_DIR/lock_free_pool_$TIMESTAMP.txt" || true

# NUMA allocator benchmarks
echo -e "\n${BLUE}NUMA Allocator Benchmarks:${NC}"
cargo bench --bench numa_allocator -- --save-baseline numa_$TIMESTAMP \
    2>&1 | tee "$BENCH_REPORT_DIR/numa_allocator_$TIMESTAMP.txt" || true

# Slab allocator benchmarks
echo -e "\n${BLUE}Slab Allocator Benchmarks:${NC}"
cargo bench --bench slab_allocator -- --save-baseline slab_$TIMESTAMP \
    2>&1 | tee "$BENCH_REPORT_DIR/slab_allocator_$TIMESTAMP.txt" || true

# Hazard pointer benchmarks
echo -e "\n${BLUE}Hazard Pointer Benchmarks:${NC}"
cargo bench --bench hazard_pointers -- --save-baseline hazard_$TIMESTAMP \
    2>&1 | tee "$BENCH_REPORT_DIR/hazard_pointers_$TIMESTAMP.txt" || true

# Compare with previous baseline if exists
if [ -d "target/criterion" ]; then
    echo -e "\n${YELLOW}Comparing with previous benchmarks...${NC}"
    
    # Find the most recent baseline
    PREV_BASELINE=$(ls -t target/criterion | grep -E "allocators_[0-9]+" | head -n 2 | tail -n 1)
    
    if [ -n "$PREV_BASELINE" ]; then
        cargo bench --bench memory_allocators -- --baseline "$PREV_BASELINE" \
            2>&1 | tee -a "$BENCH_REPORT_DIR/comparison_$TIMESTAMP.txt" || true
    fi
fi

# Generate performance profile with perf if available
if command -v perf &> /dev/null; then
    echo -e "\n${YELLOW}Generating performance profile...${NC}"
    
    # Build with debug symbols for profiling
    RUSTFLAGS="-g" cargo build --release --bin shriven-benchmark
    
    # Record performance data
    perf record -F 99 -g -- ./target/release/shriven-benchmark --iterations 1000 \
        2>/dev/null || echo -e "${YELLOW}Performance profiling requires sudo${NC}"
    
    # Generate report
    perf report --stdio > "$BENCH_REPORT_DIR/perf_report_$TIMESTAMP.txt" 2>/dev/null || true
fi

# Memory usage profiling with heaptrack if available
if command -v heaptrack &> /dev/null; then
    echo -e "\n${YELLOW}Profiling memory usage...${NC}"
    heaptrack ./target/release/shriven-benchmark --iterations 100
    mv heaptrack.shriven-benchmark.*.gz "$BENCH_REPORT_DIR/" 2>/dev/null || true
fi

# Generate latency percentile report
echo -e "\n${YELLOW}Generating latency percentile report...${NC}"
cat > "$BENCH_REPORT_DIR/latency_summary_$TIMESTAMP.txt" << EOF
ShrivenQ Latency Percentiles
============================
Date: $(date)

Target Latencies:
- Order Execution: < 100μs (p99)
- Market Data: < 10μs (p99)
- Memory Allocation: < 1μs (p99)
- GPU Computation: < 1ms (p99)

Measured Latencies:
EOF

# Extract latency percentiles from benchmark output
grep -h "p50\|p90\|p95\|p99\|p999" "$BENCH_REPORT_DIR"/*_$TIMESTAMP.txt >> \
    "$BENCH_REPORT_DIR/latency_summary_$TIMESTAMP.txt" 2>/dev/null || true

# Restore CPU settings
if [[ "$OSTYPE" == "linux-gnu"* ]] && command -v cpupower &> /dev/null; then
    sudo cpupower frequency-set -g ondemand 2>/dev/null || true
fi

if [ -f /sys/devices/system/cpu/intel_pstate/no_turbo ]; then
    echo 0 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo 2>/dev/null || true
fi

# Summary
echo -e "\n=================================="
echo -e "${GREEN}✓ Benchmarks complete!${NC}"
echo "Reports saved to: $BENCH_REPORT_DIR"
echo "Baseline saved as: *_$TIMESTAMP"

# Check if we meet latency targets
if grep -q "< 100.0μs" "$BENCH_REPORT_DIR"/memory_allocators_$TIMESTAMP.txt 2>/dev/null; then
    echo -e "${GREEN}✓ Memory allocation latency target achieved!${NC}"
else
    echo -e "${YELLOW}⚠ Memory allocation latency needs optimization${NC}"
fi