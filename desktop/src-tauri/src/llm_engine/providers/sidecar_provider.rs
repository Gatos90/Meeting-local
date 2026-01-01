//! Sidecar LLM provider
//!
//! Runs LLM inference in a separate process to avoid GGML symbol conflicts
//! with whisper-rs. Communicates via JSON-RPC over stdin/stdout.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Windows flag to prevent console window from appearing
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

use crate::llm_engine::provider::{
    CompletionRequest, CompletionResponse, FunctionCall, LlmError, LlmModelInfo, LlmProvider,
    Message, MessageRole, ProviderCapabilities, StreamCallback, ToolCall,
};

// ============================================================================
// JSON-RPC Types (matching sidecar)
// ============================================================================

#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

impl JsonRpcRequest {
    fn new(id: u64, method: &str, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        }
    }
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct SidecarConfig {
    /// Directory where GGUF models are stored
    pub models_dir: PathBuf,
    /// Path to the sidecar binary
    pub sidecar_path: Option<PathBuf>,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            models_dir: dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("meeting-local")
                .join("llm_models"),
            sidecar_path: None,
        }
    }
}

// ============================================================================
// Sidecar Process Manager
// ============================================================================

struct SidecarProcess {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
    request_id: u64,
}

impl SidecarProcess {
    async fn send_request(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, LlmError> {
        self.request_id += 1;
        let request = JsonRpcRequest::new(self.request_id, method, params);

        let request_json = serde_json::to_string(&request)
            .map_err(|e| LlmError::RequestFailed(format!("Failed to serialize request: {}", e)))?;

        // Send request
        self.stdin
            .write_all(request_json.as_bytes())
            .await
            .map_err(|e| LlmError::RequestFailed(format!("Failed to write to sidecar: {}", e)))?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| LlmError::RequestFailed(format!("Failed to write newline: {}", e)))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| LlmError::RequestFailed(format!("Failed to flush: {}", e)))?;

        // Read response
        let mut line = String::new();
        self.stdout
            .read_line(&mut line)
            .await
            .map_err(|e| LlmError::RequestFailed(format!("Failed to read from sidecar: {}", e)))?;

        let response: JsonRpcResponse = serde_json::from_str(&line)
            .map_err(|e| LlmError::RequestFailed(format!("Failed to parse response: {}", e)))?;

        if let Some(error) = response.error {
            return Err(LlmError::RequestFailed(error.message));
        }

        response.result.ok_or_else(|| LlmError::RequestFailed("Empty response".to_string()))
    }

    async fn send_streaming_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
        callback: &StreamCallback,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<serde_json::Value, LlmError> {
        self.request_id += 1;
        let request = JsonRpcRequest::new(self.request_id, method, params);

        let request_json = serde_json::to_string(&request)
            .map_err(|e| LlmError::RequestFailed(format!("Failed to serialize request: {}", e)))?;

        // Send request
        self.stdin
            .write_all(request_json.as_bytes())
            .await
            .map_err(|e| LlmError::RequestFailed(format!("Failed to write to sidecar: {}", e)))?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| LlmError::RequestFailed(format!("Failed to write newline: {}", e)))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| LlmError::RequestFailed(format!("Failed to flush: {}", e)))?;

        // Read streaming responses with cancellation support
        loop {
            let mut line = String::new();

            // Use tokio::select! to check for cancellation while reading
            let read_result = if let Some(token) = cancel_token {
                tokio::select! {
                    biased;
                    _ = token.cancelled() => {
                        return Err(LlmError::RequestFailed("Cancelled".to_string()));
                    }
                    result = self.stdout.read_line(&mut line) => result,
                }
            } else {
                self.stdout.read_line(&mut line).await
            };

            read_result
                .map_err(|e| LlmError::RequestFailed(format!("Failed to read from sidecar: {}", e)))?;

            let response: JsonRpcResponse = serde_json::from_str(&line)
                .map_err(|e| LlmError::RequestFailed(format!("Failed to parse response: {}", e)))?;

            if let Some(error) = response.error {
                return Err(LlmError::RequestFailed(error.message));
            }

            if let Some(ref result) = response.result {
                // Check if this is a token or final response
                if let Some(token) = result.get("token").and_then(|t| t.as_str()) {
                    callback(token.to_string());
                }

                if result.get("done").and_then(|d| d.as_bool()).unwrap_or(false) {
                    return Ok(response.result.unwrap());
                }
            }
        }
    }

    /// Kill this sidecar process (used for cancellation)
    fn kill(&mut self) {
        // child.start_kill() initiates process termination
        let _ = self.child.start_kill();
    }
}

