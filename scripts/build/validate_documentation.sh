#!/bin/bash
set -euo pipefail

# ShrivenQ Documentation Validation Script
# Validates documentation consistency and builds reference graphs

RED='\033[0;31m'
GREEN='\033[0;32m' 
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔍 ShrivenQ Documentation Validation${NC}"
echo "=================================================="

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Cargo not found. Please install Rust toolchain.${NC}"
    exit 1
fi

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    echo -e "${RED}❌ Cargo.toml not found. Please run from project root.${NC}"
    exit 1
fi

# Ensure docs directory exists
if [[ ! -d "docs" ]]; then
    echo -e "${RED}❌ docs directory not found.${NC}"
    exit 1
fi

echo -e "${YELLOW}📊 Building documentation reference graph...${NC}"

# Build the doc-tracker tool if needed
echo "Building doc-tracker tool..."
if ! cargo build --bin doc-tracker --features development-tools --quiet; then
    echo -e "${RED}❌ Failed to build doc-tracker tool${NC}"
    exit 1
fi

# Create docs metadata directory
mkdir -p docs/.metadata

# Scan documentation and build reference graph
echo -e "${YELLOW}🔍 Scanning documentation files...${NC}"
if cargo run --bin doc-tracker --features development-tools -- scan --docs-path docs --output docs/.metadata/doc-graph.json --include-source; then
    echo -e "${GREEN}✅ Documentation graph created successfully${NC}"
else
    echo -e "${RED}❌ Failed to create documentation graph${NC}"
    exit 1
fi

# Validate all references are correct
echo -e "${YELLOW}🔍 Validating documentation references...${NC}"
if cargo run --bin doc-tracker --features development-tools -- validate --docs-path docs --verbose; then
    echo -e "${GREEN}✅ Documentation validation passed${NC}"
else
    echo -e "${RED}❌ Documentation validation failed${NC}"
    validation_failed=1
fi

# Generate documentation metrics
echo -e "${YELLOW}📈 Generating documentation metrics...${NC}"
if cargo run --bin doc-tracker --features development-tools -- metrics --docs-path docs --format markdown > docs/metrics/documentation-health.md; then
    echo -e "${GREEN}✅ Documentation metrics generated${NC}"
else
    echo -e "${YELLOW}⚠️  Failed to generate metrics (non-critical)${NC}"
fi

# Check for common documentation issues
echo -e "${YELLOW}🔍 Checking for common documentation issues...${NC}"

# Check for TODO/FIXME markers in documentation
todo_count=$(find docs -name "*.md" -exec grep -l "TODO\|FIXME\|XXX" {} \; | wc -l)
if [[ $todo_count -gt 0 ]]; then
    echo -e "${YELLOW}⚠️  Found $todo_count files with TODO/FIXME markers${NC}"
    find docs -name "*.md" -exec grep -Hn "TODO\|FIXME\|XXX" {} \;
fi

# Check for files without proper headings
echo "Checking for files without proper headings..."
files_without_headings=0
for file in $(find docs -name "*.md"); do
    if ! head -n 5 "$file" | grep -q "^# "; then
        echo -e "${YELLOW}⚠️  $file: Missing main heading${NC}"
        ((files_without_headings++))
    fi
done

if [[ $files_without_headings -eq 0 ]]; then
    echo -e "${GREEN}✅ All files have proper headings${NC}"
fi

# Check for extremely short files (potential stubs)
echo "Checking for potential stub files..."
stub_files=0
for file in $(find docs -name "*.md"); do
    word_count=$(wc -w < "$file")
    if [[ $word_count -lt 50 ]]; then
        echo -e "${YELLOW}⚠️  $file: Only $word_count words (potential stub)${NC}"
        ((stub_files++))
    fi
done

# Check for very long files (might need splitting)
echo "Checking for overly long files..."
long_files=0
for file in $(find docs -name "*.md"); do
    line_count=$(wc -l < "$file")
    if [[ $line_count -gt 1000 ]]; then
        echo -e "${YELLOW}⚠️  $file: $line_count lines (consider splitting)${NC}"
        ((long_files++))
    fi
done

# Generate summary report
echo ""
echo -e "${BLUE}📋 Documentation Validation Summary${NC}"
echo "=================================================="

if [[ -f "docs/.metadata/doc-graph.json" ]]; then
    # Extract metrics from the generated file
    total_files=$(grep -o '"total_files":[0-9]*' docs/.metadata/doc-graph.json | cut -d: -f2)
    total_refs=$(grep -o '"total_references":[0-9]*' docs/.metadata/doc-graph.json | cut -d: -f2)
    broken_refs=$(grep -o '"broken_references":[0-9]*' docs/.metadata/doc-graph.json | cut -d: -f2)
    
    echo "📊 Files scanned: ${total_files:-0}"
    echo "🔗 Total references: ${total_refs:-0}"
    echo "💥 Broken references: ${broken_refs:-0}"
    echo "📝 TODO markers: $todo_count"
    echo "📰 Files without headings: $files_without_headings"
    echo "📄 Potential stub files: $stub_files"
    echo "📚 Long files (>1000 lines): $long_files"
fi

# Final status
echo ""
if [[ ${validation_failed:-0} -eq 0 ]]; then
    echo -e "${GREEN}🎉 Documentation validation completed successfully!${NC}"
    
    # Add files to git if they don't exist
    if [[ -f "docs/.metadata/doc-graph.json" ]] && ! git ls-files --error-unmatch docs/.metadata/doc-graph.json &>/dev/null; then
        echo "📝 Adding documentation metadata to git..."
        git add docs/.metadata/doc-graph.json docs/metrics/documentation-health.md 2>/dev/null || true
    fi
    
    exit 0
else
    echo -e "${RED}❌ Documentation validation failed. Please fix the issues above.${NC}"
    exit 1
fi