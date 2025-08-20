#!/usr/bin/env bash
#
# build_development_quick.sh - Quick development build with basic checks
# Fast iteration for development with essential checks only
#

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}⚡ ShrivenQ Quick Development Build${NC}"
echo "==================================="

cd "$PROJECT_ROOT"

# Minimal warning flags for development
export RUSTFLAGS="-D warnings -D unused-imports -D dead-code"

# Fast incremental compilation
export CARGO_INCREMENTAL=1
export CARGO_TARGET_DIR="target/dev-quick"

# Quick format check (non-blocking)
echo -n "Format check... "
if cargo fmt -- --check 2>/dev/null; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${YELLOW}⚠ (formatting issues found, run 'cargo fmt')${NC}"
fi

# Quick build
echo -n "Building... "
if cargo build 2>/dev/null; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
fi

# Quick clippy check (warnings only)
echo -n "Clippy check... "
if cargo clippy -- -W clippy::all 2>/dev/null; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${YELLOW}⚠ Clippy warnings${NC}"
fi

# Quick test of core module only
echo -n "Core tests... "
if cargo test -p shriven-q --lib 2>/dev/null; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗ Tests failed${NC}"
    exit 1
fi

echo -e "\n${GREEN}✓ Quick build complete!${NC}"
echo "Run './scripts/build/build_strict_all.sh' for comprehensive checks"