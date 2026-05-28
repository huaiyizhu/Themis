mod mini_mode;
mod overlay_ui;
mod window_presets;

use mini_mode::{is_mini_mode, toggle_mini_mode, MiniModeState};
use overlay_ui::{apply_overlay_ui, spawn_adaptive_poll, OverlayUiSettings, OverlayUiState};
use window_presets::{apply_preset, list_presets, WindowPresetDto};
use serde::Serialize;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State,
};
use themis_core::{AnalysisPrefs, ThemisConfig};
use themis_ipc::client::connect;
use themis_ipc::{
    ExpandInsightRequest, GetDiagnosticsRequest, GetStatusRequest, ResetSessionRequest,
    StartCaptureRequest, StopCaptureRequest, SubscribeTranscriptsRequest,
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
    capture_detail: String,
}

#[derive(Clone, Serialize, serde::Deserialize, Default)]
struct InsightsDto {
    keywords: Vec<String>,
    terms: Vec<TermInsightDto>,
    questions: Vec<QuestionInsightDto>,
}

#[derive(Clone, Serialize, serde::Deserialize)]
struct TermInsightDto {
    term: String,
    explanation: String,
}

#[derive(Clone, Serialize, serde::Deserialize)]
struct QuestionInsightDto {
    question: String,
    answer: String,
}

#[derive(Clone, Serialize)]
struct TranscriptPayload {
    text: String,
    is_final: bool,
    feedback: Option<String>,
    insights: Option<InsightsDto>,
    session_summary: Option<String>,
    timestamp_unix_ms: i64,
    latency: Option<LatencyBreakdownDto>,
}

