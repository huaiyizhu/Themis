use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewWindow};

const MINI_LOGICAL: f32 = 52.0;
const RESTORE_MIN_W: f32 = 400.0;
const RESTORE_MIN_H: f32 = 160.0;

#[derive(Clone, Default)]
struct SavedGeometry {
    width: f32,
    height: f32,
    x: f32,
    y: f32,
    fullscreen: bool,
}

pub struct MiniModeState {
    active: bool,
    saved: Option<SavedGeometry>,
}

impl Default for MiniModeState {
    fn default() -> Self {
        Self {
            active: false,
            saved: None,
        }
    }
}

fn overlay_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("overlay")
        .ok_or_else(|| "overlay window missing".to_string())
}

fn read_geometry(window: &WebviewWindow) -> Result<SavedGeometry, String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())? as f32;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let fullscreen = window.is_fullscreen().map_err(|e| e.to_string())?;
    Ok(SavedGeometry {
        width: size.width as f32 / scale,
        height: size.height as f32 / scale,
        x: pos.x as f32 / scale,
        y: pos.y as f32 / scale,
        fullscreen,
    })
}

fn enter_mini(window: &WebviewWindow) -> Result<(), String> {
    window.set_fullscreen(false).map_err(|e| e.to_string())?;
    let mini = LogicalSize::new(MINI_LOGICAL, MINI_LOGICAL);
    window
        .set_min_size(Some(mini))
        .map_err(|e| e.to_string())?;
    window
        .set_max_size(Some(mini))
        .map_err(|e| e.to_string())?;
    window.set_resizable(false).map_err(|e| e.to_string())?;
    window.set_size(mini).map_err(|e| e.to_string())?;
    Ok(())
}

fn exit_mini(window: &WebviewWindow, saved: &SavedGeometry) -> Result<(), String> {
    window.set_resizable(true).map_err(|e| e.to_string())?;
    window
        .set_max_size(None::<LogicalSize<f32>>)
        .map_err(|e| e.to_string())?;
    window
        .set_min_size(Some(LogicalSize::new(RESTORE_MIN_W, RESTORE_MIN_H)))
        .map_err(|e| e.to_string())?;
    if saved.fullscreen {
        window.set_fullscreen(true).map_err(|e| e.to_string())?;
    } else {
        window
            .set_size(LogicalSize::new(saved.width, saved.height))
            .map_err(|e| e.to_string())?;
        window
            .set_position(LogicalPosition::new(saved.x, saved.y))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn toggle_mini_mode(app: &AppHandle, state: &Arc<Mutex<MiniModeState>>) -> Result<bool, String> {
    let window = overlay_window(app)?;
    let mut guard = state
        .lock()
        .map_err(|_| "mini mode lock poisoned".to_string())?;

    if guard.active {
        let saved = guard
            .saved
            .clone()
            .ok_or_else(|| "missing saved window geometry".to_string())?;
        exit_mini(&window, &saved)?;
        guard.active = false;
        guard.saved = None;
        let _ = app.emit("mini-mode-changed", false);
        Ok(false)
    } else {
        let saved = read_geometry(&window)?;
        enter_mini(&window)?;
        guard.active = true;
        guard.saved = Some(saved);
        let _ = app.emit("mini-mode-changed", true);
        Ok(true)
    }
}

pub fn is_mini_mode(state: &Arc<Mutex<MiniModeState>>) -> bool {
    state.lock().map(|g| g.active).unwrap_or(false)
}
