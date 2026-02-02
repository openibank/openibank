//! OpeniBank LLM - Unified LLM Provider Abstraction
//!
//! This crate provides a single interface for both local and cloud LLMs:
//!
//! ## Local Providers (no API keys required)
//! - Ollama (default): `http://localhost:11434`
//! - OpenAI-compatible: vLLM, llama.cpp, etc.
//!
//! ## Cloud Providers
//! - OpenAI (GPT)
//! - Anthropic (Claude)
//! - Google (Gemini)
//! - xAI (Grok)
//!
//! ## Key Design Principles
//!
//! 1. LLMs may **propose** intents, NEVER **execute** money
//! 2. All LLM outputs are validated before use
//! 3. Deterministic fallback when no LLM is available
//! 4. JSON-mode for structured outputs

pub mod providers;
pub mod router;
pub mod types;

pub use providers::*;
pub use router::*;
pub use types::*;
