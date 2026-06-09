#!/usr/bin/env bash
set -euo pipefail

# Script tải embedding model cho Code RAG
# Model: Xenova/bge-small-en-v1.5 (ONNX format, ~33MB)
# - BGE-small-en-v1.5 from BAAI, converted to ONNX by Xenova
# - 384-dim embeddings, lightweight CPU inference
#
# Usage: bash download_embedding_model.sh
# Output: tauri_app/app_data/model_embedding/bge-small-en-v1.5/

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MODEL_DIR="$PROJECT_ROOT/tauri_app/app_data/model_embedding/bge-small-en-v1.5"

echo "[download_embedding_model] Target: $MODEL_DIR"
mkdir -p "$MODEL_DIR"

# Base URL trên HuggingFace Hub (raw file download)
BASE_URL="https://huggingface.co/Xenova/bge-small-en-v1.5/resolve/main"

# Files cần tải
FILES=(
  "onnx/model.onnx"
  "tokenizer.json"
)

for file in "${FILES[@]}"; do
  url="$BASE_URL/$file"
  # Tạo tên file local (bỏ prefix "onnx/")
  local_name="${file#onnx/}"
  output_path="$MODEL_DIR/$local_name"

  if [ -f "$output_path" ] && [ -s "$output_path" ]; then
    echo "[download_embedding_model]   EXISTS: $output_path ($(du -h "$output_path" | cut -f1))"
  else
    echo "[download_embedding_model]   Downloading: $url"
    if command -v curl &>/dev/null; then
      curl -fsSL "$url" -o "$output_path"
    elif command -v wget &>/dev/null; then
      wget -q "$url" -O "$output_path"
    else
      echo "[download_embedding_model] ERROR: Need curl or wget to download."
      exit 1
    fi
    echo "[download_embedding_model]   Saved: $output_path ($(du -h "$output_path" | cut -f1))"
  fi
done

echo ""
echo "[download_embedding_model] ✅ Complete!"
echo "    Model files at: $MODEL_DIR"
ls -lh "$MODEL_DIR"
