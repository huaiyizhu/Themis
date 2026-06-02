use crate::macos_window::{apply_overlay_topmost, apply_overlay_transparency};
use crate::overlay_ui::OverlayUiState;
#[cfg(target_os = "macos")]
use crate::macos_mini_panel::{hide_mini_panel, refresh_mini_panel, show_mini_panel};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, WebviewWindow};

const RESTORE_MIN_W: f32 = 400.0;
const RESTORE_MIN_H: f32 = 160.0;

static MINI_MODE_ACTIVE: AtomicBool = AtomicBool::new(false);

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
    #[cfg(target_os = "macos")]
    stop_elevate: Option<Arc<AtomicBool>>,
}

impl Default for MiniModeState {
    fn default() -> Self {
        Self {
            active: false,
            saved: None,
            #[cfg(target_os = "macos")]
            stop_elevate: None,
        }
    }
}

pub fn mini_mode_active() -> bool {
    MINI_MODE_ACTIVE.load(Ordering::Relaxed)
}

fn overlay_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("overlay")
        .ok_or_else(|| "overlay window missing".to_string())
}

#[cfg(all(not(target_os = "macos"), not(windows)))]
fn mini_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("mini")
        .ok_or_else(|| "mini window missing".to_string())
}

const MINI_FLOATER_SIZE: f32 = 52.0;

fn mini_floater_position(overlay: &WebviewWindow) -> Result<(f32, f32), String> {
    let scale = overlay.scale_factor().map_err(|e| e.to_string())? as f32;
    let pos = overlay.outer_position().map_err(|e| e.to_string())?;
    let size = overlay.outer_size().map_err(|e| e.to_string())?;
    let w = size.width as f32 / scale;
    let h = size.height as f32 / scale;
    let x = pos.x as f32 / scale + (w - MINI_FLOATER_SIZE) / 2.0;
    let y = pos.y as f32 / scale + (h - MINI_FLOATER_SIZE) / 2.0;
    Ok((x.max(0.0), y.max(0.0)))
}

fn enable_overlay_mini_ui(overlay: &WebviewWindow) -> Result<(), String> {
    overlay
        .eval(
            "document.body.classList.add('is-mini-mode');\
             document.documentElement.classList.add('is-mini-mode');\
             const f=document.getElementById('mini-floater');\
             if(f)f.classList.remove('hidden');",
        )
        .map_err(|e| e.to_string())
}

fn disable_overlay_mini_ui(overlay: &WebviewWindow) -> Result<(), String> {
    overlay
        .eval(
            "document.body.classList.remove('is-mini-mode');\
             document.documentElement.classList.remove('is-mini-mode');\
             const f=document.getElementById('mini-floater');\
             if(f)f.classList.add('hidden');",
        )
        .map_err(|e| e.to_string())
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

fn floater_position(overlay: &WebviewWindow) -> Result<(f64, f64), String> {
    let (x, y) = mini_floater_position(overlay)?;
    Ok((x as f64, y as f64))
}

#[cfg(target_os = "macos")]
fn stop_elevate_task(state: &mut MiniModeState) {
    if let Some(flag) = state.stop_elevate.take() {
        flag.store(true, Ordering::Relaxed);
    }
}

#[cfg(not(target_os = "macos"))]
fn stop_elevate_task(_state: &mut MiniModeState) {}

#[cfg(target_os = "macos")]
fn start_elevate_task(app: &AppHandle, state: &mut MiniModeState) {
    stop_elevate_task(state);
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&stop);
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(200));
        while !stop_flag.load(Ordering::Relaxed) {
            interval.tick().await;
            if mini_mode_active() {
                let _ = refresh_mini_panel();
            }
        }
    });
    state.stop_elevate = Some(stop);
}

#[cfg(not(target_os = "macos"))]
fn start_elevate_task(_app: &AppHandle, _state: &mut MiniModeState) {}

#[cfg(target_os = "macos")]
fn enter_mini(
    app: &AppHandle,
    overlay: &WebviewWindow,
    state: &mut MiniModeState,
) -> Result<(), String> {
    overlay.set_fullscreen(false).map_err(|e| e.to_string())?;
    let (x, y) = floater_position(overlay)?;
    overlay.hide().map_err(|e| e.to_string())?;
    show_mini_panel(x, y)?;
    MINI_MODE_ACTIVE.store(true, Ordering::Relaxed);
    start_elevate_task(app, state);
    Ok(())
}

