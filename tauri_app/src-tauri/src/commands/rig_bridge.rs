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
            MessageContent::ToolCalls(tcs) => {
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
                    "content": null,
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
            MessageContent::ToolCalls(tcs) => {
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

/// Call OpenAI-compatible API with native tool calling support
async fn call_openai_compatible(
    api_key: &str,
    model: &str,
    api_url: &str,
    temperature: f32,
    top_p: f32,
    messages: &[Value],
    tools: &[Value],
    window: Option<&tauri::WebviewWindow>,
) -> Result<LlmResponse, String> {
    let client = reqwest::Client::new();
    let base_url = if api_url.is_empty() {
        "https://api.openai.com/v1".to_string()
    } else {
        api_url.trim_end_matches('/').to_string()
    };

    let mut body = json!({
        "model": model,
        "messages": messages,
        "temperature": temperature,
        "top_p": top_p,
        "max_tokens": 32000
    });

    if !tools.is_empty() {
        body["tools"] = json!(tools);
        body["tool_choice"] = json!("auto");
    }

    agent_log(window, &format!("[ALOUETTE LLM] Đang gửi HTTP Request tới: {}/chat/completions (model = '{}')", base_url, model));

    let response = client
        .post(format!("{}/chat/completions", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            agent_log(window, &format!("[ALOUETTE LLM ERROR] HTTP Request failed: {}", e));
            format!("HTTP request failed: {}", e)
        })?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .map_err(|e| {
            agent_log(window, &format!("[ALOUETTE LLM ERROR] Failed to read response text: {}", e));
            format!("Failed to read response: {}", e)
        })?;

    if !status.is_success() {
        agent_log(window, &format!(
            "[ALOUETTE LLM ERROR] API trả về lỗi (status {}): {}",
            status,
            &response_text[..response_text.len().min(500)]
        ));
        return Err(format!(
            "API error ({}): {}",
            status,
            &response_text[..response_text.len().min(500)]
        ));
    }

    agent_log(window, &format!("[ALOUETTE LLM] Nhận phản hồi thành công từ: {}", base_url));

    let response_json: Value = serde_json::from_str(&response_text).map_err(|e| {
        format!(
            "Failed to parse JSON: {} — raw: {}",
            e,
            &response_text[..response_text.len().min(200)]
        )
    })?;

    let choices = response_json["choices"]
        .as_array()
        .ok_or_else(|| format!("No choices in response: {}", &response_text[..200]))?;

    if choices.is_empty() {
        return Err("Empty choices array in response".to_string());
    }

    let message = &choices[0]["message"];
    let raw_text = serde_json::to_string(&response_json).unwrap_or_default();

    // Extract native tool_calls
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    if let Some(tc_array) = message["tool_calls"].as_array() {
        for tc in tc_array {
            let func = &tc["function"];
            let name = func["name"].as_str().unwrap_or("").to_string();
            let args_str = func["arguments"].as_str().unwrap_or("{}").to_string();
            if name.is_empty() {
                continue;
            }
            let args_val: Value =
                serde_json::from_str(&args_str).unwrap_or(Value::Object(Default::default()));
            tool_calls.push(ToolCall {
                name,
                arguments: args_val,
                raw_arguments: args_str,
                call_id: Some(tc["id"].as_str().unwrap_or("call_0").to_string()),
            });
        }
    }

    let text = message["content"].as_str().map(|s| s.to_string());

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

    agent_log(window, &format!(
        "\n[ALOUETTE AGENT LOOP] === Bắt đầu gọi LLM ({}) ===",
        model_name
    ));
    agent_log(window, &format!(
        "[ALOUETTE AGENT LOOP] API Standard: '{}' | Endpoint: '{}'",
        api_standard,
        if api_url.is_empty() { "mặc định" } else { api_url }
    ));

    let messages = build_messages_json(system_prompt, history);
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
                window,
            )
            .await
        }
        "claude" => {
            // Anthropic via Rig Agent — fallback to text parsing
            let conversation = build_conversation_text(system_prompt, history);
            let client = providers::anthropic::Client::new(api_key)
                .map_err(|e| format!("Anthropic client: {}", e))?;
            let m = client.completion_model(model_name);
            let agent = AgentBuilder::new(m).preamble(system_prompt).build();
            agent_log(window, &format!("[ALOUETTE LLM] Đang gửi yêu cầu tới Anthropic/Claude qua Rig Agent (model = '{}')...", model_name));
            let response_text = agent
                .prompt(&conversation)
                .await
                .map_err(|e| {
                    agent_log(window, &format!("[ALOUETTE LLM ERROR] Rig Claude failed: {}", e));
                    format!("Rig Claude: {}", e)
                })?;
            agent_log(window, "[ALOUETTE LLM] Đã nhận phản hồi thành công từ Anthropic/Claude.");

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
            // Gemini via Rig Agent — fallback to text parsing
            let conversation = build_conversation_text(system_prompt, history);
            let client = providers::gemini::Client::new(api_key)
                .map_err(|e| format!("Gemini client: {}", e))?;
            let m = client.completion_model(model_name);
            let agent = AgentBuilder::new(m).preamble(system_prompt).build();
            agent_log(window, &format!("[ALOUETTE LLM] Đang gửi yêu cầu tới Google/Gemini qua Rig Agent (model = '{}')...", model_name));
            let response_text = agent
                .prompt(&conversation)
                .await
                .map_err(|e| {
                    agent_log(window, &format!("[ALOUETTE LLM ERROR] Rig Gemini failed: {}", e));
                    format!("Rig Gemini: {}", e)
                })?;
            agent_log(window, "[ALOUETTE LLM] Đã nhận phản hồi thành công từ Google/Gemini.");

            let tool_calls = parse_tool_calls_from_text(&response_text);
            let text =
                core_engine::agent_harness::parser::parse_model_response(&response_text).plain_text;

            Ok(LlmResponse {
                text,
                tool_calls,
                raw_text: response_text,
            })
        }
        _ => {
            // Fallback: any other provider via Rig Agent with text prompt
            let conversation = build_conversation_text(system_prompt, history);

            let response_text = if api_standard == "deepseek" {
                let client = providers::deepseek::Client::new(api_key)
                    .map_err(|e| format!("DeepSeek client: {}", e))?;
                let m = client.completion_model(model_name);
                let agent = AgentBuilder::new(m).preamble(system_prompt).build();
                agent
                    .prompt(&conversation)
                    .await
                    .map_err(|e| format!("Rig DeepSeek: {}", e))?
            } else {
                let client = providers::openai::Client::new(api_key)
                    .map_err(|e| format!("OpenAI client: {}", e))?;
                let m = client.completion_model(model_name);
                let agent = AgentBuilder::new(m).preamble(system_prompt).build();
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
