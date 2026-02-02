//! LLM Router - Selects and manages LLM providers

use std::sync::Arc;

use crate::providers::*;
use crate::types::*;

/// The LLM Router selects and manages providers based on configuration
pub struct LLMRouter {
    provider: Arc<dyn LLMProvider>,
    kind: ProviderKind,
}

impl LLMRouter {
    /// Create a router with a specific provider
    pub fn new(provider: Arc<dyn LLMProvider>) -> Self {
        let kind = provider.kind();
        Self { provider, kind }
    }

    /// Create a router from environment variables
    ///
    /// Reads `OPENIBANK_LLM_PROVIDER` to select the provider:
    /// - `ollama` (default): Local Ollama instance
    /// - `openai_compat`: OpenAI-compatible local server
    /// - `openai`: OpenAI API
    /// - `anthropic`: Anthropic Claude API
    /// - `gemini`: Google Gemini API
    /// - `grok`: xAI Grok API
    /// - `deterministic`: No LLM, deterministic fallback
    pub fn from_env() -> Self {
        // Try to load .env file (ignore errors)
        let _ = dotenvy::dotenv();

        let provider_name = std::env::var("OPENIBANK_LLM_PROVIDER")
            .unwrap_or_else(|_| "ollama".to_string());

        let kind = ProviderKind::from_str(&provider_name)
            .unwrap_or(ProviderKind::Ollama);

        Self::from_kind(kind)
    }

    /// Create a router for a specific provider kind
    pub fn from_kind(kind: ProviderKind) -> Self {
        let provider: Arc<dyn LLMProvider> = match kind {
            ProviderKind::Ollama => Arc::new(OllamaProvider::from_env()),
            ProviderKind::OpenAICompat => Arc::new(OpenAICompatProvider::from_env()),
            ProviderKind::OpenAI => {
                if let Some(p) = OpenAIProvider::from_env() {
                    Arc::new(p)
                } else {
                    tracing::warn!("OpenAI API key not found, using deterministic fallback");
                    Arc::new(DeterministicProvider::new())
                }
            }
            ProviderKind::Anthropic => {
                if let Some(p) = AnthropicProvider::from_env() {
                    Arc::new(p)
                } else {
                    tracing::warn!("Anthropic API key not found, using deterministic fallback");
                    Arc::new(DeterministicProvider::new())
                }
            }
            ProviderKind::Gemini => {
                // Gemini not fully implemented, fall back
                tracing::warn!("Gemini provider not yet implemented, using deterministic fallback");
                Arc::new(DeterministicProvider::new())
            }
            ProviderKind::Grok => {
                // Grok not fully implemented, fall back
                tracing::warn!("Grok provider not yet implemented, using deterministic fallback");
                Arc::new(DeterministicProvider::new())
            }
            ProviderKind::Deterministic => Arc::new(DeterministicProvider::new()),
        };

        Self { provider, kind }
    }

    /// Get the current provider
    pub fn provider(&self) -> &Arc<dyn LLMProvider> {
        &self.provider
    }

    /// Get the provider kind
    pub fn kind(&self) -> ProviderKind {
        self.kind
    }

    /// Check if the provider is available
    pub async fn is_available(&self) -> bool {
        self.provider.is_available().await
    }

    /// Complete a request using the current provider
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        self.provider.complete(request).await
    }

    /// Try to use the configured provider, falling back to deterministic if unavailable
    pub async fn complete_with_fallback(
        &self,
        request: CompletionRequest,
    ) -> (CompletionResponse, ProviderKind) {
        // Try the configured provider first
        if self.provider.is_available().await {
            match self.provider.complete(request.clone()).await {
                Ok(response) => return (response, self.kind),
                Err(e) => {
                    tracing::warn!("Provider {} failed: {}, using fallback", self.kind, e);
                }
            }
        }

        // Fall back to deterministic
        let fallback = DeterministicProvider::new();
        let response = fallback
            .complete(request)
            .await
            .unwrap_or_else(|_| CompletionResponse::new("Fallback failed"));

        (response, ProviderKind::Deterministic)
    }
}

