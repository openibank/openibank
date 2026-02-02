#!/bin/bash
# Run the OpeniBank viral demo with local LLM
# Requires Ollama to be installed and running

set -e

echo "╔══════════════════════════════════════════════════════════════════════╗"
echo "║           OpeniBank Demo with Local LLM                              ║"
echo "╚══════════════════════════════════════════════════════════════════════╝"
echo ""

# Check if Ollama is running
echo "Checking Ollama..."
if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "  ✓ Ollama is running"
else
    echo "  ✗ Ollama is not running"
    echo ""
    echo "Please start Ollama:"
    echo "  1. Install: brew install ollama (macOS) or https://ollama.com/download"
    echo "  2. Pull model: ollama pull llama3.1:8b"
    echo "  3. Run: ollama serve"
    echo ""
    echo "Continuing with deterministic mode..."
fi

echo ""
echo "Running asset_cycle demo..."
echo ""

# Set environment and run
export OPENIBANK_LLM_PROVIDER=ollama
export OPENIBANK_OLLAMA_MODEL=${OPENIBANK_OLLAMA_MODEL:-llama3.1:8b}
export RUST_LOG=info

cargo run --example asset_cycle
