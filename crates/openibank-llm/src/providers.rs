//! LLM Provider implementations

use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::types::*;

/// Trait for LLM providers
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &'static str;

    /// Get the provider kind
    fn kind(&self) -> ProviderKind;

    /// Check if the provider is available
    async fn is_available(&self) -> bool;

    /// Complete a conversation
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Stream a completion (optional)
    async fn complete_stream(
        &self,
        request: CompletionRequest,
    ) -> Result<BoxStream<'static, Result<StreamChunk>>> {
        // Default implementation: non-streaming fallback
        let response = self.complete(request).await?;
        let chunk = StreamChunk {
            delta: response.content,
            is_final: true,
        };
        Ok(Box::pin(futures::stream::once(async { Ok(chunk) })))
    }
}

// ============================================================================
// Ollama Provider (Local, Default)
// ============================================================================

/// Configuration for Ollama provider
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub model: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("OPENIBANK_OLLAMA_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            model: std::env::var("OPENIBANK_OLLAMA_MODEL")
                .unwrap_or_else(|_| "llama3.1:8b".to_string()),
        }
    }
}

/// Ollama local LLM provider
pub struct OllamaProvider {
    config: OllamaConfig,
    client: reqwest::Client,
}

impl OllamaProvider {
    pub fn new(config: OllamaConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Self {
        Self::new(OllamaConfig::default())
    }
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
    #[serde(default)]
    done: bool,
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    fn name(&self) -> &'static str {
        "Ollama"
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Ollama
    }

    async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.config.base_url);
        self.client.get(&url).send().await.is_ok()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Convert messages to a single prompt
        let prompt = request
            .messages
            .iter()
            .map(|m| match m.role {
                MessageRole::User => format!("User: {}", m.content),
                MessageRole::Assistant => format!("Assistant: {}", m.content),
                MessageRole::System => format!("System: {}", m.content),
                MessageRole::Tool => format!("Tool: {}", m.content),
            })
            .collect::<Vec<_>>()
            .join("\n\n")
            + "\n\nAssistant:";

        // Add JSON mode instruction if needed
        let system = if request.json_mode {
            Some(
                request
                    .system
                    .clone()
                    .unwrap_or_default()
                    + "\n\nIMPORTANT: You must respond with valid JSON only. No other text.",
            )
        } else {
            request.system.clone()
        };

        let ollama_request = OllamaRequest {
            model: request.model.unwrap_or_else(|| self.config.model.clone()),
            prompt,
            stream: false,
            system,
            options: OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            },
        };

        let url = format!("{}/api/generate", self.config.base_url);
        let response = self
            .client
            .post(&url)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| LLMError::NetworkError {
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            return Err(LLMError::RequestFailed {
                message: format!("HTTP {}", response.status()),
            });
        }

        let ollama_response: OllamaResponse =
            response.json().await.map_err(|e| LLMError::InvalidResponse {
                message: e.to_string(),
            })?;

        Ok(CompletionResponse {
            content: ollama_response.response.trim().to_string(),
            tool_calls: vec![],
            usage: TokenUsage::default(),
            raw_response: None,
            model: Some(self.config.model.clone()),
        })
    }
}

// ============================================================================
// OpenAI-Compatible Provider
// ============================================================================

/// Configuration for OpenAI-compatible provider
#[derive(Debug, Clone)]
pub struct OpenAICompatConfig {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
}

impl Default for OpenAICompatConfig {
    fn default() -> Self {
        Self {
            base_url: std::env::var("OPENIBANK_OPENAI_COMPAT_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8000/v1".to_string()),
            api_key: std::env::var("OPENIBANK_OPENAI_COMPAT_API_KEY").ok(),
            model: std::env::var("OPENIBANK_OPENAI_COMPAT_MODEL")
                .unwrap_or_else(|_| "default".to_string()),
        }
    }
}

/// OpenAI-compatible API provider (vLLM, llama.cpp, etc.)
pub struct OpenAICompatProvider {
    config: OpenAICompatConfig,
    client: reqwest::Client,
}

impl OpenAICompatProvider {
    pub fn new(config: OpenAICompatConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Self {
        Self::new(OpenAICompatConfig::default())
    }
}

#[derive(Serialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<OpenAIChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct OpenAIChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChatChoice>,
    #[serde(default)]
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIChatChoice {
    message: OpenAIChatMessage,
}

#[derive(Deserialize, Default)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[async_trait]
impl LLMProvider for OpenAICompatProvider {
    fn name(&self) -> &'static str {
        "OpenAI-Compatible"
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAICompat
    }

    async fn is_available(&self) -> bool {
        let url = format!("{}/models", self.config.base_url);
        let mut req = self.client.get(&url);
        if let Some(ref key) = self.config.api_key {
            req = req.bearer_auth(key);
        }
        req.send().await.is_ok()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let mut messages: Vec<OpenAIChatMessage> = vec![];

        // Add system message if present
        if let Some(ref system) = request.system {
            messages.push(OpenAIChatMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        // Add conversation messages
        for msg in &request.messages {
            messages.push(OpenAIChatMessage {
                role: match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                }
                .to_string(),
                content: msg.content.clone(),
            });
        }

        let chat_request = OpenAIChatRequest {
            model: request.model.unwrap_or_else(|| self.config.model.clone()),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: false,
            response_format: if request.json_mode {
                Some(serde_json::json!({"type": "json_object"}))
            } else {
                None
            },
        };

        let url = format!("{}/chat/completions", self.config.base_url);
        let mut req = self.client.post(&url).json(&chat_request);
        if let Some(ref key) = self.config.api_key {
            req = req.bearer_auth(key);
        }

        let response = req.send().await.map_err(|e| LLMError::NetworkError {
            message: e.to_string(),
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LLMError::RequestFailed {
                message: format!("HTTP {}: {}", status, body),
            });
        }

        let chat_response: OpenAIChatResponse =
            response.json().await.map_err(|e| LLMError::InvalidResponse {
                message: e.to_string(),
            })?;

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let usage = chat_response.usage.unwrap_or_default();

        Ok(CompletionResponse {
            content,
            tool_calls: vec![],
            usage: TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            },
            raw_response: None,
            model: Some(self.config.model.clone()),
        })
    }
}

