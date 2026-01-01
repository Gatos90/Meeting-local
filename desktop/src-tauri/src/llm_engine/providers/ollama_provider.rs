//! Ollama API provider
//!
//! Connects to a running Ollama server (default: localhost:11434)

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::llm_engine::provider::{
    CompletionRequest, CompletionResponse, LlmError, LlmModelInfo, LlmProvider,
    Message, MessageRole, ProviderCapabilities, StreamCallback,
};

/// Ollama API message format
#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

impl From<&Message> for OllamaMessage {
    fn from(msg: &Message) -> Self {
        Self {
            role: match msg.role {
                MessageRole::System => "system".to_string(),
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::Tool => "tool".to_string(),
            },
            content: msg.content.clone(),
        }
    }
}

/// Ollama chat request
#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
}

/// Ollama chat response
#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
    model: String,
    done: bool,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

/// Ollama model list response
#[derive(Debug, Deserialize)]
struct OllamaModelList {
    models: Vec<OllamaModelEntry>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelEntry {
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    details: Option<OllamaModelDetails>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelDetails {
    #[serde(default)]
    parameter_size: Option<String>,
    #[serde(default)]
    family: Option<String>,
}

/// Ollama version response
#[derive(Debug, Deserialize)]
struct OllamaVersion {
    version: String,
}

/// Ollama provider configuration
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub base_url: String,
    pub timeout_secs: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            timeout_secs: 120,
        }
    }
}

/// Ollama LLM provider
pub struct OllamaProvider {
    config: OllamaConfig,
    client: Client,
    current_model: Arc<RwLock<Option<String>>>,
}

impl OllamaProvider {
    pub fn new(config: OllamaConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            current_model: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(OllamaConfig::default())
    }

