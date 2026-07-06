use core_engine::agent_harness::parser::ToolCall;
use core_engine::agent_harness::tool_definitions;
use core_engine::agent_harness::{ChatMessage, LlmResponse, MessageContent};
use rig_core::{agent::AgentBuilder, client::CompletionClient, completion::Prompt, providers};
use serde_json::{json, Value};
use tauri::Emitter;

/// Emit a console log statement to the frontend and print to stdout
pub fn agent_log(window: Option<&tauri::WebviewWindow>, msg: &str) {
    println!("{}", msg);
    if let Some(w) = window {
        let _ = w.emit(
            "agent-console-log",
            serde_json::json!({
                "message": msg,
                "timestamp": chrono::Local::now().to_rfc3339(),
            }),
        );
    }
}

/// Convert ChatMessage history to OpenAI-compatible message JSON array
fn build_messages_json(system_prompt: &str, history: &[ChatMessage]) -> Vec<Value> {
    let mut messages: Vec<Value> = Vec::new();

    messages.push(json!({ "role": "system", "content": system_prompt }));

    for msg in history {
        let role = match msg.role.as_str() {
            "user" => "user",
            "assistant" | "model" => "assistant",
            "tool" => "tool",
            _ => "user",
        };

        match &msg.content {
            MessageContent::Text(text) => {
                messages.push(json!({ "role": role, "content": text }));
            }
            MessageContent::ToolCalls(tcs, text_opt) => {
                let tc_json: Vec<Value> = tcs
                    .iter()
                    .enumerate()
                    .map(|(i, tc)| {
                        json!({
                            "id": tc.call_id.clone().unwrap_or_else(|| format!("call_{}", i)),
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": tc.raw_arguments
                            }
                        })
                    })
                    .collect();
                messages.push(json!({
                    "role": "assistant",
                    "content": text_opt,
                    "tool_calls": tc_json
                }));
            }
            MessageContent::ToolResult {
                tool_call_id,
                tool_name,
                result,
                ..
            } => {
                messages.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "name": tool_name,
                    "content": result
                }));
            }
        }
    }

    messages
}

/// Build conversation text for fallback providers
fn build_conversation_text(system_prompt: &str, history: &[ChatMessage]) -> String {
    let mut conversation = String::from(system_prompt);
    conversation.push_str("\n\n=== Conversation ===\n");
    for msg in history {
        let prefix = match msg.role.as_str() {
            "user" => "User",
            "assistant" | "model" => "Assistant",
            "tool" => "[Tool Result]",
            _ => "[System]",
        };
        match &msg.content {
            MessageContent::Text(t) => {
                conversation.push_str(&format!("\n{}: {}", prefix, t));
            }
            MessageContent::ToolResult {
                tool_name, result, ..
            } => {
                conversation.push_str(&format!("\n{} ({})\n{}", prefix, tool_name, result));
            }
            MessageContent::ToolCalls(tcs, text_opt) => {
                if let Some(t) = text_opt {
                    conversation.push_str(&format!("\nAssistant: {}", t));
                }
                for tc in tcs {
                    conversation.push_str(&format!(
                        "\nAssistant calls: {}({})",
                        tc.name, tc.raw_arguments
                    ));
                }
            }
        }
    }
    conversation.push_str("\n\n=== Continue ===\nAssistant:");
    conversation
}

/// Parse tool_calls from response text (fallback when no native tool calls)
fn parse_tool_calls_from_text(text: &str) -> Vec<ToolCall> {
    let parsed = core_engine::agent_harness::parser::parse_model_response(text);
    if !parsed.tool_calls.is_empty() {
        return parsed.tool_calls;
    }
    if let Some(tc) = parsed.tool_call {
        return vec![tc];
    }
    vec![]
}

use futures_util::StreamExt;

#[derive(Default, Clone)]
struct StreamToolCall {
    id: String,
    name: String,
    arguments: String,
}