// ============================================================================
// OpenAI Provider
// ============================================================================

/// Configuration for OpenAI provider
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub model: String,
}

impl OpenAIConfig {
    pub fn from_env() -> Option<Self> {
        Some(Self {
            api_key: std::env::var("OPENAI_API_KEY").ok()?,
            model: std::env::var("OPENIBANK_OPENAI_MODEL")
                .unwrap_or_else(|_| "gpt-4o-mini".to_string()),
        })
    }
}

/// OpenAI API provider
pub struct OpenAIProvider {
    config: OpenAIConfig,
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(config: OpenAIConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Option<Self> {
        Some(Self::new(OpenAIConfig::from_env()?))
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    fn name(&self) -> &'static str {
        "OpenAI"
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::OpenAI
    }

    async fn is_available(&self) -> bool {
        // Just check if we have an API key
        !self.config.api_key.is_empty()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let mut messages: Vec<OpenAIChatMessage> = vec![];

        if let Some(ref system) = request.system {
            messages.push(OpenAIChatMessage {
                role: "system".to_string(),
                content: system.clone(),
            });
        }

        for msg in &request.messages {
            messages.push(OpenAIChatMessage {
                role: match msg.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                }
                .to_string(),
                content: msg.content.clone(),
            });
        }

        let chat_request = OpenAIChatRequest {
            model: request.model.unwrap_or_else(|| self.config.model.clone()),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: false,
            response_format: if request.json_mode {
                Some(serde_json::json!({"type": "json_object"}))
            } else {
                None
            },
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.config.api_key)
            .json(&chat_request)
            .send()
            .await
            .map_err(|e| LLMError::NetworkError {
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LLMError::RequestFailed {
                message: format!("HTTP {}: {}", status, body),
            });
        }

        let chat_response: OpenAIChatResponse =
            response.json().await.map_err(|e| LLMError::InvalidResponse {
                message: e.to_string(),
            })?;

        let content = chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let usage = chat_response.usage.unwrap_or_default();

        Ok(CompletionResponse {
            content,
            tool_calls: vec![],
            usage: TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            },
            raw_response: None,
            model: Some(self.config.model.clone()),
        })
    }
}

// ============================================================================
// Anthropic Provider
// ============================================================================

/// Configuration for Anthropic provider
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub model: String,
}

impl AnthropicConfig {
    pub fn from_env() -> Option<Self> {
        Some(Self {
            api_key: std::env::var("ANTHROPIC_API_KEY").ok()?,
            model: std::env::var("OPENIBANK_ANTHROPIC_MODEL")
                .unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string()),
        })
    }
}

/// Anthropic Claude API provider
pub struct AnthropicProvider {
    config: AnthropicConfig,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(config: AnthropicConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Option<Self> {
        Some(Self::new(AnthropicConfig::from_env()?))
    }
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
}

#[derive(Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "Anthropic"
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }

    async fn is_available(&self) -> bool {
        !self.config.api_key.is_empty()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let mut messages: Vec<AnthropicMessage> = vec![];

        for msg in &request.messages {
            let role = match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                // System and Tool messages are handled separately
                MessageRole::System | MessageRole::Tool => continue,
            };
            messages.push(AnthropicMessage {
                role: role.to_string(),
                content: msg.content.clone(),
            });
        }

        let system = if request.json_mode {
            Some(
                request
                    .system
                    .clone()
                    .unwrap_or_default()
                    + "\n\nIMPORTANT: Respond with valid JSON only.",
            )
        } else {
            request.system.clone()
        };

        let anthropic_request = AnthropicRequest {
            model: request
                .model
                .unwrap_or_else(|| self.config.model.clone()),
            max_tokens: request.max_tokens.unwrap_or(4096),
            system,
            messages,
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| LLMError::NetworkError {
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LLMError::RequestFailed {
                message: format!("HTTP {}: {}", status, body),
            });
        }

        let anthropic_response: AnthropicResponse =
            response.json().await.map_err(|e| LLMError::InvalidResponse {
                message: e.to_string(),
            })?;

        let content = anthropic_response
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        Ok(CompletionResponse {
            content,
            tool_calls: vec![],
            usage: TokenUsage {
                prompt_tokens: anthropic_response.usage.input_tokens,
                completion_tokens: anthropic_response.usage.output_tokens,
                total_tokens: anthropic_response.usage.input_tokens
                    + anthropic_response.usage.output_tokens,
            },
            raw_response: None,
            model: Some(self.config.model.clone()),
        })
    }
}

// ============================================================================
// Deterministic Provider (Fallback)
// ============================================================================

/// Deterministic fallback when no LLM is available
pub struct DeterministicProvider;

impl DeterministicProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DeterministicProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMProvider for DeterministicProvider {
    fn name(&self) -> &'static str {
        "Deterministic"
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Deterministic
    }

    async fn is_available(&self) -> bool {
        true // Always available
    }

    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        // Return a structured fallback response
        Ok(CompletionResponse {
            content: r#"{"error": "No LLM available, using deterministic fallback"}"#.to_string(),
            tool_calls: vec![],
            usage: TokenUsage::default(),
            raw_response: None,
            model: Some("deterministic".to_string()),
        })
    }
}
