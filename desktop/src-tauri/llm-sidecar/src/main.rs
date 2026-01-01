//! LLM Sidecar Binary
//!
//! Runs as a separate process to handle LLM inference using mistral.rs.
//! Communicates with the main Tauri app via JSON-RPC over stdin/stdout.
//!
//! Uses mistral.rs for automatic:
//! - KV cache management with PagedAttention
//! - Context handling
//! - Chat template formatting
//! - GPU memory management

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};

/// Temporarily redirects stdout to stderr during model loading to prevent
/// mistral.rs's println! statements from corrupting our JSON-RPC protocol.
#[cfg(windows)]
mod stdout_redirect {
    use std::os::windows::io::RawHandle;
    use windows_sys::Win32::System::Console::{GetStdHandle, SetStdHandle, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE};

    pub struct StdoutRedirect {
        original_stdout: RawHandle,
    }

    impl StdoutRedirect {
        pub fn to_stderr() -> Option<Self> {
            unsafe {
                let original_stdout = GetStdHandle(STD_OUTPUT_HANDLE);
                let stderr = GetStdHandle(STD_ERROR_HANDLE);
                if SetStdHandle(STD_OUTPUT_HANDLE, stderr as _) != 0 {
                    Some(Self { original_stdout: original_stdout as _ })
                } else {
                    None
                }
            }
        }
    }

    impl Drop for StdoutRedirect {
        fn drop(&mut self) {
            unsafe {
                SetStdHandle(STD_OUTPUT_HANDLE, self.original_stdout as _);
            }
        }
    }
}

#[cfg(not(windows))]
mod stdout_redirect {
    pub struct StdoutRedirect;

    impl StdoutRedirect {
        pub fn to_stderr() -> Option<Self> {
            // On Unix, we'd use dup2 here, but for now just skip
            None
        }
    }
}

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use std::collections::HashMap;

use mistralrs::{
    GgufModelBuilder, Model, PagedAttentionMetaBuilder, MemoryGpuConfig, PagedCacheType,
    Response, TextMessageRole, RequestBuilder, Tool, ToolType, Function, ToolChoice,
    ToolCallType,
    DeviceMapSetting, AutoDeviceMapParams,
};

// ============================================================================
// JSON-RPC Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

