use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use crate::error::{Result, CoreError};

/// Quản lý Giao dịch (Unit of Work). 
/// Gom nhóm nhiều Repository call vào chung 1 giao dịch an toàn.
pub struct TransactionManager {
    conn: PooledConnection<SqliteConnectionManager>,
}

impl TransactionManager {
    pub fn new(conn: PooledConnection<SqliteConnectionManager>) -> Self {
        Self { conn }
    }

    /// Khởi chạy một giao dịch đồng bộ. 
    /// Nếu closure trả về Err, toàn bộ thay đổi sẽ bị vứt bỏ (Rollback).
    pub fn execute_transaction<F, R>(&mut self, f: F) -> Result<R>
    where
        F: FnOnce(&rusqlite::Transaction) -> Result<R>,
    {
        // Tạo transaction block
        let tx = self.conn.transaction().map_err(|e| CoreError::Internal(format!("Failed to begin tx: {}", e)))?;
        
        // Thực thi business logic
        let result = f(&tx)?;
        
        // Commit nếu không có lỗi nào
        tx.commit().map_err(|e| CoreError::Internal(format!("Failed to commit tx: {}", e)))?;
        
        Ok(result)
    }
}
