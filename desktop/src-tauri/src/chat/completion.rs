//! Chat completion logic - runs LLM completion with tool execution loop

use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tauri::Emitter;

use crate::database::{ChatMessageStatus, ChatRole};
use crate::llm_engine::model_manager::has_native_tool_support_with_override;
use crate::llm_engine::provider::{CompletionRequest, Message, MessageRole, ToolDefinition};
use crate::tools::executor::{execute_tool, ToolContext};
use crate::chat::tool_orchestration::{
    build_tool_system_prompt, run_simulated_tool_loop, SimulatedToolConfig,
};

/// Run the actual chat completion in background
pub async fn run_chat_completion(
    app_handle: tauri::AppHandle,
    llm_engine: Arc<tokio::sync::RwLock<crate::llm_engine::engine::LlmEngine>>,
    database: Arc<tokio::sync::RwLock<Option<crate::state::DbWrapper>>>,
    mcp_manager: Arc<tokio::sync::RwLock<Option<crate::mcp::McpManager>>>,
    session_id: String,
    recording_id: String,
    message_id: String,
    cancel_token: CancellationToken,
    _tool_ids: Option<Vec<String>>, // Now unused - tools are loaded from session DB
) -> Result<(), String> {
    // Get database - hold reference within scope
    let db_guard = database.read().await;
    let db_wrapper = db_guard.as_ref().ok_or("Database not initialized")?;
    let db = db_wrapper.inner();

    // Update status to streaming
    db.update_chat_message_status(&message_id, ChatMessageStatus::Streaming, None)
        .map_err(|e| e.to_string())?;

    // Load tools from session database (user's current selection)
    let session_tools = db.get_session_tools(&session_id).map_err(|e| e.to_string())?;

    log::info!(
        "Session {} has {} tools selected: {:?}",
        session_id,
        session_tools.len(),
        session_tools.iter().map(|t| t.name.as_str()).collect::<Vec<_>>()
    );

    let tools = session_tools;

    // Convert tools to ToolDefinition format
    let tool_definitions: Option<Vec<ToolDefinition>> = if tools.is_empty() {
        None
    } else {
        Some(tools.iter().map(|t| {
            let schema: serde_json::Value = serde_json::from_str(&t.function_schema)
                .unwrap_or_else(|_| serde_json::json!({}));
            let parameters = schema.get("parameters")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));

            ToolDefinition {
                name: t.name.clone(),
                description: t.description.clone().unwrap_or_default(),
                parameters,
            }
        }).collect())
    };

    // Load transcript for context
    let segments = db
        .get_transcript_segments(&recording_id)
        .map_err(|e| e.to_string())?;

    // Build transcript text for context
    let transcript_text = if segments.is_empty() {
        "No transcript available for this recording.".to_string()
    } else {
        segments
            .iter()
            .map(|s| {
                let speaker = s.speaker_label.as_deref().unwrap_or("Unknown");
                format!("[{}] {}: {}", s.display_time, speaker, s.text)
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    // Load chat history for this session
    let chat_messages = db
        .get_chat_messages_by_session(&session_id)
        .map_err(|e| e.to_string())?;

    // Build messages for LLM (excluding the pending assistant message)
    let mut messages: Vec<Message> = Vec::new();

    // System message with transcript context
    let system_content = format!(
        "You are a helpful assistant analyzing a meeting transcript. \
        Answer questions about the meeting based on the transcript below.\n\n\
        TRANSCRIPT:\n{}\n\n\
        Provide clear, concise answers based on the transcript content.",
        transcript_text
    );
    messages.push(Message {
        role: MessageRole::System,
        content: system_content,
        tool_calls: None,
        tool_call_id: None,
    });

    // Add chat history (excluding system messages and pending)
    for msg in &chat_messages {
        if msg.status == ChatMessageStatus::Pending || msg.status == ChatMessageStatus::Streaming {
            continue;
        }
        let role = match msg.role {
            ChatRole::User => MessageRole::User,
            ChatRole::Assistant => MessageRole::Assistant,
            ChatRole::System => continue,
        };
        messages.push(Message {
            role,
            content: msg.content.clone(),
            tool_calls: None,
            tool_call_id: None,
        });
    }

    // Drop the database lock before the long-running operation
    drop(db_guard);

    // Check for cancellation before starting
    if cancel_token.is_cancelled() {
        let db_lock = database.read().await;
        if let Some(db) = db_lock.as_ref() {
            let _ = db.update_chat_message_status(&message_id, ChatMessageStatus::Cancelled, None);
        }
        return Err("Cancelled".to_string());
    }

    // Get LLM engine and run completion
    let engine = llm_engine.read().await;

    // Check if LLM is ready
    if !engine.is_ready().await {
        let db_lock = database.read().await;
        if let Some(db) = db_lock.as_ref() {
            let _ = db.update_chat_message_status(
                &message_id,
                ChatMessageStatus::Error,
                Some("LLM engine not ready. Please configure an LLM provider in settings."),
            );
        }
        return Err("LLM engine not ready".to_string());
    }

    // Determine if model has native tool support
    let model_id = engine.current_model().await.unwrap_or_default();

    // Check for user-defined tool support override from database
    let user_tool_support_override = {
        let db_lock = database.read().await;
        db_lock.as_ref()
            .and_then(|db| db.get_model_tool_support(&model_id).ok())
            .flatten()
    };

    let use_native_tools = has_native_tool_support_with_override(&model_id, user_tool_support_override);

    log::info!(
        "Model '{}' native tool support: {}, tools enabled: {}",
        model_id,
        use_native_tools,
        tool_definitions.is_some()
    );

    // Handle non-native models with tools using simulated tool calling
    if tool_definitions.is_some() && !use_native_tools {
        log::info!("Using simulated tool calling for non-native model: {}", model_id);

        // Build enhanced system prompt with tool definitions
        let tool_system_prompt = build_tool_system_prompt(
            &messages[0].content,
            tool_definitions.as_ref().unwrap(),
        );
        let mut sim_messages = messages.clone();
        sim_messages[0].content = tool_system_prompt;

        // Run simulated tool loop (non-streaming)
        let result = run_simulated_tool_loop(
            &engine,
            sim_messages,
            &tools,
            tool_definitions.as_ref().unwrap(),
            mcp_manager.clone(),
            database.clone(),
            &recording_id,
            cancel_token.clone(),
            SimulatedToolConfig::default(),
        )
        .await;

        // Handle simulated loop result
        return match result {
            Ok(final_answer) => {
                let db_lock = database.read().await;
                if let Some(db) = db_lock.as_ref() {
                    db.update_chat_message_content(&message_id, &final_answer)
                        .map_err(|e| e.to_string())?;
                    db.update_chat_message_status(&message_id, ChatMessageStatus::Complete, None)
                        .map_err(|e| e.to_string())?;
                }

                // Emit final event
                let _ = app_handle.emit(
                    &format!("chat-stream-{}", session_id),
                    serde_json::json!({
                        "message_id": message_id,
                        "token": "",
                        "content": final_answer
                    }),
                );

                Ok(())
            }
            Err(e) => {
                let db_lock = database.read().await;
                if let Some(db) = db_lock.as_ref() {
                    let _ = db.update_chat_message_status(
                        &message_id,
                        ChatMessageStatus::Error,
                        Some(&e),
                    );
                }
                Err(e)
            }
        };
    }

    // Native tool support or no tools - use existing streaming flow
    let request = CompletionRequest {
        messages,
        max_tokens: Some(2048),
        temperature: Some(0.7),
        stream: true,
        tools: tool_definitions.clone(),
        tool_choice: if tool_definitions.is_some() { Some("auto".to_string()) } else { None },
        ..Default::default()
    };

    // Setup streaming callback
    let message_id_for_callback = message_id.clone();
    let database_for_callback = database.clone();
    let app_handle_for_callback = app_handle.clone();
    let session_id_for_callback = session_id.clone();
    let accumulated_content = Arc::new(tokio::sync::Mutex::new(String::new()));
    let accumulated_for_callback = accumulated_content.clone();
    let cancel_token_for_callback = cancel_token.clone();

    let callback = Box::new(move |token: String| {
        if cancel_token_for_callback.is_cancelled() {
            return;
        }

        let accumulated = accumulated_for_callback.clone();
        let message_id = message_id_for_callback.clone();
        let database = database_for_callback.clone();
        let app_handle = app_handle_for_callback.clone();
        let session_id = session_id_for_callback.clone();

        tokio::spawn(async move {
            let mut content = accumulated.lock().await;
            content.push_str(&token);
            let current_content = content.clone();

            let db_lock = database.read().await;
            if let Some(db) = db_lock.as_ref() {
                let _ = db.update_chat_message_content(&message_id, &current_content);
            }

            let _ = app_handle.emit(
                &format!("chat-stream-{}", session_id),
                serde_json::json!({
                    "message_id": message_id,
                    "token": token,
                    "content": current_content
                }),
            );
        });
    });

    // Run completion with streaming
    let result = engine.complete_streaming(request.clone(), callback, Some(cancel_token.clone())).await;

    // Handle result, including tool call loop
    match result {
        Ok(mut response) => {
            let mut current_messages = request.messages.clone();

            // Tool call loop
            const MAX_TOOL_ITERATIONS: usize = 10;
            let mut iteration = 0;

            while let Some(ref tool_calls) = response.tool_calls {
                if tool_calls.is_empty() || iteration >= MAX_TOOL_ITERATIONS {
                    break;
                }
                iteration += 1;

                log::info!("Processing {} tool call(s), iteration {}", tool_calls.len(), iteration);

                current_messages.push(Message::assistant_with_tool_calls(
                    response.content.clone(),
                    tool_calls.clone(),
                ));

                // Execute each tool call
                for tool_call in tool_calls {
                    let tool_name = &tool_call.function.name;
                    let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                        .unwrap_or_default();

                    let tool_info = tools.iter().find(|t| &t.name == tool_name);

                    let tool_result = match tool_info {
                        Some(t) if t.tool_type == "mcp" => {
                            log::info!("Routing MCP tool '{}' to MCP manager", tool_name);
                            let mcp_guard = mcp_manager.read().await;
                            match mcp_guard.as_ref() {
                                Some(mcp) => {
                                    match mcp.call_tool(&t.id, args).await {
                                        Ok(result) => result,
                                        Err(e) => format!("MCP tool error: {}", e),
                                    }
                                }
                                None => "MCP manager not initialized".to_string(),
                            }
                        }
                        _ => {
                            let db_lock = database.read().await;
                            let db_ref = db_lock.as_ref().ok_or("Database not initialized")?;
                            let context = ToolContext {
                                recording_id: recording_id.clone(),
                                db: db_ref.inner(),
                            };
                            match execute_tool(tool_name, args, &context).await {
                                Ok(result) => result,
                                Err(e) => format!("Error executing tool: {}", e),
                            }
                        }
                    };

                    log::info!("Tool {} returned: {}", tool_name,
                        if tool_result.len() > 100 {
                            format!("{}...", &tool_result[..100])
                        } else {
                            tool_result.clone()
                        });

                    current_messages.push(Message::tool_result(&tool_call.id, tool_result));
                }

                // Check for cancellation
                if cancel_token.is_cancelled() {
                    let db_lock = database.read().await;
                    if let Some(db) = db_lock.as_ref() {
                        let _ = db.update_chat_message_status(&message_id, ChatMessageStatus::Cancelled, None);
                    }
                    return Err("Cancelled".to_string());
                }

                // Run another completion
                let next_request = CompletionRequest {
                    messages: current_messages.clone(),
                    max_tokens: Some(2048),
                    temperature: Some(0.7),
                    stream: false,
                    tools: tool_definitions.clone(),
                    tool_choice: Some("auto".to_string()),
                    ..Default::default()
                };

                response = engine.complete(next_request).await.map_err(|e| e.to_string())?;

                {
                    let db_lock = database.read().await;
                    if let Some(db) = db_lock.as_ref() {
                        let _ = db.update_chat_message_content(&message_id, &response.content);
                    }
                }

                let _ = app_handle.emit(
                    &format!("chat-stream-{}", session_id),
                    serde_json::json!({
                        "message_id": message_id,
                        "token": "",
                        "content": response.content.clone()
                    }),
                );
            }

            // Final update
            let db_lock = database.read().await;
            if let Some(db) = db_lock.as_ref() {
                db.update_chat_message_content(&message_id, &response.content)
                    .map_err(|e| e.to_string())?;
                db.update_chat_message_status(&message_id, ChatMessageStatus::Complete, None)
                    .map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        Err(e) => {
            let db_lock = database.read().await;
            if let Some(db) = db_lock.as_ref() {
                let _ = db.update_chat_message_status(
                    &message_id,
                    ChatMessageStatus::Error,
                    Some(&e.to_string()),
                );
            }
            Err(e.to_string())
        }
    }
}
