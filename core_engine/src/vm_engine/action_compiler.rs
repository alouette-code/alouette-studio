use crate::vm_engine::qmp_client::VmInputAction;
use crate::vm_engine::vnc_stream::VmObservation;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FastCompileResult {
    pub model_used: String,
    pub endpoint_used: String,
    pub actions: Vec<VmInputAction>,
    pub raw_model_output: String,
}

/// Calls a fast local small LLM (Ollama / llama.cpp OpenAI-compatible endpoint)
/// to compile a natural-language UI instruction + lightweight observation into
/// a precise batch of QMP input actions.
pub async fn compile_instruction_with_small_model(
    instruction: &str,
    observation: &VmObservation,
    endpoint_override: Option<&str>,
    model_override: Option<&str>,
) -> Result<FastCompileResult, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

    let (endpoint, model) = resolve_local_endpoint_and_model(
        &client,
        endpoint_override,
        model_override,
    )
    .await?;

    let mut ui_elements_json = String::new();
    if !observation.ocr_boxes.is_empty() {
        let compact: Vec<serde_json::Value> = observation
            .ocr_boxes
            .iter()
            .take(100)
            .map(|b| json!({ "t": b.text, "cx": b.cx, "cy": b.cy }))
            .collect();
        ui_elements_json = serde_json::to_string(&compact).unwrap_or_else(|_| "[]".to_string());
    }

    let system_prompt = r#"You are an ultra-fast VM Action Compiler.
Convert the user's instruction and current screen UI elements into a JSON array of actions ONLY (no markdown, no explanation).
Allowed action objects:
- {"type":"click","x":123,"y":456,"button":"left","double":false}
- {"type":"move","x":123,"y":456}
- {"type":"type","text":"ls -la\n"}
- {"type":"key","key":"ctrl+c"} (keys: ret, esc, tab, spc, backspace, delete, up, down, left, right, super, ctrl+c, alt+tab, f1..f12)
- {"type":"scroll","x":500,"y":400,"direction":"down","amount":3}
- {"type":"wait","ms":300}
Rules:
1. Use the exact (cx, cy) coordinates of matching UI elements when available.
2. Batch sequential steps cleanly (e.g. click input -> type text -> key ret).
3. Output strictly a valid JSON array `[...]`."#;

    let user_prompt = format!(
        "Screen: {}x{}\nVisible text elements (text, center x/y): {}\nInstruction: {}",
        observation.width,
        observation.height,
        if ui_elements_json.is_empty() {
            "none (use keyboard shortcuts or coordinates)".to_string()
        } else {
            ui_elements_json
        },
        instruction
    );

    let payload = json!({
        "model": model,
        "temperature": 0.0,
        "max_tokens": 256,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_prompt }
        ]
    });

    let resp = client
        .post(&endpoint)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to reach local small model at '{}': {}", endpoint, e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!(
            "Local small model HTTP {} from '{}': {}",
            status, endpoint, body
        ));
    }

    let json_resp: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Invalid JSON response from small model: {}", e))?;

    let raw_content = json_resp["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| format!("Unexpected chat completion payload: {}", json_resp))?
        .trim()
        .to_string();

    let actions = parse_actions_json(&raw_content)?;
    if actions.is_empty() {
        return Err(format!(
            "Small model '{}' returned empty action list for instruction.",
            model
        ));
    }

    Ok(FastCompileResult {
        model_used: model,
        endpoint_used: endpoint,
        actions,
        raw_model_output: raw_content,
    })
}

async fn resolve_local_endpoint_and_model(
    client: &reqwest::Client,
    endpoint_override: Option<&str>,
    model_override: Option<&str>,
) -> Result<(String, String), String> {
    if let Some(ep) = endpoint_override.filter(|s| !s.trim().is_empty()) {
        let normalized = if ep.ends_with("/chat/completions") {
            ep.to_string()
        } else {
            format!("{}/v1/chat/completions", ep.trim_end_matches('/'))
        };
        let model = model_override
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("qwen2.5:0.5b")
            .to_string();
        return Ok((normalized, model));
    }

    // Probe Ollama (11434) and llama.cpp server (8080)
    if let Ok(resp) = client
        .get("http://127.0.0.1:11434/api/tags")
        .timeout(Duration::from_millis(600))
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(tags) = resp.json::<serde_json::Value>().await {
                if let Some(arr) = tags.get("models").and_then(|m| m.as_array()) {
                    let picked = model_override
                        .filter(|s| !s.trim().is_empty())
                        .map(ToString::to_string)
                        .or_else(|| pick_smallest_model(arr, "name"));
                    if let Some(m) = picked {
                        return Ok((
                            "http://127.0.0.1:11434/v1/chat/completions".to_string(),
                            m,
                        ));
                    }
                }
            }
        }
    }

    if let Ok(resp) = client
        .get("http://127.0.0.1:8080/v1/models")
        .timeout(Duration::from_millis(600))
        .send()
        .await
    {
        if resp.status().is_success() {
            let model = if let Some(m) = model_override.filter(|s| !s.trim().is_empty()) {
                m.to_string()
            } else if let Ok(v) = resp.json::<serde_json::Value>().await {
                v["data"][0]["id"]
                    .as_str()
                    .unwrap_or("local-model")
                    .to_string()
            } else {
                "local-model".to_string()
            };
            return Ok((
                "http://127.0.0.1:8080/v1/chat/completions".to_string(),
                model,
            ));
        }
    }

    Err("No local small model server detected on 127.0.0.1:11434 (Ollama) or 127.0.0.1:8080 (llama-server). Start a small fast model (e.g. `ollama pull qwen2.5:0.5b`) or pass `endpoint`, or use `vm_act` directly.".to_string())
}

fn pick_smallest_model(models: &[serde_json::Value], key: &str) -> Option<String> {
    // Prefer ultra-fast small models first
    let preferred = ["0.5b", "1b", "1.5b", "2b", "3b", "mini", "nano", "tiny"];
    for pat in preferred {
        for m in models {
            if let Some(name) = m.get(key).and_then(|v| v.as_str()) {
                if name.to_lowercase().contains(pat) {
                    return Some(name.to_string());
                }
            }
        }
    }
    models
        .first()
        .and_then(|m| m.get(key))
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

pub fn parse_actions_json(raw: &str) -> Result<Vec<VmInputAction>, String> {
    let trimmed = raw.trim();
    let candidate = if let (Some(start), Some(end)) = (trimmed.find('['), trimmed.rfind(']')) {
        &trimmed[start..=end]
    } else {
        trimmed
    };

    serde_json::from_str::<Vec<VmInputAction>>(candidate)
        .map_err(|e| format!("Failed to parse compiled actions JSON ({}): {}", e, raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_actions_json_strips_fences() {
        let raw = "```json\n[{\"type\":\"click\",\"x\":100,\"y\":200},{\"type\":\"type\",\"text\":\"hi\\n\"}]\n```";
        let actions = parse_actions_json(raw).unwrap();
        assert_eq!(actions.len(), 2);
        match &actions[0] {
            VmInputAction::Click { x, y, .. } => {
                assert_eq!((*x, *y), (100, 200));
            }
            _ => panic!("Expected click"),
        }
    }
}
