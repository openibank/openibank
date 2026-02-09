# =============================================================================
# OpeniBank API Server - Multi-stage Docker Build
# =============================================================================
# Production-ready container for the OpeniBank REST API server.
#
# Build:
#   docker build -t openibank-api-server .
#
# Run:
#   docker run -p 3000:3000 -e DATABASE_URL=postgres://... openibank-api-server
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build Environment
# -----------------------------------------------------------------------------
FROM rust:1.84-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY services/ ./services/

# Build release binary
RUN cargo build --release --package openibank-api-server

# -----------------------------------------------------------------------------
# Stage 2: Runtime Environment
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd --gid 1000 openibank \
    && useradd --uid 1000 --gid openibank --shell /bin/bash --create-home openibank

# Copy binary from builder
COPY --from=builder /app/target/release/openibank-api-server /usr/local/bin/

# Create config directory
RUN mkdir -p /etc/openibank && chown openibank:openibank /etc/openibank

# Switch to non-root user
USER openibank
WORKDIR /home/openibank

# Environment variables with defaults
ENV OPENIBANK_HOST=0.0.0.0
ENV OPENIBANK_PORT=3000
ENV OPENIBANK_LOG_LEVEL=info
ENV OPENIBANK_LOG_FORMAT=json
ENV RUST_BACKTRACE=1

# Expose ports
EXPOSE 3000
EXPOSE 9090

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Run the server
ENTRYPOINT ["openibank-api-server"]
CMD ["--dev-mode"]
