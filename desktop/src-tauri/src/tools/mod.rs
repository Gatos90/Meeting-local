//! Tools module for Meeting-Local
//! Provides AI function calling tools for chat conversations

pub mod commands;
pub mod executor;

pub use executor::{execute_tool, ToolContext, ToolResult};