impl JsonRpcResponse {
    fn success(id: u64, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: u64, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
struct InitializeParams {
    /// Local path to the GGUF model file
    model_path: String,
    /// Optional chat template path or literal Jinja template
    #[serde(default)]
    chat_template: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListModelsParams {
    models_dir: String,
}

#[derive(Debug, Serialize)]
struct ModelInfo {
    id: String,
    name: String,
    size_bytes: u64,
    is_loaded: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct Message {
    role: String,
    content: String,
    /// Tool calls made by the assistant (for message history)
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
    /// Tool call ID (for tool result messages)
    #[serde(default)]
    tool_call_id: Option<String>,
}

/// Tool call from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCall {
    id: String,
    function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

/// Tool definition passed to the LLM
#[derive(Debug, Deserialize)]
struct ToolDefinition {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct CompleteParams {
    messages: Vec<Message>,
    #[serde(default = "default_max_tokens")]
    max_tokens: u32,
    #[serde(default)]
    stream: bool,
    /// Tools available for the LLM to call
    #[serde(default)]
    tools: Option<Vec<ToolDefinition>>,
    /// Tool choice: "auto", "none", or "required"
    #[serde(default = "default_tool_choice")]
    tool_choice: String,
}

fn default_max_tokens() -> u32 {
    512
}

fn default_tool_choice() -> String {
    "auto".to_string()
}

// ============================================================================
// Message Preprocessing (OpenAI-style)
// ============================================================================

/// Preprocess messages to handle models that don't support system messages.
/// This mimics how the OpenAI API / mistralrs-server handles messages internally.
/// System messages are prepended to the first user message.
fn preprocess_messages(messages: Vec<Message>) -> Vec<Message> {
    let mut result = Vec::new();
    let mut system_content: Option<String> = None;

    for msg in messages {
        match msg.role.as_str() {
            "system" => {
                // Collect system messages to prepend to first user message
                system_content = Some(match system_content {
                    Some(existing) => format!("{}\n\n{}", existing, msg.content),
                    None => msg.content,
                });
            }
            "user" => {
                // Prepend any collected system content to first user message
                let content = if let Some(sys) = system_content.take() {
                    format!("{}\n\n{}", sys, msg.content)
                } else {
                    msg.content.clone()
                };
                result.push(Message {
                    role: "user".to_string(),
                    content,
                    tool_calls: None,
                    tool_call_id: None,
                });
            }
            "tool" => {
                // Tool results - keep as-is
                result.push(msg);
            }
            _ => {
                result.push(msg);
            }
        }
    }

    // If there's remaining system content with no user message, add as user
    if let Some(sys) = system_content {
        result.insert(0, Message {
            role: "user".to_string(),
            content: sys,
            tool_calls: None,
            tool_call_id: None,
        });
    }

    result
}

/// Convert our ToolDefinition to mistral.rs Tool format
fn convert_tools(tools: &[ToolDefinition]) -> Vec<Tool> {
    tools.iter().map(|t| {
        // Convert parameters from serde_json::Value to HashMap<String, serde_json::Value>
        let parameters: Option<HashMap<String, serde_json::Value>> =
            serde_json::from_value(t.parameters.clone()).ok();

        Tool {
            tp: ToolType::Function,
            function: Function {
                name: t.name.clone(),
                description: Some(t.description.clone()),
                parameters,
            },
        }
    }).collect()
}

/// Convert tool_choice string to ToolChoice enum
fn parse_tool_choice(choice: &str) -> ToolChoice {
    match choice {
        "none" => ToolChoice::None,
        // "required" is treated as Auto since ToolChoice doesn't have Required
        _ => ToolChoice::Auto, // default to "auto"
    }
}

// ============================================================================
// Prompt Injection for Non-Native Tool Support
// ============================================================================

/// Models known to have native function calling support in their chat templates
const NATIVE_TOOL_MODELS: &[&str] = &[
    "qwen2.5",
    "qwen2",
    "qwen",
    "hermes",
    "mistral",
    "mixtral",
    "command-r",
    "functionary",
    "gorilla",
    "nexusraven",
    "firefunction",
];

/// Check if a model has native tool calling support based on its name
fn has_native_tool_support(model_id: &str) -> bool {
    let name_lower = model_id.to_lowercase();
    NATIVE_TOOL_MODELS.iter().any(|m| name_lower.contains(m))
}

/// Format tools as a prompt for models without native tool support
fn format_tools_for_prompt(tools: &[ToolDefinition]) -> String {
    let mut prompt = String::from(
        "\n\n=== IMPORTANT: AVAILABLE TOOLS ===\n\
        You MUST use a tool when the user asks for data or information you don't have.\n\n"
    );

    for tool in tools {
        prompt.push_str(&format!("TOOL: {}\n", tool.name));
        prompt.push_str(&format!("Description: {}\n", tool.description));
        prompt.push_str(&format!("Parameters: {}\n\n",
            serde_json::to_string_pretty(&tool.parameters).unwrap_or_else(|_| "{}".to_string())
        ));
    }

    prompt.push_str(
        "=== HOW TO USE TOOLS ===\n\
        When you need to use a tool, respond with ONLY this JSON (nothing else before or after):\n\
        ```json\n\
        {\"tool_call\": {\"name\": \"tool_name\", \"arguments\": {\"arg1\": \"value1\"}}}\n\
        ```\n\n\
        DO NOT explain. DO NOT add text around it. ONLY output the JSON block if using a tool.\n\
        If you don't need a tool, respond normally."
    );

    prompt
}

/// Inject tool definitions into the messages for non-native tool support
/// Tools are APPENDED to the system message (after transcript) so they're closer to user message
fn inject_tools_into_messages(messages: &mut Vec<Message>, tools: &[ToolDefinition]) {
    if tools.is_empty() {
        return;
    }

    let tool_prompt = format_tools_for_prompt(tools);

    // Find the system message and APPEND tools to it (after transcript content)
    // This puts tool instructions closer to the user message where models pay more attention
    if let Some(system_msg) = messages.iter_mut().find(|m| m.role == "system") {
        system_msg.content = format!("{}{}", system_msg.content, tool_prompt);
    } else {
        // Insert a new system message at the beginning
        messages.insert(0, Message {
            role: "system".to_string(),
            content: tool_prompt,
            tool_calls: None,
            tool_call_id: None,
        });
    }
}

/// Wrapper for parsing tool call JSON from model response
#[derive(Debug, Deserialize)]
struct ToolCallWrapper {
    tool_call: ToolCallInner,
}

#[derive(Debug, Deserialize)]
struct ToolCallInner {
    name: String,
    arguments: serde_json::Value,
}

/// Try to find and extract a complete JSON object from a string
fn find_complete_json(s: &str) -> Option<String> {
    // Look for JSON in code blocks first
    if let Some(start) = s.find("```json") {
        let after_marker = &s[start + 7..];
        if let Some(end) = after_marker.find("```") {
            return Some(after_marker[..end].trim().to_string());
        }
    }

    // Also try without code blocks - look for the tool_call JSON
    if let Some(start) = s.find("{\"tool_call\"") {
        let mut depth = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let chars: Vec<char> = s[start..].chars().collect();

        for (i, &c) in chars.iter().enumerate() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match c {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(chars[..=i].iter().collect());
                    }
                }
                _ => {}
            }
        }
    }

    None
}

/// Parse tool calls from model response text (for non-native tool support)
fn parse_tool_calls_from_response(content: &str) -> Vec<ToolCall> {
    let mut tool_calls = Vec::new();

    if let Some(json_str) = find_complete_json(content) {
        if let Ok(wrapper) = serde_json::from_str::<ToolCallWrapper>(&json_str) {
            tool_calls.push(ToolCall {
                id: format!("call_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..24].to_string()),
                function: FunctionCall {
                    name: wrapper.tool_call.name,
                    arguments: serde_json::to_string(&wrapper.tool_call.arguments).unwrap_or_else(|_| "{}".to_string()),
                },
            });
        }
    }

    tool_calls
}

// ============================================================================
// LLM State
// ============================================================================

struct LlmState {
    model: Option<Model>,
    model_id: Option<String>,
}

impl LlmState {
    fn new() -> Self {
        Self {
            model: None,
            model_id: None,
        }
    }
}

type SharedState = Arc<RwLock<LlmState>>;

// ============================================================================
// Handler Functions
// ============================================================================

async fn handle_initialize(state: SharedState, params: InitializeParams) -> Result<serde_json::Value> {
    log::info!("Initializing model: {}", params.model_path);

    // WORKAROUND: Redirect stdout to stderr BEFORE any mistral.rs calls.
    // mistral.rs has println! statements that can occur during both GgufModelBuilder::new()
    // and build(). These would corrupt our JSON-RPC protocol on stdout.
    let _redirect = stdout_redirect::StdoutRedirect::to_stderr();

    // Unload any existing model first to free GPU memory
    {
        let mut state_guard = state.write().await;
        if state_guard.model.is_some() {
            log::info!("Unloading previous model: {:?}", state_guard.model_id);
            state_guard.model = None;
            state_guard.model_id = None;
        }
    }

    let path = PathBuf::from(&params.model_path);
    if !path.exists() {
        log::error!("Model file not found: {}", params.model_path);
        return Err(anyhow!("Model file not found: {}", params.model_path));
    }

    // Check file size to ensure it's a valid model
    let file_size = std::fs::metadata(&path)
        .map(|m| m.len())
        .unwrap_or(0);
    log::info!("Model file size: {} bytes ({:.2} GB)", file_size, file_size as f64 / 1_000_000_000.0);

    if file_size < 10_000_000 {
        log::error!("Model file too small ({} bytes), likely not a valid GGUF", file_size);
        return Err(anyhow!("Model file too small ({} bytes), likely not a valid GGUF model", file_size));
    }

    // Split path into directory and filename
    let model_dir = path.parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    let model_filename = path.file_name()
        .map(|f| f.to_string_lossy().to_string())
        .ok_or_else(|| anyhow!("Invalid model path - no filename"))?;

    // Extract model ID from filename for display purposes
    let model_id = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    log::info!("Loading GGUF from dir: {}, file: {}", model_dir, model_filename);

    // Build the model using mistral.rs
    // For LOCAL files:
    //   - First param: local directory path
    //   - Second param: just the filename (not full path!)
    // Tokenizer is extracted from GGUF metadata (no HuggingFace fetch needed)
    log::info!("Creating GgufModelBuilder...");

    // Optimized configuration based on Ollama/LM Studio best practices:
    // - Fixed 8K context (Ollama's recommended minimum for agents)
    // - FP8 KV cache quantization (halves memory usage)
    // - Prefix caching for system prompt reuse
    let device_map_params = AutoDeviceMapParams::Text {
        max_seq_len: 8192,
        max_batch_size: 1,
    };

    let builder_result = GgufModelBuilder::new(
        &model_dir,                             // Local directory containing the GGUF
        vec![model_filename.clone()]            // Just the filename, not full path!
    )
    .with_device_mapping(DeviceMapSetting::Auto(device_map_params))
    .with_prefix_cache_n(Some(16))
    .with_paged_attn(|| {
        PagedAttentionMetaBuilder::default()
            .with_block_size(32)
            .with_gpu_memory(MemoryGpuConfig::ContextSize(8192))
            .with_paged_cache_type(PagedCacheType::F8E4M3)
            .build()
    });

    let mut builder = match builder_result {
        Ok(b) => {
            log::info!("GgufModelBuilder created successfully");
            b
        }
        Err(e) => {
            log::error!("Failed to create GgufModelBuilder: {:?}", e);
            return Err(anyhow!("Failed to create model builder: {:?}", e));
        }
    };

    // Set chat template if provided
    if let Some(ref template) = params.chat_template {
        log::info!("Using chat template: {}", template);
        builder = builder.with_chat_template(template);
    }

    log::info!("Building model (this may take a moment)...");

    let model = match builder.build().await {
        Ok(m) => {
            log::info!("Model built successfully");
            m
        }
        Err(e) => {
            log::error!("Failed to build model: {:?}", e);
            return Err(anyhow!("Failed to load model: {:?}", e));
        }
    };

    // Update state
    {
        let mut state_guard = state.write().await;
        state_guard.model = Some(model);
        state_guard.model_id = Some(model_id.clone());
    }

    log::info!("Model loaded successfully: {}", model_id);

    Ok(serde_json::json!({
        "success": true,
        "model_id": model_id,
    }))
}

async fn handle_list_models(state: SharedState, params: ListModelsParams) -> Result<serde_json::Value> {
    let models_dir = PathBuf::from(&params.models_dir);
    let mut models = Vec::new();

    if models_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&models_dir) {
            let loaded_id = {
                let state_guard = state.read().await;
                state_guard.model_id.clone()
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "gguf").unwrap_or(false) {
                    if let Some(stem) = path.file_stem() {
                        let id = stem.to_string_lossy().to_string();
                        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                        let is_loaded = loaded_id.as_ref() == Some(&id);

                        models.push(ModelInfo {
                            id: id.clone(),
                            name: id,
                            size_bytes: size,
                            is_loaded,
                        });
                    }
                }
            }
        }
    }

