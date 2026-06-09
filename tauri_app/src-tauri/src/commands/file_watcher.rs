use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use tauri::{AppHandle, Emitter};

/// Spawn a background file-system watcher using Rust's `notify` crate.
/// Khi phát hiện thay đổi trong workspace, emit event `file-system-changed` lên frontend.
pub fn spawn_file_watcher(app_handle: AppHandle, watch_dir: PathBuf) {
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

    // Khởi tạo watcher với channel-based event delivery
    let mut watcher = match RecommendedWatcher::new(tx, Config::default()) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[FileWatcher] Failed to create watcher: {e}");
            return;
        }
    };

    // Watch thư mục workspace (recursive)
    if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::Recursive) {
        eprintln!(
            "[FileWatcher] Failed to watch '{}': {e}",
            watch_dir.display()
        );
        return;
    }

    // Chạy event loop trên một thread riêng (notify không phải async, cần blocking thread)
    std::thread::Builder::new()
        .name("alouette-file-watcher".into())
        .spawn(move || {
            // Debounce: chỉ emit tối đa 1 event mỗi 300ms để tránh spam
            use std::time::{Duration, Instant};
            let debounce = Duration::from_millis(300);
            let mut last_emit = Instant::now();

            loop {
                match rx.recv() {
                    Ok(Ok(event)) => {
                        // Bỏ qua events không phải file modification
                        let should_emit = matches!(
                            event.kind,
                            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                        );

                        if should_emit && last_emit.elapsed() >= debounce {
                            // Chỉ emit nếu path tồn tại (không watch vô ích)
                            let paths: Vec<String> = event
                                .paths
                                .iter()
                                .filter(|p| p.exists())
                                .map(|p| p.to_string_lossy().to_string())
                                .collect();

                            if !paths.is_empty() {
                                let _ = app_handle.emit(
                                    "file-system-changed",
                                    serde_json::json!({ "paths": paths }),
                                );
                                last_emit = Instant::now();
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        eprintln!("[FileWatcher] Watch error: {e}");
                    }
                    Err(mpsc::RecvError) => {
                        eprintln!("[FileWatcher] Channel closed, watcher stopping.");
                        break;
                    }
                }
            }
        })
        .expect("[FileWatcher] Failed to spawn watcher thread");
}
