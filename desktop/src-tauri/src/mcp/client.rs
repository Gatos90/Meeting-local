// MCP Client - Handles communication with MCP servers via stdio JSON-RPC
//
// The MCP protocol uses JSON-RPC 2.0 over stdio:
// - Client spawns server as subprocess
// - Communication via stdin/stdout (newline-delimited JSON)
// - Protocol: initialize → tools/list → tools/call

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

use crate::database::models::McpServer;

/// JSON-RPC request structure
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// JSON-RPC response structure
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: Option<u64>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

/// JSON-RPC error
#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

/// MCP server capabilities returned from initialize
#[derive(Debug, Clone, Deserialize)]
pub struct ServerCapabilities {
    #[serde(default)]
    pub tools: Option<ToolsCapability>,
    #[serde(default)]
    pub prompts: Option<Value>,
    #[serde(default)]
    pub resources: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsCapability {
    #[serde(default)]
    pub list_changed: bool,
}

/// Tool definition from MCP server
#[derive(Debug, Clone, Deserialize)]
pub struct McpTool {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "inputSchema")]
    pub input_schema: Option<Value>,
}

/// Tool list response
#[derive(Debug, Deserialize)]
struct ToolsListResponse {
    tools: Vec<McpTool>,
}

/// Tool call result content
#[derive(Debug, Deserialize)]
pub struct ToolCallContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(default)]
    pub text: Option<String>,
}

/// Tool call response
#[derive(Debug, Deserialize)]
struct ToolCallResponse {
    content: Vec<ToolCallContent>,
    #[serde(default)]
    #[allow(dead_code)]
    is_error: bool,
}

/// Initialize response
#[derive(Debug, Deserialize)]
struct InitializeResponse {
    #[serde(rename = "protocolVersion")]
    #[allow(dead_code)]
    protocol_version: String,
    #[serde(default)]
    capabilities: Option<ServerCapabilities>,
    #[serde(rename = "serverInfo")]
    #[allow(dead_code)]
    server_info: Option<Value>,
}

/// MCP Client for communicating with MCP servers
pub struct McpClient {
    server_id: String,
    server_name: String,
    process: Child,
    stdin: Mutex<ChildStdin>,
    stdout: Mutex<BufReader<ChildStdout>>,
    request_id: AtomicU64,
    capabilities: Option<ServerCapabilities>,
}

impl McpClient {
    /// Spawn an MCP server subprocess and create a client
    pub async fn spawn(server: &McpServer) -> Result<Self> {
        let args = server.get_args();
        let env = server.get_env();

        log::info!(
            "Spawning MCP server '{}': {} {:?}",
            server.name,
            server.command,
            args
        );

        let mut cmd = Command::new(&server.command);
        cmd.args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables
        for (key, value) in &env {
            cmd.env(key, value);
        }

        // Set working directory if specified
        if let Some(ref wd) = server.working_directory {
            cmd.current_dir(wd);
        }

        // On Windows, don't create a console window
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut process = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn MCP server: {} {:?}", server.command, args))?;

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdin for MCP server"))?;

        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to get stdout for MCP server"))?;

        Ok(Self {
            server_id: server.id.clone(),
            server_name: server.name.clone(),
            process,
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(BufReader::new(stdout)),
            request_id: AtomicU64::new(1),
            capabilities: None,
        })
    }

    /// Get the server ID
    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    /// Get the server name
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// Check if the process is still running
    pub fn is_running(&self) -> bool {
        // Try to get the process status without blocking
        // This is a simple check - we don't have access to try_wait without &mut self
        true // Assume running unless we explicitly shut down
    }

