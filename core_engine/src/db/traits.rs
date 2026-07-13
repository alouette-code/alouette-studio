use crate::config::ProjectConfig;
use crate::error::Result;

/// Cung cấp giao diện trừu tượng (Dependency Inversion) cho Project Persistence.
/// Ẩn đi hoàn toàn việc lưu trữ bằng SQLite hay bộ nhớ.
pub trait IProjectRepository: Send + Sync {
    fn save_project(&self, config: &ProjectConfig) -> Result<()>;
    fn delete_project(&self, project_id: &str) -> Result<()>;
    // Các hàm khác sẽ được khai báo sau
}
