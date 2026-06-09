use std::path::Path;
use std::sync::Arc;
use tokenizers::Tokenizer;
use tract_onnx::prelude::*;

/// Embedding Model sử dụng BGE-small-en-v1.5 (ONNX)
///
/// Đặc điểm:
/// - Model: BAAI/bge-small-en-v1.5 (Xenova ONNX export)
/// - Kích thước: ~127MB on disk, ~150MB RAM khi load
/// - Output: 384-dim vector, chuẩn hóa unit vector
/// - Tốc độ: ~10-30ms / embedding trên CPU yếu
///
/// Tối ưu cho máy yếu:
/// - Dùng tract-onnx (không cần CUDA/CUDNN)
/// - Mean pooling + normalization tự động
/// - Query instruction prefix cho retrieval quality tốt hơn
pub struct EmbeddingModel {
    model: Arc<TypedRunnableModel>,
    tokenizer: Tokenizer,
    max_length: usize,
}

/// Prefix instruction dành cho query (bge-small format)
/// Giúp model phân biệt query vs document, tăng chất lượng retrieval
const QUERY_PREFIX: &str = "Represent this function for searching similar code: ";

impl EmbeddingModel {
    /// Load model từ thư mục chứa model.onnx + tokenizer.json
    ///
    /// # Arguments
    /// * `model_dir` - Thư mục chứa:
    ///   - `model.onnx` (bắt buộc)
    ///   - `tokenizer.json` (bắt buộc)
    ///
    /// # Errors
    /// Trả về String nếu load thất bại (file không tồn tại, ONNX ops không support, ...)
    pub fn load(model_dir: &Path) -> Result<Self, String> {
        let model_path = model_dir.join("model.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            return Err(format!(
                "ONNX model not found at: {}. Please run scripts/download_embedding_model.sh",
                model_path.display()
            ));
        }
        if !tokenizer_path.exists() {
            return Err(format!(
                "Tokenizer not found at: {}. Please run scripts/download_embedding_model.sh",
                tokenizer_path.display()
            ));
        }

        // Load ONNX model with tract-onnx
        let model = tract_onnx::onnx()
            .model_for_path(&model_path)
            .map_err(|e| format!("Failed to load ONNX model: {}", e))?;

        // into_optimized → into_runnable (nhanh nhất)
        let runnable = match model.into_optimized() {
            Ok(optimized) => match optimized.into_runnable() {
                Ok(r) => r,
                Err(e) => {
                    eprintln!(
                        "[Embedding] into_runnable after optimization failed: {}. Trying into_typed...",
                        e
                    );
                    // Fallback: into_typed → into_runnable
                    let m = tract_onnx::onnx()
                        .model_for_path(&model_path)
                        .map_err(|e| format!("Failed to reload model: {}", e))?;
                    m.into_typed()
                        .map_err(|e| format!("into_typed failed: {}", e))?
                        .into_runnable()
                        .map_err(|e| format!("into_runnable failed: {}", e))?
                }
            },
            Err(e) => {
                eprintln!(
                    "[Embedding] into_optimized failed: {}. Trying into_typed...",
                    e
                );
                let m = tract_onnx::onnx()
                    .model_for_path(&model_path)
                    .map_err(|e| format!("Failed to reload model: {}", e))?;
                m.into_typed()
                    .map_err(|e| format!("into_typed failed: {}", e))?
                    .into_runnable()
                    .map_err(|e| format!("into_runnable failed: {}", e))?
            }
        };

        // Load tokenizer (HuggingFace tokenizers crate)
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| format!("Failed to load tokenizer: {}", e))?;

        eprintln!(
            "[Embedding] Model loaded successfully ({})",
            model_path.display()
        );

        Ok(Self {
            model: runnable,
            tokenizer,
            max_length: 512,
        })
    }

    /// Embed một đoạn text thành vector 384 chiều
    ///
    /// Luồng xử lý:
    /// 1. Thêm query prefix cho BGE model
    /// 2. Tokenize (WordPiece) → input_ids, attention_mask, token_type_ids
    /// 3. Pad/truncate về max_length (512)
    /// 4. ONNX inference → last_hidden_state [1, seq_len, 384]
    /// 5. Mean pooling (weighted by attention_mask) → [1, 384]
    /// 6. L2-normalize → unit vector
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        // Step 1: Add query prefix
        let input = format!("{}{}", QUERY_PREFIX, text);

        // Step 2: Tokenize
        let encoding = self
            .tokenizer
            .encode(input, true)
            .map_err(|e| format!("Tokenization failed: {}", e))?;

        let input_ids = encoding.get_ids();
        let attention_mask = encoding.get_attention_mask();
        let token_type_ids = encoding.get_type_ids();

        let seq_len = input_ids.len().min(self.max_length);

        // Step 3: Pad/truncate
        let mut padded_input_ids = vec![0i64; self.max_length];
        let mut padded_attention_mask = vec![0i64; self.max_length];
        let mut padded_token_type_ids = vec![0i64; self.max_length];

        for i in 0..seq_len {
            padded_input_ids[i] = input_ids[i] as i64;
            padded_attention_mask[i] = attention_mask[i] as i64;
            padded_token_type_ids[i] = token_type_ids[i] as i64;
        }

        // Step 4: Create tensors
        let input_ids_t = Tensor::from_shape(&[1, self.max_length], &padded_input_ids)
            .map_err(|e| e.to_string())?;
        let attention_mask_t = Tensor::from_shape(&[1, self.max_length], &padded_attention_mask)
            .map_err(|e| e.to_string())?;
        let token_type_ids_t = Tensor::from_shape(&[1, self.max_length], &padded_token_type_ids)
            .map_err(|e| e.to_string())?;

        // Step 5: Run inference
        let result = self
            .model
            .run(tvec!(
                input_ids_t.into(),
                attention_mask_t.into(),
                token_type_ids_t.into()
            ))
            .map_err(|e| e.to_string())?;

        // Step 6: Extract embedding from output
        let output = result.get(0).ok_or("No output from model")?;
        let output_view = output
            .to_plain_array_view::<f32>()
            .map_err(|e| e.to_string())?;

        let shape = output_view.shape().to_vec();
        let data = output_view.as_slice().ok_or("Failed to get output slice")?;

        let embedding = if shape.len() == 3 && shape[0] == 1 && shape[2] == 384 {
            // [1, seq_len, 384] → Mean pooling
            let seq_len = shape[1];
            let mut pooled = vec![0.0f32; 384];
            let mut mask_sum = 0.0f32;

            for i in 0..seq_len {
                if padded_attention_mask[i] > 0 {
                    mask_sum += 1.0;
                    for j in 0..384 {
                        pooled[j] += data[i * 384 + j];
                    }
                }
            }

            if mask_sum > 0.0 {
                for j in 0..384 {
                    pooled[j] /= mask_sum;
                }
            }
            pooled
        } else if shape.len() == 2 && shape[0] == 1 {
            // [1, embedding_dim] or [1, seq_len] → đã pooled hoặc logits
            // Lấy toàn bộ output vector
            data.to_vec()
        } else {
            return Err(format!(
                "Unexpected output shape from model: {:?}. Expected [1, seq_len, 384] or [1, 384]",
                shape
            ));
        };

        // Step 7: L2-normalize → unit vector
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized: Vec<f32> = if norm > 1e-8 {
            embedding.iter().map(|x| x / norm).collect()
        } else {
            embedding
        };

        Ok(normalized)
    }

    /// Embed một danh sách text (batch) — dùng khi index
    /// Hiện tại xử lý tuần tự, sau này có thể batch inference
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// Trả về số chiều của embedding vector
    pub fn dimension(&self) -> usize {
        384
    }

    /// Trả về max sequence length
    pub fn max_length(&self) -> usize {
        self.max_length
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_test_model_dir() -> PathBuf {
        // Khi chạy test, model nằm trong app_data của project
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // core_engine/ → lên 1 level → alouette_studio/
        dir.pop();
        dir.join("tauri_app/app_data/model_embedding/bge-small-en-v1.5")
    }

    #[test]
    fn test_model_loads() {
        let model_dir = get_test_model_dir();
        if !model_dir.exists() {
            eprintln!("Skipping test: model not found at {:?}", model_dir);
            eprintln!("Run: bash scripts/download_embedding_model.sh");
            return;
        }

        let model = EmbeddingModel::load(&model_dir).expect("Model should load");
        assert_eq!(model.dimension(), 384);
        assert_eq!(model.max_length(), 512);
    }

    #[test]
    fn test_embed_returns_384_dims() {
        let model_dir = get_test_model_dir();
        if !model_dir.exists() {
            eprintln!("Skipping test: model not found");
            return;
        }

        let model = EmbeddingModel::load(&model_dir).expect("Model should load");
        let embedding = model
            .embed("fn calculate_sum(a: i32, b: i32) -> i32")
            .expect("Embedding should succeed");

        assert_eq!(embedding.len(), 384);
    }

    #[test]
    fn test_embed_is_normalized() {
        let model_dir = get_test_model_dir();
        if !model_dir.exists() {
            eprintln!("Skipping test: model not found");
            return;
        }

        let model = EmbeddingModel::load(&model_dir).expect("Model should load");
        let embedding = model
            .embed("hello world")
            .expect("Embedding should succeed");

        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-4,
            "Expected unit vector (norm≈1.0), got norm={}",
            norm
        );
    }

    #[test]
    fn test_similar_code_has_higher_score() {
        let model_dir = get_test_model_dir();
        if !model_dir.exists() {
            eprintln!("Skipping test: model not found");
            return;
        }

        let model = EmbeddingModel::load(&model_dir).expect("Model should load");

        // Code function signatures
        let calc1 = model
            .embed("def calculate_sum(a, b): return a + b")
            .unwrap();
        let calc2 = model.embed("fn add(x: i32, y: i32) -> i32").unwrap();
        let unrelated = model.embed("def send_http_request(url):").unwrap();

        // Cosine similarity
        let sim_similar: f32 = calc1.iter().zip(calc2.iter()).map(|(a, b)| a * b).sum();
        let sim_unrelated: f32 = calc1.iter().zip(unrelated.iter()).map(|(a, b)| a * b).sum();

        assert!(
            sim_similar > sim_unrelated,
            "Similar code should have higher similarity (similar={:.4}, unrelated={:.4})",
            sim_similar,
            sim_unrelated
        );
    }

    #[test]
    fn test_deterministic() {
        let model_dir = get_test_model_dir();
        if !model_dir.exists() {
            eprintln!("Skipping test: model not found");
            return;
        }

        let model = EmbeddingModel::load(&model_dir).expect("Model should load");

        let a = model.embed("fn hello()").unwrap();
        let b = model.embed("fn hello()").unwrap();

        assert_eq!(a, b, "Same input should produce same embedding");
    }
}