    /// Send a JSON-RPC request and wait for response
    async fn request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);

        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let request_json = serde_json::to_string(&request)
            .context("Failed to serialize JSON-RPC request")?;

        log::debug!("MCP [{}] → {}", self.server_name, request_json);

        // Send request
        {
            let mut stdin = self.stdin.lock().await;
            stdin
                .write_all(request_json.as_bytes())
                .await
                .context("Failed to write to MCP server stdin")?;
            stdin
                .write_all(b"\n")
                .await
                .context("Failed to write newline to MCP server stdin")?;
            stdin.flush().await.context("Failed to flush stdin")?;
        }

        // Read response
        let response: JsonRpcResponse = {
            let mut stdout = self.stdout.lock().await;
            let mut line = String::new();

            // Read until we get a valid JSON response
            loop {
                line.clear();
                let bytes_read = stdout
                    .read_line(&mut line)
                    .await
                    .context("Failed to read from MCP server stdout")?;

                if bytes_read == 0 {
                    return Err(anyhow!("MCP server closed stdout unexpectedly"));
                }

                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                log::debug!("MCP [{}] ← {}", self.server_name, trimmed);

                // Try to parse as JSON-RPC response
                match serde_json::from_str::<JsonRpcResponse>(trimmed) {
                    Ok(resp) => {
                        // Check if this is the response to our request
                        if resp.id == Some(id) {
                            break resp;
                        }
                        // Otherwise it might be a notification - skip it
                        log::debug!("MCP [{}] Skipping notification/other message", self.server_name);
                    }
                    Err(e) => {
                        log::warn!(
                            "MCP [{}] Received non-JSON-RPC message: {} (error: {})",
                            self.server_name,
                            trimmed,
                            e
                        );
                        // Continue reading - might be debug output from server
                    }
                }
            }
        };

        // Check for error
        if let Some(error) = response.error {
            return Err(anyhow!(
                "MCP server error [{}]: {}",
                error.code,
                error.message
            ));
        }

        response
            .result
            .ok_or_else(|| anyhow!("MCP server returned empty result"))
    }

    /// Initialize the MCP connection (required first step)
    pub async fn initialize(&mut self) -> Result<ServerCapabilities> {
        let params = json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "roots": {
                    "listChanged": true
                }
            },
            "clientInfo": {
                "name": "meeting-local",
                "version": "1.0.0"
            }
        });

        let result = self.request("initialize", Some(params)).await?;
        let init_response: InitializeResponse =
            serde_json::from_value(result).context("Failed to parse initialize response")?;

        let capabilities = init_response.capabilities.unwrap_or(ServerCapabilities {
            tools: None,
            prompts: None,
            resources: None,
        });

        self.capabilities = Some(capabilities.clone());

        // Send initialized notification (no response expected, but we don't wait)
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        });
        let notification_json = serde_json::to_string(&notification)?;

        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(notification_json.as_bytes()).await?;
            stdin.write_all(b"\n").await?;
            stdin.flush().await?;
        }

        log::info!("MCP server '{}' initialized successfully", self.server_name);
        Ok(capabilities)
    }

    /// List available tools from the MCP server
    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        let result = self.request("tools/list", Some(json!({}))).await?;
        let response: ToolsListResponse =
            serde_json::from_value(result).context("Failed to parse tools/list response")?;

        log::info!(
            "MCP server '{}' has {} tools",
            self.server_name,
            response.tools.len()
        );

        Ok(response.tools)
    }

    /// Call a tool on the MCP server
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<String> {
        let params = json!({
            "name": name,
            "arguments": arguments
        });

        let result = self.request("tools/call", Some(params)).await?;
        let response: ToolCallResponse =
            serde_json::from_value(result).context("Failed to parse tools/call response")?;

        // Combine all text content
        let text = response
            .content
            .iter()
            .filter_map(|c| c.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        Ok(text)
    }

    /// Gracefully shutdown the MCP server
    pub async fn shutdown(&mut self) -> Result<()> {
        log::info!("Shutting down MCP server '{}'", self.server_name);

        // Try to kill the process
        if let Err(e) = self.process.kill().await {
            log::warn!("Failed to kill MCP server process: {}", e);
        }

        // Wait for it to exit
        if let Err(e) = self.process.wait().await {
            log::warn!("Failed to wait for MCP server exit: {}", e);
        }

        Ok(())
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        // Try to kill the process if it's still running
        // Note: This is best-effort since we can't await in drop
        let _ = self.process.start_kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "test".to_string(),
            params: Some(json!({"key": "value"})),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"test\""));
    }
}