impl Default for LLMRouter {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Builder for LLM router with explicit configuration
pub struct LLMRouterBuilder {
    kind: Option<ProviderKind>,
    ollama_config: Option<OllamaConfig>,
    openai_compat_config: Option<OpenAICompatConfig>,
    openai_config: Option<OpenAIConfig>,
    anthropic_config: Option<AnthropicConfig>,
}

impl LLMRouterBuilder {
    pub fn new() -> Self {
        Self {
            kind: None,
            ollama_config: None,
            openai_compat_config: None,
            openai_config: None,
            anthropic_config: None,
        }
    }

    pub fn with_kind(mut self, kind: ProviderKind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn with_ollama(mut self, config: OllamaConfig) -> Self {
        self.ollama_config = Some(config);
        self.kind = Some(ProviderKind::Ollama);
        self
    }

    pub fn with_openai_compat(mut self, config: OpenAICompatConfig) -> Self {
        self.openai_compat_config = Some(config);
        self.kind = Some(ProviderKind::OpenAICompat);
        self
    }

    pub fn with_openai(mut self, config: OpenAIConfig) -> Self {
        self.openai_config = Some(config);
        self.kind = Some(ProviderKind::OpenAI);
        self
    }

    pub fn with_anthropic(mut self, config: AnthropicConfig) -> Self {
        self.anthropic_config = Some(config);
        self.kind = Some(ProviderKind::Anthropic);
        self
    }

    pub fn build(self) -> LLMRouter {
        let kind = self.kind.unwrap_or(ProviderKind::Deterministic);

        let provider: Arc<dyn LLMProvider> = match kind {
            ProviderKind::Ollama => {
                let config = self.ollama_config.unwrap_or_default();
                Arc::new(OllamaProvider::new(config))
            }
            ProviderKind::OpenAICompat => {
                let config = self.openai_compat_config.unwrap_or_default();
                Arc::new(OpenAICompatProvider::new(config))
            }
            ProviderKind::OpenAI => {
                if let Some(config) = self.openai_config {
                    Arc::new(OpenAIProvider::new(config))
                } else {
                    Arc::new(DeterministicProvider::new())
                }
            }
            ProviderKind::Anthropic => {
                if let Some(config) = self.anthropic_config {
                    Arc::new(AnthropicProvider::new(config))
                } else {
                    Arc::new(DeterministicProvider::new())
                }
            }
            _ => Arc::new(DeterministicProvider::new()),
        };

        LLMRouter::new(provider)
    }
}

impl Default for LLMRouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_deterministic_fallback() {
        let router = LLMRouter::from_kind(ProviderKind::Deterministic);
        assert!(router.is_available().await);

        let request = CompletionRequest::new(vec![Message::user("Hello")]);
        let response = router.complete(request).await.unwrap();

        assert!(response.content.contains("deterministic"));
    }

    #[test]
    fn test_provider_kind_parsing() {
        assert_eq!(
            ProviderKind::from_str("ollama"),
            Some(ProviderKind::Ollama)
        );
        assert_eq!(
            ProviderKind::from_str("anthropic"),
            Some(ProviderKind::Anthropic)
        );
        assert_eq!(
            ProviderKind::from_str("claude"),
            Some(ProviderKind::Anthropic)
        );
        assert_eq!(ProviderKind::from_str("openai"), Some(ProviderKind::OpenAI));
        assert_eq!(ProviderKind::from_str("unknown"), None);
    }

    #[test]
    fn test_router_builder() {
        let router = LLMRouterBuilder::new()
            .with_kind(ProviderKind::Deterministic)
            .build();

        assert_eq!(router.kind(), ProviderKind::Deterministic);
    }
}