    Ok(serde_json::to_value(models)?)
}

async fn handle_complete(
    state: SharedState,
    params: CompleteParams,
    request_id: u64,
) -> Result<serde_json::Value> {
    let state_guard = state.read().await;
    let model = state_guard.model.as_ref()
        .ok_or_else(|| anyhow!("No model loaded"))?;
    let model_id = state_guard.model_id.clone().unwrap_or_else(|| "unknown".to_string());

    // Check if model has native tool support
    let has_tools = params.tools.as_ref().map(|t| !t.is_empty()).unwrap_or(false);
    let use_native_tools = has_tools && has_native_tool_support(&model_id);
    let use_prompt_injection = has_tools && !use_native_tools;

    if has_tools {
        if use_native_tools {
            log::info!("Model {} has native tool support, using mistral.rs tools", model_id);
        } else {
            log::info!("Model {} lacks native tool support, using prompt injection for {} tools", model_id, params.tools.as_ref().unwrap().len());
        }
    }

    // For non-native tool support, inject tools into messages
    let mut messages_to_process = params.messages.clone();
    if use_prompt_injection {
        inject_tools_into_messages(&mut messages_to_process, params.tools.as_ref().unwrap());
        log::debug!("After tool injection - {} messages:", messages_to_process.len());
        for (i, msg) in messages_to_process.iter().enumerate() {
            let preview = if msg.content.len() > 200 {
                format!("{}...", &msg.content[..200])
            } else {
                msg.content.clone()
            };
            log::debug!("  [{}] {}: {}", i, msg.role, preview);
        }
    }

    // Preprocess messages: merge system messages into first user message
    // This handles models (like Mistral) that don't support system messages in their chat template
    let processed_messages = preprocess_messages(messages_to_process);

    if use_prompt_injection {
        log::debug!("After preprocessing - {} messages:", processed_messages.len());
        for (i, msg) in processed_messages.iter().enumerate() {
            let preview = if msg.content.len() > 300 {
                format!("{}... [truncated, total {} chars]", &msg.content[..300], msg.content.len())
            } else {
                msg.content.clone()
            };
            log::debug!("  [{}] {}: {}", i, msg.role, preview);
        }
    }

    // Build request using RequestBuilder (required for tools support)
    let mut request_builder = RequestBuilder::new();

    // Add messages
    for msg in &processed_messages {
        match msg.role.as_str() {
            "user" => {
                request_builder = request_builder.add_message(TextMessageRole::User, &msg.content);
            }
            "assistant" => {
                // Check if this assistant message has tool calls
                if let Some(ref tool_calls) = msg.tool_calls {
                    // Add message with tool calls
                    let mistral_tool_calls: Vec<mistralrs::ToolCallResponse> = tool_calls.iter().enumerate().map(|(idx, tc)| {
                        mistralrs::ToolCallResponse {
                            index: idx,
                            id: tc.id.clone(),
                            tp: ToolCallType::Function,
                            function: mistralrs::CalledFunction {
                                name: tc.function.name.clone(),
                                arguments: tc.function.arguments.clone(),
                            },
                        }
                    }).collect();
                    request_builder = request_builder.add_message_with_tool_call(
                        TextMessageRole::Assistant,
                        &msg.content,
                        mistral_tool_calls,
                    );
                } else {
                    request_builder = request_builder.add_message(TextMessageRole::Assistant, &msg.content);
                }
            }
            "tool" => {
                // Tool result message - use add_tool_message(content, tool_call_id)
                if let Some(ref tool_call_id) = msg.tool_call_id {
                    request_builder = request_builder.add_tool_message(&msg.content, tool_call_id);
                }
            }
            _ => {
                // Fallback to user role
                request_builder = request_builder.add_message(TextMessageRole::User, &msg.content);
            }
        }
    }

    // Add native tools only if the model supports them
    if use_native_tools {
        let mistral_tools = convert_tools(params.tools.as_ref().unwrap());
        let tool_choice = parse_tool_choice(&params.tool_choice);
        request_builder = request_builder.set_tools(mistral_tools).set_tool_choice(tool_choice);
        log::info!("Added {} native tools to request with choice {:?}", params.tools.as_ref().unwrap().len(), params.tool_choice);
    }

    let stdout = io::stdout();

    // Note: max_tokens is set via sampling params on the messages
    // For now, we use mistral.rs defaults and let the model decide
    // TODO: Add max_tokens support via RequestBuilder sampling params

    if params.stream {
        // Streaming response
        let mut stream = model.stream_chat_request(request_builder).await
            .map_err(|e| anyhow!("Failed to start streaming: {:?}", e))?;

        let mut full_content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        while let Some(response) = stream.next().await {
            match response {
                Response::Chunk(chunk) => {
                    for choice in &chunk.choices {
                        if let Some(ref content) = choice.delta.content {
                            full_content.push_str(content);

                            // Send streaming token
                            let response = JsonRpcResponse::success(
                                request_id,
                                serde_json::json!({ "token": content }),
                            );
                            let mut handle = stdout.lock();
                            writeln!(handle, "{}", serde_json::to_string(&response)?)?;
                            handle.flush()?;
                        }

                        // Check for tool calls in delta
                        if let Some(ref delta_tool_calls) = choice.delta.tool_calls {
                            for tc in delta_tool_calls {
                                tool_calls.push(ToolCall {
                                    id: tc.id.clone(),
                                    function: FunctionCall {
                                        name: tc.function.name.clone(),
                                        arguments: tc.function.arguments.clone(),
                                    },
                                });
                            }
                        }
                    }
                }
                Response::Done(done) => {
                    // Check for tool calls in final response
                    if let Some(ref choices) = done.choices.first() {
                        if let Some(ref final_tool_calls) = choices.message.tool_calls {
                            tool_calls = final_tool_calls.iter().map(|tc| ToolCall {
                                id: tc.id.clone(),
                                function: FunctionCall {
                                    name: tc.function.name.clone(),
                                    arguments: tc.function.arguments.clone(),
                                },
                            }).collect();
                        }
                    }
                    break;
                }
                Response::InternalError(e) => {
                    return Err(anyhow!("Internal error during streaming: {:?}", e));
                }
                Response::ValidationError(e) => {
                    return Err(anyhow!("Validation error: {:?}", e));
                }
                Response::ModelError(msg, _) => {
                    return Err(anyhow!("Model error: {}", msg));
                }
                _ => {}
            }
        }

        // For prompt injection: parse tool calls from response text if no native tool calls found
        if tool_calls.is_empty() && use_prompt_injection {
            let parsed_calls = parse_tool_calls_from_response(&full_content);
            if !parsed_calls.is_empty() {
                log::info!("Parsed {} tool call(s) from response text", parsed_calls.len());
                tool_calls = parsed_calls;
            }
        }

        // Determine finish reason
        let (finish_reason, response_tool_calls) = if !tool_calls.is_empty() {
            ("tool_calls", Some(tool_calls))
        } else {
            ("stop", None)
        };

        Ok(serde_json::json!({
            "done": true,
            "content": full_content,
            "model": model_id,
            "finish_reason": finish_reason,
            "tool_calls": response_tool_calls
        }))
    } else {
        // Non-streaming response
        let response = model.send_chat_request(request_builder).await
            .map_err(|e| anyhow!("Failed to complete: {:?}", e))?;

        let first_choice = response.choices.first();

        let content = first_choice
            .and_then(|c| c.message.content.as_ref())
            .cloned()
            .unwrap_or_default();

        // Check for native tool calls first
        let mut tool_calls: Option<Vec<ToolCall>> = first_choice
            .and_then(|c| c.message.tool_calls.as_ref())
            .map(|tcs| tcs.iter().map(|tc| ToolCall {
                id: tc.id.clone(),
                function: FunctionCall {
                    name: tc.function.name.clone(),
                    arguments: tc.function.arguments.clone(),
                },
            }).collect());

        // For prompt injection: parse tool calls from response text if no native tool calls found
        if tool_calls.is_none() && use_prompt_injection {
            let parsed_calls = parse_tool_calls_from_response(&content);
            if !parsed_calls.is_empty() {
                log::info!("Parsed {} tool call(s) from response text (non-streaming)", parsed_calls.len());
                tool_calls = Some(parsed_calls);
            }
        }

        let finish_reason = if tool_calls.is_some() {
            "tool_calls"
        } else {
            "stop"
        };

        Ok(serde_json::json!({
            "done": true,
            "content": content,
            "model": model_id,
            "finish_reason": finish_reason,
            "tool_calls": tool_calls
        }))
    }
}