#[cfg(windows)]
fn enter_mini(
    _app: &AppHandle,
    overlay: &WebviewWindow,
    _state: &mut MiniModeState,
) -> Result<(), String> {
    use crate::macos_window::{set_macos_mini_floater_elevated, set_mini_circular_clip};

    overlay.set_fullscreen(false).map_err(|e| e.to_string())?;
    let (x, y) = mini_floater_position(overlay)?;
    enable_overlay_mini_ui(overlay)?;
    overlay.set_resizable(false).map_err(|e| e.to_string())?;
    overlay
        .set_max_size(Some(LogicalSize::new(
            MINI_FLOATER_SIZE,
            MINI_FLOATER_SIZE,
        )))
        .map_err(|e| e.to_string())?;
    overlay
        .set_min_size(Some(LogicalSize::new(
            MINI_FLOATER_SIZE,
            MINI_FLOATER_SIZE,
        )))
        .map_err(|e| e.to_string())?;
    overlay
        .set_size(LogicalSize::new(MINI_FLOATER_SIZE, MINI_FLOATER_SIZE))
        .map_err(|e| e.to_string())?;
    overlay
        .set_position(LogicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    apply_overlay_transparency(overlay);
    set_mini_circular_clip(overlay, true);
    set_macos_mini_floater_elevated(overlay, true)?;
    overlay.show().map_err(|e| e.to_string())?;
    set_mini_circular_clip(overlay, true);
    MINI_MODE_ACTIVE.store(true, Ordering::Relaxed);
    Ok(())
}

#[cfg(all(not(target_os = "macos"), not(windows)))]
fn enter_mini(
    app: &AppHandle,
    overlay: &WebviewWindow,
    _state: &mut MiniModeState,
) -> Result<(), String> {
    use crate::macos_window::{set_macos_mini_floater_elevated, set_mini_circular_clip};
    let mini = mini_window(app)?;
    overlay.set_fullscreen(false).map_err(|e| e.to_string())?;
    let (x, y) = mini_floater_position(overlay)?;
    mini.set_position(LogicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    apply_overlay_transparency(&mini);
    set_mini_circular_clip(&mini, true);
    set_macos_mini_floater_elevated(&mini, true)?;
    overlay.hide().map_err(|e| e.to_string())?;
    mini.show().map_err(|e| e.to_string())?;
    set_mini_circular_clip(&mini, true);
    MINI_MODE_ACTIVE.store(true, Ordering::Relaxed);
    Ok(())
}

#[cfg(target_os = "macos")]
fn exit_mini(
    app: &AppHandle,
    overlay: &WebviewWindow,
    saved: &SavedGeometry,
) -> Result<(), String> {
    MINI_MODE_ACTIVE.store(false, Ordering::Relaxed);
    hide_mini_panel()?;
    restore_overlay(app, overlay, saved)
}

#[cfg(windows)]
fn exit_mini(app: &AppHandle, overlay: &WebviewWindow, saved: &SavedGeometry) -> Result<(), String> {
    use crate::macos_window::{set_macos_mini_floater_elevated, set_mini_circular_clip};

    MINI_MODE_ACTIVE.store(false, Ordering::Relaxed);
    disable_overlay_mini_ui(overlay)?;
    set_macos_mini_floater_elevated(overlay, false)?;
    set_mini_circular_clip(overlay, false);
    restore_overlay(app, overlay, saved)
}

#[cfg(all(not(target_os = "macos"), not(windows)))]
fn exit_mini(app: &AppHandle, overlay: &WebviewWindow, saved: &SavedGeometry) -> Result<(), String> {
    use crate::macos_window::{set_macos_mini_floater_elevated, set_mini_circular_clip};
    let mini = mini_window(app)?;
    MINI_MODE_ACTIVE.store(false, Ordering::Relaxed);
    set_macos_mini_floater_elevated(&mini, false)?;
    set_mini_circular_clip(&mini, false);
    mini.hide().map_err(|e| e.to_string())?;
    restore_overlay(app, overlay, saved)
}

fn restore_overlay(
    app: &AppHandle,
    overlay: &WebviewWindow,
    saved: &SavedGeometry,
) -> Result<(), String> {
    apply_overlay_transparency(overlay);
    overlay.set_resizable(true).map_err(|e| e.to_string())?;
    overlay
        .set_max_size(None::<LogicalSize<f32>>)
        .map_err(|e| e.to_string())?;
    overlay
        .set_min_size(Some(LogicalSize::new(RESTORE_MIN_W, RESTORE_MIN_H)))
        .map_err(|e| e.to_string())?;
    if saved.fullscreen {
        overlay.set_fullscreen(true).map_err(|e| e.to_string())?;
    } else {
        overlay
            .set_size(LogicalSize::new(saved.width, saved.height))
            .map_err(|e| e.to_string())?;
        overlay
            .set_position(LogicalPosition::new(saved.x, saved.y))
            .map_err(|e| e.to_string())?;
    }
    overlay.show().map_err(|e| e.to_string())?;
    let always_on_top = app.state::<Arc<OverlayUiState>>().get().always_on_top;
    apply_overlay_topmost(overlay, false, always_on_top)?;
    let _ = overlay.set_focus();
    Ok(())
}

pub fn toggle_mini_mode(app: &AppHandle, state: &Arc<Mutex<MiniModeState>>) -> Result<bool, String> {
    let overlay = overlay_window(app)?;
    let mut guard = state
        .lock()
        .map_err(|_| "mini mode lock poisoned".to_string())?;

    if guard.active {
        let saved = guard
            .saved
            .clone()
            .ok_or_else(|| "missing saved window geometry".to_string())?;
        stop_elevate_task(&mut guard);
        #[cfg(target_os = "macos")]
        exit_mini(app, &overlay, &saved)?;
        #[cfg(not(target_os = "macos"))]
        exit_mini(app, &overlay, &saved)?;
        guard.active = false;
        guard.saved = None;
        let _ = app.emit("mini-mode-changed", false);
        Ok(false)
    } else {
        let saved = read_geometry(&overlay)?;
        enter_mini(app, &overlay, &mut guard)?;
        guard.active = true;
        guard.saved = Some(saved);
        let _ = app.emit("mini-mode-changed", true);
        Ok(true)
    }
}

pub fn is_mini_mode(state: &Arc<Mutex<MiniModeState>>) -> bool {
    state.lock().map(|g| g.active).unwrap_or(false)
}

pub fn exit_mini_mode_without_restore(
    app: &AppHandle,
    state: &Arc<Mutex<MiniModeState>>,
) -> Result<(), String> {
    let mut guard = state
        .lock()
        .map_err(|_| "mini mode lock poisoned".to_string())?;
    if !guard.active {
        return Ok(());
    }
    let overlay = overlay_window(app)?;
    stop_elevate_task(&mut guard);
    MINI_MODE_ACTIVE.store(false, Ordering::Relaxed);
    #[cfg(target_os = "macos")]
    hide_mini_panel()?;
    #[cfg(all(not(target_os = "macos"), not(windows)))]
    {
        use crate::macos_window::{set_macos_mini_floater_elevated, set_mini_circular_clip};
        let mini = mini_window(app)?;
        set_macos_mini_floater_elevated(&mini, false)?;
        set_mini_circular_clip(&mini, false);
        mini.hide().map_err(|e| e.to_string())?;
    }
    #[cfg(windows)]
    {
        use crate::macos_window::{set_macos_mini_floater_elevated, set_mini_circular_clip};
        set_macos_mini_floater_elevated(&overlay, false)?;
        set_mini_circular_clip(&overlay, false);
        disable_overlay_mini_ui(&overlay)?;
    }
    apply_overlay_transparency(&overlay);
    overlay.set_resizable(true).map_err(|e| e.to_string())?;
    overlay
        .set_max_size(None::<LogicalSize<f32>>)
        .map_err(|e| e.to_string())?;
    overlay
        .set_min_size(Some(LogicalSize::new(280.0, 160.0)))
        .map_err(|e| e.to_string())?;
    guard.active = false;
    guard.saved = None;
    overlay.show().map_err(|e| e.to_string())?;
    let _ = app.emit("mini-mode-changed", false);
    Ok(())
}

pub fn refresh_mini_floater(app: &AppHandle) -> Result<(), String> {
    if !mini_mode_active() {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        return refresh_mini_panel();
    }
    #[cfg(windows)]
    {
        use crate::macos_window::set_macos_mini_floater_elevated;
        let overlay = overlay_window(app)?;
        return set_macos_mini_floater_elevated(&overlay, true);
    }
    #[cfg(all(not(target_os = "macos"), not(windows)))]
    {
        use crate::macos_window::set_macos_mini_floater_elevated;
        let mini = mini_window(app)?;
        set_macos_mini_floater_elevated(&mini, true)
    }
}
