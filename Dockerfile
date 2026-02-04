# =============================================================================
# OpeniBank - AI Agent Banking Server
# Multi-stage Docker build for minimal production image
# =============================================================================

# Stage 1: Build
FROM rust:1.78-bookworm AS builder

WORKDIR /build

# Copy maple dependency (must be available as sibling)
COPY maple/ /build/maple/

# Copy workspace
COPY openibank/ /build/openibank/

WORKDIR /build/openibank

# Build release binary
RUN cargo build --release --bin openibank-server

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -s /bin/bash openibank

WORKDIR /app

# Copy binary
COPY --from=builder /build/openibank/target/release/openibank-server /app/openibank-server

# Set ownership
RUN chown -R openibank:openibank /app

USER openibank

# Default port
EXPOSE 8080

# Environment defaults
ENV OPENIBANK_HOST=0.0.0.0
ENV OPENIBANK_PORT=8080
ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

ENTRYPOINT ["/app/openibank-server"]
