//! Tool orchestration for LLMs without native function calling support
//!
//! This module implements simulated tool calling by:
//! 1. Embedding tool definitions in the system prompt
//! 2. Parsing JSON tool call blocks from model output
//! 3. Executing tools and feeding results back in a loop

use std::sync::Arc;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio_util::sync::CancellationToken;

use crate::database::models::Tool;
use crate::llm_engine::engine::LlmEngine;
use crate::llm_engine::provider::{CompletionRequest, Message, ToolDefinition};
use crate::mcp::McpManager;
use crate::state::DbWrapper;
use crate::tools::executor::{execute_tool, ToolContext};

/// Result of parsing model output for tool calls
#[derive(Debug, Clone)]
pub enum ParsedToolCall {
    /// Model wants to call a tool
    ToolRequest {
        tool: String,
        arguments: serde_json::Value,
    },
    /// Model provided final answer (no tool call found)
    FinalAnswer(String),
    /// Malformed tool call attempt
    MalformedToolCall {
        raw: String,
        error: String,
    },
}

/// Configuration for simulated tool calling
#[derive(Debug, Clone)]
pub struct SimulatedToolConfig {
    pub max_iterations: usize,
}

impl Default for SimulatedToolConfig {
    fn default() -> Self {
        Self { max_iterations: 10 }
    }
}

// Regex patterns for parsing tool calls
static JSON_BLOCK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"```json\s*(\{[\s\S]*?\})\s*```").expect("Invalid regex")
});

static BARE_JSON_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\{"tool"\s*:\s*"[^"]+""#).expect("Invalid regex")
});

/// Build system prompt with tool definitions embedded
pub fn build_tool_system_prompt(base_prompt: &str, tools: &[ToolDefinition]) -> String {
    let mut prompt = base_prompt.to_string();

    prompt.push_str("\n\n## Available Tools\n\n");
    prompt.push_str("You have access to tools. To use a tool, you MUST respond with ONLY a JSON code block in this exact format:\n");
    prompt.push_str("```json\n{\"tool\": \"<tool_name>\", \"arguments\": {...}}\n```\n\n");
    prompt.push_str("IMPORTANT RULES:\n");
    prompt.push_str("- When you need information you don't have (like current time, searching data, etc.), you MUST use the appropriate tool\n");
    prompt.push_str("- Do NOT just mention or describe tools - actually USE them by outputting the JSON block\n");
    prompt.push_str("- When calling a tool, output ONLY the JSON block, nothing else\n");
    prompt.push_str("- After receiving a tool result, you can call more tools or give your final answer\n");
    prompt.push_str("- Only provide your final answer (without JSON block) when you have all needed information\n\n");
    prompt.push_str("Example - if asked \"what time is it?\", respond with:\n");
    prompt.push_str("```json\n{\"tool\": \"get_current_time\", \"arguments\": {}}\n```\n\n");
    prompt.push_str("### Tools:\n\n");

    for tool in tools {
        prompt.push_str(&format!("**{}**: {}\n", tool.name, tool.description));

        // Add parameters from JSON schema
        if let Some(props) = tool.parameters.get("properties") {
            let required: Vec<&str> = tool
                .parameters
                .get("required")
                .and_then(|r| r.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            prompt.push_str("Parameters:\n");
            if let Some(obj) = props.as_object() {
                for (name, schema) in obj {
                    let typ = schema.get("type").and_then(|t| t.as_str()).unwrap_or("any");
                    let desc = schema
                        .get("description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("");
                    let req = if required.contains(&name.as_str()) {
                        "required"
                    } else {
                        "optional"
                    };
                    prompt.push_str(&format!("- {} ({}, {}): {}\n", name, typ, req, desc));
                }
            }
        }
        prompt.push('\n');
    }

    prompt
}

/// Parse model output for tool calls
pub fn parse_tool_call(output: &str) -> ParsedToolCall {
    // Try fenced JSON block first (```json {...} ```)
    if let Some(caps) = JSON_BLOCK_RE.captures(output) {
        let json_str = caps.get(1).unwrap().as_str();
        return parse_json_tool_call(json_str);
    }

    // Try bare JSON object starting with {"tool":
    if let Some(m) = BARE_JSON_RE.find(output) {
        if let Some(json_str) = extract_json_object(&output[m.start()..]) {
            return parse_json_tool_call(&json_str);
        }
    }

    // No tool call found - this is the final answer
    ParsedToolCall::FinalAnswer(output.to_string())
}

/// Parse a JSON string as a tool call
fn parse_json_tool_call(json_str: &str) -> ParsedToolCall {
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(val) => {
            let tool = val.get("tool").and_then(|t| t.as_str());
            let args = val
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));

            match tool {
                Some(name) => ParsedToolCall::ToolRequest {
                    tool: name.to_string(),
                    arguments: args,
                },
                None => ParsedToolCall::MalformedToolCall {
                    raw: json_str.to_string(),
                    error: "Missing 'tool' field".to_string(),
                },
            }
        }
        Err(e) => ParsedToolCall::MalformedToolCall {
            raw: json_str.to_string(),
            error: format!("Invalid JSON: {}", e),
        },
    }
}

