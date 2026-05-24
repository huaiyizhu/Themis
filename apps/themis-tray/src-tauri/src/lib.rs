use serde::Serialize;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State,
};
use themis_core::ThemisConfig;
use themis_ipc::client::connect;
use themis_ipc::{
    GetDiagnosticsRequest, GetStatusRequest, StartCaptureRequest, StopCaptureRequest,
    SubscribeTranscriptsRequest,
};
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone, Serialize)]
struct StatusDto {
    state: String,
    message: String,
    audio_peak: u32,
    audio_frames: u64,
    capture_mode: String,
    audio_sessions: u32,
}

#[derive(Clone, Serialize)]
struct TranscriptPayload {
    text: String,
    is_final: bool,
    feedback: Option<String>,
    timestamp_unix_ms: i64,
    latency: Option<LatencyBreakdownDto>,
}

#[derive(Clone, Serialize)]
struct LatencyBreakdownDto {
    buffer_ms: u32,
    azure_ms: u32,
    stt_wall_ms: u32,
    estimated_e2e_ms: u32,
    language: String,
}

#[derive(Clone, Serialize)]
struct LatencyRecordDto {
    id: u64,
    text: String,
    is_final: bool,
    emitted_unix_ms: i64,
    received_unix_ms: Option<i64>,
    breakdown: Option<LatencyBreakdownDto>,
}

#[derive(Clone, Serialize)]
struct LatencySummaryDto {
    count: u32,
    avg_azure_ms: u32,
    avg_e2e_ms: u32,
    max_e2e_ms: u32,
    last_azure_ms: u32,
}

#[derive(Clone, Serialize)]
struct DiagnosticsDto {
    overlay_display: String,
    partial: String,
    committed_line_count: usize,
    last_ui_latency_ms: Option<u32>,
    service_online: bool,
    summary: LatencySummaryDto,
    records: Vec<LatencyRecordDto>,
}

#[derive(Default)]
struct OverlayMirror {
    committed: Vec<String>,
    partial: String,
    last_ui_latency_ms: Option<u32>,
}

#[derive(Clone)]
struct AppState {
    config: ThemisConfig,
    capturing: Arc<Mutex<bool>>,
    stream_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    overlay: Arc<Mutex<OverlayMirror>>,
}

#[tauri::command]
async fn get_status(state: State<'_, AppState>) -> Result<StatusDto, String> {
    let mut client = connect(state.config.grpc_port)
        .await
        .map_err(|e| e.to_string())?;
    let resp = client
        .get_status(GetStatusRequest {})
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    Ok(StatusDto {
        state: resp.state,
        message: resp.message,
        audio_peak: resp.audio_peak,
        audio_frames: resp.audio_frames,
        capture_mode: resp.capture_mode,
        audio_sessions: resp.audio_sessions,
    })
}

#[tauri::command]
async fn get_diagnostics(state: State<'_, AppState>) -> Result<DiagnosticsDto, String> {
    let overlay = state.overlay.lock().await;
    let overlay_display = if overlay.committed.is_empty() && overlay.partial.is_empty() {
        String::new()
    } else {
        let mut lines = overlay.committed.clone();
        if !overlay.partial.is_empty() {
            lines.push(format!("{} …", overlay.partial));
        }
        lines.join("\n")
    };
    let partial = overlay.partial.clone();
    let committed_line_count = overlay.committed.len();
    let last_ui_latency_ms = overlay.last_ui_latency_ms;
    drop(overlay);

    let mut client = connect(state.config.grpc_port)
        .await
        .map_err(|e| e.to_string())?;
    let resp = client
        .get_diagnostics(GetDiagnosticsRequest {})
        .await
        .map_err(|e| e.to_string())?
        .into_inner();

    let summary = resp.summary.unwrap_or(themis_ipc::LatencySummary {
        count: 0,
        avg_azure_ms: 0,
        avg_e2e_ms: 0,
        max_e2e_ms: 0,
        last_azure_ms: 0,
    });

    let records = resp
        .records
        .into_iter()
        .map(|r| {
            let breakdown = r.breakdown.map(|b| LatencyBreakdownDto {
                buffer_ms: b.buffer_ms,
                azure_ms: b.azure_ms,
                stt_wall_ms: b.stt_wall_ms,
                estimated_e2e_ms: b.estimated_e2e_ms,
                language: b.language,
            });
            LatencyRecordDto {
                id: r.id,
                text: r.text,
                is_final: r.is_final,
                emitted_unix_ms: r.emitted_unix_ms,
                received_unix_ms: None,
                breakdown,
            }
        })
        .collect();

    Ok(DiagnosticsDto {
        overlay_display,
        partial,
        committed_line_count,
        last_ui_latency_ms,
        service_online: true,
        summary: LatencySummaryDto {
            count: summary.count,
            avg_azure_ms: summary.avg_azure_ms,
            avg_e2e_ms: summary.avg_e2e_ms,
            max_e2e_ms: summary.max_e2e_ms,
            last_azure_ms: summary.last_azure_ms,
        },
        records,
    })
}

