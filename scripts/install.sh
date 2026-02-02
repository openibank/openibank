#!/bin/bash
# OpeniBank Quick Installer
# Usage: curl -fsSL https://openibank.com/install.sh | bash
#    or: curl -fsSL https://github.com/openibank/openibank/raw/main/scripts/install.sh | bash

set -e

REPO_URL="https://github.com/openibank/openibank"
INSTALL_DIR="${OPENIBANK_INSTALL_DIR:-$HOME/.openibank}"

echo ""
echo "╔══════════════════════════════════════════════════════════════════════╗"
echo "║                                                                      ║"
echo "║     ██████╗ ██████╗ ███████╗███╗   ██╗██╗██████╗  █████╗ ███╗   ██╗██╗  ██╗"
echo "║    ██╔═══██╗██╔══██╗██╔════╝████╗  ██║██║██╔══██╗██╔══██╗████╗  ██║██║ ██╔╝"
echo "║    ██║   ██║██████╔╝█████╗  ██╔██╗ ██║██║██████╔╝███████║██╔██╗ ██║█████╔╝ "
echo "║    ██║   ██║██╔═══╝ ██╔══╝  ██║╚██╗██║██║██╔══██╗██╔══██║██║╚██╗██║██╔═██╗ "
echo "║    ╚██████╔╝██║     ███████╗██║ ╚████║██║██████╔╝██║  ██║██║ ╚████║██║  ██╗"
echo "║     ╚═════╝ ╚═╝     ╚══════╝╚═╝  ╚═══╝╚═╝╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝"
echo "║                                                                      ║"
echo "║          Programmable Wallets + Receipts for AI Agents               ║"
echo "║                                                                      ║"
echo "╚══════════════════════════════════════════════════════════════════════╝"
echo ""

# Check for required tools
echo "Checking prerequisites..."

if ! command -v git &> /dev/null; then
    echo "  ✗ git not found. Please install git first."
    exit 1
fi
echo "  ✓ git"

if ! command -v cargo &> /dev/null; then
    echo "  ✗ cargo not found. Please install Rust first."
    echo "    Visit: https://rustup.rs/"
    exit 1
fi
echo "  ✓ cargo ($(cargo --version | cut -d' ' -f2))"

# Optional: Check for Ollama
if command -v ollama &> /dev/null; then
    echo "  ✓ ollama (for local LLM support)"
else
    echo "  ○ ollama not found (optional - for local LLM support)"
    echo "    Install later: brew install ollama (macOS) or https://ollama.com"
fi

echo ""

# Clone or update repository
if [ -d "$INSTALL_DIR" ]; then
    echo "Updating existing installation..."
    cd "$INSTALL_DIR"
    git pull
else
    echo "Cloning OpeniBank..."
    git clone "$REPO_URL" "$INSTALL_DIR"
    cd "$INSTALL_DIR"
fi

echo ""
echo "Building OpeniBank..."
cargo build --release

echo ""
echo "Running tests..."
cargo test --release -- --quiet || true

echo ""
echo "╔══════════════════════════════════════════════════════════════════════╗"
echo "║                     Installation Complete!                           ║"
echo "╚══════════════════════════════════════════════════════════════════════╝"
echo ""
echo "OpeniBank installed to: $INSTALL_DIR"
echo ""
echo "Quick Start:"
echo "  cd $INSTALL_DIR"
echo "  cargo run --example asset_cycle"
echo ""
echo "With local LLM (requires Ollama):"
echo "  ollama pull llama3.1:8b"
echo "  ollama serve"
echo "  OPENIBANK_LLM_PROVIDER=ollama cargo run --example asset_cycle"
echo ""
echo "Learn more:"
echo "  Website: https://www.openibank.com/"
echo "  GitHub:  https://github.com/openibank/openibank/"
echo "  Docs:    $INSTALL_DIR/docs/"
echo ""
echo "Happy banking! 🏦"
