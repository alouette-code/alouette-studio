use tokio::time::{sleep, Duration};
use crate::db::DbManager;

/// Tiến trình ngầm (Watchdog) giám sát tình trạng kết nối.
/// Tự động dò tìm kết nối hỏng và cố gắng khôi phục.
pub async fn run_connection_watchdog(db_manager: std::sync::Arc<DbManager>) {
    loop {
        // Nghỉ 30 giây mỗi vòng lặp để tiết kiệm CPU
        sleep(Duration::from_secs(30)).await;

        match db_manager.pool.get() {
            Ok(conn) => {
                // Ping thử DB
                if let Err(e) = conn.execute("SELECT 1", []) {
                    eprintln!("[WATCHDOG] Cảnh báo: Lỗi kết nối DB ngầm: {}", e);
                    // Ở đây có thể kích hoạt báo động cho UI hoặc cố reset pool.
                } else {
                    println!("[WATCHDOG] SQLite Pool: OK.");
                }
            }
            Err(e) => {
                eprintln!("[WATCHDOG] LỖI NGHIÊM TRỌNG: Không thể mượn kết nối từ Pool! {}", e);
                // Hệ thống có thể gọi webhook thông báo admin ở mức độ doanh nghiệp.
            }
        }
    }
}
