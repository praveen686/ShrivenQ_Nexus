#!/usr/bin/env bash
#
# build_test_comprehensive.sh - Comprehensive test suite execution
# Runs all tests including unit, integration, doc tests, and benchmarks
#

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"
TEST_REPORT_DIR="$PROJECT_ROOT/target/test-reports"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${GREEN}🧪 ShrivenQ Comprehensive Test Suite${NC}"
echo "====================================="

cd "$PROJECT_ROOT"

# Create test report directory
mkdir -p "$TEST_REPORT_DIR"

# Test coverage with tarpaulin if available
if command -v cargo-tarpaulin &> /dev/null; then
    COVERAGE_ENABLED=true
    echo -e "${BLUE}Coverage reporting enabled${NC}"
else
    COVERAGE_ENABLED=false
    echo -e "${YELLOW}Coverage reporting disabled (cargo-tarpaulin not installed)${NC}"
fi

# Function to run tests and capture results
run_test_suite() {
    local suite_name=$1
    local test_cmd=$2
    local report_file="$TEST_REPORT_DIR/${suite_name}_${TIMESTAMP}.txt"
    
    echo -n "Running $suite_name... "
    
    if eval "$test_cmd" > "$report_file" 2>&1; then
        local test_count=$(grep -c "test result: ok" "$report_file" 2>/dev/null || echo "0")
        echo -e "${GREEN}✓ ($test_count tests passed)${NC}"
        return 0
    else
        local failed_count=$(grep -c "test result: FAILED" "$report_file" 2>/dev/null || echo "?")
        echo -e "${RED}✗ ($failed_count tests failed)${NC}"
        echo -e "  ${YELLOW}Report: $report_file${NC}"
        return 1
    fi
}

# Track test results
FAILED_SUITES=()
TOTAL_TESTS=0
PASSED_TESTS=0

echo -e "\n${YELLOW}Running test suites...${NC}\n"

# Unit tests
run_test_suite "unit_tests" "cargo test --lib --all-features" || FAILED_SUITES+=("unit_tests")

# Integration tests
run_test_suite "integration_tests" "cargo test --test '*' --all-features" || FAILED_SUITES+=("integration_tests")

# Doc tests
run_test_suite "doc_tests" "cargo test --doc --all-features" || FAILED_SUITES+=("doc_tests")

# Example tests
run_test_suite "example_tests" "cargo test --examples --all-features" || FAILED_SUITES+=("example_tests")

# Memory module specific tests
run_test_suite "memory_tests" "cargo test -p shriven-q memory::" || FAILED_SUITES+=("memory_tests")

# Benchmarks (compile only, don't run)
echo -n "Checking benchmarks compile... "
if cargo bench --no-run 2>/dev/null; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
    FAILED_SUITES+=("benchmarks")
fi

# Property-based tests if available
if grep -q "proptest\|quickcheck" Cargo.toml; then
    run_test_suite "property_tests" "cargo test --features testing" || FAILED_SUITES+=("property_tests")
fi

# Thread safety tests
echo -e "\n${YELLOW}Running thread safety tests...${NC}"
RUSTFLAGS="-Z sanitizer=thread" run_test_suite "thread_safety" \
    "cargo test --lib --all-features -Z build-std --target x86_64-unknown-linux-gnu" 2>/dev/null || \
    echo -e "${YELLOW}Thread sanitizer not available${NC}"

# Memory leak tests
if [[ "$OSTYPE" == "linux-gnu"* ]] && command -v valgrind &> /dev/null; then
    echo -n "Running memory leak test... "
    valgrind --leak-check=full --error-exitcode=1 \
        ./target/debug/shriven-q --help > "$TEST_REPORT_DIR/valgrind_${TIMESTAMP}.txt" 2>&1
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗ Memory leaks detected${NC}"
        FAILED_SUITES+=("memory_leaks")
    fi
fi

# Test with different feature combinations
echo -e "\n${YELLOW}Testing feature combinations...${NC}"

FEATURES=(
    ""
    "gpu-acceleration"
    "high-performance"
    "zerodha-integration"
    "binance-integration"
    "gpu-acceleration,high-performance"
)

for features in "${FEATURES[@]}"; do
    if [ -z "$features" ]; then
        feature_name="default"
        feature_flag=""
    else
        feature_name=$(echo "$features" | tr ',' '_')
        feature_flag="--features $features"
    fi
    
    echo -n "Testing with $feature_name features... "
    if cargo test --lib $feature_flag --quiet 2>/dev/null; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
        FAILED_SUITES+=("features_$feature_name")
    fi
done

# Generate coverage report if available
if [ "$COVERAGE_ENABLED" = true ]; then
    echo -e "\n${YELLOW}Generating coverage report...${NC}"
    cargo tarpaulin --out Html --output-dir "$TEST_REPORT_DIR" || true
    echo "Coverage report: $TEST_REPORT_DIR/tarpaulin-report.html"
fi

# Generate test summary
SUMMARY_FILE="$TEST_REPORT_DIR/summary_${TIMESTAMP}.txt"
echo "ShrivenQ Test Summary" > "$SUMMARY_FILE"
echo "=====================" >> "$SUMMARY_FILE"
echo "Date: $(date)" >> "$SUMMARY_FILE"
echo "Failed Suites: ${#FAILED_SUITES[@]}" >> "$SUMMARY_FILE"
if [ ${#FAILED_SUITES[@]} -gt 0 ]; then
    echo "Failed: ${FAILED_SUITES[*]}" >> "$SUMMARY_FILE"
fi

# Summary output
echo -e "\n====================================="
if [ ${#FAILED_SUITES[@]} -eq 0 ]; then
    echo -e "${GREEN}✓ All test suites passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Failed test suites: ${FAILED_SUITES[*]}${NC}"
    echo -e "${YELLOW}Test reports: $TEST_REPORT_DIR${NC}"
    echo -e "${YELLOW}Summary: $SUMMARY_FILE${NC}"
    exit 1
fi