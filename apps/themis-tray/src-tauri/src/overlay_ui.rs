use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tracing::warn;

use crate::macos_window::apply_overlay_topmost;
use crate::mini_mode::mini_mode_active;

pub const THEMES: &[&str] = &[
    // Glass (translucent)
    "dark-glass",
    "light-glass",
    // Solid opaque
    "solid-dark",
    "solid-light",
    "midnight",
    "slate",
    "paper",
    "cream",
    // High contrast & minimal
    "high-contrast-dark",
    "high-contrast-light",
    "outline",
];

const LIGHT_THEMES: &[&str] = &[
    "light-glass",
    "solid-light",
    "paper",
    "cream",
    "high-contrast-light",
];

pub fn is_light_theme(theme: &str) -> bool {
    LIGHT_THEMES.contains(&theme)
}

/// Pick a readable panel for the desktop luminance behind the overlay.
pub fn adaptive_effective_theme(preferred: &str, background_lum: f32) -> String {
    let want_dark_panel = background_lum >= 128.0;
    let preferred_is_light = is_light_theme(preferred);

    if want_dark_panel {
        if preferred_is_light {
            light_to_dark_counterpart(preferred)
        } else {
            preferred.to_string()
        }
    } else if !preferred_is_light {
        dark_to_light_counterpart(preferred)
    } else {
        preferred.to_string()
    }
}

fn light_to_dark_counterpart(theme: &str) -> String {
    match theme {
        "solid-light" | "paper" | "cream" => "solid-dark".into(),
        "high-contrast-light" => "high-contrast-dark".into(),
        _ => "dark-glass".into(),
    }
}

fn dark_to_light_counterpart(theme: &str) -> String {
    match theme {
        "solid-dark" | "midnight" | "slate" => "solid-light".into(),
        "high-contrast-dark" => "high-contrast-light".into(),
        "outline" => "light-glass".into(),
        _ => "light-glass".into(),
    }
}

const OPACITY_MIN: f64 = 0.35;
const OPACITY_MAX: f64 = 1.0;
pub const OPACITY_STEP: f64 = 0.05;

const FONT_SCALE_MIN: f64 = 0.75;
const FONT_SCALE_MAX: f64 = 1.5;
pub const FONT_SCALE_STEP: f64 = 0.1;

fn default_font_scale() -> f64 {
    1.0
}

fn default_always_on_top() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayUiSettings {
    pub opacity: f64,
    pub theme: String,
    /// Sample screen behind overlay and pick dark/light panel automatically.
    pub adaptive: bool,
    /// Multiplier for overlay content text (1.0 = default).
    #[serde(default = "default_font_scale")]
    pub font_scale: f64,
    /// When true, overlay stays above other windows; when false, normal stacking.
    #[serde(default = "default_always_on_top")]
    pub always_on_top: bool,
}

impl Default for OverlayUiSettings {
    fn default() -> Self {
        Self {
            opacity: 0.92,
            theme: THEMES[0].to_string(),
            adaptive: false,
            font_scale: 1.0,
            always_on_top: true,
        }
    }
}

pub struct OverlayUiState {
    settings: Mutex<OverlayUiSettings>,
    path: Mutex<Option<PathBuf>>,
}

impl OverlayUiState {
    pub fn new() -> Self {
        Self {
            settings: Mutex::new(OverlayUiSettings::default()),
            path: Mutex::new(None),
        }
    }

    pub fn init_path(&self, path: PathBuf) {
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(s) = serde_json::from_str::<OverlayUiSettings>(&data) {
                    *self.settings.lock().unwrap() = s;
                }
            }
        }
        *self.path.lock().unwrap() = Some(path);
    }

    fn save(&self) {
        let path = self.path.lock().unwrap().clone();
        let settings = self.settings.lock().unwrap().clone();
        if let Some(path) = path {
            if let Ok(json) = serde_json::to_string_pretty(&settings) {
                let _ = std::fs::write(path, json);
            }
        }
    }

    pub fn get(&self) -> OverlayUiSettings {
        self.settings.lock().unwrap().clone()
    }

    pub fn set(&self, settings: OverlayUiSettings) -> OverlayUiSettings {
        let mut s = self.settings.lock().unwrap();
        s.opacity = s.opacity.clamp(OPACITY_MIN, OPACITY_MAX);
        s.font_scale = s.font_scale.clamp(FONT_SCALE_MIN, FONT_SCALE_MAX);
        if !THEMES.contains(&s.theme.as_str()) {
            s.theme = THEMES[0].to_string();
        }
        *s = settings;
        let out = s.clone();
        drop(s);
        self.save();
        out
    }

    pub fn adjust_opacity(&self, delta: f64) -> OverlayUiSettings {
        let mut s = self.get();
        s.opacity = (s.opacity + delta).clamp(OPACITY_MIN, OPACITY_MAX);
        self.set(s)
    }

    pub fn adjust_font_scale(&self, delta: f64) -> OverlayUiSettings {
        let mut s = self.get();
        s.font_scale = (s.font_scale + delta).clamp(FONT_SCALE_MIN, FONT_SCALE_MAX);
        self.set(s)
    }

    pub fn reset_font_scale(&self) -> OverlayUiSettings {
        let mut s = self.get();
        s.font_scale = 1.0;
        self.set(s)
    }

    pub fn cycle_theme(&self) -> OverlayUiSettings {
        let mut s = self.get();
        let idx = THEMES
            .iter()
            .position(|t| *t == s.theme)
            .unwrap_or(0);
        s.theme = THEMES[(idx + 1) % THEMES.len()].to_string();
        self.set(s)
    }

    pub fn toggle_adaptive(&self) -> OverlayUiSettings {
        let mut s = self.get();
        s.adaptive = !s.adaptive;
        self.set(s)
    }

    pub fn toggle_always_on_top(&self) -> OverlayUiSettings {
        let mut s = self.get();
        s.always_on_top = !s.always_on_top;
        self.set(s)
    }
}