async fn handle_current_model(state: SharedState) -> Result<serde_json::Value> {
    let state_guard = state.read().await;
    Ok(serde_json::json!({
        "model_id": state_guard.model_id
    }))
}

async fn handle_is_ready(state: SharedState) -> Result<serde_json::Value> {
    let state_guard = state.read().await;
    Ok(serde_json::json!({
        "ready": state_guard.model.is_some()
    }))
}

async fn handle_shutdown(state: SharedState) -> Result<serde_json::Value> {
    log::info!("Shutting down...");
    let mut state_guard = state.write().await;
    state_guard.model = None;
    state_guard.model_id = None;

    Ok(serde_json::json!({
        "success": true
    }))
}

// ============================================================================
// Main Loop
// ============================================================================

async fn process_request(state: SharedState, request: JsonRpcRequest) -> JsonRpcResponse {
    let result = match request.method.as_str() {
        "initialize" => {
            match serde_json::from_value::<InitializeParams>(request.params) {
                Ok(params) => handle_initialize(state, params).await,
                Err(e) => Err(anyhow!("Invalid params: {}", e)),
            }
        }
        "list_models" => {
            match serde_json::from_value::<ListModelsParams>(request.params) {
                Ok(params) => handle_list_models(state, params).await,
                Err(e) => Err(anyhow!("Invalid params: {}", e)),
            }
        }
        "complete" => {
            match serde_json::from_value::<CompleteParams>(request.params) {
                Ok(params) => handle_complete(state, params, request.id).await,
                Err(e) => Err(anyhow!("Invalid params: {}", e)),
            }
        }
        "current_model" => handle_current_model(state).await,
        "is_ready" => handle_is_ready(state).await,
        "shutdown" => handle_shutdown(state).await,
        _ => Err(anyhow!("Unknown method: {}", request.method)),
    };

    match result {
        Ok(value) => JsonRpcResponse::success(request.id, value),
        Err(e) => JsonRpcResponse::error(request.id, -32000, e.to_string()),
    }
}

