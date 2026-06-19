// ──────────────────────────────────────────────────────────────────────────────
// MiniCPM5 model (LlamaForCausalLM với explicit head_dim)
//
// Candle's built-in Llama tính head_dim = hidden_size / num_attention_heads
// nhưng MiniCPM5-1B có head_dim=128 tách biệt (không phải 1536/16=96).
// Module này copy từ candle llama.rs và thêm hỗ trợ explicit head_dim.
// ──────────────────────────────────────────────────────────────────────────────

use candle_core::{DType, Device, IndexOp, Result, Tensor};
use candle_nn::{embedding, linear_no_bias as linear, rms_norm, Embedding, Linear, Module, RmsNorm, VarBuilder};
use std::collections::HashMap;

// ──────────────────────────────────────────────────────────────────────────────
// Config
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MiniCpmConfig {
    pub hidden_size: usize,
    pub intermediate_size: usize,
    pub vocab_size: usize,
    pub num_hidden_layers: usize,
    pub num_attention_heads: usize,
    pub num_key_value_heads: usize,
    pub head_dim: usize,       // explicit head_dim (128 for MiniCPM5-1B)
    pub rms_norm_eps: f64,
    pub rope_theta: f32,
    pub max_position_embeddings: usize,
    pub tie_word_embeddings: bool,
}