pub fn overlay_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("overlay")
}

pub fn apply_overlay_ui(app: &AppHandle, settings: &OverlayUiSettings) -> Result<(), String> {
    let window = overlay_window(app).ok_or_else(|| "overlay window missing".to_string())?;
    apply_overlay_topmost(&window, mini_mode_active(), settings.always_on_top)?;

    let effective_theme = if settings.adaptive {
        adaptive_theme_for_window(&window)
            .map(|lum| adaptive_effective_theme(&settings.theme, lum))
            .unwrap_or_else(|| settings.theme.clone())
    } else {
        settings.theme.clone()
    };

    let payload = OverlayUiPayload {
        opacity: settings.opacity,
        theme: settings.theme.clone(),
        effective_theme,
        adaptive: settings.adaptive,
        font_scale: settings.font_scale,
        always_on_top: settings.always_on_top,
    };
    app.emit("overlay-ui", payload)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Clone, Serialize)]
pub struct OverlayUiPayload {
    pub opacity: f64,
    pub theme: String,
    pub effective_theme: String,
    pub adaptive: bool,
    pub font_scale: f64,
    pub always_on_top: bool,
}

pub fn adaptive_theme_for_window(window: &WebviewWindow) -> Option<f32> {
    #[cfg(windows)]
    {
        let hwnd = window.hwnd().ok()?;
        return sample_background_luminance(hwnd.0 as isize);
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        None
    }
}

#[cfg(windows)]
fn sample_background_luminance(hwnd: isize) -> Option<f32> {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{GetDC, GetPixel, ReleaseDC};
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    unsafe {
        let hwnd = HWND(hwnd as _);
        let mut rect = RECT::default();
        GetWindowRect(hwnd, &mut rect).ok()?;
        let hdc = GetDC(None);
        if hdc.is_invalid() {
            return None;
        }
        let w = (rect.right - rect.left).max(1);
        let h = (rect.bottom - rect.top).max(1);
        let mut sum = 0u64;
        let mut n = 0u64;
        for row in 0..3 {
            for col in 0..3 {
                let x = rect.left + (w * (col + 1)) / 4;
                let y = rect.top + (h * (row + 1)) / 4;
                let c = GetPixel(hdc, x, y).0;
                if c == 0xFFFF_FFFF {
                    continue;
                }
                let r = (c & 0xFF) as u64;
                let g = ((c >> 8) & 0xFF) as u64;
                let b = ((c >> 16) & 0xFF) as u64;
                sum += (r * 299 + g * 587 + b * 114) / 1000;
                n += 1;
            }
        }
        let _ = ReleaseDC(None, hdc);
        if n == 0 {
            return None;
        }
        Some(sum as f32 / n as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_picks_dark_panel_on_bright_background() {
        assert_eq!(
            adaptive_effective_theme("solid-light", 200.0),
            "solid-dark"
        );
        assert_eq!(adaptive_effective_theme("dark-glass", 200.0), "dark-glass");
    }

    #[test]
    fn adaptive_picks_light_panel_on_dark_background() {
        assert_eq!(
            adaptive_effective_theme("solid-dark", 50.0),
            "solid-light"
        );
        assert_eq!(
            adaptive_effective_theme("midnight", 50.0),
            "solid-light"
        );
    }
}

pub fn spawn_adaptive_poll(app: AppHandle, ui: std::sync::Arc<OverlayUiState>) {
    tauri::async_runtime::spawn(async move {
        let mut last_theme = String::new();
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            let settings = ui.get();
            if !settings.adaptive {
                last_theme.clear();
                continue;
            }
            if let Err(e) = apply_overlay_ui(&app, &settings) {
                warn!(error = %e, "adaptive overlay ui apply failed");
                continue;
            }
            if let Some(w) = overlay_window(&app) {
                if let Some(lum) = adaptive_theme_for_window(&w) {
                    let theme = adaptive_effective_theme(&settings.theme, lum);
                    if theme != last_theme {
                        last_theme = theme.clone();
                        let payload = OverlayUiPayload {
                            opacity: settings.opacity,
                            theme: settings.theme.clone(),
                            effective_theme: theme,
                            adaptive: true,
                            font_scale: settings.font_scale,
                            always_on_top: settings.always_on_top,
                        };
                        let _ = app.emit("overlay-ui", payload);
                    }
                }
            }
        }
    });
}
