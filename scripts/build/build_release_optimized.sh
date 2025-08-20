#!/usr/bin/env bash
#
# build_release_optimized.sh - Production release build with maximum optimizations
# Creates the fastest possible binary for deployment
#

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"
RELEASE_DIR="$PROJECT_ROOT/target/optimized-release"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}🚀 ShrivenQ Optimized Release Build${NC}"
echo "===================================="

cd "$PROJECT_ROOT"

# Clean previous optimized build
rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

# Maximum optimization flags
export RUSTFLAGS="-C target-cpu=native \
    -C opt-level=3 \
    -C lto=fat \
    -C codegen-units=1 \
    -C embed-bitcode=yes \
    -C overflow-checks=no \
    -C debug-assertions=no \
    -C panic=abort \
    -C strip=symbols"

# Additional optimization for specific architectures
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Linux-specific optimizations
    export RUSTFLAGS="$RUSTFLAGS -C link-arg=-Wl,--gc-sections"
    
    # Use jemalloc for better memory performance
    export CARGO_FEATURE_FLAGS="--features jemalloc"
else
    export CARGO_FEATURE_FLAGS=""
fi

# Profile-guided optimization if available
if command -v llvm-profdata &> /dev/null; then
    echo -e "${YELLOW}Building with Profile-Guided Optimization (PGO)...${NC}"
    
    # Step 1: Build with profiling
    RUSTFLAGS="$RUSTFLAGS -C profile-generate=$RELEASE_DIR/pgo-data" \
        cargo build --release --target-dir "$RELEASE_DIR/pgo-build" $CARGO_FEATURE_FLAGS
    
    # Step 2: Run profiling workload
    echo "Running profiling workload..."
    "$RELEASE_DIR/pgo-build/release/shriven-q" benchmark --iterations 100 || true
    
    # Step 3: Process profile data
    llvm-profdata merge -o "$RELEASE_DIR/pgo-data/merged.profdata" "$RELEASE_DIR/pgo-data"
    
    # Step 4: Build with profile data
    RUSTFLAGS="$RUSTFLAGS -C profile-use=$RELEASE_DIR/pgo-data/merged.profdata" \
        cargo build --release --target-dir "$RELEASE_DIR" $CARGO_FEATURE_FLAGS
else
    echo -e "${BLUE}Building optimized release (without PGO)...${NC}"
    cargo build --release --target-dir "$RELEASE_DIR" $CARGO_FEATURE_FLAGS
fi

# Build all binaries
echo -e "\n${YELLOW}Building all binaries...${NC}"
cargo build --release --bins --target-dir "$RELEASE_DIR" $CARGO_FEATURE_FLAGS

# Strip symbols for smaller binaries (if not already done)
if command -v strip &> /dev/null; then
    echo "Stripping symbols from binaries..."
    find "$RELEASE_DIR/release" -type f -executable -exec strip {} \;
fi

# Compress binaries with UPX if available
if command -v upx &> /dev/null; then
    echo "Compressing binaries with UPX..."
    find "$RELEASE_DIR/release" -type f -executable -exec upx --best {} \; 2>/dev/null || true
fi

# Generate build info
BUILD_INFO="$RELEASE_DIR/build-info.txt"
echo "ShrivenQ Optimized Release Build" > "$BUILD_INFO"
echo "================================" >> "$BUILD_INFO"
echo "Build Date: $(date)" >> "$BUILD_INFO"
echo "Git Commit: $(git rev-parse HEAD 2>/dev/null || echo 'unknown')" >> "$BUILD_INFO"
echo "Rust Version: $(rustc --version)" >> "$BUILD_INFO"
echo "Build Flags: $RUSTFLAGS" >> "$BUILD_INFO"
echo "Features: $CARGO_FEATURE_FLAGS" >> "$BUILD_INFO"

# Size report
echo -e "\n${YELLOW}Binary sizes:${NC}"
ls -lh "$RELEASE_DIR/release/shriven-q" 2>/dev/null || true
ls -lh "$RELEASE_DIR/release/shriven-backtest" 2>/dev/null || true
ls -lh "$RELEASE_DIR/release/shriven-benchmark" 2>/dev/null || true

echo -e "\n${GREEN}✓ Optimized release build complete!${NC}"
echo "Binaries location: $RELEASE_DIR/release/"
echo "Build info: $BUILD_INFO"