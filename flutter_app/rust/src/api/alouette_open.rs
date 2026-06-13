use std::sync::atomic::{AtomicBool, Ordering};

static ALOUETTE_OPEN_ENABLED: AtomicBool = AtomicBool::new(true);

/// Toggle Alouette Open error monitoring
pub fn toggle_alouette_open(enabled: bool) {
    ALOUETTE_OPEN_ENABLED.store(enabled, Ordering::Relaxed);
}

/// Check if Alouette Open is active
pub fn is_alouette_open_active() -> bool {
    ALOUETTE_OPEN_ENABLED.load(Ordering::Relaxed)
}
