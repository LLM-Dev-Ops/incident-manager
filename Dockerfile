# Multi-stage build for LLM Incident Manager

# Stage 1: Builder
FROM rust:1.84-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY config ./config

# Build for release
RUN cargo build --release --bin llm-incident-manager

# Stage 2: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/llm-incident-manager /usr/local/bin/llm-incident-manager

# Copy configuration
COPY config /app/config

# Create data directory
RUN mkdir -p /app/data

# Expose ports
EXPOSE 8080 9000 9090

# Set environment
ENV RUST_LOG=info
ENV CONFIG_PATH=/app/config/default.toml

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the binary
CMD ["llm-incident-manager"]
