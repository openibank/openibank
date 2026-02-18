# OpeniBank — task runner (https://github.com/casey/just)
# Install just: cargo install just
#
# Usage:
#   just            # list all recipes
#   just build      # release build
#   just test       # run all tests
#   just demo       # run local demo TUI
#   just server     # start management server
#   just fmt        # format code
#   just lint       # clippy + fmt check
#   just release    # create release artifacts + checksums

set shell := ["bash", "-euo", "pipefail", "-c"]
set dotenv-load := true

# ── Variables ─────────────────────────────────────────────────────────────────

VERSION := `cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; d=json.load(sys.stdin); print(next(p['version'] for p in d['packages'] if p['name']=='openibank-cli'))"`
RELEASE_DIR := "dist"

# ── Default: list recipes ─────────────────────────────────────────────────────

@default:
    just --list

# ── Build ─────────────────────────────────────────────────────────────────────

# Build all crates (debug)
build:
    cargo build --workspace

# Build release binary
build-release:
    cargo build --release -p openibank-cli -p openibank-server

# Build for a specific target
build-target target:
    cargo build --release -p openibank-cli --target {{ target }}

# ── Test ──────────────────────────────────────────────────────────────────────

# Run all tests
test:
    cargo test --workspace --all-features

# Run tests for a specific crate
test-crate crate:
    cargo test -p {{ crate }} --all-features

# Run doc tests only
test-doc:
    cargo test --workspace --doc

# Run tests with nextest (if installed)
nextest:
    cargo nextest run --workspace --all-features

# ── Code quality ──────────────────────────────────────────────────────────────

# Format all code
fmt:
    cargo fmt --all

# Check formatting (CI mode)
fmt-check:
    cargo fmt --all -- --check

# Run clippy with deny warnings
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Security audit
audit:
    cargo audit

# Full CI check (fmt + lint + test)
ci: fmt-check lint test

# ── Run ───────────────────────────────────────────────────────────────────────

# Launch the management server (localhost:8080)
server:
    cargo run --release -p openibank-server

# Launch server on custom port
server-port port:
    cargo run --release -p openibank-server -- --port {{ port }}

# Launch interactive TUI demo
demo:
    cargo run --release -p openibank-cli -- demo full

# Launch TUI demo with seed
demo-seed seed:
    cargo run --release -p openibank-cli -- demo --seed {{ seed }}

# Run arena competition (n rounds)
arena rounds="20":
    cargo run --release -p openibank-cli -- arena run --rounds {{ rounds }}

# Show wallet identity
wallet-id name="demo-agent":
    cargo run --release -p openibank-cli -- wallet identity --name {{ name }}

# Verify a receipt file
verify file:
    cargo run --release -p openibank-cli -- receipt verify --file {{ file }}

# ── Install ───────────────────────────────────────────────────────────────────

# Install CLI to ~/.local/bin (release mode)
install:
    cargo install --path crates/openibank-cli --force

# Install via release artifact + SHA-256 (production mode)
install-release:
    bash scripts/install.sh

# Dev install (expects ../maple sibling)
install-dev:
    bash scripts/install.sh --dev

# ── Release artifacts ─────────────────────────────────────────────────────────

# Build release archives for current platform
package:
    #!/usr/bin/env bash
    set -euo pipefail
    BIN="openibank"
    TARGET=$(rustc -vV | awk '/^host:/ { print $2 }')
    ARTIFACT="${BIN}-v{{ VERSION }}-${TARGET}.tar.gz"
    mkdir -p {{ RELEASE_DIR }}
    cargo build --release -p openibank-cli
    cp "target/release/${BIN}" {{ RELEASE_DIR }}/
    tar -czf "{{ RELEASE_DIR }}/${ARTIFACT}" -C {{ RELEASE_DIR }} "${BIN}"
    echo "Packaged: {{ RELEASE_DIR }}/${ARTIFACT}"

# Generate checksums.txt from all archives in dist/
checksums:
    #!/usr/bin/env bash
    set -euo pipefail
    cd {{ RELEASE_DIR }}
    sha256sum *.tar.gz > checksums.txt
    echo "Generated {{ RELEASE_DIR }}/checksums.txt:"
    cat checksums.txt

# Full release: package + checksums
release: package checksums
    @echo "Release artifacts ready in {{ RELEASE_DIR }}/"
    @echo "Version: v{{ VERSION }}"

# ── Docker ────────────────────────────────────────────────────────────────────

# Build Docker image
docker-build:
    docker build -t openibank:latest .

# Start services with Docker Compose
docker-up:
    docker compose up

# Start with Ollama LLM
docker-up-llm:
    docker compose --profile llm up

# Stop Docker services
docker-down:
    docker compose down

# ── Maintenance ───────────────────────────────────────────────────────────────

# Update all dependencies
update:
    cargo update

# Clean build artifacts
clean:
    cargo clean

# Remove release dist directory
clean-dist:
    rm -rf {{ RELEASE_DIR }}

# Print workspace version
version:
    @echo "v{{ VERSION }}"

# Print size of release binary
binary-size:
    #!/usr/bin/env bash
    BIN="target/release/openibank"
    if [ ! -f "${BIN}" ]; then
        echo "Binary not found. Run: just build-release"
        exit 1
    fi
    SIZE=$(du -sh "${BIN}" | cut -f1)
    echo "openibank release binary: ${SIZE}"

# Show all crate dependencies (tree)
deps:
    cargo tree --workspace
