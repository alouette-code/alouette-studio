// ──────────────────────────────────────────────────────────────────────────────
// MiniCPM Inference Engine (candle-native, no Python/GGUF needed)
//
// MiniCPM5-1B dùng kiến trúc LlamaForCausalLM với head_dim=128 (không phải
// head_dim = hidden_size / num_heads = 96). Dùng custom minicpm.rs module.
// ──────────────────────────────────────────────────────────────────────────────

use crate::minicpm::{MiniCpmCache, MiniCpmConfig, MiniCpmModel};
use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokenizers::Tokenizer;

const HF_MODEL_ID: &str = "openbmb/MiniCPM5-1B";

// ──────────────────────────────────────────────────────────────────────────────
// Public interface
// ──────────────────────────────────────────────────────────────────────────────

pub struct MiniCpmInference {
    model: MiniCpmModel,
    cache: MiniCpmCache,
    config: MiniCpmConfig,
    tokenizer: Tokenizer,
    device: Device,
    eos_token_ids: Vec<u32>,
    cancel_flag: Arc<AtomicBool>,
}

impl MiniCpmInference {
    /// Load model từ thư mục chứa .safetensors
    pub fn load(model_dir: &Path, cancel_flag: Arc<AtomicBool>) -> Result<Self, String> {
        // 1. Ensure config.json + tokenizer.json tồn tại
        ensure_model_files(model_dir)?;

        // 2. Load config từ config.json
        let config = load_config(model_dir)?;

        // 3. Detect device
        let device = detect_device();

        // 4. Load safetensors weights
        let safetensors_path = find_safetensors_file(model_dir)?;
        eprintln!(
            "[Inference] Loading safetensors: {}",
            safetensors_path.display()
        );
        let tensors = candle_core::safetensors::load(&safetensors_path, &device)
            .map_err(|e| format!("Failed to load safetensors: {}", e))?;

        // 5. Build MiniCPM model
        let vb = VarBuilder::from_tensors(tensors, DType::F32, &device);
        eprintln!(
            "[Inference] Building MiniCPM5 model (hidden={}, heads={}, head_dim={})...",
            config.hidden_size, config.num_attention_heads, config.head_dim
        );
        let model = MiniCpmModel::load(vb, &config)
            .map_err(|e| format!("Failed to build model: {}", e))?;

        // 6. Build KV cache
        let cache = MiniCpmCache::new(true, DType::F32, &config, &device)
            .map_err(|e| format!("Failed to create KV cache: {}", e))?;

        // 7. Load tokenizer
        let tokenizer_path = model_dir.join("tokenizer.json");
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| format!("Failed to load tokenizer: {}", e))?;

        // Determine EOS token IDs
        let eos_token_ids = resolve_eos_token_ids(&tokenizer);

        crate::state::log_to_app_file(&format!(
            "MiniCPM5 inference engine loaded (device: {:?}, eos: {:?}, vocab: {})",
            device, eos_token_ids, config.vocab_size
        ));

        Ok(Self {
            model,
            cache,
            config,
            tokenizer,
            device,
            eos_token_ids,
            cancel_flag,
        })
    }

    /// Generate text with streaming tokens via callback.
    /// Returns the full generated text.
    pub fn generate_stream(
        &mut self,
        prompt: &str,
        temperature: f64,
        top_p: f64,
        max_tokens: usize,
        max_seq_len: usize,
        mut on_token: impl FnMut(&str) -> Result<(), String>,
    ) -> Result<String, String> {
        // Reset KV cache for new generation
        self.cache.clear();

        // Tokenize
        let encoding = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| format!("Tokenization failed: {}", e))?;
        let mut tokens: Vec<u32> = encoding.get_ids().iter().map(|&t| t as u32).collect();
        let prompt_len = tokens.len();

        if tokens.is_empty() {
            return Err("Empty input after tokenization".into());
        }

        let mut full_response = String::new();

        for i in 0..max_tokens {
            if self.cancel_flag.load(Ordering::SeqCst) {
                break;
            }

            // First pass = all tokens, subsequent = last token only (KV cache)
            let input_slice = if i == 0 {
                &tokens[..]
            } else {
                &tokens[tokens.len() - 1..]
            };
            let seq_len = input_slice.len();
            let seqlen_offset = if i == 0 { 0 } else { tokens.len() - 1 };

            let input_ids = Tensor::from_slice(input_slice, (1, seq_len), &self.device)
                .map_err(|e| format!("Tensor creation failed: {}", e))?;

            let logits = self
                .model
                .forward(&input_ids, seqlen_offset, &mut self.cache)
                .map_err(|e| format!("Model forward failed: {}", e))?;

            let next_token = if temperature <= 0.0 {
                sample_greedy(&logits)
            } else {
                sample_top_p(&logits, temperature, top_p)
            }
            .map_err(|e| format!("Sampling failed: {}", e))?;

            tokens.push(next_token);

            // Check EOS
            if self.eos_token_ids.contains(&next_token) {
                break;
            }

            let token_str = self
                .tokenizer
                .decode(&[next_token], true)
                .unwrap_or_default();

            full_response.push_str(&token_str);
            on_token(&token_str)?;

            if tokens.len() > max_seq_len {
                break;
            }
        }

        eprintln!(
            "[Inference] Generated {} tokens (prompt: {}, generated: {})",
            tokens.len(),
            prompt_len,
            tokens.len() - prompt_len
        );

        Ok(full_response)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Sampling
