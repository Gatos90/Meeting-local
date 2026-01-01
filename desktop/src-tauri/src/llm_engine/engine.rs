//! LLM Engine - manages multiple LLM providers
//!
//! Handles provider selection, initialization, and provides a unified interface

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use std::path::PathBuf;

use crate::llm_engine::provider::{
    CompletionRequest, CompletionResponse, LlmError, LlmModelInfo, LlmProvider,
    ProviderCapabilities, ProviderType, StreamCallback,
};
use crate::llm_engine::providers::{OllamaProvider, SidecarProvider, SidecarConfig};

/// The main LLM engine that manages providers
pub struct LlmEngine {
    /// All registered providers
    providers: HashMap<ProviderType, Arc<dyn LlmProvider>>,
    /// Currently active provider
    active_provider: Arc<RwLock<Option<ProviderType>>>,
}

impl LlmEngine {
    /// Create a new LLM engine with default providers
    pub fn new() -> Self {
        Self::with_models_dir(None)
    }

    /// Create a new LLM engine with a custom models directory
    pub fn with_models_dir(models_dir: Option<PathBuf>) -> Self {
        let mut providers: HashMap<ProviderType, Arc<dyn LlmProvider>> = HashMap::new();

        // Register Ollama provider
        providers.insert(
            ProviderType::Ollama,
            Arc::new(OllamaProvider::with_default_config()),
        );

        // Register embedded provider (via sidecar for GGML isolation)
        let sidecar_config = if let Some(dir) = models_dir {
            SidecarConfig {
                models_dir: dir,
                ..SidecarConfig::default()
            }
        } else {
            SidecarConfig::default()
        };
        providers.insert(
            ProviderType::Embedded,
            Arc::new(SidecarProvider::new(sidecar_config)),
        );

        // TODO: Register OpenAI provider
        // TODO: Register Claude provider

        Self {
            providers,
            active_provider: Arc::new(RwLock::new(None)),
        }
    }

    /// Get list of available provider types
    pub fn available_providers(&self) -> Vec<ProviderType> {
        self.providers.keys().cloned().collect()
    }

    /// Get capabilities for a specific provider
    pub fn provider_capabilities(&self, provider_type: &ProviderType) -> Option<ProviderCapabilities> {
        self.providers.get(provider_type).map(|p| p.capabilities())
    }

    /// Get the active provider type
    pub async fn active_provider_type(&self) -> Option<ProviderType> {
        self.active_provider.read().await.clone()
    }

    /// Set the active provider
    pub async fn set_active_provider(&self, provider_type: ProviderType) -> Result<(), LlmError> {
        if !self.providers.contains_key(&provider_type) {
            return Err(LlmError::ProviderUnavailable(format!(
                "Provider {:?} not registered",
                provider_type
            )));
        }

        *self.active_provider.write().await = Some(provider_type);
        Ok(())
    }

    /// Get the active provider
    async fn get_active_provider(&self) -> Result<Arc<dyn LlmProvider>, LlmError> {
        let provider_type = self
            .active_provider
            .read()
            .await
            .clone()
            .ok_or(LlmError::NotInitialized)?;

        self.providers
            .get(&provider_type)
            .cloned()
            .ok_or(LlmError::NotInitialized)
    }

    /// Get a specific provider
    pub fn get_provider(&self, provider_type: &ProviderType) -> Option<Arc<dyn LlmProvider>> {
        self.providers.get(provider_type).cloned()
    }

    /// Check if the active provider is ready
    pub async fn is_ready(&self) -> bool {
        if let Ok(provider) = self.get_active_provider().await {
            provider.is_ready().await
        } else {
            false
        }
    }

    /// Initialize the active provider with a model
    pub async fn initialize(&self, model_id: &str) -> Result<(), LlmError> {
        let provider = self.get_active_provider().await?;
        provider.initialize(model_id).await
    }

    /// List models for the active provider
    pub async fn list_models(&self) -> Result<Vec<LlmModelInfo>, LlmError> {
        let provider = self.get_active_provider().await?;
        provider.list_models().await
    }

    /// List models for a specific provider
    pub async fn list_models_for_provider(
        &self,
        provider_type: &ProviderType,
    ) -> Result<Vec<LlmModelInfo>, LlmError> {
        let provider = self
            .providers
            .get(provider_type)
            .ok_or(LlmError::ProviderUnavailable(format!(
                "Provider {:?} not registered",
                provider_type
            )))?;
        provider.list_models().await
    }

    /// Get the currently loaded model
    pub async fn current_model(&self) -> Option<String> {
        if let Ok(provider) = self.get_active_provider().await {
            provider.current_model().await
        } else {
            None
        }
    }

    /// Run a completion request
    pub async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let provider = self.get_active_provider().await?;
        provider.complete(request).await
    }

    /// Run a streaming completion request
    /// Optional cancel_token allows cancelling the stream mid-generation
    pub async fn complete_streaming(
        &self,
        request: CompletionRequest,
        callback: StreamCallback,
        cancel_token: Option<tokio_util::sync::CancellationToken>,
    ) -> Result<CompletionResponse, LlmError> {
        let provider = self.get_active_provider().await?;
        provider.complete_streaming(request, callback, cancel_token).await
    }

    /// Shutdown the active provider
    pub async fn shutdown(&self) -> Result<(), LlmError> {
        if let Ok(provider) = self.get_active_provider().await {
            provider.shutdown().await?;
        }
        *self.active_provider.write().await = None;
        Ok(())
    }

    // === Ollama-specific methods ===

    /// Check Ollama connection and return version
    pub async fn ollama_check_connection(&self) -> Result<String, LlmError> {
        // Get the Ollama provider directly since we know its concrete type
        if let Some(provider) = self.providers.get(&ProviderType::Ollama) {
            // We store OllamaProvider wrapped in Arc<dyn LlmProvider>
            // Since we control registration, we keep a separate typed reference
            self.ollama_provider_check().await
        } else {
            Err(LlmError::ProviderUnavailable("Ollama provider not registered".to_string()))
        }
    }

    /// Internal helper for Ollama connection check
    async fn ollama_provider_check(&self) -> Result<String, LlmError> {
        // Create a temporary provider to check connection
        let ollama = OllamaProvider::with_default_config();
        ollama.check_connection().await
    }
}

impl Default for LlmEngine {
    fn default() -> Self {
        Self::new()
    }
}