// ============================================================================
// Provider Implementation
// ============================================================================

pub struct SidecarProvider {
    config: SidecarConfig,
    process: Arc<RwLock<Option<SidecarProcess>>>,
    current_model: Arc<RwLock<Option<String>>>,
}

impl SidecarProvider {
    pub fn new(config: SidecarConfig) -> Self {
        // Ensure models directory exists
        if !config.models_dir.exists() {
            std::fs::create_dir_all(&config.models_dir).ok();
        }

        Self {
            config,
            process: Arc::new(RwLock::new(None)),
            current_model: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(SidecarConfig::default())
    }

    /// Find the sidecar binary path
    fn find_sidecar_path(&self) -> Result<PathBuf, LlmError> {
        // Check configured path first
        if let Some(ref path) = self.config.sidecar_path {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        let sidecar_name = if cfg!(windows) {
            "llm-sidecar.exe"
        } else {
            "llm-sidecar"
        };

        // Check relative to executable
        if let Ok(exe_path) = std::env::current_exe() {
            let exe_dir = exe_path.parent().unwrap_or(std::path::Path::new("."));

            // 1. Same directory as main exe (workspace build or bundled app)
            let path = exe_dir.join(sidecar_name);
            if path.exists() {
                log::debug!("Found sidecar in exe dir: {}", path.display());
                return Ok(path);
            }

            // 2. Check parent directories (for dev builds where exe is in target/debug)
            let mut current = exe_dir;
            for _ in 0..3 {
                if let Some(parent) = current.parent() {
                    // Check target/debug and target/release
                    for profile in &["debug", "release"] {
                        let path = parent.join("target").join(profile).join(sidecar_name);
                        if path.exists() {
                            log::debug!("Found sidecar at: {}", path.display());
                            return Ok(path);
                        }
                    }
                    current = parent;
                }
            }
        }

        Err(LlmError::ProviderUnavailable(
            "LLM sidecar binary not found. Please build it with: cargo build -p llm-sidecar".to_string()
        ))
    }

    /// Start the sidecar process
    async fn start_sidecar(&self) -> Result<(), LlmError> {
        let sidecar_path = self.find_sidecar_path()?;

        log::info!("Starting LLM sidecar: {}", sidecar_path.display());

        let mut cmd = Command::new(&sidecar_path);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()); // Let sidecar logs go to our stderr

        // Hide console window on Windows
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd.spawn()
            .map_err(|e| LlmError::ProviderUnavailable(format!("Failed to start sidecar: {}", e)))?;

        let stdin = child.stdin.take()
            .ok_or_else(|| LlmError::ProviderUnavailable("Failed to get sidecar stdin".to_string()))?;
        let stdout = child.stdout.take()
            .ok_or_else(|| LlmError::ProviderUnavailable("Failed to get sidecar stdout".to_string()))?;

        let process = SidecarProcess {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            request_id: 0,
        };

        let mut guard = self.process.write().await;
        *guard = Some(process);

        log::info!("LLM sidecar started successfully");
        Ok(())
    }

    /// Ensure sidecar is running
    async fn ensure_sidecar(&self) -> Result<(), LlmError> {
        let guard = self.process.read().await;
        if guard.is_none() {
            drop(guard);
            self.start_sidecar().await?;
        }
        Ok(())
    }

    /// Kill and restart the sidecar process (used for cancellation)
    pub async fn restart_sidecar(&self) -> Result<(), LlmError> {
        log::info!("Restarting sidecar process for cancellation");

        // Kill current process if running
        {
            let mut guard = self.process.write().await;
            if let Some(mut process) = guard.take() {
                process.kill();
            }
        }

        // Clear current model (will need to reload after restart)
        *self.current_model.write().await = None;

        // Sidecar will be respawned on next request via ensure_sidecar
        Ok(())
    }

    /// Get list of available GGUF models
    fn available_models(&self) -> Vec<(String, PathBuf, u64)> {
        let mut models = Vec::new();

        if !self.config.models_dir.exists() {
            return models;
        }

        if let Ok(entries) = std::fs::read_dir(&self.config.models_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "gguf").unwrap_or(false) {
                    if let Some(stem) = path.file_stem() {
                        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                        models.push((stem.to_string_lossy().to_string(), path, size));
                    }
                }
            }
        }

