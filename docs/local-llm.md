# Local LLM Setup for OpeniBank

OpeniBank works without any LLM - all agents have deterministic fallback behavior. But if you want your agents to have "brains", here's how to set up local LLM support.

## Ollama (Recommended)

Ollama is the easiest way to run local LLMs. No API keys required.

### Installation

**macOS:**
```bash
brew install ollama
```

**Linux:**
```bash
curl -fsSL https://ollama.com/install.sh | sh
```

**Windows:**
Download from https://ollama.com/download/windows

### Pull a Model

```bash
# Recommended: Llama 3.1 8B (good balance of speed/quality)
ollama pull llama3.1:8b

# Alternative: Smaller/faster
ollama pull llama3.2:3b

# Alternative: Larger/better
ollama pull llama3.1:70b
```

### Start Ollama

```bash
ollama serve
```

This starts the Ollama server at `http://localhost:11434`.

### Run OpeniBank with Ollama

```bash
# Default configuration
OPENIBANK_LLM_PROVIDER=ollama cargo run --example asset_cycle

# With specific model
OPENIBANK_LLM_PROVIDER=ollama OPENIBANK_OLLAMA_MODEL=llama3.2:3b cargo run --example asset_cycle

# Custom URL (if running Ollama elsewhere)
OPENIBANK_LLM_PROVIDER=ollama OPENIBANK_OLLAMA_URL=http://192.168.1.100:11434 cargo run --example asset_cycle
```

## OpenAI-Compatible Servers

OpeniBank supports any server that implements the OpenAI Chat Completions API.

### vLLM

```bash
# Start vLLM server
python -m vllm.entrypoints.openai.api_server \
  --model meta-llama/Llama-3.1-8B-Instruct \
  --port 8000

# Run OpeniBank
OPENIBANK_LLM_PROVIDER=openai_compat \
OPENIBANK_OPENAI_COMPAT_BASE_URL=http://localhost:8000/v1 \
OPENIBANK_OPENAI_COMPAT_MODEL=meta-llama/Llama-3.1-8B-Instruct \
cargo run --example asset_cycle
```

### llama.cpp Server

```bash
# Start llama.cpp server
./llama-server -m llama-3.1-8b.gguf --port 8080

# Run OpeniBank
OPENIBANK_LLM_PROVIDER=openai_compat \
OPENIBANK_OPENAI_COMPAT_BASE_URL=http://localhost:8080/v1 \
cargo run --example asset_cycle
```

### LM Studio

1. Download from https://lmstudio.ai/
2. Load a model
3. Start the server (usually at `http://localhost:1234`)

```bash
OPENIBANK_LLM_PROVIDER=openai_compat \
OPENIBANK_OPENAI_COMPAT_BASE_URL=http://localhost:1234/v1 \
cargo run --example asset_cycle
```

## Cloud Providers

If you prefer cloud LLMs:

### OpenAI

```bash
OPENIBANK_LLM_PROVIDER=openai \
OPENAI_API_KEY=sk-... \
OPENIBANK_OPENAI_MODEL=gpt-4o-mini \
cargo run --example asset_cycle
```

### Anthropic Claude

```bash
OPENIBANK_LLM_PROVIDER=anthropic \
ANTHROPIC_API_KEY=sk-ant-... \
OPENIBANK_ANTHROPIC_MODEL=claude-3-5-sonnet-20241022 \
cargo run --example asset_cycle
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OPENIBANK_LLM_PROVIDER` | `ollama` | Provider: ollama, openai_compat, openai, anthropic, gemini, grok, deterministic |
| `OPENIBANK_OLLAMA_URL` | `http://localhost:11434` | Ollama server URL |
| `OPENIBANK_OLLAMA_MODEL` | `llama3.1:8b` | Ollama model name |
| `OPENIBANK_OPENAI_COMPAT_BASE_URL` | `http://localhost:8000/v1` | OpenAI-compatible server URL |
| `OPENIBANK_OPENAI_COMPAT_API_KEY` | (none) | API key (optional for local) |
| `OPENIBANK_OPENAI_COMPAT_MODEL` | `default` | Model name |
| `OPENAI_API_KEY` | (none) | OpenAI API key |
| `OPENIBANK_OPENAI_MODEL` | `gpt-4o-mini` | OpenAI model |
| `ANTHROPIC_API_KEY` | (none) | Anthropic API key |
| `OPENIBANK_ANTHROPIC_MODEL` | `claude-3-5-sonnet-20241022` | Claude model |

## Using .env File

Create a `.env` file in the project root:

```env
# Local Ollama (default)
OPENIBANK_LLM_PROVIDER=ollama
OPENIBANK_OLLAMA_MODEL=llama3.1:8b

# Or cloud
# OPENIBANK_LLM_PROVIDER=openai
# OPENAI_API_KEY=sk-...
```

OpeniBank will automatically load this file.

## Deterministic Mode

To run without any LLM:

```bash
OPENIBANK_LLM_PROVIDER=deterministic cargo run --example asset_cycle
```

Or simply don't set any LLM environment variables - if Ollama isn't running, agents will use deterministic logic.

## Troubleshooting

### "Provider not available"

1. Check if Ollama is running: `curl http://localhost:11434/api/tags`
2. Check if the model is downloaded: `ollama list`
3. Try pulling the model again: `ollama pull llama3.1:8b`

### Slow responses

1. Use a smaller model: `llama3.2:3b`
2. If using GPU, ensure CUDA/ROCm is properly configured
3. Check available RAM - larger models need more memory

### API key errors

1. Ensure the key is correct
2. Check for trailing whitespace in the key
3. Verify the key has the required permissions

## Security Note

**LLMs may PROPOSE intents, NEVER EXECUTE money.**

All LLM outputs go through the Guard, which:
- Validates amounts against permits/budgets
- Checks counterparties against allowlists
- Detects prompt injection attempts
- Falls back to deterministic behavior on any error

Your funds are safe even if the LLM is compromised.