#[tauri::command]
fn toggle_diagnose_window(app: AppHandle) -> Result<bool, String> {
    let w = app
        .get_webview_window("diagnose")
        .ok_or_else(|| "diagnose window not found".to_string())?;
    let visible = w.is_visible().map_err(|e| e.to_string())?;
    if visible {
        w.hide().map_err(|e| e.to_string())?;
        Ok(false)
    } else {
        w.show().map_err(|e| e.to_string())?;
        w.set_focus().map_err(|e| e.to_string())?;
        Ok(true)
    }
}

#[tauri::command]
async fn toggle_capture(app: AppHandle) -> Result<bool, String> {
    let state = app.state::<AppState>();
    let capturing = *state.capturing.lock().await;
    let mut client = connect(state.config.grpc_port)
        .await
        .map_err(|e| e.to_string())?;

    if capturing {
        client
            .stop_capture(StopCaptureRequest {})
            .await
            .map_err(|e| e.to_string())?;
        *state.capturing.lock().await = false;
        stop_transcript_stream(&state).await;
        let _ = app.emit("capture-stopped", ());
        Ok(false)
    } else {
        *state.capturing.lock().await = true;
        {
            let mut o = state.overlay.lock().await;
            *o = OverlayMirror::default();
        }
        // Subscribe before StartCapture so we do not miss early transcript events.
        start_transcript_stream(app.clone(), state.inner().clone()).await;
        client
            .start_capture(StartCaptureRequest {})
            .await
            .map_err(|e| e.to_string())?;
        let _ = app.emit("capture-started", ());
        Ok(true)
    }
}

fn is_system_transcript(text: &str) -> bool {
    text.starts_with("Azure ")
        || text.contains("connected…")
        || text.contains("transcribing every")
        || text.contains("picking best match")
}

async fn update_overlay_mirror(state: &AppState, text: &str, is_final: bool) {
    if is_system_transcript(text) {
        return;
    }
    let mut o = state.overlay.lock().await;
    if is_final {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            o.committed.push(trimmed.to_string());
            o.partial.clear();
        }
    } else {
        o.partial = text.trim().to_string();
    }
}

