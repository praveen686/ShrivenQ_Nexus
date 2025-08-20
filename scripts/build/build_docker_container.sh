#!/usr/bin/env bash
#
# build_docker_container.sh - Build ShrivenQ Docker containers
# Creates optimized containers for different deployment scenarios
#

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"
DOCKER_BUILD_DIR="$PROJECT_ROOT/docker"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${GREEN}🐳 ShrivenQ Docker Container Build${NC}"
echo "=================================="

cd "$PROJECT_ROOT"

# Check if Docker is available
if ! command -v docker &> /dev/null; then
    echo -e "${YELLOW}Docker not found. Please install Docker first.${NC}"
    exit 1
fi

# Create Dockerfile if it doesn't exist
if [ ! -f "$PROJECT_ROOT/Dockerfile" ]; then
    echo "Creating optimized Dockerfile..."
    cat > "$PROJECT_ROOT/Dockerfile" << 'EOF'
# Multi-stage build for minimal image size
FROM rust:1.75 as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    cmake \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /usr/src/shriven-q

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build dependencies (cached layer)
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY . .

# Build with maximum optimizations
ENV RUSTFLAGS="-C target-cpu=native -C opt-level=3 -C lto=fat -C codegen-units=1"
RUN cargo build --release --features high-performance

# Runtime stage - minimal image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 shriven

# Copy binary from builder
COPY --from=builder /usr/src/shriven-q/target/release/shriven-q /usr/local/bin/
COPY --from=builder /usr/src/shriven-q/target/release/shriven-backtest /usr/local/bin/
COPY --from=builder /usr/src/shriven-q/target/release/shriven-benchmark /usr/local/bin/

# Set ownership
RUN chown -R shriven:shriven /usr/local/bin/shriven*

USER shriven
WORKDIR /home/shriven

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD shriven-q --help || exit 1

# Default command
ENTRYPOINT ["shriven-q"]
CMD ["--help"]
EOF
fi

# Build variants
echo -e "\n${YELLOW}Building Docker images...${NC}\n"

# Production image
echo "Building production image..."
docker build -t shriven-q:latest \
    -t shriven-q:$(git describe --tags --always 2>/dev/null || echo "dev") \
    --target runtime \
    . || docker build -t shriven-q:latest .

# Development image with build tools
echo -e "\nBuilding development image..."
cat > "$PROJECT_ROOT/Dockerfile.dev" << 'EOF'
FROM rust:1.75

# Install development tools
RUN apt-get update && apt-get install -y \
    cmake \
    gdb \
    valgrind \
    perf-tools-unstable \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Install Rust tools
RUN rustup component add \
    rustfmt \
    clippy \
    rust-src \
    rust-analyzer

# Install cargo tools
RUN cargo install \
    cargo-watch \
    cargo-edit \
    cargo-outdated \
    cargo-audit \
    cargo-tarpaulin

WORKDIR /workspace

# Keep container running for development
CMD ["bash"]
EOF

docker build -f Dockerfile.dev -t shriven-q:dev .

# GPU-enabled image if CUDA is needed
if command -v nvidia-docker &> /dev/null; then
    echo -e "\nBuilding GPU-enabled image..."
    cat > "$PROJECT_ROOT/Dockerfile.gpu" << 'EOF'
FROM nvidia/cuda:12.2.0-devel-ubuntu22.04 as builder

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Install build dependencies
RUN apt-get update && apt-get install -y \
    cmake \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/shriven-q
COPY . .

# Build with GPU support
RUN cargo build --release --features gpu-acceleration

# Runtime stage
FROM nvidia/cuda:12.2.0-runtime-ubuntu22.04

COPY --from=builder /usr/src/shriven-q/target/release/shriven-q /usr/local/bin/

ENTRYPOINT ["shriven-q"]
CMD ["--gpu"]
EOF
    
    docker build -f Dockerfile.gpu -t shriven-q:gpu .
fi

# Create docker-compose.yml
echo -e "\n${YELLOW}Creating docker-compose configuration...${NC}"
cat > "$PROJECT_ROOT/docker-compose.yml" << 'EOF'
version: '3.8'

services:
  shriven-q:
    image: shriven-q:latest
    container_name: shriven-q-prod
    restart: unless-stopped
    ports:
      - "8080:8080"
      - "9090:9090"  # Metrics
    volumes:
      - ./config:/home/shriven/config:ro
      - shriven-data:/home/shriven/data
    environment:
      - SHRIVEN_Q_EXECUTION_MODE=paper
      - SHRIVEN_Q_LOG_LEVEL=info
      - RUST_BACKTRACE=1
    networks:
      - shriven-network
    deploy:
      resources:
        limits:
          cpus: '4'
          memory: 8G
        reservations:
          cpus: '2'
          memory: 4G

  shriven-dev:
    image: shriven-q:dev
    container_name: shriven-q-dev
    stdin_open: true
    tty: true
    volumes:
      - .:/workspace
      - cargo-cache:/usr/local/cargo/registry
    networks:
      - shriven-network
    profiles:
      - development

volumes:
  shriven-data:
  cargo-cache:

networks:
  shriven-network:
    driver: bridge
EOF

# Test the images
echo -e "\n${YELLOW}Testing Docker images...${NC}"
docker run --rm shriven-q:latest --help > /dev/null 2>&1 && \
    echo -e "${GREEN}✓ Production image works${NC}" || \
    echo -e "${YELLOW}⚠ Production image test failed${NC}"

# Image sizes
echo -e "\n${BLUE}Docker image sizes:${NC}"
docker images | grep shriven-q

# Cleanup
rm -f Dockerfile.dev Dockerfile.gpu 2>/dev/null || true

echo -e "\n${GREEN}✓ Docker build complete!${NC}"
echo "Run with: docker run --rm shriven-q:latest"
echo "Or use docker-compose: docker-compose up"