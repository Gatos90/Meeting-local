// MCP (Model Context Protocol) module for Meeting-Local
// Handles MCP server lifecycle, communication, and tool discovery

pub mod client;
pub mod manager;
pub mod commands;

pub use client::McpClient;
pub use manager::McpManager;
