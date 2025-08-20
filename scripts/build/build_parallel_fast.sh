#!/usr/bin/env bash
#
# build_parallel_fast.sh - Fast parallel build using all CPU cores
# Optimized for quick iteration during development
#

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}⚡ ShrivenQ Fast Parallel Build${NC}"
echo "================================"

cd "$PROJECT_ROOT"

# Detect number of CPU cores
if [[ "$OSTYPE" == "darwin"* ]]; then
    CPU_CORES=$(sysctl -n hw.ncpu)
else
    CPU_CORES=$(nproc)
fi

echo "Using $CPU_CORES CPU cores for parallel build"

# Set parallel job count for cargo
export CARGO_BUILD_JOBS=$CPU_CORES
export CARGO_TARGET_DIR="target"

# Enable incremental compilation for speed
export CARGO_INCREMENTAL=1

# Use faster linker if available
if command -v mold &> /dev/null; then
    export RUSTFLAGS="-C link-arg=-fuse-ld=mold"
    echo "Using mold linker for faster linking"
elif command -v lld &> /dev/null; then
    export RUSTFLAGS="-C link-arg=-fuse-ld=lld"
    echo "Using lld linker for faster linking"
fi

# Run parallel builds
echo -e "\n${YELLOW}Starting parallel builds...${NC}"

# Function to run build in background
build_target() {
    local target=$1
    local features=$2
    local name=$3
    
    echo "Building $name..."
    if [ -z "$features" ]; then
        cargo build --target-dir "target/$target" --release &
    else
        cargo build --target-dir "target/$target" --release --features "$features" &
    fi
}

# Start parallel builds
build_target "default" "" "Default configuration"
build_target "gpu" "gpu-acceleration" "GPU-accelerated build"
build_target "high-perf" "high-performance" "High-performance build"
build_target "full" "gpu-acceleration,high-performance,zerodha-integration,binance-integration" "Full features build"

# Wait for all background jobs
echo -e "\n${YELLOW}Waiting for parallel builds to complete...${NC}"
wait

# Run quick tests in parallel
echo -e "\n${YELLOW}Running parallel tests...${NC}"

cargo test --lib --bins --release --jobs=$CPU_CORES &
cargo test --doc --release --jobs=$CPU_CORES &

wait

echo -e "\n${GREEN}✓ Fast parallel build complete!${NC}"
echo "Build artifacts available in: target/"