#[tokio::main]
async fn main() {
    // Set up panic hook to log panics to stderr before exiting
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("LLM Sidecar PANIC: {}", panic_info);
        if let Some(location) = panic_info.location() {
            eprintln!("  at {}:{}:{}", location.file(), location.line(), location.column());
        }
    }));

    // Initialize logging to stderr (stdout is for JSON-RPC)
    // Using warn level by default to minimize output that could pollute stdout
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("llm_sidecar=info".parse().unwrap())
                .add_directive("mistralrs=info".parse().unwrap())
                .add_directive("mistralrs_core=info".parse().unwrap())
                .add_directive("candle=warn".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    log::info!("LLM Sidecar starting (using mistral.rs)...");

    // Initialize state
    let state: SharedState = Arc::new(RwLock::new(LlmState::new()));

    let stdin = io::stdin();
    let stdout = io::stdout();

    // Read JSON-RPC requests line by line from stdin
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to read line: {}", e);
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        // Parse request
        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                log::error!("Failed to parse request: {}", e);
                let response = JsonRpcResponse::error(0, -32700, format!("Parse error: {}", e));
                let mut handle = stdout.lock();
                let _ = writeln!(handle, "{}", serde_json::to_string(&response).unwrap());
                let _ = handle.flush();
                continue;
            }
        };

        log::debug!("Received request: {} (id={})", request.method, request.id);

        // Process request
        let response = process_request(state.clone(), request).await;

        // Send response
        let mut handle = stdout.lock();
        if let Err(e) = writeln!(handle, "{}", serde_json::to_string(&response).unwrap()) {
            log::error!("Failed to write response: {}", e);
        }
        let _ = handle.flush();
    }

    log::info!("LLM Sidecar shutting down");
}
