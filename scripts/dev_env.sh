#!/bin/bash
# OpeniBank Development Environment Setup
# Sets sensible defaults for local development

set -e

echo "Setting up OpeniBank development environment..."

# Default to Ollama (local, no API key needed)
export OPENIBANK_LLM_PROVIDER=${OPENIBANK_LLM_PROVIDER:-ollama}
export OPENIBANK_OLLAMA_URL=${OPENIBANK_OLLAMA_URL:-http://localhost:11434}
export OPENIBANK_OLLAMA_MODEL=${OPENIBANK_OLLAMA_MODEL:-llama3.1:8b}

# Logging
export RUST_LOG=${RUST_LOG:-info}

echo "  OPENIBANK_LLM_PROVIDER=$OPENIBANK_LLM_PROVIDER"
echo "  OPENIBANK_OLLAMA_URL=$OPENIBANK_OLLAMA_URL"
echo "  OPENIBANK_OLLAMA_MODEL=$OPENIBANK_OLLAMA_MODEL"
echo "  RUST_LOG=$RUST_LOG"
echo ""
echo "Environment configured. Run your cargo commands now."