impl MiniCpmConfig {
    /// Build config từ parsed config.json của MiniCPM5-1B
    #[allow(dead_code)]
    pub fn minicpm5_1b() -> Self {
        Self {
            hidden_size: 1536,
            intermediate_size: 4608,
            vocab_size: 130560,
            num_hidden_layers: 24,
            num_attention_heads: 16,
            num_key_value_heads: 2,
            head_dim: 128,
            rms_norm_eps: 1e-6,
            rope_theta: 5_000_000.0,
            max_position_embeddings: 131072,
            tie_word_embeddings: false,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// KV Cache
// ──────────────────────────────────────────────────────────────────────────────

pub struct MiniCpmCache {
    pub use_kv_cache: bool,
    kvs: Vec<Option<(Tensor, Tensor)>>,
    masks: HashMap<(usize, usize), Tensor>,
    cos: Tensor,
    sin: Tensor,
    device: Device,
}

impl MiniCpmCache {
    pub fn new(use_kv_cache: bool, dtype: DType, cfg: &MiniCpmConfig, device: &Device) -> Result<Self> {
        let head_dim = cfg.head_dim;
        let theta: Vec<f32> = (0..head_dim)
            .step_by(2)
            .map(|i| 1f32 / cfg.rope_theta.powf(i as f32 / head_dim as f32))
            .collect();

        let theta = Tensor::new(theta, device)?;
        let idx_theta = Tensor::arange(0, cfg.max_position_embeddings as u32, device)?
            .to_dtype(DType::F32)?
            .reshape((cfg.max_position_embeddings, 1))?
            .matmul(&theta.reshape((1, theta.elem_count()))?)?;

        let cos = idx_theta.cos()?.to_dtype(dtype)?;
        let sin = idx_theta.sin()?.to_dtype(dtype)?;

        Ok(Self {
            use_kv_cache,
            kvs: vec![None; cfg.num_hidden_layers],
            masks: HashMap::new(),
            device: device.clone(),
            cos,
            sin,
        })
    }

    pub fn clear(&mut self) {
        for kv in self.kvs.iter_mut() {
            *kv = None;
        }
        self.masks.clear();
    }

    fn mask(&mut self, seq_len: usize, index_pos: usize) -> Result<Tensor> {
        let kv_len = index_pos + seq_len;
        if let Some(mask) = self.masks.get(&(seq_len, kv_len)) {
            return Ok(mask.clone());
        }
        // Causal mask: upper triangular of -inf
        let mask: Vec<u8> = (0..seq_len)
            .flat_map(|i| (0..kv_len).map(move |j| if j > i + index_pos { 1u8 } else { 0u8 }))
            .collect();
        let mask = Tensor::from_slice(&mask, (seq_len, kv_len), &self.device)?;
        self.masks.insert((seq_len, kv_len), mask.clone());
        Ok(mask)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Attention
// ──────────────────────────────────────────────────────────────────────────────

struct Attention {
    q_proj: Linear,
    k_proj: Linear,
    v_proj: Linear,
    o_proj: Linear,
    num_heads: usize,
    num_kv_heads: usize,
    head_dim: usize,
    max_position_embeddings: usize,
}

impl Attention {
    fn load(vb: VarBuilder, cfg: &MiniCpmConfig) -> Result<Self> {
        let hd = cfg.head_dim;
        let size_q = hd * cfg.num_attention_heads;    // 128 * 16 = 2048
        let size_kv = hd * cfg.num_key_value_heads;   // 128 * 2  = 256

        let q_proj = linear(cfg.hidden_size, size_q, vb.pp("q_proj"))?;
        let k_proj = linear(cfg.hidden_size, size_kv, vb.pp("k_proj"))?;
        let v_proj = linear(cfg.hidden_size, size_kv, vb.pp("v_proj"))?;
        let o_proj = linear(size_q, cfg.hidden_size, vb.pp("o_proj"))?;

        Ok(Self {
            q_proj,
            k_proj,
            v_proj,
            o_proj,
            num_heads: cfg.num_attention_heads,
            num_kv_heads: cfg.num_key_value_heads,
            head_dim: hd,
            max_position_embeddings: cfg.max_position_embeddings,
        })
    }

    fn apply_rotary_emb(&self, x: &Tensor, index_pos: usize, cache: &MiniCpmCache) -> Result<Tensor> {
        let (_b, _h, seq_len, _hd) = x.dims4()?;
        let cos = cache.cos.narrow(0, index_pos, seq_len)?;
        let sin = cache.sin.narrow(0, index_pos, seq_len)?;
        candle_nn::rotary_emb::rope(x, &cos, &sin)
    }

    fn repeat_kv(&self, x: Tensor) -> Result<Tensor> {
        let n_rep = self.num_heads / self.num_kv_heads;
        if n_rep == 1 {
            return Ok(x);
        }
        let (b, h, s, d) = x.dims4()?;
        x.unsqueeze(2)?
            .expand((b, h, n_rep, s, d))?
            .reshape((b, h * n_rep, s, d))
    }

    fn forward(&self, x: &Tensor, index_pos: usize, block_idx: usize, cache: &mut MiniCpmCache) -> Result<Tensor> {
        let (b_sz, seq_len, _) = x.dims3()?;

        let q = self.q_proj.forward(x)?;
        let k = self.k_proj.forward(x)?;
        let v = self.v_proj.forward(x)?;

        // Reshape to [b, heads, seq, head_dim]
        let q = q.reshape((b_sz, seq_len, self.num_heads, self.head_dim))?
            .transpose(1, 2)?.contiguous()?;
        let k = k.reshape((b_sz, seq_len, self.num_kv_heads, self.head_dim))?
            .transpose(1, 2)?.contiguous()?;
        let mut v = v.reshape((b_sz, seq_len, self.num_kv_heads, self.head_dim))?
            .transpose(1, 2)?;

        // Apply RoPE
        let q = self.apply_rotary_emb(&q, index_pos, cache)?;
        let mut k = self.apply_rotary_emb(&k, index_pos, cache)?;

        // KV cache
        if cache.use_kv_cache {
            if let Some((cache_k, cache_v)) = &cache.kvs[block_idx] {
                k = Tensor::cat(&[cache_k, &k], 2)?.contiguous()?;
                v = Tensor::cat(&[cache_v, &v], 2)?.contiguous()?;
                // Trim if exceeding max length
                let k_len = k.dims()[2];
                if k_len > self.max_position_embeddings {
                    k = k.narrow(2, k_len - self.max_position_embeddings, self.max_position_embeddings)?
                        .contiguous()?;
                }
                let v_len = v.dims()[2];
                if v_len > self.max_position_embeddings {
                    v = v.narrow(2, v_len - self.max_position_embeddings, self.max_position_embeddings)?
                        .contiguous()?;
                }
            }
            cache.kvs[block_idx] = Some((k.clone(), v.clone()));
        }

        // Repeat KV heads to match Q heads (GQA)
        let k = self.repeat_kv(k)?;
        let v = self.repeat_kv(v)?;

        // Scaled dot-product attention
        let in_dtype = q.dtype();
        let q = q.to_dtype(DType::F32)?;
        let k = k.to_dtype(DType::F32)?;
        let v = v.to_dtype(DType::F32)?;

        let scale = (self.head_dim as f64).sqrt();
        let att = q.matmul(&k.t()?)?.affine(1.0 / scale, 0.0)?;

        // Apply causal mask when seq_len > 1
        let att = if seq_len == 1 {
            att
        } else {
            let mask = cache.mask(seq_len, index_pos)?;
            let mask_f = mask
                .to_dtype(DType::F32)?
                .broadcast_as(att.shape())?;
            let neg_inf = (mask_f * f32::NEG_INFINITY as f64)?;
            let mask_bool = mask.broadcast_as(att.shape())?;
            mask_bool.where_cond(&neg_inf, &att)?
        };

        let att = candle_nn::ops::softmax_last_dim(&att)?;
        let y = att.matmul(&v.contiguous()?)?.to_dtype(in_dtype)?;

        // [b, heads, seq, hd] → [b, seq, hidden]
        let hidden_size = self.num_heads * self.head_dim;
        let y = y.transpose(1, 2)?.reshape(&[b_sz, seq_len, hidden_size])?;
        self.o_proj.forward(&y)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// MLP (SwiGLU)
// ──────────────────────────────────────────────────────────────────────────────

struct Mlp {
    gate_proj: Linear,
    up_proj: Linear,
    down_proj: Linear,
}

impl Mlp {
    fn load(vb: VarBuilder, cfg: &MiniCpmConfig) -> Result<Self> {
        Ok(Self {
            gate_proj: linear(cfg.hidden_size, cfg.intermediate_size, vb.pp("gate_proj"))?,
            up_proj:   linear(cfg.hidden_size, cfg.intermediate_size, vb.pp("up_proj"))?,
            down_proj: linear(cfg.intermediate_size, cfg.hidden_size, vb.pp("down_proj"))?,
        })
    }

    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        let gate = candle_nn::ops::silu(&self.gate_proj.forward(x)?)?;
        let up = self.up_proj.forward(x)?;
        self.down_proj.forward(&(gate * up)?)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Transformer Block
// ──────────────────────────────────────────────────────────────────────────────

struct Block {
    input_layernorm: RmsNorm,
    self_attn: Attention,
    post_attention_layernorm: RmsNorm,
    mlp: Mlp,
}

impl Block {
    fn load(vb: VarBuilder, cfg: &MiniCpmConfig) -> Result<Self> {
        Ok(Self {
            input_layernorm: rms_norm(cfg.hidden_size, cfg.rms_norm_eps, vb.pp("input_layernorm"))?,
            self_attn: Attention::load(vb.pp("self_attn"), cfg)?,
            post_attention_layernorm: rms_norm(cfg.hidden_size, cfg.rms_norm_eps, vb.pp("post_attention_layernorm"))?,
            mlp: Mlp::load(vb.pp("mlp"), cfg)?,
        })
    }

    fn forward(&self, x: &Tensor, index_pos: usize, block_idx: usize, cache: &mut MiniCpmCache) -> Result<Tensor> {
        let residual = x;
        let x = self.input_layernorm.forward(x)?;
        let x = (self.self_attn.forward(&x, index_pos, block_idx, cache)? + residual)?;
        let residual = &x;
        let x = (self.mlp.forward(&self.post_attention_layernorm.forward(&x)?)? + residual)?;
        Ok(x)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Main Model
// ──────────────────────────────────────────────────────────────────────────────

pub struct MiniCpmModel {
    embed_tokens: Embedding,
    layers: Vec<Block>,
    norm: RmsNorm,
    lm_head: Linear,
}

impl MiniCpmModel {
    pub fn load(vb: VarBuilder, cfg: &MiniCpmConfig) -> Result<Self> {
        let embed_tokens = embedding(cfg.vocab_size, cfg.hidden_size, vb.pp("model.embed_tokens"))?;
        let norm = rms_norm(cfg.hidden_size, cfg.rms_norm_eps, vb.pp("model.norm"))?;
        let lm_head = if cfg.tie_word_embeddings {
            Linear::new(embed_tokens.embeddings().clone(), None)
        } else {
            linear(cfg.hidden_size, cfg.vocab_size, vb.pp("lm_head"))?
        };
        let layers = (0..cfg.num_hidden_layers)
            .map(|i| Block::load(vb.pp(format!("model.layers.{i}")), cfg))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self { embed_tokens, layers, norm, lm_head })
    }

    pub fn forward(&self, input_ids: &Tensor, index_pos: usize, cache: &mut MiniCpmCache) -> Result<Tensor> {
        let (_b_sz, seq_len) = input_ids.dims2()?;
        let mut x = self.embed_tokens.forward(input_ids)?;
        for (block_idx, block) in self.layers.iter().enumerate() {
            x = block.forward(&x, index_pos, block_idx, cache)?;
        }
        let x = self.norm.forward(&x)?;
        let x = x.i((.., seq_len - 1, ..))?.contiguous()?;
        let logits = self.lm_head.forward(&x)?;
        logits.to_dtype(DType::F32)
    }
}