/// Call OpenAI-compatible API with native tool calling support
async fn call_openai_compatible(
    api_key: &str,
    model: &str,
    api_url: &str,
    temperature: f32,
    top_p: f32,
    messages: &[Value],
    tools: &[Value],
    thinking_mode: Option<&str>,
    window: Option<&tauri::WebviewWindow>,
) -> Result<LlmResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300)) // total request timeout
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
    let base_url = if api_url.trim().is_empty() {
        "https://api.openai.com/v1".to_string()
    } else {
        api_url.trim().trim_end_matches('/').to_string()
    };

    let mut body = json!({
        "model": model,
        "messages": messages,
        "temperature": temperature,
        "top_p": top_p,
        "max_tokens": 32000,
        "stream": true
    });

    if thinking_mode == Some("high") && model.to_lowercase().contains("gemini") {
        body["temperature"] = json!(1.0);
        body["thinking_config"] = json!({
            "thinking_budget": 2048
        });
    }

    if !tools.is_empty() {
        body["tools"] = json!(tools);
        body["tool_choice"] = json!("auto");
    }

    agent_log(
        window,
        &format!(
            "[ALOUETTE LLM] Đang gửi HTTP Request tới: {}/chat/completions (model = '{}')",
            base_url, model
        ),
    );

    let response = client
        .post(format!("{}/chat/completions", base_url))
        .header("Authorization", format!("Bearer {}", api_key.trim()))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            agent_log(
                window,
                &format!("[ALOUETTE LLM ERROR] HTTP Request failed: {}", e),
            );
            format!("HTTP request failed: {}", e)
        })?;

    let status = response.status();
    if !status.is_success() {
        let response_text = response.text().await.unwrap_or_default();
        agent_log(
            window,
            &format!(
                "[ALOUETTE LLM ERROR] API trả về lỗi (status {}): {}",
                status,
                &response_text[..response_text.len().min(500)]
            ),
        );
        return Err(format!(
            "API error ({}): {}",
            status,
            &response_text[..response_text.len().min(500)]
        ));
    }

    agent_log(
        window,
        &format!("[ALOUETTE LLM] Nhận phản hồi thành công từ: {}", base_url),
    );

    let mut stream = response.bytes_stream();
    let mut buffer = Vec::new();

    let mut accumulated_text = String::new();
    let mut accumulated_thought = String::new();
    let mut streamed_tool_calls: Vec<StreamToolCall> = Vec::new();
    let mut raw_response_chunks = Vec::new();
    let mut stream_ended = false;

    // Buffering variables for throttling
    let mut current_thought_chunk = String::new();
    let mut current_text_chunk = String::new();
    let mut last_emit_time = std::time::Instant::now();

    // Total stream duration safeguard (5 minutes max for entire streaming)
    let stream_start = std::time::Instant::now();
    const MAX_STREAM_DURATION: std::time::Duration = std::time::Duration::from_secs(300);

    let mut stream_error: Option<String> = None;

    loop {
        if stream_ended || stream_error.is_some() {
            break;
        }

        // Check total stream duration
        if stream_start.elapsed() > MAX_STREAM_DURATION {
            agent_log(
                window,
                "[ALOUETTE LLM WARNING] Total stream duration exceeded 5 minutes, ending stream.",
            );
            break;
        }

        // ── Đọc chunk với timeout ──
        let chunk_opt =
            match tokio::time::timeout(std::time::Duration::from_secs(30), stream.next()).await {
                Ok(Some(chunk_res)) => match chunk_res {
                    Ok(bytes) => Some(bytes),
                    Err(e) => {
                        // ⚠️ KHÔNG dùng ? ở đây! Log lỗi và vẫn trả về dữ liệu đã accumulate
                        let err_msg = format!("Stream chunk error: {}", e);
                        agent_log(window, &format!("[ALOUETTE LLM ERROR] {}", err_msg));
                        stream_error = Some(err_msg);
                        None
                    }
                },
                Ok(None) => {
                    agent_log(window, "[ALOUETTE LLM DEBUG] Stream ended naturally (EOF).");
                    None
                }
                Err(_) => {
                    agent_log(
                        window,
                        "[ALOUETTE LLM WARNING] Stream read timeout (30 seconds), ending stream.",
                    );
                    None
                }
            };

        let chunk = match chunk_opt {
            Some(bytes) => bytes,
            None => {
                break;
            }
        };
        buffer.extend_from_slice(&chunk);

        // ── Xử lý các dòng trong buffer ──
        while let Some(newline_idx) = buffer.iter().position(|&b| b == b'\n') {
            let line_bytes = buffer.drain(..=newline_idx).collect::<Vec<u8>>();
            let line = String::from_utf8_lossy(&line_bytes);
            let line_trimmed = line.trim();

            if line_trimmed.starts_with("data: ") {
                let data = &line_trimmed["data: ".len()..];
                if data == "[DONE]" {
                    stream_ended = true;
                    break;
                }
                raw_response_chunks.push(data.to_string());
                if let Ok(val) = serde_json::from_str::<Value>(data) {
                    if let Some(choices) = val["choices"].as_array() {
                        if !choices.is_empty() {
                            let delta = &choices[0]["delta"];

                            // 1. Check for thinking content
                            if let Some(reasoning) =
                                delta.get("reasoning_content").and_then(|r| r.as_str())
                            {
                                current_thought_chunk.push_str(reasoning);
                                accumulated_thought.push_str(reasoning);
                            } else if let Some(reasoning) =
                                delta.get("reasoning").and_then(|r| r.as_str())
                            {
                                current_thought_chunk.push_str(reasoning);
                                accumulated_thought.push_str(reasoning);
                            }

                            // 2. Check for normal text content
                            if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                current_text_chunk.push_str(content);
                                accumulated_text.push_str(content);
                            }

                            // 3. Check for tool calls
                            if let Some(tc_array) =
                                delta.get("tool_calls").and_then(|tc| tc.as_array())
                            {
                                for tc in tc_array {
                                    let idx = tc.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                                    while streamed_tool_calls.len() <= idx {
                                        streamed_tool_calls.push(StreamToolCall::default());
                                    }
                                    let stc = &mut streamed_tool_calls[idx];
                                    if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                                        stc.id = id.to_string();
                                    }
                                    if let Some(func) =
                                        tc.get("function").and_then(|f| f.as_object())
                                    {
                                        if let Some(name) =
                                            func.get("name").and_then(|n| n.as_str())
                                        {
                                            stc.name.push_str(name);
                                        }
                                        if let Some(args) =
                                            func.get("arguments").and_then(|a| a.as_str())
                                        {
                                            stc.arguments.push_str(args);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if stream_ended || stream_error.is_some() {
            break;
        }

        // Throttled emit: if 100ms elapsed, flush both buffers
        if last_emit_time.elapsed() >= std::time::Duration::from_millis(100) {
            if !current_thought_chunk.is_empty() {
                if let Some(w) = window {
                    let _ = w.emit("agent-thought-chunk", &current_thought_chunk);
                }
                current_thought_chunk.clear();
            }
            if !current_text_chunk.is_empty() {
                if let Some(w) = window {
                    let _ = w.emit("agent-text-chunk", &current_text_chunk);
                }
                current_text_chunk.clear();
            }
            last_emit_time = std::time::Instant::now();
        }
    }

    // ── Xử lý dữ liệu còn sót trong buffer ──
    if !buffer.is_empty() {
        let remaining = String::from_utf8_lossy(&buffer);
        let trimmed = remaining.trim();
        if trimmed.starts_with("data: ") {
            let data = &trimmed["data: ".len()..];
            if data != "[DONE]" {
                agent_log(
                    window,
                    "[ALOUETTE LLM DEBUG] Processing remaining buffer data as final line.",
                );
                raw_response_chunks.push(data.to_string());
                if let Ok(val) = serde_json::from_str::<Value>(data) {
                    if let Some(choices) = val["choices"].as_array() {
                        if !choices.is_empty() {
                            let delta = &choices[0]["delta"];

                            if let Some(reasoning) =
                                delta.get("reasoning_content").and_then(|r| r.as_str())
                            {
                                current_thought_chunk.push_str(reasoning);
                                accumulated_thought.push_str(reasoning);
                            } else if let Some(reasoning) =
                                delta.get("reasoning").and_then(|r| r.as_str())
                            {
                                current_thought_chunk.push_str(reasoning);
                                accumulated_thought.push_str(reasoning);
                            }

                            if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                current_text_chunk.push_str(content);
                                accumulated_text.push_str(content);
                            }

                            if let Some(tc_array) =
                                delta.get("tool_calls").and_then(|tc| tc.as_array())
                            {
                                for tc in tc_array {
                                    if let Some(idx) = tc.get("index").and_then(|i| i.as_u64()) {
                                        let idx = idx as usize;
                                        while streamed_tool_calls.len() <= idx {
                                            streamed_tool_calls.push(StreamToolCall::default());
                                        }
                                        let stc = &mut streamed_tool_calls[idx];
                                        if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                                            stc.id = id.to_string();
                                        }
                                        if let Some(func) =
                                            tc.get("function").and_then(|f| f.as_object())
                                        {
                                            if let Some(name) =
                                                func.get("name").and_then(|n| n.as_str())
                                            {
                                                stc.name.push_str(name);
                                            }
                                            if let Some(args) =
                                                func.get("arguments").and_then(|a| a.as_str())
                                            {
                                                stc.arguments.push_str(args);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ── Flush buffer emit còn lại ──
    if !current_thought_chunk.is_empty() {
        if let Some(w) = window {
            let _ = w.emit("agent-thought-chunk", &current_thought_chunk);
        }
    }
    if !current_text_chunk.is_empty() {
        if let Some(w) = window {
            let _ = w.emit("agent-text-chunk", &current_text_chunk);
        }
    }

    // ── Reconstruct tool calls ──
    let mut tool_calls = Vec::new();
    for stc in streamed_tool_calls {
        if stc.name.is_empty() {
            continue;
        }
        let args_val: Value =
            serde_json::from_str(&stc.arguments).unwrap_or(Value::Object(Default::default()));
        tool_calls.push(ToolCall {
            name: stc.name,
            arguments: args_val,
            raw_arguments: stc.arguments,
            call_id: Some(if stc.id.is_empty() {
                "call_0".to_string()
            } else {
                stc.id
            }),
        });
    }

    let has_text = !accumulated_text.is_empty();
    let accumulated_text_len = accumulated_text.len();
    let text = if has_text {
        Some(accumulated_text)
    } else {
        None
    };
    let raw_text = raw_response_chunks.join("\n");

    if !accumulated_thought.is_empty() {
        if let Some(w) = window {
            let _ = w.emit("agent-thought-final", &accumulated_thought);
        }
    }

    // 🔥 EMIT STREAM COMPLETE EVENT để frontend biết stream đã kết thúc
    if let Some(w) = window {
        let _ = w.emit(
            "agent-stream-complete",
            serde_json::json!({
                "has_error": stream_error.is_some(),
                "error": stream_error,
                "has_text": has_text,
            }),
        );
    }

    // Nếu có lỗi stream nhưng đã accumulate được dữ liệu, vẫn trả về Ok
    // Chỉ return Err nếu KHÔNG có dữ liệu nào được accumulate
    if let Some(err) = stream_error {
        if !has_text && accumulated_thought.is_empty() && tool_calls.is_empty() {
            return Err(err);
        }
        agent_log(
            window,
            &format!(
                "[ALOUETTE LLM WARNING] Stream có lỗi nhưng đã accumulate được {} bytes text, {} bytes thought, {} tool calls. Vẫn trả về Ok.",
                accumulated_text_len,
                accumulated_thought.len(),
                tool_calls.len()
            ),
        );
    }

    Ok(LlmResponse {
        text,
        tool_calls,
        raw_text,
    })
}

/// Main entry: call LLM with structured messages + native tool calling.
///
/// Returns `LlmResponse` with native `tool_calls` when the provider supports it.
/// Falls back to text parsing for providers without native tool support.
pub async fn call_rig(
    api_standard: &str,
    api_key: &str,
    model: &str,
    api_url: &str,
    temperature: f32,
    top_p: f32,
    system_prompt: &str,
    history: &[ChatMessage],
    thinking_mode: Option<&str>,
    window: Option<&tauri::WebviewWindow>,
) -> Result<LlmResponse, String> {
    let model_name = if model.is_empty() {
        match api_standard {
            "claude" => "claude-sonnet-5",
            "gemini" => "gemini-1.5-flash",
            _ if api_url.contains("deepseek") => "deepseek-v4-flash",
            _ => "gpt-4o",
        }
    } else {
        model
    };

    agent_log(
        window,
        &format!(
            "\n[ALOUETTE AGENT LOOP] === Bắt đầu gọi LLM ({}) ===",
            model_name
        ),
    );
    
    // Emit empty chunk to create stream placeholder early in UI
    if let Some(w) = window {
        let _ = w.emit("agent-text-chunk", "");
    }
    agent_log(
        window,
        &format!(
            "[ALOUETTE AGENT LOOP] API Standard: '{}' | Endpoint: '{}'",
            api_standard,
            if api_url.is_empty() {
                "mặc định"
            } else {
                api_url
            }
        ),
    );

    let mut final_system_prompt = system_prompt.to_string();
    if thinking_mode == Some("high") {
        final_system_prompt.push_str("\n\nIMPORTANT: You MUST use deep thinking/reasoning before answering. Think step-by-step in detail and output your thought process.");
    } else if thinking_mode == Some("low") {
        final_system_prompt.push_str("\n\nIMPORTANT: You do not need to use deep thinking/reasoning if the request is simple. Answer directly and concisely.");
    }

    let messages = build_messages_json(&final_system_prompt, history);
    let tools_json = tool_definitions::tools_json_for_api();
    let tools_array = tools_json.as_array().cloned().unwrap_or_default();

    match api_standard {
        "openai" => {
            call_openai_compatible(
                api_key,
                model_name,
                api_url,
                temperature,
                top_p,
                &messages,
                &tools_array,
                thinking_mode,
                window,
            )
            .await
        }
        "claude" => {
            // Anthropic via Rig Agent — fallback to text parsing
            let conversation = build_conversation_text(&final_system_prompt, history);
            let client = providers::anthropic::Client::new(api_key.trim())
                .map_err(|e| format!("Anthropic client: {}", e))?;
            let m = client.completion_model(model_name);
            let agent = AgentBuilder::new(m).preamble(&final_system_prompt).build();
            agent_log(window, &format!("[ALOUETTE LLM] Đang gửi yêu cầu tới Anthropic/Claude qua Rig Agent (model = '{}')...", model_name));
            let response_text = agent.prompt(&conversation).await.map_err(|e| {
                agent_log(
                    window,
                    &format!("[ALOUETTE LLM ERROR] Rig Claude failed: {}", e),
                );
                format!("Rig Claude: {}", e)
            })?;
            agent_log(
                window,
                "[ALOUETTE LLM] Đã nhận phản hồi thành công từ Anthropic/Claude.",
            );

            let tool_calls = parse_tool_calls_from_text(&response_text);
            let text =
                core_engine::agent_harness::parser::parse_model_response(&response_text).plain_text;

            Ok(LlmResponse {
                text,
                tool_calls,
                raw_text: response_text,
            })
        }
        "gemini" => {
            // Sử dụng OpenAI compatibility layer của Google để hỗ trợ Native Tool Calls trọn vẹn
            let mut gemini_url = api_url.trim().trim_end_matches('/').to_string();
            if gemini_url.contains("generativelanguage.googleapis.com") && !gemini_url.ends_with("openai") {
                gemini_url = format!("{}/openai", gemini_url);
            }
            call_openai_compatible(
                api_key.trim(),
                model_name,
                &gemini_url,
                temperature,
                top_p,
                &messages,
                &tools_array,
                thinking_mode,
                window,
            )
            .await
        }
        _ => {
            // Fallback: any other provider via Rig Agent with text prompt
            let conversation = build_conversation_text(&final_system_prompt, history);

            let response_text = if api_standard == "deepseek" {
                let client = providers::deepseek::Client::new(api_key)
                    .map_err(|e| format!("DeepSeek client: {}", e))?;
                let m = client.completion_model(model_name);
                let agent = AgentBuilder::new(m).preamble(&final_system_prompt).build();
                agent
                    .prompt(&conversation)
                    .await
                    .map_err(|e| format!("Rig DeepSeek: {}", e))?
            } else {
                let client = providers::openai::Client::new(api_key)
                    .map_err(|e| format!("OpenAI client: {}", e))?;
                let m = client.completion_model(model_name);
                let agent = AgentBuilder::new(m).preamble(&final_system_prompt).build();
                agent
                    .prompt(&conversation)
                    .await
                    .map_err(|e| format!("Rig OpenAI: {}", e))?
            };

            let tool_calls = parse_tool_calls_from_text(&response_text);
            let text =
                core_engine::agent_harness::parser::parse_model_response(&response_text).plain_text;

            Ok(LlmResponse {
                text,
                tool_calls,
                raw_text: response_text,
            })
        }
    }
}