async fn start_transcript_stream(app: AppHandle, state: AppState) {
    let port = state.config.grpc_port;
    let loop_state = state.clone();
    let handle = tokio::spawn(async move {
        loop {
            match connect(port).await {
                Ok(mut client) => {
                    if let Ok(mut stream) = client
                        .subscribe_transcripts(SubscribeTranscriptsRequest {})
                        .await
                        .map(|r| r.into_inner())
                    {
                        use tokio_stream::StreamExt;
                        while let Some(Ok(msg)) = stream.next().await {
                            let received = chrono::Utc::now().timestamp_millis();
                            let ui_ms = (received - msg.timestamp_unix_ms).max(0) as u32;
                            if msg.is_final && ui_ms < 120_000 {
                                loop_state.overlay.lock().await.last_ui_latency_ms = Some(ui_ms);
                            }

                            update_overlay_mirror(&loop_state, &msg.text, msg.is_final).await;

                            let latency = msg.latency.map(|b| LatencyBreakdownDto {
                                buffer_ms: b.buffer_ms,
                                azure_ms: b.azure_ms,
                                stt_wall_ms: b.stt_wall_ms,
                                estimated_e2e_ms: b.estimated_e2e_ms,
                                language: b.language,
                            });

                            let _ = app.emit(
                                "transcript",
                                TranscriptPayload {
                                    text: msg.text,
                                    is_final: msg.is_final,
                                    feedback: if msg.feedback.is_empty() {
                                        None
                                    } else {
                                        Some(msg.feedback)
                                    },
                                    timestamp_unix_ms: msg.timestamp_unix_ms,
                                    latency,
                                },
                            );
                        }
                    }
                }
                Err(e) => tracing::warn!(error = %e, "grpc connect failed"),
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    });
    *state.stream_task.lock().await = Some(handle);
}

async fn stop_transcript_stream(state: &AppState) {
    if let Some(h) = state.stream_task.lock().await.take() {
        h.abort();
    }
}

fn spawn_service_if_needed(config: &ThemisConfig) {
    let port = config.grpc_port;
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        if rt.block_on(connect(port)).is_ok() {
            return;
        }
        if let Some(path) = find_service_binary() {
            info!(path = %path.display(), "spawning themis-service");
            let _ = std::process::Command::new(path).spawn();
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });
}

fn find_service_binary() -> Option<std::path::PathBuf> {
    let name = if cfg!(windows) {
        "themis-service.exe"
    } else {
        "themis-service"
    };

    let mut candidates = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(name));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for sub in ["target/debug", "target/release", "../../target/debug", "../../target/release"]
        {
            candidates.push(cwd.join(sub).join(name));
        }
    }

    candidates.into_iter().find(|p| p.exists())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = ThemisConfig::from_env();
    spawn_service_if_needed(&config);

    let hotkey_toggle = if cfg!(target_os = "macos") {
        "Command+Shift+KeyT"
    } else {
        "Ctrl+Shift+KeyT"
    };
    let hotkey_diagnose = if cfg!(target_os = "macos") {
        "Command+Shift+KeyD"
    } else {
        "Ctrl+Shift+KeyD"
    };

    let sc_toggle: tauri_plugin_global_shortcut::Shortcut = hotkey_toggle
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_toggle}: {e}"));
    let sc_diagnose: tauri_plugin_global_shortcut::Shortcut = hotkey_diagnose
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_diagnose}: {e}"));

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([hotkey_toggle, hotkey_diagnose])
                .unwrap_or_else(|e| panic!("invalid hotkeys: {e}"))
                .with_handler({
                    move |app, shortcut, event| {
                        use tauri_plugin_global_shortcut::ShortcutState;
                        if event.state != ShortcutState::Pressed {
                            return;
                        }
                        if shortcut == &sc_toggle {
                            let app = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = toggle_capture(app).await;
                            });
                        } else if shortcut == &sc_diagnose {
                            let _ = toggle_diagnose_window(app.clone());
                        }
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let show_i = MenuItem::with_id(app, "show", "Show overlay", true, None::<&str>)?;
            let toggle_i = MenuItem::with_id(app, "toggle", "Toggle capture", true, None::<&str>)?;
            let diag_i = MenuItem::with_id(
                app,
                "diagnose",
                "Diagnostics (Ctrl+Shift+D)",
                true,
                None::<&str>,
            )?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &toggle_i, &diag_i, &quit_i])?;

            let icon = app
                .default_window_icon()
                .cloned()
                .expect("default window icon");

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("overlay") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "toggle" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = toggle_capture(app).await;
                        });
                    }
                    "diagnose" => {
                        let _ = toggle_diagnose_window(app.clone());
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("overlay") {
                            let visible = w.is_visible().unwrap_or(false);
                            if visible {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .manage(AppState {
            config: config.clone(),
            capturing: Arc::new(Mutex::new(false)),
            stream_task: Arc::new(Mutex::new(None)),
            overlay: Arc::new(Mutex::new(OverlayMirror::default())),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            toggle_capture,
            get_diagnostics,
            toggle_diagnose_window
        ])
        .run(tauri::generate_context!())
        .expect("error running Themis tray");
}
