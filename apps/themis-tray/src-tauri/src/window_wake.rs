//! Bring the overlay capture window to the front and keep it on top.

use crate::macos_window::apply_overlay_topmost;
use crate::mini_mode::mini_mode_active;
use tauri::WebviewWindow;

pub fn wake_overlay_window(window: &WebviewWindow) -> Result<(), String> {
    if window.is_minimized().map_err(|e| e.to_string())? {
        window.unminimize().map_err(|e| e.to_string())?;
    }
    window.show().map_err(|e| e.to_string())?;
    apply_overlay_topmost(window, mini_mode_active())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}
