// Simple debug logger for Reticulum

use std::sync::atomic::{AtomicBool, Ordering};

static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn enable_debug() {
    DEBUG_ENABLED.store(true, Ordering::SeqCst);
    debug_log("Debug logging enabled");
}

#[allow(dead_code)]
pub fn disable_debug() {
    debug_log("Debug logging disabled");
    DEBUG_ENABLED.store(false, Ordering::SeqCst);
}

pub fn is_debug_enabled() -> bool {
    DEBUG_ENABLED.load(Ordering::SeqCst)
}

pub fn debug_log(message: &str) {
    if is_debug_enabled() {
        println!("[DEBUG] {}", message);
    }
}