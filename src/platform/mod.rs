//! Platform-specific functionality

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::{get_open_file_paths, set_repaint_ctx};

#[cfg(not(target_os = "macos"))]
pub fn get_open_file_paths() -> Vec<std::path::PathBuf> {
    Vec::new()
}

/// Only macOS delivers file opens out of band, so everywhere else this is a
/// no-op — nothing outside the frame loop needs to request a repaint.
#[cfg(not(target_os = "macos"))]
pub fn set_repaint_ctx(_ctx: &eframe::egui::Context) {}

#[cfg(target_os = "windows")]
pub fn allow_set_foreground_window(process_id: u32) -> bool {
    extern "system" {
        fn AllowSetForegroundWindow(dw_process_id: u32) -> i32;
    }

    unsafe { AllowSetForegroundWindow(process_id) != 0 }
}

#[cfg(not(target_os = "windows"))]
pub fn allow_set_foreground_window(_process_id: u32) -> bool {
    false
}