/// Extract a complete JSON object from a string (balance braces)
fn extract_json_object(s: &str) -> Option<String> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    let mut end = 0;

    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match c {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    end = i + 1;
                    break;
                }
            }
            _ => {}
        }
    }

    if depth == 0 && end > 0 {
        Some(s[..end].to_string())
    } else {
        None
    }
}

/// Format tool result for feeding back to model
pub fn format_tool_result(tool_name: &str, result: &str, is_error: bool) -> String {
    if is_error {
        format!("Tool error for `{}`:\n{}", tool_name, result)
    } else {
        format!("Tool result for `{}`:\n{}", tool_name, result)
    }
}

/// Run the simulated tool calling loop for non-native models
pub async fn run_simulated_tool_loop(
    engine: &LlmEngine,
    initial_messages: Vec<Message>,
    tools: &[Tool],
    _tool_definitions: &[ToolDefinition],
    mcp_manager: Arc<tokio::sync::RwLock<Option<McpManager>>>,
    database: Arc<tokio::sync::RwLock<Option<DbWrapper>>>,
    recording_id: &str,
    cancel_token: CancellationToken,
    config: SimulatedToolConfig,
) -> Result<String, String> {
    let mut messages = initial_messages;
    let mut iteration = 0;

    loop {
        // Check cancellation
        if cancel_token.is_cancelled() {
            return Err("Cancelled".to_string());
        }

        // Check iteration limit
        if iteration >= config.max_iterations {
            log::warn!("Simulated tool loop reached max iterations ({})", config.max_iterations);
            return Err("Max tool iterations reached".to_string());
        }
        iteration += 1;

        log::info!("Simulated tool loop iteration {}", iteration);

        // Make completion request (non-streaming, no native tools)
        let request = CompletionRequest {
            messages: messages.clone(),
            max_tokens: Some(2048),
            temperature: Some(0.7),
            stream: false,
            tools: None, // Don't pass tools to non-native model
            tool_choice: None,
            ..Default::default()
        };

        let response = engine.complete(request).await.map_err(|e| e.to_string())?;

        log::debug!("Model response: {}", &response.content[..response.content.len().min(200)]);

        // Parse for tool calls
        match parse_tool_call(&response.content) {
            ParsedToolCall::FinalAnswer(answer) => {
                log::info!("Simulated tool loop complete - got final answer");
                return Ok(answer);
            }

            ParsedToolCall::ToolRequest { tool, arguments } => {
                log::info!("Simulated tool call: {} with {:?}", tool, arguments);

                // Add assistant message with tool request
                messages.push(Message::assistant(response.content.clone()));

                // Find and execute tool
                let tool_result = execute_tool_by_name(
                    &tool,
                    arguments,
                    tools,
                    mcp_manager.clone(),
                    database.clone(),
                    recording_id,
                )
                .await;

                // Format result and add as user message
                let formatted_result =
                    format_tool_result(&tool, &tool_result.content, !tool_result.success);
                messages.push(Message::user(formatted_result));
            }

            ParsedToolCall::MalformedToolCall { raw, error } => {
                log::warn!("Malformed tool call: {} - {}", raw, error);

                // Add assistant message
                messages.push(Message::assistant(response.content.clone()));

                // Add error feedback
                let error_msg = format!(
                    "Your tool call was malformed: {}\n\
                    Please use the exact format:\n\
                    ```json\n{{\"tool\": \"<name>\", \"arguments\": {{...}}}}\n```",
                    error
                );
                messages.push(Message::user(error_msg));
            }
        }
    }
}

/// Result of tool execution
struct ToolExecutionResult {
    content: String,
    success: bool,
}