fn parse_insights_json(json: &str) -> Option<InsightsDto> {
    if json.trim().is_empty() {
        return None;
    }
    serde_json::from_str(json).ok()
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

#[derive(Clone, Serialize, serde::Deserialize, Default)]
struct AnalysisSummaryDto {
    count: u32,
    llm_configured: bool,
    last_llm_status: String,
}

#[derive(Clone, Serialize, serde::Deserialize, Default)]
struct AnalysisInsightRecordDto {
    id: u64,
    text: String,
    emitted_unix_ms: i64,
    heuristic: InsightsDto,
    llm: Option<InsightsDto>,
    merged: InsightsDto,
    llm_configured: bool,
    llm_status: String,
    heuristic_ms: u32,
    llm_ms: Option<u32>,
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
    analysis_summary: AnalysisSummaryDto,
    analysis_records: Vec<AnalysisInsightRecordDto>,
}

#[derive(Default)]
struct OverlayMirror {
    committed: Vec<String>,
    partial: String,
    last_ui_latency_ms: Option<u32>,
}

#[derive(Clone, Serialize)]
struct InsightSettingsDto {
    insight_dwell_ms: u32,
    localize_zh: bool,
}

#[derive(Clone)]
struct AppState {
    config: ThemisConfig,
    capturing: Arc<Mutex<bool>>,
    stream_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    overlay: Arc<Mutex<OverlayMirror>>,
    mini_mode: Arc<std::sync::Mutex<MiniModeState>>,
}

#[tauri::command]
fn get_overlay_ui(ui: State<'_, Arc<OverlayUiState>>) -> OverlayUiSettings {
    ui.get()
}

#[tauri::command]
fn adjust_overlay_opacity(
    app: AppHandle,
    ui: State<'_, Arc<OverlayUiState>>,
    delta: f64,
) -> Result<OverlayUiSettings, String> {
    let settings = ui.adjust_opacity(delta);
    apply_overlay_ui(&app, &settings)?;
    Ok(settings)
}

#[tauri::command]
fn cycle_overlay_theme(
    app: AppHandle,
    ui: State<'_, Arc<OverlayUiState>>,
) -> Result<OverlayUiSettings, String> {
    let settings = ui.cycle_theme();
    apply_overlay_ui(&app, &settings)?;
    Ok(settings)
}

#[tauri::command]
fn toggle_overlay_adaptive(
    app: AppHandle,
    ui: State<'_, Arc<OverlayUiState>>,
) -> Result<OverlayUiSettings, String> {
    let settings = ui.toggle_adaptive();
    apply_overlay_ui(&app, &settings)?;
    Ok(settings)
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn get_insight_settings(state: State<'_, AppState>) -> InsightSettingsDto {
    let prefs = AnalysisPrefs::load();
    InsightSettingsDto {
        insight_dwell_ms: state.config.insight_dwell_secs.saturating_mul(1000),
        localize_zh: prefs.localize_zh,
    }
}

#[tauri::command]
fn set_insight_localize(localize_zh: bool) -> Result<InsightSettingsDto, String> {
    let mut prefs = AnalysisPrefs::load();
    prefs.localize_zh = localize_zh;
    prefs.save().map_err(|e| e.to_string())?;
    Ok(InsightSettingsDto {
        insight_dwell_ms: 0,
        localize_zh: prefs.localize_zh,
    })
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
        capture_detail: resp.capture_detail,
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

    let a_sum = resp.analysis_summary.unwrap_or(themis_ipc::AnalysisDiagnosticsSummary {
        count: 0,
        llm_configured: false,
        last_llm_status: String::new(),
    });

    let analysis_records = resp
        .analysis_records
        .into_iter()
        .map(|r| AnalysisInsightRecordDto {
            id: r.id,
            text: r.text,
            emitted_unix_ms: r.emitted_unix_ms,
            heuristic: parse_insights_json(&r.heuristic_json).unwrap_or_default(),
            llm: if r.llm_json.trim().is_empty() {
                None
            } else {
                parse_insights_json(&r.llm_json)
            },
            merged: parse_insights_json(&r.merged_json).unwrap_or_default(),
            llm_configured: r.llm_configured,
            llm_status: r.llm_status,
            heuristic_ms: r.heuristic_ms,
            llm_ms: if r.llm_ms > 0 { Some(r.llm_ms) } else { None },
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
        analysis_summary: AnalysisSummaryDto {
            count: a_sum.count,
            llm_configured: a_sum.llm_configured,
            last_llm_status: a_sum.last_llm_status,
        },
        analysis_records,
    })
}

fn set_overlay_visible(app: &AppHandle, visible: bool) -> Result<bool, String> {
    let w = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window missing".to_string())?;
    if visible {
        w.show().map_err(|e| e.to_string())?;
        w.set_focus().map_err(|e| e.to_string())?;
    } else {
        w.hide().map_err(|e| e.to_string())?;
    }
    let _ = app.emit("overlay-visibility", visible);
    Ok(visible)
}

#[tauri::command]
fn toggle_overlay_visibility(app: AppHandle) -> Result<bool, String> {
    let w = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window missing".to_string())?;
    let visible = w.is_visible().map_err(|e| e.to_string())?;
    set_overlay_visible(&app, !visible)
}

#[tauri::command]
fn is_overlay_visible(app: AppHandle) -> Result<bool, String> {
    let w = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window missing".to_string())?;
    w.is_visible().map_err(|e| e.to_string())
}

#[tauri::command]
fn is_diagnose_visible(app: AppHandle) -> Result<bool, String> {
    let w = app
        .get_webview_window("diagnose")
        .ok_or_else(|| "diagnose window not found".to_string())?;
    w.is_visible().map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_diagnose_window(app: AppHandle) -> Result<bool, String> {
    let w = app
        .get_webview_window("diagnose")
        .ok_or_else(|| "diagnose window not found".to_string())?;
    let visible = w.is_visible().map_err(|e| e.to_string())?;
    if visible {
        w.hide().map_err(|e| e.to_string())?;
        let _ = app.emit("diagnose-visibility", false);
        Ok(false)
    } else {
        w.show().map_err(|e| e.to_string())?;
        w.set_focus().map_err(|e| e.to_string())?;
        let _ = app.emit("diagnose-visibility", true);
        Ok(true)
    }
}

#[tauri::command]
fn list_window_presets() -> Vec<WindowPresetDto> {
    list_presets()
}

#[tauri::command]
fn apply_window_preset(app: AppHandle, preset: String) -> Result<String, String> {
    let state = app.state::<AppState>();
    if is_mini_mode(&state.mini_mode) {
        return Err("exit mini floater mode before changing window size".into());
    }
    apply_preset(&app, preset.trim())
}

#[tauri::command]
fn toggle_overlay_mini_mode(app: AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    toggle_mini_mode(&app, &state.mini_mode)
}

#[tauri::command]
fn is_overlay_mini_mode(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(is_mini_mode(&state.mini_mode))
}

#[tauri::command]
async fn expand_insight(
    state: State<'_, AppState>,
    kind: String,
    subject: String,
    brief: String,
) -> Result<String, String> {
    let mut client = connect(state.config.grpc_port)
        .await
        .map_err(|e| e.to_string())?;
    let resp = client
        .expand_insight(ExpandInsightRequest {
            kind,
            subject,
            brief,
        })
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    if resp.ok {
        Ok(resp.detail)
    } else {
        Err(if resp.message.is_empty() {
            "expand insight failed".into()
        } else {
            resp.message
        })
    }
}

#[tauri::command]
async fn clear_listening_session(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut client = connect(state.config.grpc_port)
        .await
        .map_err(|e| e.to_string())?;
    let resp = client
        .reset_session(ResetSessionRequest {})
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    if !resp.ok {
        return Err(resp.message);
    }
    {
        let mut o = state.overlay.lock().await;
        *o = OverlayMirror::default();
    }
    let _ = app.emit("session-cleared", ());
    Ok(())
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

                            let insights = parse_insights_json(&msg.insights_json);
                            let session_summary = if msg.session_summary.is_empty() {
                                None
                            } else {
                                Some(msg.session_summary.clone())
                            };
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
                                    insights,
                                    session_summary,
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
    let hotkey_quit = if cfg!(target_os = "macos") {
        "Command+Shift+KeyQ"
    } else {
        "Ctrl+Shift+KeyQ"
    };
    let hotkey_opacity_down = if cfg!(target_os = "macos") {
        "Command+Shift+BracketLeft"
    } else {
        "Ctrl+Shift+BracketLeft"
    };
    let hotkey_opacity_up = if cfg!(target_os = "macos") {
        "Command+Shift+BracketRight"
    } else {
        "Ctrl+Shift+BracketRight"
    };
    let hotkey_style = if cfg!(target_os = "macos") {
        "Command+Shift+KeyS"
    } else {
        "Ctrl+Shift+KeyS"
    };
    let hotkey_adaptive = if cfg!(target_os = "macos") {
        "Command+Shift+KeyA"
    } else {
        "Ctrl+Shift+KeyA"
    };
    let hotkey_font_down = if cfg!(target_os = "macos") {
        "Command+Shift+Minus"
    } else {
        "Ctrl+Shift+Minus"
    };
    let hotkey_font_up = if cfg!(target_os = "macos") {
        "Command+Shift+Equal"
    } else {
        "Ctrl+Shift+Equal"
    };
    let hotkey_font_reset = if cfg!(target_os = "macos") {
        "Command+Shift+Digit0"
    } else {
        "Ctrl+Shift+Digit0"
    };
    let hotkey_overlay_visibility = if cfg!(target_os = "macos") {
        "Command+Shift+KeyH"
    } else {
        "Ctrl+Shift+KeyH"
    };

    let sc_toggle: tauri_plugin_global_shortcut::Shortcut = hotkey_toggle
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_toggle}: {e}"));
    let sc_diagnose: tauri_plugin_global_shortcut::Shortcut = hotkey_diagnose
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_diagnose}: {e}"));
    let sc_quit: tauri_plugin_global_shortcut::Shortcut = hotkey_quit
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_quit}: {e}"));
    let sc_opacity_down: tauri_plugin_global_shortcut::Shortcut = hotkey_opacity_down
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_opacity_down}: {e}"));
    let sc_opacity_up: tauri_plugin_global_shortcut::Shortcut = hotkey_opacity_up
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_opacity_up}: {e}"));
    let sc_style: tauri_plugin_global_shortcut::Shortcut = hotkey_style
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_style}: {e}"));
    let sc_adaptive: tauri_plugin_global_shortcut::Shortcut = hotkey_adaptive
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_adaptive}: {e}"));
    let sc_font_down: tauri_plugin_global_shortcut::Shortcut = hotkey_font_down
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_font_down}: {e}"));
    let sc_font_up: tauri_plugin_global_shortcut::Shortcut = hotkey_font_up
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_font_up}: {e}"));
    let sc_font_reset: tauri_plugin_global_shortcut::Shortcut = hotkey_font_reset
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_font_reset}: {e}"));
    let sc_overlay_visibility: tauri_plugin_global_shortcut::Shortcut = hotkey_overlay_visibility
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_overlay_visibility}: {e}"));

    let ui_state = Arc::new(OverlayUiState::new());

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([
                    hotkey_toggle,
                    hotkey_diagnose,
                    hotkey_quit,
                    hotkey_opacity_down,
                    hotkey_opacity_up,
                    hotkey_style,
                    hotkey_adaptive,
                    hotkey_font_down,
                    hotkey_font_up,
                    hotkey_font_reset,
                    hotkey_overlay_visibility,
                ])
                .unwrap_or_else(|e| panic!("invalid hotkeys: {e}"))
                .with_handler({
                    let ui_state = Arc::clone(&ui_state);
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
                        } else if shortcut == &sc_quit {
                            app.exit(0);
                        } else if shortcut == &sc_opacity_down {
                            let s = ui_state.adjust_opacity(-overlay_ui::OPACITY_STEP);
                            let _ = apply_overlay_ui(app, &s);
                        } else if shortcut == &sc_opacity_up {
                            let s = ui_state.adjust_opacity(overlay_ui::OPACITY_STEP);
                            let _ = apply_overlay_ui(app, &s);
                        } else if shortcut == &sc_style {
                            let s = ui_state.cycle_theme();
                            let _ = apply_overlay_ui(app, &s);
                        } else if shortcut == &sc_adaptive {
                            let s = ui_state.toggle_adaptive();
                            let _ = apply_overlay_ui(app, &s);
                        } else if shortcut == &sc_font_down {
                            let s = ui_state.adjust_font_scale(-overlay_ui::FONT_SCALE_STEP);
                            let _ = apply_overlay_ui(app, &s);
                        } else if shortcut == &sc_font_up {
                            let s = ui_state.adjust_font_scale(overlay_ui::FONT_SCALE_STEP);
                            let _ = apply_overlay_ui(app, &s);
                        } else if shortcut == &sc_font_reset {
                            let s = ui_state.reset_font_scale();
                            let _ = apply_overlay_ui(app, &s);
                        } else if shortcut == &sc_overlay_visibility {
                            let _ = app.emit("toggle-transcript-panel", ());
                        }
                    }
                })
                .build(),
        )
        .setup({
            let ui_state = Arc::clone(&ui_state);
            move |app| {
            if let Ok(dir) = app.path().app_data_dir() {
                let _ = std::fs::create_dir_all(&dir);
                ui_state.init_path(dir.join("overlay-ui.json"));
            }
            let settings = ui_state.get();
            let _ = apply_overlay_ui(&app.handle(), &settings);
            spawn_adaptive_poll(app.handle().clone(), Arc::clone(&ui_state));
            let show_i = MenuItem::with_id(app, "show", "Show overlay", true, None::<&str>)?;
            let hide_i = MenuItem::with_id(
                app,
                "hide_overlay",
                "Hide overlay window",
                true,
                None::<&str>,
            )?;
            let toggle_i = MenuItem::with_id(app, "toggle", "Toggle capture", true, None::<&str>)?;
            let diag_i = MenuItem::with_id(
                app,
                "diagnose",
                "Diagnostics (Ctrl+Shift+D)",
                true,
                None::<&str>,
            )?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit (Ctrl+Shift+Q)", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_i, &hide_i, &toggle_i, &diag_i, &quit_i])?;

            if let Some(w) = app.get_webview_window("overlay") {
                let _ = w.set_always_on_top(true);
                let _ = w.set_shadow(false);
                let _ = w.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
            }

            let icon = app
                .default_window_icon()
                .cloned()
                .expect("default window icon");

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        let _ = set_overlay_visible(app, true);
                    }
                    "hide_overlay" => {
                        let _ = set_overlay_visible(app, false);
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
                        let _ = toggle_overlay_visibility(app.clone());
                    }
                })
                .build(app)?;

            Ok(())
        }
        })
        .manage(ui_state)
        .manage(AppState {
            config: config.clone(),
            capturing: Arc::new(Mutex::new(false)),
            stream_task: Arc::new(Mutex::new(None)),
            overlay: Arc::new(Mutex::new(OverlayMirror::default())),
            mini_mode: Arc::new(std::sync::Mutex::new(MiniModeState::default())),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            toggle_capture,
            get_diagnostics,
            toggle_diagnose_window,
            is_diagnose_visible,
            get_overlay_ui,
            get_insight_settings,
            set_insight_localize,
            adjust_overlay_opacity,
            cycle_overlay_theme,
            toggle_overlay_adaptive,
            clear_listening_session,
            list_window_presets,
            apply_window_preset,
            toggle_overlay_mini_mode,
            is_overlay_mini_mode,
            expand_insight,
            toggle_overlay_visibility,
            is_overlay_visible,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error running Themis tray");
}
