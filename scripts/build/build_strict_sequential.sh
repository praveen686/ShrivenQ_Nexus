#!/usr/bin/env bash
#
# build_strict_sequential.sh - Build with ALL strictest compiler flags and lints (sequential execution)
# This runs comprehensive build checks sequentially to avoid cargo lock contention
#

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"
BUILD_LOG_DIR="$PROJECT_ROOT/target/build-logs"
mkdir -p "$BUILD_LOG_DIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 ShrivenQ Strict Build - All Checks${NC}"
echo "================================================"

# Create log directory
mkdir -p "$BUILD_LOG_DIR"

# All possible strict flags
# Strict but sensible RUSTFLAGS - unsafe-code is handled in Cargo.toml per-crate
export RUSTFLAGS="-D warnings \
    -D rust-2018-idioms \
    -D rust-2021-compatibility \
    -D missing-debug-implementations \
    -D missing-copy-implementations \
    -D trivial-casts \
    -D trivial-numeric-casts \
    -D unstable-features \
    -D unused-extern-crates \
    -D unused-import-braces \
    -D unused-qualifications \
    -D unused-results \
    -D variant-size-differences \
    -D unreachable-pub \
    -D unsafe-op-in-unsafe-fn \
    -D unused-lifetimes \
    -D unused-macro-rules \
    -D dead-code \
    -W unused-imports \
    -W unused-variables"

# Use workspace lint configuration from Cargo.toml instead of overwhelming CLI flags
# This avoids the clippy::restriction disaster and uses sensible defaults
export CLIPPY_FLAGS="-D warnings"

cd "$PROJECT_ROOT"

# Function to run command and log output
run_check() {
    local name=$1
    local cmd=$2
    local log_file="$BUILD_LOG_DIR/${name}_${TIMESTAMP}.log"
    
    echo -n "Running $name check... "
    # Create the log file first
    touch "$log_file"
    
    # Run the command and capture output
    if bash -c "$cmd" > "$log_file" 2>&1; then
        echo -e "${GREEN}✓${NC}"
        return 0
    else
        echo -e "${RED}✗${NC}"
        echo -e "${YELLOW}  See log: $log_file${NC}"
        return 1
    fi
}

# Track failures
FAILED_CHECKS=()

# Run all checks - sequenced to avoid cargo lock contention
echo -e "\n${YELLOW}Running comprehensive checks...${NC}"

# Clean build first
cargo clean

# Recreate the log directory after clean
mkdir -p "$BUILD_LOG_DIR"

# Auto-format code first (fixes any formatting issues)
echo -n "Auto-formatting code... "
cargo fmt
echo -e "${GREEN}✓${NC}"

# Format check (verify formatting is correct)
run_check "format" "cargo fmt -- --check" || FAILED_CHECKS+=("format")

# Build with strict flags FIRST - this creates the build artifacts
run_check "build_strict" "cargo build --all-targets" || FAILED_CHECKS+=("build_strict")

# Now run checks that depend on build artifacts
# These can reuse the build cache without lock contention

# Clippy with all lints (uses build artifacts)
run_check "clippy" "cargo clippy --all-targets --all-features -- $CLIPPY_FLAGS" || FAILED_CHECKS+=("clippy")

# Test with strict flags (uses build artifacts)
run_check "test" "cargo test --all-features" || FAILED_CHECKS+=("test")

# Doc build with warnings (can run after main build)
run_check "doc" "RUSTDOCFLAGS='-D warnings' cargo doc --no-deps --all-features" || FAILED_CHECKS+=("doc")

# Check for security advisories
if command -v cargo-audit &> /dev/null; then
    run_check "audit" "cargo audit" || FAILED_CHECKS+=("audit")
else
    echo -e "${YELLOW}Skipping audit check (cargo-audit not installed)${NC}"
fi

# Check dependencies
if command -v cargo-outdated &> /dev/null; then
    run_check "outdated" "cargo outdated" || FAILED_CHECKS+=("outdated")
else
    echo -e "${YELLOW}Skipping outdated check (cargo-outdated not installed)${NC}"
fi

# Check for unused dependencies
if command -v cargo-machete &> /dev/null; then
    run_check "unused_deps" "cargo machete" || FAILED_CHECKS+=("unused_deps")
else
    echo -e "${YELLOW}Skipping unused deps check (cargo-machete not installed)${NC}"
fi

# Summary
echo -e "\n================================================"
if [ ${#FAILED_CHECKS[@]} -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Failed checks: ${FAILED_CHECKS[*]}${NC}"
    echo -e "${YELLOW}Check logs in: $BUILD_LOG_DIR${NC}"
    exit 1
fi