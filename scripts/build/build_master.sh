#!/usr/bin/env bash
#
# build_master.sh - Master build orchestrator for ShrivenQ
# Provides a menu-driven interface to run various build configurations
#

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'
BOLD='\033[1m'

# Print ShrivenQ banner
print_banner() {
    echo -e "${CYAN}"
    cat << 'EOF'
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║   ███████╗██╗  ██╗██████╗ ██╗██╗   ██╗███████╗███╗   ██╗    ║
║   ██╔════╝██║  ██║██╔══██╗██║██║   ██║██╔════╝████╗  ██║    ║
║   ███████╗███████║██████╔╝██║██║   ██║█████╗  ██╔██╗ ██║    ║
║   ╚════██║██╔══██║██╔══██╗██║╚██╗ ██╔╝██╔══╝  ██║╚██╗██║    ║
║   ███████║██║  ██║██║  ██║██║ ╚████╔╝ ███████╗██║ ╚████║    ║
║   ╚══════╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝  ╚══════╝╚═╝  ╚═══╝    ║
║                                                               ║
║                     BUILD SYSTEM v1.0                        ║
╚═══════════════════════════════════════════════════════════════╝
EOF
    echo -e "${NC}"
}

# Check if script exists and is executable
check_script() {
    local script=$1
    if [ ! -f "$script" ]; then
        echo -e "${RED}Error: Script not found: $script${NC}"
        return 1
    fi
    if [ ! -x "$script" ]; then
        chmod +x "$script"
    fi
    return 0
}

# Run build script with timing
run_build() {
    local script_name=$1
    local script_path="$SCRIPT_DIR/$script_name"
    
    if ! check_script "$script_path"; then
        return 1
    fi
    
    echo -e "\n${YELLOW}Starting: $script_name${NC}"
    echo "═══════════════════════════════════════════"
    
    local start_time=$(date +%s)
    
    if bash "$script_path"; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        echo -e "\n${GREEN}✓ Completed in ${duration}s${NC}"
        return 0
    else
        echo -e "\n${RED}✗ Build failed${NC}"
        return 1
    fi
}

# Main menu
show_menu() {
    echo -e "${BOLD}${BLUE}Select Build Configuration:${NC}"
    echo -e "═══════════════════════════════════════════"
    echo -e "${GREEN}1)${NC} 🚀 Quick Development Build"
    echo -e "${GREEN}2)${NC} ⚡ Fast Parallel Build"
    echo -e "${GREEN}3)${NC} 🔍 Strict Build (All Checks)"
    echo -e "${GREEN}4)${NC} 🎯 Optimized Release Build"
    echo -e "${GREEN}5)${NC} 🧪 Comprehensive Test Suite"
    echo -e "${GREEN}6)${NC} 📊 Performance Benchmarks"
    echo -e "${GREEN}7)${NC} 🐳 Docker Container Build"
    echo -e "${GREEN}8)${NC} 🔧 Custom Build (enter flags)"
    echo -e "${GREEN}9)${NC} 🎨 Run All Builds (Parallel)"
    echo -e "${GREEN}0)${NC} Exit"
    echo -e "═══════════════════════════════════════════"
}

# Custom build with user-specified flags
custom_build() {
    echo -e "${YELLOW}Enter custom RUSTFLAGS:${NC}"
    read -r custom_flags
    
    echo -e "${YELLOW}Enter cargo command (e.g., 'build --release'):${NC}"
    read -r cargo_cmd
    
    echo -e "\n${BLUE}Running: RUSTFLAGS='$custom_flags' cargo $cargo_cmd${NC}"
    
    cd "$PROJECT_ROOT"
    RUSTFLAGS="$custom_flags" cargo $cargo_cmd
}

# Run all builds in parallel
run_all_parallel() {
    echo -e "${YELLOW}Running all builds in parallel...${NC}"
    echo -e "Check ${BLUE}target/build-logs/${NC} for individual logs"
    
    local pids=()
    local scripts=(
        "build_development_quick.sh"
        "build_parallel_fast.sh"
        "build_test_comprehensive.sh"
    )
    
    for script in "${scripts[@]}"; do
        (run_build "$script" > "/tmp/${script}.log" 2>&1) &
        pids+=($!)
        echo -e "${BLUE}Started: $script (PID: ${pids[-1]})${NC}"
    done
    
    # Wait for all builds to complete
    local failed=0
    for i in "${!pids[@]}"; do
        if wait "${pids[$i]}"; then
            echo -e "${GREEN}✓ ${scripts[$i]} completed${NC}"
        else
            echo -e "${RED}✗ ${scripts[$i]} failed${NC}"
            failed=$((failed + 1))
        fi
    done
    
    if [ $failed -eq 0 ]; then
        echo -e "\n${GREEN}✓ All parallel builds completed successfully!${NC}"
    else
        echo -e "\n${YELLOW}⚠ $failed builds failed${NC}"
    fi
}

# Make all scripts executable
make_scripts_executable() {
    echo "Setting execute permissions on all build scripts..."
    chmod +x "$SCRIPT_DIR"/*.sh
}

# Main execution
main() {
    print_banner
    make_scripts_executable
    
    # Check for required tools
    echo -e "${BLUE}Checking build environment...${NC}"
    
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: Rust/Cargo not found${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✓ Rust version:${NC} $(rustc --version)"
    echo -e "${GREEN}✓ Cargo version:${NC} $(cargo --version)"
    
    # Interactive menu loop
    while true; do
        echo
        show_menu
        echo -n -e "${BOLD}${MAGENTA}Enter choice [0-9]: ${NC}"
        read -r choice
        
        case $choice in
            1) run_build "build_development_quick.sh" ;;
            2) run_build "build_parallel_fast.sh" ;;
            3) run_build "build_strict_all.sh" ;;
            4) run_build "build_release_optimized.sh" ;;
            5) run_build "build_test_comprehensive.sh" ;;
            6) run_build "build_benchmark_performance.sh" ;;
            7) run_build "build_docker_container.sh" ;;
            8) custom_build ;;
            9) run_all_parallel ;;
            0) 
                echo -e "${GREEN}Goodbye!${NC}"
                exit 0
                ;;
            *)
                echo -e "${RED}Invalid option${NC}"
                ;;
        esac
        
        echo -e "\n${YELLOW}Press Enter to continue...${NC}"
        read -r
    done
}

# Handle command line arguments
if [ $# -gt 0 ]; then
    case "$1" in
        quick|dev) run_build "build_development_quick.sh" ;;
        parallel|fast) run_build "build_parallel_fast.sh" ;;
        strict|all) run_build "build_strict_all.sh" ;;
        release|opt) run_build "build_release_optimized.sh" ;;
        test) run_build "build_test_comprehensive.sh" ;;
        bench|benchmark) run_build "build_benchmark_performance.sh" ;;
        docker) run_build "build_docker_container.sh" ;;
        help|--help|-h)
            echo "Usage: $0 [quick|parallel|strict|release|test|bench|docker]"
            echo "  Or run without arguments for interactive menu"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown command: $1${NC}"
            echo "Run '$0 help' for usage"
            exit 1
            ;;
    esac
else
    # Run interactive menu
    main
fi