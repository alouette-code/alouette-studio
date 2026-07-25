use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Bộ nhớ dùng chung (Shared Context) cho các Agent, thay vì truyền toàn bộ lịch sử trò chuyện.
#[derive(Debug, Default, Clone)]
pub struct MemoryStore {
    data: Arc<RwLock<HashMap<String, String>>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Lấy giá trị theo key
    pub async fn get(&self, key: &str) -> Option<String> {
        let lock = self.data.read().await;
        lock.get(key).cloned()
    }

    /// Ghi giá trị (Cập nhật hoặc thêm mới)
    pub async fn set(&self, key: &str, value: String) {
        let mut lock = self.data.write().await;
        lock.insert(key.to_string(), value);
    }

    /// Xóa một khóa
    pub async fn remove(&self, key: &str) {
        let mut lock = self.data.write().await;
        lock.remove(key);
    }
    
    /// Trả về tất cả các khóa (Keys)
    pub async fn keys(&self) -> Vec<String> {
        let lock = self.data.read().await;
        lock.keys().cloned().collect()
    }
}
