//! Tool execution engine for built-in and custom tools
//!
//! Handles executing tool calls made by the LLM.

use anyhow::{anyhow, Result};
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::database::DatabaseManager;

/// Context for tool execution (provides access to recording data)
pub struct ToolContext<'a> {
    pub recording_id: String,
    pub db: &'a DatabaseManager,
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub result: String,
    pub is_error: bool,
}

/// Execute a tool call and return the result
pub async fn execute_tool(
    tool_name: &str,
    arguments: Value,
    context: &ToolContext<'_>,
) -> Result<String> {
    match tool_name {
        "get_current_time" => execute_get_current_time(arguments),
        "search_transcript" => execute_search_transcript(arguments, context).await,
        "list_speakers" => execute_list_speakers(context).await,
        "get_segment" => execute_get_segment(arguments, context).await,
        _ => Err(anyhow!("Unknown tool: {}", tool_name)),
    }
}

// ============================================================================
// Built-in Tool Implementations
// ============================================================================

/// Get current date and time
fn execute_get_current_time(arguments: Value) -> Result<String> {
    let format = arguments
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("%Y-%m-%d %H:%M:%S");

    let now = Local::now();
    Ok(now.format(format).to_string())
}

/// Search within the meeting transcript
async fn execute_search_transcript(
    arguments: Value,
    context: &ToolContext<'_>,
) -> Result<String> {
    let query = arguments
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing required parameter: query"))?;

    let limit = arguments
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(5) as usize;

    // Get segments for this recording
    let segments = context.db.get_transcript_segments(&context.recording_id)?;

    // Simple text search (case-insensitive)
    let query_lower = query.to_lowercase();
    let matches: Vec<_> = segments
        .iter()
        .filter(|s| s.text.to_lowercase().contains(&query_lower))
        .take(limit)
        .map(|s| {
            serde_json::json!({
                "timestamp": format_time(s.audio_start_time),
                "speaker": s.speaker_label.as_ref().unwrap_or(&"Unknown".to_string()),
                "text": s.text
            })
        })
        .collect();

    if matches.is_empty() {
        Ok(format!("No matches found for query: \"{}\"", query))
    } else {
        Ok(serde_json::to_string_pretty(&matches)?)
    }
}

/// List all speakers in the meeting
async fn execute_list_speakers(context: &ToolContext<'_>) -> Result<String> {
    let segments = context.db.get_transcript_segments(&context.recording_id)?;

    // Collect unique speakers
    let mut speakers: Vec<String> = segments
        .iter()
        .filter_map(|s| s.speaker_label.clone())
        .collect();
    speakers.sort();
    speakers.dedup();

    if speakers.is_empty() {
        Ok("No speakers identified in this recording.".to_string())
    } else {
        Ok(format!(
            "Speakers in this meeting:\n{}",
            speakers
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}. {}", i + 1, s))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}

/// Get transcript segment by time range
async fn execute_get_segment(
    arguments: Value,
    context: &ToolContext<'_>,
) -> Result<String> {
    let start_time = arguments
        .get("start_time")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing required parameter: start_time"))?;

    let end_time = arguments
        .get("end_time")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Missing required parameter: end_time"))?;

    // Parse times (expected format: "HH:MM:SS" or "MM:SS" or seconds as number)
    let start_secs = parse_time(start_time)?;
    let end_secs = parse_time(end_time)?;

    let segments = context.db.get_transcript_segments(&context.recording_id)?;

    // Filter segments within time range
    let matches: Vec<_> = segments
        .iter()
        .filter(|s| s.audio_start_time >= start_secs && s.audio_start_time <= end_secs)
        .map(|s| {
            serde_json::json!({
                "timestamp": format_time(s.audio_start_time),
                "speaker": s.speaker_label.as_ref().unwrap_or(&"Unknown".to_string()),
                "text": s.text
            })
        })
        .collect();

    if matches.is_empty() {
        Ok(format!(
            "No transcript found between {} and {}",
            start_time, end_time
        ))
    } else {
        Ok(serde_json::to_string_pretty(&matches)?)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Parse time string to seconds
/// Supports formats: "HH:MM:SS", "MM:SS", or just seconds as a number
fn parse_time(time_str: &str) -> Result<f64> {
    // Try parsing as a number first
    if let Ok(secs) = time_str.parse::<f64>() {
        return Ok(secs);
    }

    // Try parsing as HH:MM:SS or MM:SS
    let parts: Vec<&str> = time_str.split(':').collect();
    match parts.len() {
        2 => {
            // MM:SS
            let mins: f64 = parts[0].parse()?;
            let secs: f64 = parts[1].parse()?;
            Ok(mins * 60.0 + secs)
        }
        3 => {
            // HH:MM:SS
            let hours: f64 = parts[0].parse()?;
            let mins: f64 = parts[1].parse()?;
            let secs: f64 = parts[2].parse()?;
            Ok(hours * 3600.0 + mins * 60.0 + secs)
        }
        _ => Err(anyhow!(
            "Invalid time format: {}. Expected HH:MM:SS, MM:SS, or seconds",
            time_str
        )),
    }
}

/// Format seconds as MM:SS or HH:MM:SS
fn format_time(seconds: f64) -> String {
    let total_secs = seconds as u64;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, mins, secs)
    } else {
        format!("{:02}:{:02}", mins, secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time() {
        assert_eq!(parse_time("60").unwrap(), 60.0);
        assert_eq!(parse_time("1:30").unwrap(), 90.0);
        assert_eq!(parse_time("01:30").unwrap(), 90.0);
        assert_eq!(parse_time("1:01:30").unwrap(), 3690.0);
    }

    #[test]
    fn test_format_time() {
        assert_eq!(format_time(90.0), "01:30");
        assert_eq!(format_time(3690.0), "01:01:30");
        assert_eq!(format_time(0.0), "00:00");
    }
}
