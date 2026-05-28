use serde::Serialize;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewWindow};

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

const CENTER_QUARTER_ID: &str = "center-quarter";

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
        id: CENTER_QUARTER_ID.into(),
        label: "居中¼屏".into(),
        width: 0,
        height: 0,
        fullscreen: false,
    });
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

fn monitor_for_window(window: &WebviewWindow) -> Result<tauri::Monitor, String> {
    if let Some(m) = window
        .current_monitor()
        .map_err(|e| e.to_string())?
    {
        return Ok(m);
    }
    window
        .primary_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no monitor detected".to_string())
}

fn apply_center_quarter(window: &WebviewWindow) -> Result<(), String> {
    window.set_fullscreen(false).map_err(|e| e.to_string())?;
    window
        .set_max_size(None::<LogicalSize<f32>>)
        .map_err(|e| e.to_string())?;

    let monitor = monitor_for_window(window)?;
    let scale = monitor.scale_factor();
    let work = monitor.work_area();

    let area_w = work.size.width as f64 / scale;
    let area_h = work.size.height as f64 / scale;
    let area_x = work.position.x as f64 / scale;
    let area_y = work.position.y as f64 / scale;

    let width = (area_w / 4.0).max(280.0);
    let height = area_h;
    let x = area_x + (area_w - width) / 2.0;
    let y = area_y;

    window
        .set_min_size(Some(LogicalSize::new(280.0, 160.0)))
        .map_err(|e| e.to_string())?;
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    window
        .set_position(LogicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn apply_preset(app: &AppHandle, preset_id: &str) -> Result<String, String> {
    let window = overlay_window(app)?;

    if preset_id == "fullscreen" {
        window
            .set_fullscreen(true)
            .map_err(|e| e.to_string())?;
        return Ok("fullscreen".into());
    }

    if preset_id == CENTER_QUARTER_ID {
        apply_center_quarter(&window)?;
        return Ok(CENTER_QUARTER_ID.into());
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
        .set_min_size(Some(LogicalSize::new(400.0, 160.0)))
        .map_err(|e| e.to_string())?;
    window
        .set_size(LogicalSize::new(
            preset.width as f32,
            preset.height as f32,
        ))
        .map_err(|e| e.to_string())?;

    Ok(preset.id.to_string())
}
