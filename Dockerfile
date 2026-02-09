# =============================================================================
# OpeniBank - AI Agent Banking Platform
# Multi-stage Docker build for all services
# =============================================================================
#
# Build specific service:
#   docker build --build-arg SERVICE=openibank-server -t openibank-server .
#   docker build --build-arg SERVICE=openibank-api-server -t openibank-api-server .
#   docker build --build-arg SERVICE=resonancex-server -t resonancex-server .
#   docker build --build-arg SERVICE=openibank-portal -t openibank-portal .
#   docker build --build-arg SERVICE=openibank-web -t openibank-web .
#   docker build --build-arg SERVICE=openibank-marketplace-server -t openibank-marketplace .
#   docker build --build-arg SERVICE=openibank-issuer-resonator -t openibank-issuer .
#   docker build --build-arg SERVICE=openibank-playground -t openibank-playground .
#
# =============================================================================

# Build argument for service selection
ARG SERVICE=openibank-server

# Stage 1: Chef - Dependency caching
FROM rust:1.78-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /build

# Stage 2: Planner - Compute dependency graph
FROM chef AS planner

# Copy maple dependency (must be available as sibling)
COPY maple/ /build/maple/

# Copy workspace
COPY . /build/openibank/

WORKDIR /build/openibank
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder - Build dependencies then application
FROM chef AS builder

ARG SERVICE

# Copy maple dependency
COPY maple/ /build/maple/

# Copy recipe for dependency caching
COPY --from=planner /build/openibank/recipe.json /build/openibank/recipe.json

WORKDIR /build/openibank

# Build dependencies (cached layer)
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source code
COPY . /build/openibank/

# Build the specified service
RUN cargo build --release --bin ${SERVICE}

# Stage 4: Runtime - Minimal production image
FROM debian:bookworm-slim AS runtime

ARG SERVICE

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -s /bin/bash openibank

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/openibank/target/release/${SERVICE} /app/service

# Copy static files if they exist (for web services)
COPY --from=builder /build/openibank/services/${SERVICE}/static /app/static 2>/dev/null || true

# Set ownership
RUN chown -R openibank:openibank /app

USER openibank

# Default port (can be overridden)
EXPOSE 8080

# Environment defaults
ENV RUST_LOG=info
ENV HOST=0.0.0.0

# Health check (generic, works for most services)
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
    CMD curl -f http://localhost:${PORT:-8080}/health || curl -f http://localhost:${PORT:-8080}/api/health || exit 1

ENTRYPOINT ["/app/service"]
