# Runtime image for LLM Incident Manager
# Binary is pre-built by Cloud Build step

FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy pre-built binary from workspace (set by Cloud Build)
# In Cloud Build, the binary is at /workspace/bin/llm-incident-manager
COPY bin/llm-incident-manager /usr/local/bin/llm-incident-manager

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

# Run the service
ENTRYPOINT ["/usr/local/bin/llm-incident-manager"]