        models
    }
}

#[async_trait]
impl LlmProvider for SidecarProvider {
    fn provider_name(&self) -> &'static str {
        "embedded"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            chat: true,
            function_calling: true, // Sidecar supports tool calling via mistral.rs
            vision: false,
            embedded: true,
            requires_api_key: false,
            supports_download: true,
        }
    }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>, LlmError> {
        let current = self.current_model.read().await.clone();

        Ok(self
            .available_models()
            .into_iter()
            .map(|(id, _path, size)| {
                let is_loaded = current.as_ref() == Some(&id);

                LlmModelInfo {
                    id: id.clone(),
                    name: id.clone(),
                    description: Some("Local GGUF model".to_string()),
                    size_bytes: Some(size),
                    is_local: true,
                    is_loaded,
                    context_length: None,
                    provider: "embedded".to_string(),
                }
            })
            .collect())
    }

    async fn is_ready(&self) -> bool {
        self.current_model.read().await.is_some()
    }

    async fn initialize(&self, model_id: &str) -> Result<(), LlmError> {
        // Check if already loaded
        {
            let current = self.current_model.read().await;
            if current.as_ref() == Some(&model_id.to_string()) {
                log::info!("Model {} already loaded", model_id);
                return Ok(());
            }
        }

        // Find model file
        let model_path = self.config.models_dir.join(format!("{}.gguf", model_id));
        if !model_path.exists() {
            return Err(LlmError::ModelNotFound(format!(
                "Model file not found: {}",
                model_path.display()
            )));
        }

        // Ensure sidecar is running
        self.ensure_sidecar().await?;

        // Send initialize request (tokenizer is extracted from GGUF metadata)
        let params = serde_json::json!({
            "model_path": model_path.to_string_lossy()
        });

        let mut guard = self.process.write().await;
        let process = guard.as_mut().ok_or(LlmError::NotInitialized)?;

        let result = process.send_request("initialize", params).await?;

        if result.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
            *self.current_model.write().await = Some(model_id.to_string());

            log::info!("Model {} loaded successfully", model_id);
            Ok(())
        } else {
            Err(LlmError::ModelLoadFailed("Sidecar failed to load model".to_string()))
        }
    }

    async fn current_model(&self) -> Option<String> {
        self.current_model.read().await.clone()
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        self.ensure_sidecar().await?;

        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                let mut msg = serde_json::json!({
                    "role": match m.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::Tool => "tool",
                    },
                    "content": m.content
                });

                // Add tool_calls for assistant messages
                if let Some(ref tool_calls) = m.tool_calls {
                    msg["tool_calls"] = serde_json::to_value(tool_calls).unwrap_or_default();
                }

                // Add tool_call_id for tool result messages
                if let Some(ref tool_call_id) = m.tool_call_id {
                    msg["tool_call_id"] = serde_json::Value::String(tool_call_id.clone());
                }

                msg
            })
            .collect();

        // Build params with optional tools
        let mut params = serde_json::json!({
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(512),
            "stream": false
        });

        // Add tools if provided
        if let Some(ref tools) = request.tools {
            params["tools"] = serde_json::to_value(tools).unwrap_or_default();
        }
        if let Some(ref tool_choice) = request.tool_choice {
            params["tool_choice"] = serde_json::Value::String(tool_choice.clone());
        }

        let mut guard = self.process.write().await;
        let process = guard.as_mut().ok_or(LlmError::NotInitialized)?;

        let result = process.send_request("complete", params).await?;

        let content = result.get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        let model = result.get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown")
            .to_string();
        let finish_reason = result.get("finish_reason")
            .and_then(|f| f.as_str())
            .unwrap_or("stop")
            .to_string();

        // Parse tool_calls if present
        let tool_calls: Option<Vec<ToolCall>> = result.get("tool_calls")
            .and_then(|tc| serde_json::from_value(tc.clone()).ok());

        Ok(CompletionResponse {
            content,
            model,
            prompt_tokens: None,
            completion_tokens: None,
            truncated: false,
            finish_reason: Some(finish_reason),
            tool_calls,
        })
    }

    async fn complete_streaming(
        &self,
        request: CompletionRequest,
        callback: StreamCallback,
        cancel_token: Option<CancellationToken>,
    ) -> Result<CompletionResponse, LlmError> {
        self.ensure_sidecar().await?;

        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                let mut msg = serde_json::json!({
                    "role": match m.role {
                        MessageRole::System => "system",
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::Tool => "tool",
                    },
                    "content": m.content
                });

                // Add tool_calls for assistant messages
                if let Some(ref tool_calls) = m.tool_calls {
                    msg["tool_calls"] = serde_json::to_value(tool_calls).unwrap_or_default();
                }

                // Add tool_call_id for tool result messages
                if let Some(ref tool_call_id) = m.tool_call_id {
                    msg["tool_call_id"] = serde_json::Value::String(tool_call_id.clone());
                }

                msg
            })
            .collect();

        // Build params with optional tools
        let mut params = serde_json::json!({
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(512),
            "stream": true
        });

        // Add tools if provided
        if let Some(ref tools) = request.tools {
            params["tools"] = serde_json::to_value(tools).unwrap_or_default();
        }
        if let Some(ref tool_choice) = request.tool_choice {
            params["tool_choice"] = serde_json::Value::String(tool_choice.clone());
        }

        let result = {
            let mut guard = self.process.write().await;
            let process = guard.as_mut().ok_or(LlmError::NotInitialized)?;
            process.send_streaming_request("complete", params, &callback, cancel_token.as_ref()).await
        };

        // Handle cancellation - restart sidecar since generation can't be cleanly stopped
        match &result {
            Err(LlmError::RequestFailed(msg)) if msg == "Cancelled" => {
                log::info!("Streaming cancelled, restarting sidecar");
                self.restart_sidecar().await?;
                return Err(LlmError::RequestFailed("Cancelled".to_string()));
            }
            Err(e) => return Err(e.clone()),
            Ok(_) => {}
        }

        let result = result?;

        let content = result.get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        let model = result.get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown")
            .to_string();
        let finish_reason = result.get("finish_reason")
            .and_then(|f| f.as_str())
            .unwrap_or("stop")
            .to_string();

        // Parse tool_calls if present
        let tool_calls: Option<Vec<ToolCall>> = result.get("tool_calls")
            .and_then(|tc| serde_json::from_value(tc.clone()).ok());

        Ok(CompletionResponse {
            content,
            model,
            prompt_tokens: None,
            completion_tokens: None,
            truncated: false,
            finish_reason: Some(finish_reason),
            tool_calls,
        })
    }

    async fn shutdown(&self) -> Result<(), LlmError> {
        let mut guard = self.process.write().await;
        if let Some(mut process) = guard.take() {
            // Send shutdown request
            let _ = process.send_request("shutdown", serde_json::json!({})).await;

            // Kill process
            let _ = process.child.kill().await;
        }

        *self.current_model.write().await = None;
        log::info!("Sidecar provider shut down");
        Ok(())
    }
}
