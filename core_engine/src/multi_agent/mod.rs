pub mod types;
pub mod errors;
pub mod blackboard;
pub mod worker;
pub mod orchestration;

// Kiến trúc Multi-Agent Core V2 (Deep Structure)
// Hoạt động hoàn toàn độc lập, tái sử dụng các cơ chế chạy Tool, Sandbox, và Telemetry
// từ hệ thống lõi nhưng dùng hệ thống giao tiếp, quản lý State hoàn toàn mới.