    /// Check if Ollama server is running
    pub async fn check_connection(&self) -> Result<String, LlmError> {
        let url = format!("{}/api/version", self.config.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| LlmError::ProviderUnavailable(format!("Cannot connect to Ollama: {}", e)))?;

        if !response.status().is_success() {
            return Err(LlmError::ProviderUnavailable(
                "Ollama server returned error".to_string(),
            ));
        }

        let version: OllamaVersion = response
            .json()
            .await
            .map_err(|e| LlmError::ProviderUnavailable(format!("Invalid response: {}", e)))?;

        Ok(version.version)
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn provider_name(&self) -> &'static str {
        "ollama"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            chat: true,
            function_calling: false, // Some Ollama models support this, but not all
            vision: false,           // Some models support, but needs detection
            embedded: false,
            requires_api_key: false,
            supports_download: true, // Ollama can pull models
        }
    }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>, LlmError> {
        let url = format!("{}/api/tags", self.config.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| LlmError::ProviderUnavailable(format!("Cannot connect to Ollama: {}", e)))?;

        if !response.status().is_success() {
            return Err(LlmError::RequestFailed(
                "Failed to list Ollama models".to_string(),
            ));
        }

        let model_list: OllamaModelList = response
            .json()
            .await
            .map_err(|e| LlmError::RequestFailed(format!("Invalid response: {}", e)))?;

        let current = self.current_model.read().await;

        Ok(model_list
            .models
            .into_iter()
            .map(|m| {
                let description = m.details.as_ref().map(|d| {
                    let mut parts = Vec::new();
                    if let Some(ref family) = d.family {
                        parts.push(family.clone());
                    }
                    if let Some(ref size) = d.parameter_size {
                        parts.push(size.clone());
                    }
                    parts.join(" - ")
                });

                LlmModelInfo {
                    id: m.name.clone(),
                    name: m.name.clone(),
                    description,
                    size_bytes: Some(m.size),
                    is_local: true,
                    is_loaded: current.as_ref() == Some(&m.name),
                    context_length: None, // Ollama doesn't expose this directly
                    provider: "ollama".to_string(),
                }
            })
            .collect())
    }

    async fn is_ready(&self) -> bool {
        self.check_connection().await.is_ok() && self.current_model.read().await.is_some()
    }

    async fn initialize(&self, model_id: &str) -> Result<(), LlmError> {
        // Verify the model exists by trying to get its info
        let models = self.list_models().await?;

        if !models.iter().any(|m| m.id == model_id) {
            return Err(LlmError::ModelNotFound(format!(
                "Model '{}' not found in Ollama. Available models: {:?}",
                model_id,
                models.iter().map(|m| &m.id).collect::<Vec<_>>()
            )));
        }

        // Set the current model
        *self.current_model.write().await = Some(model_id.to_string());

        log::info!("Ollama provider initialized with model: {}", model_id);
        Ok(())
    }

    async fn current_model(&self) -> Option<String> {
        self.current_model.read().await.clone()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let model = self
            .current_model
            .read()
            .await
            .clone()
            .ok_or(LlmError::NotInitialized)?;

        let url = format!("{}/api/chat", self.config.base_url);

        let ollama_request = OllamaChatRequest {
            model: model.clone(),
            messages: request.messages.iter().map(OllamaMessage::from).collect(),
            stream: false,
            options: Some(OllamaOptions {
                temperature: request.temperature,
                top_p: request.top_p,
                num_predict: request.max_tokens,
                stop: request.stop,
            }),
        };

        let response = self
            .client
            .post(&url)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::RequestFailed(format!(
                "Ollama returned error: {}",
                error_text
            )));
        }

        let ollama_response: OllamaChatResponse = response
            .json()
            .await
            .map_err(|e| LlmError::RequestFailed(format!("Invalid response: {}", e)))?;

        Ok(CompletionResponse {
            content: ollama_response.message.content,
            model: ollama_response.model,
            prompt_tokens: ollama_response.prompt_eval_count,
            completion_tokens: ollama_response.eval_count,
            truncated: false,
            finish_reason: if ollama_response.done {
                Some("stop".to_string())
            } else {
                None
            },
            tool_calls: None, // Ollama doesn't support tool calling yet
        })
    }

    async fn complete_streaming(
        &self,
        request: CompletionRequest,
        callback: StreamCallback,
        _cancel_token: Option<tokio_util::sync::CancellationToken>,
    ) -> Result<CompletionResponse, LlmError> {
        let model = self
            .current_model
            .read()
            .await
            .clone()
            .ok_or(LlmError::NotInitialized)?;

        let url = format!("{}/api/chat", self.config.base_url);

        let ollama_request = OllamaChatRequest {
            model: model.clone(),
            messages: request.messages.iter().map(OllamaMessage::from).collect(),
            stream: true,
            options: Some(OllamaOptions {
                temperature: request.temperature,
                top_p: request.top_p,
                num_predict: request.max_tokens,
                stop: request.stop,
            }),
        };

        let response = self
            .client
            .post(&url)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| LlmError::RequestFailed(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::RequestFailed(format!(
                "Ollama returned error: {}",
                error_text
            )));
        }

        let mut full_content = String::new();
        let mut prompt_tokens = None;
        let mut completion_tokens = None;

        // Stream the response
        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result
                .map_err(|e| LlmError::RequestFailed(format!("Stream error: {}", e)))?;

            // Parse NDJSON - each line is a JSON object
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                if line.trim().is_empty() {
                    continue;
                }

                if let Ok(resp) = serde_json::from_str::<OllamaChatResponse>(line) {
                    if !resp.message.content.is_empty() {
                        callback(resp.message.content.clone());
                        full_content.push_str(&resp.message.content);
                    }

                    if resp.done {
                        prompt_tokens = resp.prompt_eval_count;
                        completion_tokens = resp.eval_count;
                    }
                }
            }
        }

        Ok(CompletionResponse {
            content: full_content,
            model,
            prompt_tokens,
            completion_tokens,
            truncated: false,
            finish_reason: Some("stop".to_string()),
            tool_calls: None, // Ollama doesn't support tool calling yet
        })
    }

    async fn shutdown(&self) -> Result<(), LlmError> {
        *self.current_model.write().await = None;
        log::info!("Ollama provider shut down");
        Ok(())
    }
}
