#!/usr/bin/env sh
# Test nhanh các module Code RAG bằng Cargo test

set -e

echo "╔════════════════════════════════════════╗"
echo "║   Code RAG — Quick Test Suite         ║"
echo "╚════════════════════════════════════════╝"
echo ""

# 1. Test config & languages
echo "▶ Testing language config & tier definitions..."
cargo test --package core_engine --lib code_rag::tier -- --nocapture 2>&1 | tail -5

echo ""
echo "▶ Testing languages list (50+ languages)..."
cargo test --package core_engine --lib code_rag::languages -- --nocapture 2>&1 | tail -10

echo ""
echo "▶ Testing language resolver..."
cargo test --package core_engine --lib code_rag::language_resolver -- --nocapture 2>&1 | tail -10

echo ""
echo "▶ Testing normalizer..."
cargo test --package core_engine --lib code_rag::normalizer -- --nocapture 2>&1 | tail -10

echo ""
echo "▶ Testing extractor (Python, Rust, Fortran, filter)..."
cargo test --package core_engine --lib code_rag::extractor -- --nocapture 2>&1 | tail -10

echo ""
echo "▶ Testing vector DB..."
cargo test --package core_engine --lib code_rag::db -- --nocapture 2>&1 | tail -10

echo ""
echo "▶ Testing query engine..."
cargo test --package core_engine --lib code_rag::query -- --nocapture 2>&1 | tail -10

echo ""
echo "▶ Testing indexer..."
cargo test --package core_engine --lib code_rag::indexer -- --nocapture 2>&1 | tail -10

echo ""
echo "▶ Testing parser pool & LRU eviction..."
cargo test --package core_engine --lib code_rag::parser -- --nocapture 2>&1 | tail -10

echo ""
echo "╔════════════════════════════════════════╗"
echo "║   ALL TESTS COMPLETED                  ║"
echo "╚════════════════════════════════════════╝"
