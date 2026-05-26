use serde::Serialize;
use tauri::{AppHandle, LogicalSize, Manager, WebviewWindow};

#[derive(Clone, Serialize)]
pub struct WindowPresetDto {
    pub id: String,
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
}

struct WindowPreset {
    id: &'static str,
    label: &'static str,
    width: u32,
    height: u32,
}

const PRESETS: &[WindowPreset] = &[
    WindowPreset {
        id: "compact",
        label: "小",
        width: 420,
        height: 280,
    },
    WindowPreset {
        id: "default",
        label: "默认",
        width: 520,
        height: 320,
    },
    WindowPreset {
        id: "medium",
        label: "中",
        width: 720,
        height: 480,
    },
    WindowPreset {
        id: "large",
        label: "大",
        width: 960,
        height: 640,
    },
    WindowPreset {
        id: "wide",
        label: "宽条",
        width: 1280,
        height: 320,
    },
    WindowPreset {
        id: "tall",
        label: "高",
        width: 520,
        height: 720,
    },
];

pub fn list_presets() -> Vec<WindowPresetDto> {
    let mut out: Vec<WindowPresetDto> = PRESETS
        .iter()
        .map(|p| WindowPresetDto {
            id: p.id.into(),
            label: p.label.into(),
            width: p.width,
            height: p.height,
            fullscreen: false,
        })
        .collect();
    out.push(WindowPresetDto {
        id: "fullscreen".into(),
        label: "全屏".into(),
        width: 0,
        height: 0,
        fullscreen: true,
    });
    out
}

fn overlay_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    app.get_webview_window("overlay")
        .ok_or_else(|| "overlay window missing".to_string())
}

pub fn apply_preset(app: &AppHandle, preset_id: &str) -> Result<String, String> {
    let window = overlay_window(app)?;

    if preset_id == "fullscreen" {
        window
            .set_fullscreen(true)
            .map_err(|e| e.to_string())?;
        return Ok("fullscreen".into());
    }

    let preset = PRESETS
        .iter()
        .find(|p| p.id == preset_id)
        .ok_or_else(|| format!("unknown window preset: {preset_id}"))?;

    window
        .set_fullscreen(false)
        .map_err(|e| e.to_string())?;
    window
        .set_max_size(None::<LogicalSize<f32>>)
        .map_err(|e| e.to_string())?;
    window
        .set_size(LogicalSize::new(
            preset.width as f32,
            preset.height as f32,
        ))
        .map_err(|e| e.to_string())?;

    Ok(preset.id.to_string())
}
