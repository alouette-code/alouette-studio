//! # Tree-sitter Parser Hub
//!
//! Quản lý Object Pool cho Tree-sitter grammar.
//! Giữ tối đa 5 grammar trong RAM (LRU cache).
//! Grammar được load Lazy từ disk chỉ khi cần lần đầu.
//!
//! ## Object Pool
//! - Tối đa 5 grammar trong RAM
//! - Khi mở ngôn ngữ thứ 6 → giải phóng grammar ít dùng nhất
//! - Grammar được load lazy từ disk
//!
//! ## Incremental Parsing
//! Chỉ parse lại phần AST bị thay đổi (dirty nodes)
//! khi file được lưu → giảm thời gian parse từ vài trăm ms xuống <5ms.

use lru::LruCache;
use std::num::NonZeroUsize;

/// Grammar object pool (LRU cache)
///
/// Khi thêm grammar thứ N+1, grammar ít dùng nhất sẽ bị evict.
pub struct ParserPool {
    grammar_cache: LruCache<String, ()>, // TODO: thay () = Parser thực
    max_grammars: usize,
}

impl ParserPool {
    pub fn new(max_grammars: usize) -> Self {
        let capacity = NonZeroUsize::new(max_grammars.max(1)).unwrap();
        Self {
            grammar_cache: LruCache::new(capacity),
            max_grammars,
        }
    }

    /// Lấy parser cho một ngôn ngữ (load lazy từ disk nếu chưa có)
    pub fn get_parser(&mut self, lang_id: &str) -> Option<()> {
        // 1. Kiểm tra cache → nếu có, touch để cập nhật LRU
        if self.grammar_cache.contains(lang_id) {
            // Touch để cập nhật LRU order
            let _ = self.grammar_cache.get(lang_id);
            return Some(());
        }

        // 2. Cache miss → load grammar từ disk
        // TODO: Load tree-sitter grammar thực tế
        // self.load_grammar(lang_id)?;

        // 3. Thêm vào cache (nếu cache đầy, LRU sẽ tự động evict)
        self.grammar_cache.put(lang_id.to_string(), ());

        Some(())
    }

    /// Số grammar hiện tại trong cache
    pub fn cached_count(&self) -> usize {
        self.grammar_cache.len()
    }

    /// Tổng dung lượng tối đa
    pub fn max_capacity(&self) -> usize {
        self.max_grammars
    }
}

/// Parse Tree-sitter query từ file .scm
///
/// File .scm được lưu trong `code_rag/queries/{lang_id}/` và
/// được nạp lazy khi cần.
pub fn load_query(lang_id: &str, query_name: &str) -> Option<String> {
    let path = format!(
        "{}/src/code_rag/queries/{}/{}.scm",
        env!("CARGO_MANIFEST_DIR"),
        lang_id,
        query_name
    );
    std::fs::read_to_string(&path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let mut pool = ParserPool::new(5);
        assert_eq!(pool.max_grammars, 5);
        assert_eq!(pool.cached_count(), 0);
    }

    #[test]
    fn test_lru_eviction() {
        let mut pool = ParserPool::new(3);

        // Thêm 3 grammar
        assert!(pool.get_parser("python").is_some());
        assert!(pool.get_parser("rust").is_some());
        assert!(pool.get_parser("go").is_some());
        assert_eq!(pool.cached_count(), 3);

        // Touch "python" để nó là recently used
        assert!(pool.get_parser("python").is_some());

        // Thêm grammar thứ 4 → "rust" hoặc "go" sẽ bị evict
        assert!(pool.get_parser("java").is_some());
        assert_eq!(pool.cached_count(), 3);
    }

    #[test]
    fn test_load_query() {
        let query = load_query("python", "functions");
        assert!(query.is_some(), "Should load python/functions.scm");
        assert!(query.unwrap().contains("function_definition"));
    }

    #[test]
    fn test_load_nonexistent_query() {
        let query = load_query("nonexistent", "functions");
        assert!(query.is_none());
    }
}