/// Execute a tool by name, routing to MCP or builtin as appropriate
async fn execute_tool_by_name(
    tool_name: &str,
    arguments: serde_json::Value,
    tools: &[Tool],
    mcp_manager: Arc<tokio::sync::RwLock<Option<McpManager>>>,
    database: Arc<tokio::sync::RwLock<Option<DbWrapper>>>,
    recording_id: &str,
) -> ToolExecutionResult {
    // Find tool info
    let tool_info = tools.iter().find(|t| t.name == tool_name);

    // Check if tool exists - if not, provide helpful error with available tools
    if tool_info.is_none() {
        let available_tools: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        return ToolExecutionResult {
            content: format!(
                "Tool '{}' does not exist. Available tools are: {}. If you have the information you need, provide your final answer without a JSON tool call block.",
                tool_name,
                available_tools.join(", ")
            ),
            success: false,
        };
    }

    match tool_info {
        Some(t) if t.tool_type == "mcp" => {
            // MCP tool
            log::info!("Routing simulated tool '{}' to MCP manager", tool_name);
            let mcp_guard = mcp_manager.read().await;
            match mcp_guard.as_ref() {
                Some(mcp) => match mcp.call_tool(&t.id, arguments).await {
                    Ok(result) => ToolExecutionResult {
                        content: result,
                        success: true,
                    },
                    Err(e) => ToolExecutionResult {
                        content: format!("MCP tool error: {}", e),
                        success: false,
                    },
                },
                None => ToolExecutionResult {
                    content: "MCP manager not initialized".to_string(),
                    success: false,
                },
            }
        }
        Some(_) => {
            // Builtin tool (unknown tools already handled above)
            let db_lock = database.read().await;
            let db_ref = match db_lock.as_ref() {
                Some(db) => db.inner(),
                None => {
                    return ToolExecutionResult {
                        content: "Database not initialized".to_string(),
                        success: false,
                    }
                }
            };

            let context = ToolContext {
                recording_id: recording_id.to_string(),
                db: db_ref,
            };

            match execute_tool(tool_name, arguments, &context).await {
                Ok(result) => ToolExecutionResult {
                    content: result,
                    success: true,
                },
                Err(e) => ToolExecutionResult {
                    content: format!("Error executing tool '{}': {}", tool_name, e),
                    success: false,
                },
            }
        }
        None => {
            // Already handled above, but needed for exhaustive match
            unreachable!("None case already handled by early return")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_call_fenced() {
        let output = r#"I'll search for that.
```json
{"tool": "search_transcript", "arguments": {"query": "budget"}}
```
"#;
        match parse_tool_call(output) {
            ParsedToolCall::ToolRequest { tool, arguments } => {
                assert_eq!(tool, "search_transcript");
                assert_eq!(arguments["query"], "budget");
            }
            _ => panic!("Expected ToolRequest"),
        }
    }

    #[test]
    fn test_parse_tool_call_bare() {
        let output = r#"Let me check that. {"tool": "get_current_time", "arguments": {}}"#;
        match parse_tool_call(output) {
            ParsedToolCall::ToolRequest { tool, .. } => {
                assert_eq!(tool, "get_current_time");
            }
            _ => panic!("Expected ToolRequest"),
        }
    }

    #[test]
    fn test_parse_tool_call_final_answer() {
        let output = "The meeting discussed budget allocation for Q3.";
        match parse_tool_call(output) {
            ParsedToolCall::FinalAnswer(answer) => {
                assert_eq!(answer, output);
            }
            _ => panic!("Expected FinalAnswer"),
        }
    }

    #[test]
    fn test_parse_tool_call_malformed() {
        let output = r#"```json
{"arguments": {"query": "test"}}
```"#;
        match parse_tool_call(output) {
            ParsedToolCall::MalformedToolCall { error, .. } => {
                assert!(error.contains("Missing 'tool' field"));
            }
            _ => panic!("Expected MalformedToolCall"),
        }
    }

    #[test]
    fn test_extract_json_object() {
        let s = r#"{"tool": "test", "arguments": {"nested": {"value": 1}}}"#;
        let result = extract_json_object(s);
        assert_eq!(result, Some(s.to_string()));
    }

    #[test]
    fn test_extract_json_object_with_string() {
        let s = r#"{"tool": "test", "arguments": {"text": "hello {world}"}}"#;
        let result = extract_json_object(s);
        assert_eq!(result, Some(s.to_string()));
    }

    #[test]
    fn test_build_tool_system_prompt() {
        let base = "You are a helpful assistant.";
        let tools = vec![ToolDefinition {
            name: "search".to_string(),
            description: "Search for text".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    }
                },
                "required": ["query"]
            }),
        }];

        let result = build_tool_system_prompt(base, &tools);
        assert!(result.contains("You are a helpful assistant"));
        assert!(result.contains("## Available Tools"));
        assert!(result.contains("**search**"));
        assert!(result.contains("query (string, required)"));
    }
}
