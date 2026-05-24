use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tracing::warn;

pub const THEMES: &[&str] = &[
    "dark-glass",
    "light-glass",
    "high-contrast-dark",
    "high-contrast-light",
    "outline",
];

const OPACITY_MIN: f64 = 0.35;
const OPACITY_MAX: f64 = 1.0;
pub const OPACITY_STEP: f64 = 0.05;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayUiSettings {
    pub opacity: f64,
    pub theme: String,
    /// Sample screen behind overlay and pick dark/light panel automatically.
    pub adaptive: bool,
}

impl Default for OverlayUiSettings {
    fn default() -> Self {
        Self {
            opacity: 0.92,
            theme: THEMES[0].to_string(),
            adaptive: false,
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
}

pub fn overlay_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("overlay")
}

pub fn apply_overlay_ui(app: &AppHandle, settings: &OverlayUiSettings) -> Result<(), String> {
    let window = overlay_window(app).ok_or_else(|| "overlay window missing".to_string())?;
    let _ = window.set_always_on_top(true);

    let effective_theme = if settings.adaptive {
        adaptive_theme_for_window(&window).unwrap_or_else(|| settings.theme.clone())
    } else {
        settings.theme.clone()
    };

    let payload = OverlayUiPayload {
        opacity: settings.opacity,
        theme: settings.theme.clone(),
        effective_theme,
        adaptive: settings.adaptive,
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
}

pub fn adaptive_theme_for_window(window: &WebviewWindow) -> Option<String> {
    #[cfg(windows)]
    {
        let hwnd = window.hwnd().ok()?;
        let lum = sample_background_luminance(hwnd.0 as isize)?;
        // Bright desktop → dark panel; dark desktop → light panel.
        return Some(if lum >= 128.0 {
            "dark-glass".into()
        } else {
            "light-glass".into()
        });
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
                if let Some(theme) = adaptive_theme_for_window(&w) {
                    if theme != last_theme {
                        last_theme = theme.clone();
                        let payload = OverlayUiPayload {
                            opacity: settings.opacity,
                            theme: settings.theme.clone(),
                            effective_theme: theme,
                            adaptive: true,
                        };
                        let _ = app.emit("overlay-ui", payload);
                    }
                }
            }
        }
    });
}