// ──────────────────────────────────────────────────────────────────────────────

fn sample_greedy(logits: &Tensor) -> Result<u32, String> {
    // logits shape: [1, vocab_size] (model returns last token logits)
    let logits = logits
        .squeeze(0)
        .map_err(|e| format!("squeeze failed: {}", e))?;
    let token = logits
        .argmax(candle_core::D::Minus1)
        .map_err(|e| format!("argmax failed: {}", e))?;
    token
        .to_scalar::<u32>()
        .map_err(|e| format!("to_scalar failed: {}", e))
}

fn sample_top_p(logits: &Tensor, temperature: f64, top_p: f64) -> Result<u32, String> {
    // logits shape: [1, vocab_size]
    let logits = logits
        .squeeze(0)
        .map_err(|e| format!("squeeze failed: {}", e))?;
    let vocab_size = logits.dims()[0];

    // Temperature + softmax
    let scaled =
        (logits / temperature).map_err(|e| format!("temperature scaling failed: {}", e))?;
    let probs = candle_nn::ops::softmax(&scaled, 0)
        .map_err(|e| format!("softmax failed: {}", e))?;

    let probs_data: Vec<f32> = probs
        .to_vec1()
        .map_err(|e| format!("to_vec1 failed: {}", e))?;

    // Sorted indices by probability descending
    let mut indices: Vec<usize> = (0..vocab_size).collect();
    indices.sort_by(|&a, &b| {
        probs_data[b]
            .partial_cmp(&probs_data[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Top-p cutoff
    let mut cumsum = 0.0f32;
    let mut cutoff = vocab_size;
    for (j, &idx) in indices.iter().enumerate() {
        cumsum += probs_data[idx];
        if cumsum >= top_p as f32 {
            cutoff = j + 1;
            break;
        }
    }

    let sum: f32 = indices[..cutoff.min(vocab_size)]
        .iter()
        .map(|&i| probs_data[i])
        .sum();

    if sum <= 0.0 {
        return Ok(indices[0] as u32);
    }

    let rand: f32 = fastrand::f32() * sum;
    cumsum = 0.0;
    for &idx in &indices[..cutoff.min(vocab_size)] {
        cumsum += probs_data[idx];
        if cumsum >= rand {
            return Ok(idx as u32);
        }
    }

    Ok(indices[0] as u32)
}

// ──────────────────────────────────────────────────────────────────────────────
// Utilities
// ──────────────────────────────────────────────────────────────────────────────

/// Parse MiniCPM config.json
#[derive(serde::Deserialize)]
struct RawConfig {
    vocab_size: usize,
    hidden_size: usize,
    intermediate_size: usize,
    num_hidden_layers: usize,
    num_attention_heads: usize,
    num_key_value_heads: Option<usize>,
    head_dim: Option<usize>,
    max_position_embeddings: usize,
    tie_word_embeddings: Option<bool>,
    rope_theta: Option<f64>,
    rms_norm_eps: Option<f64>,
}

fn load_config(model_dir: &Path) -> Result<MiniCpmConfig, String> {
    let content = std::fs::read_to_string(model_dir.join("config.json"))
        .map_err(|e| format!("Cannot read config.json: {}", e))?;
    let raw: RawConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Cannot parse config.json: {}", e))?;

    let num_kv_heads = raw.num_key_value_heads.unwrap_or(raw.num_attention_heads);
    // head_dim explicit (MiniCPM5) hoặc tính từ hidden_size (standard Llama)
    let head_dim = raw.head_dim
        .unwrap_or(raw.hidden_size / raw.num_attention_heads);

    Ok(MiniCpmConfig {
        vocab_size: raw.vocab_size,
        hidden_size: raw.hidden_size,
        intermediate_size: raw.intermediate_size,
        num_hidden_layers: raw.num_hidden_layers,
        num_attention_heads: raw.num_attention_heads,
        num_key_value_heads: num_kv_heads,
        head_dim,
        max_position_embeddings: raw.max_position_embeddings,
        tie_word_embeddings: raw.tie_word_embeddings.unwrap_or(false),
        rope_theta: raw.rope_theta.unwrap_or(10000.0) as f32,
        rms_norm_eps: raw.rms_norm_eps.unwrap_or(1e-6),
    })
}

/// Resolve EOS token IDs từ tokenizer
fn resolve_eos_token_ids(tokenizer: &Tokenizer) -> Vec<u32> {
    let mut ids = Vec::new();
    for token in &["</s>", "<|im_end|>", "<|endoftext|>", "<eos>", "<end>"] {
        if let Some(id) = tokenizer.token_to_id(token) {
            if id != 0 && !ids.contains(&id) {
                ids.push(id);
            }
        }
    }
    // MiniCPM5 eos_token_id: [1, 130073]
    for &fallback in &[1u32, 130073u32] {
        if !ids.contains(&fallback) {
            ids.push(fallback);
        }
    }
    ids
}

fn find_safetensors_file(model_dir: &Path) -> Result<std::path::PathBuf, String> {
    for entry in
        std::fs::read_dir(model_dir).map_err(|e| format!("Cannot read model dir: {}", e))?
    {
        let entry = entry.map_err(|e| format!("Cannot read entry: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "safetensors") {
            return Ok(path);
        }
    }
    Err(format!(
        "No .safetensors file found in {}",
        model_dir.display()
    ))
}

fn detect_device() -> Device {
    Device::Cpu
}

/// Download missing config.json & tokenizer.json từ HuggingFace
fn ensure_model_files(model_dir: &Path) -> Result<(), String> {
    let required = ["config.json", "tokenizer.json"];
    let missing: Vec<&str> = required
        .iter()
        .filter(|f| !model_dir.join(f).exists())
        .copied()
        .collect();

    if missing.is_empty() {
        return Ok(());
    }

    eprintln!(
        "[Inference] Missing files: {:?}. Downloading from HuggingFace ({})...",
        missing, HF_MODEL_ID
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let all_files: Vec<&str> = missing
        .iter()
        .copied()
        .chain(
            [
                "tokenizer_config.json",
                "special_tokens_map.json",
                "generation_config.json",
            ]
            .iter()
            .copied(),
        )
        .collect();

    for fname in all_files {
        let url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            HF_MODEL_ID, fname
        );

        let response = client
            .get(&url)
            .send()
            .map_err(|e| format!("Failed to download {}: {}", fname, e))?;

        if !response.status().is_success() {
            if missing.contains(&fname) {
                return Err(format!(
                    "Required file {} not found on HuggingFace (status {})",
                    fname,
                    response.status()
                ));
            }
            continue;
        }

        let bytes = response
            .bytes()
            .map_err(|e| format!("Failed to read response for {}: {}", fname, e))?;

        std::fs::write(model_dir.join(fname), &bytes)
            .map_err(|e| format!("Failed to write {}: {}", fname, e))?;

        eprintln!("  ✅ Downloaded {}", fname);
    }

    for fname in &required {
        if !model_dir.join(fname).exists() {
            return Err(format!(
                "Required file {} still missing after download",
                fname
            ));
        }
    }

    Ok(())
}

/// Build chat prompt from messages using MiniCPM-compatible format
pub fn build_chat_prompt(messages: &[ChatMessage]) -> String {
    let mut prompt = String::new();
    for msg in messages {
        match msg.role.as_str() {
            "system" => prompt.push_str(&format!("<|system|>\n{}\n", msg.content)),
            "user" => prompt.push_str(&format!("<|user|>\n{}\n", msg.content)),
            "assistant" => prompt.push_str(&format!("<|assistant|>\n{}\n", msg.content)),
            _ => prompt.push_str(&format!("<|user|>\n{}\n", msg.content)),
        }
    }
    prompt.push_str("<|assistant|>\n");
    prompt
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}
