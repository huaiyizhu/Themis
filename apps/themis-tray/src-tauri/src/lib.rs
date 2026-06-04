#[cfg(target_os = "macos")]
mod macos_mini_panel;
mod macos_window;
mod mini_mode;
mod overlay_ui;
mod window_presets;
mod window_wake;
#[cfg(windows)]
mod windows_window;

use macos_window::{apply_overlay_transparency, set_mini_circular_clip};
#[cfg(target_os = "macos")]
use macos_mini_panel::{
    install_panel_app, is_mini_panel_visible, set_accessory_activation_policy,
};
use mini_mode::{
    exit_mini_mode_without_restore, is_mini_mode, refresh_mini_floater, toggle_mini_mode,
    MiniModeState,
};
use overlay_ui::{apply_overlay_ui, spawn_adaptive_poll, OverlayUiSettings, OverlayUiState};
use window_presets::{
    apply_preset, apply_wake_layout, list_presets, WindowPresetDto, WAKE_LAYOUT_PRESET_ID,
};
use serde::Serialize;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Listener, Manager, State, WebviewWindow, WindowEvent,
};
use themis_core::{
    AnalysisPrefs, ConfigStatusSnapshot, EnvSettings, ThemisConfig,
    read_env_settings, write_env_settings,
};
use themis_ipc::client::connect;
use themis_ipc::{
    ExpandInsightRequest, GetDiagnosticsRequest, GetSessionExportRequest, GetStatusRequest,
    ResetSessionRequest, StartCaptureRequest, StopCaptureRequest, SubscribeTranscriptsRequest,
};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

#[derive(Clone, Serialize)]
struct StatusDto {
    state: String,
    message: String,
    audio_peak: u32,
    audio_frames: u64,
    capture_mode: String,
    audio_sessions: u32,
    capture_detail: String,
    config: ConfigCrossCheckDto,
}

#[derive(Clone, Serialize, PartialEq, Eq)]
struct ConfigStatusDto {
    stt_configured: bool,
    stt_mode: String,
    llm_configured: bool,
    speech_region: String,
    foundry_deployment: String,
    analysis_enabled: bool,
}

#[derive(Clone, Serialize)]
struct ConfigCrossCheckDto {
    tray: ConfigStatusDto,
    service: Option<ConfigStatusDto>,
    in_sync: bool,
}

fn config_status_dto(snapshot: &ConfigStatusSnapshot) -> ConfigStatusDto {
    ConfigStatusDto {
        stt_configured: snapshot.stt_configured,
        stt_mode: snapshot.stt_mode.clone(),
        llm_configured: snapshot.llm_configured,
        speech_region: snapshot.speech_region.clone(),
        foundry_deployment: snapshot.foundry_deployment.clone(),
        analysis_enabled: snapshot.analysis_enabled,
    }
}

fn config_from_proto(proto: &themis_ipc::proto::ConfigStatus) -> ConfigStatusDto {
    ConfigStatusDto {
        stt_configured: proto.stt_configured,
        stt_mode: proto.stt_mode.clone(),
        llm_configured: proto.llm_configured,
        speech_region: proto.speech_region.clone(),
        foundry_deployment: proto.foundry_deployment.clone(),
        analysis_enabled: proto.analysis_enabled,
    }
}

fn build_config_crosscheck(
    tray: &ThemisConfig,
    service: Option<ConfigStatusDto>,
) -> ConfigCrossCheckDto {
    let tray_dto = config_status_dto(&tray.config_snapshot());
    let in_sync = service
        .as_ref()
        .is_some_and(|svc| svc == &tray_dto);
    ConfigCrossCheckDto {
        tray: tray_dto,
        service,
        in_sync,
    }
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
    config: ConfigCrossCheckDto,
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

#[derive(Clone, Serialize)]
struct EnvSettingsFileDto {
    path: String,
    exists: bool,
    settings: EnvSettings,
}

#[derive(Clone, Serialize)]
struct EnvSettingsSaveResult {
    path: String,
    config: ConfigCrossCheckDto,
    message: String,
}

#[derive(Clone)]
struct AppState {
    config: Arc<std::sync::Mutex<ThemisConfig>>,
    capturing: Arc<Mutex<bool>>,
    stream_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    overlay: Arc<Mutex<OverlayMirror>>,
    mini_mode: Arc<std::sync::Mutex<MiniModeState>>,
}

impl AppState {
    fn grpc_port(&self) -> u16 {
        self.config.lock().unwrap().grpc_port
    }

    fn config_crosscheck_with(
        &self,
        service: Option<ConfigStatusDto>,
    ) -> ConfigCrossCheckDto {
        build_config_crosscheck(&self.config.lock().unwrap(), service)
    }
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
fn adjust_overlay_font_scale(
    app: AppHandle,
    ui: State<'_, Arc<OverlayUiState>>,
    delta: f64,
) -> Result<OverlayUiSettings, String> {
    let settings = ui.adjust_font_scale(delta);
    apply_overlay_ui(&app, &settings)?;
    Ok(settings)
}

#[tauri::command]
fn reset_overlay_font_scale(
    app: AppHandle,
    ui: State<'_, Arc<OverlayUiState>>,
) -> Result<OverlayUiSettings, String> {
    let settings = ui.reset_font_scale();
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
fn toggle_overlay_always_on_top(
    app: AppHandle,
    ui: State<'_, Arc<OverlayUiState>>,
) -> Result<OverlayUiSettings, String> {
    let settings = ui.toggle_always_on_top();
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
    let cfg = state.config.lock().unwrap();
    InsightSettingsDto {
        insight_dwell_ms: cfg.insight_dwell_secs.saturating_mul(1000),
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
async fn get_config_crosscheck(state: State<'_, AppState>) -> Result<ConfigCrossCheckDto, String> {
    let port = state.grpc_port();
    let service = match connect(port).await {
        Ok(mut client) => client
            .get_status(GetStatusRequest {})
            .await
            .ok()
            .and_then(|r| r.into_inner().service_config)
            .map(|p| config_from_proto(&p)),
        Err(_) => None,
    };
    Ok(state.config_crosscheck_with(service))
}

#[tauri::command]
fn get_env_settings() -> Result<EnvSettingsFileDto, String> {
    let (path, settings) = read_env_settings()?;
    let exists = path.is_file();
    Ok(EnvSettingsFileDto {
        path: path.display().to_string(),
        exists,
        settings,
    })
}

fn stop_service_process() {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let _ = std::process::Command::new("taskkill")
            .args(["/IM", "themis-service.exe", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("pkill")
            .args(["-x", "themis-service"])
            .output();
    }
}

async fn restart_service_with_config(
    app: &AppHandle,
    state: &AppState,
    cfg: &ThemisConfig,
) -> Result<(), String> {
    let was_capturing = *state.capturing.lock().await;
    if was_capturing {
        if let Ok(mut client) = connect(state.grpc_port()).await {
            let _ = client.stop_capture(StopCaptureRequest {}).await;
        }
        *state.capturing.lock().await = false;
        stop_transcript_stream(state).await;
        let _ = app.emit("capture-stopped", ());
    }

    stop_service_process();
    sleep(Duration::from_millis(1200)).await;
    spawn_service_if_needed(cfg);

    for attempt in 0..20u32 {
        if connect(state.grpc_port()).await.is_ok() {
            break;
        }
        if attempt == 19 {
            return Err("themis-service did not come back online after reload".into());
        }
        sleep(Duration::from_millis(500)).await;
    }

    if was_capturing {
        start_capture_services(app, state).await?;
    }
    Ok(())
}

#[tauri::command]
async fn save_env_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: EnvSettings,
) -> Result<EnvSettingsSaveResult, String> {
    let (path, _) = read_env_settings()?;
    write_env_settings(&path, &settings)?;
    let new_cfg = ThemisConfig::reload_from_disk();
    *state.config.lock().unwrap() = new_cfg.clone();
    restart_service_with_config(&app, state.inner(), &new_cfg).await?;

    let service = match connect(state.grpc_port()).await {
        Ok(mut client) => client
            .get_status(GetStatusRequest {})
            .await
            .ok()
            .and_then(|r| r.into_inner().service_config)
            .map(|p| config_from_proto(&p)),
        Err(_) => None,
    };

    let _ = app.emit("env-settings-saved", ());
    Ok(EnvSettingsSaveResult {
        path: path.display().to_string(),
        config: state.config_crosscheck_with(service),
        message: "已保存到 .env 并重新加载服务".into(),
    })
}

#[tauri::command]
async fn reload_env_settings(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<EnvSettingsSaveResult, String> {
    let path = themis_core::env_file_path_or_default();
    let new_cfg = ThemisConfig::reload_from_disk();
    *state.config.lock().unwrap() = new_cfg.clone();
    restart_service_with_config(&app, state.inner(), &new_cfg).await?;
    let service = match connect(state.grpc_port()).await {
        Ok(mut client) => client
            .get_status(GetStatusRequest {})
            .await
            .ok()
            .and_then(|r| r.into_inner().service_config)
            .map(|p| config_from_proto(&p)),
        Err(_) => None,
    };
    Ok(EnvSettingsSaveResult {
        path: path.display().to_string(),
        config: state.config_crosscheck_with(service),
        message: "已从 .env 重新加载".into(),
    })
}

/// Diagnose/settings: hide on title-bar close (do not destroy), keep `is_visible` in sync with overlay buttons.
fn set_aux_window_visible(
    w: &WebviewWindow,
    app: &AppHandle,
    visibility_event: &str,
    visible: bool,
) -> Result<bool, String> {
    if visible {
        let _ = w.unminimize();
        w.show().map_err(|e| e.to_string())?;
        w.set_focus().map_err(|e| e.to_string())?;
        let _ = app.emit(visibility_event, true);
        Ok(true)
    } else {
        w.hide().map_err(|e| e.to_string())?;
        let _ = app.emit(visibility_event, false);
        Ok(false)
    }
}

fn toggle_aux_window(app: AppHandle, label: &str, visibility_event: &str) -> Result<bool, String> {
    let w = app
        .get_webview_window(label)
        .ok_or_else(|| format!("{label} window not found"))?;
    let visible = w.is_visible().map_err(|e| e.to_string())?;
    set_aux_window_visible(&w, &app, visibility_event, !visible)
}

fn handle_aux_window_close_requested(app: &AppHandle, label: &str, visibility_event: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.hide();
        let _ = app.emit(visibility_event, false);
    }
}

#[tauri::command]
fn is_settings_visible(app: AppHandle) -> Result<bool, String> {
    let w = app
        .get_webview_window("settings")
        .ok_or_else(|| "settings window not found".to_string())?;
    w.is_visible().map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_settings_window(app: AppHandle) -> Result<bool, String> {
    toggle_aux_window(app, "settings", "settings-visibility")
}

#[tauri::command]
async fn get_status(state: State<'_, AppState>) -> Result<StatusDto, String> {
    let port = state.grpc_port();
    let mut client = connect(port)
        .await
        .map_err(|e| e.to_string())?;
    let resp = client
        .get_status(GetStatusRequest {})
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    let service_config = resp
        .service_config
        .as_ref()
        .map(config_from_proto);
    Ok(StatusDto {
        state: resp.state,
        message: resp.message,
        audio_peak: resp.audio_peak,
        audio_frames: resp.audio_frames,
        capture_mode: resp.capture_mode,
        audio_sessions: resp.audio_sessions,
        capture_detail: resp.capture_detail,
        config: state.config_crosscheck_with(service_config),
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

    let mut client = connect(state.grpc_port())
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

    let service_config = resp
        .service_config
        .as_ref()
        .map(config_from_proto);

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
        config: state.config_crosscheck_with(service_config),
    })
}

fn wake_overlay(app: &AppHandle, expand_to_current_screen: bool) -> Result<(), String> {
    if expand_to_current_screen {
        let state = app.state::<AppState>();
        exit_mini_mode_without_restore(app, &state.mini_mode)?;
        apply_wake_layout(app)?;
    }

    let state = app.state::<AppState>();
    if is_mini_mode(&state.mini_mode) {
        refresh_mini_floater(app)?;
        #[cfg(target_os = "macos")]
        {
            let _ = app.emit("overlay-visibility", true);
            if expand_to_current_screen {
                let _ = app.emit("window-preset-applied", WAKE_LAYOUT_PRESET_ID);
            }
            let _ = app.emit("overlay-woken", ());
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            #[cfg(windows)]
            if let Some(overlay) = app.get_webview_window("overlay") {
                overlay.show().map_err(|e| e.to_string())?;
                let _ = overlay.set_focus();
            }
            #[cfg(all(not(target_os = "macos"), not(windows)))]
            if let Some(mini) = app.get_webview_window("mini") {
                mini.show().map_err(|e| e.to_string())?;
                let _ = mini.set_focus();
            }
        }
        let _ = app.emit("overlay-visibility", true);
        if expand_to_current_screen {
            let _ = app.emit("window-preset-applied", WAKE_LAYOUT_PRESET_ID);
        }
        let _ = app.emit("overlay-woken", ());
        return Ok(());
    }

    let w = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window missing".to_string())?;
    let always_on_top = app.state::<Arc<OverlayUiState>>().get().always_on_top;
    window_wake::wake_overlay_window(&w, always_on_top)?;
    let _ = app.emit("overlay-visibility", true);
    if expand_to_current_screen {
        let _ = app.emit("window-preset-applied", WAKE_LAYOUT_PRESET_ID);
    }
    let _ = app.emit("overlay-woken", ());
    Ok(())
}

fn set_overlay_visible(app: &AppHandle, visible: bool) -> Result<bool, String> {
    if visible {
        wake_overlay(app, false)?;
        return Ok(true);
    }
    let state = app.state::<AppState>();
    if is_mini_mode(&state.mini_mode) {
        #[cfg(target_os = "macos")]
        {
            use macos_mini_panel::hide_mini_panel;
            let _ = hide_mini_panel();
        }
        #[cfg(not(target_os = "macos"))]
        {
            #[cfg(windows)]
            {
                let w = app
                    .get_webview_window("overlay")
                    .ok_or_else(|| "overlay window missing".to_string())?;
                w.hide().map_err(|e| e.to_string())?;
            }
            #[cfg(all(not(target_os = "macos"), not(windows)))]
            if let Some(mini) = app.get_webview_window("mini") {
                mini.hide().map_err(|e| e.to_string())?;
            }
        }
    }
    let w = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window missing".to_string())?;
    w.hide().map_err(|e| e.to_string())?;
    let _ = app.emit("overlay-visibility", false);
    Ok(false)
}

#[tauri::command]
fn wake_overlay_window(app: AppHandle) -> Result<(), String> {
    wake_overlay(&app, true)
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
    let state = app.state::<AppState>();
    if is_mini_mode(&state.mini_mode) {
        #[cfg(target_os = "macos")]
        return Ok(is_mini_panel_visible());
        #[cfg(not(target_os = "macos"))]
        {
            #[cfg(windows)]
            {
                let w = app
                    .get_webview_window("overlay")
                    .ok_or_else(|| "overlay window missing".to_string())?;
                return w.is_visible().map_err(|e| e.to_string());
            }
            #[cfg(all(not(target_os = "macos"), not(windows)))]
            if let Some(mini) = app.get_webview_window("mini") {
                return mini.is_visible().map_err(|e| e.to_string());
            }
        }
    }
    let w = app
        .get_webview_window("overlay")
        .ok_or_else(|| "overlay window missing".to_string())?;
    w.is_visible().map_err(|e| e.to_string())
}

#[tauri::command]
fn hide_overlay_window(app: AppHandle) -> Result<(), String> {
    set_overlay_visible(&app, false)?;
    Ok(())
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
    toggle_aux_window(app, "diagnose", "diagnose-visibility")
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
    let mut client = connect(state.grpc_port())
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

#[derive(Clone, Serialize)]
struct TranscriptLineDto {
    text: String,
    timestamp_unix_ms: i64,
}

#[derive(Clone, Serialize)]
struct SessionExportDto {
    transcript: String,
    session_summary: String,
    line_count: u32,
    lines: Vec<TranscriptLineDto>,
}

#[tauri::command]
async fn get_session_export(state: State<'_, AppState>) -> Result<SessionExportDto, String> {
    let mut client = connect(state.grpc_port())
        .await
        .map_err(|e| e.to_string())?;
    let resp = client
        .get_session_export(GetSessionExportRequest {})
        .await
        .map_err(|e| e.to_string())?
        .into_inner();
    Ok(SessionExportDto {
        transcript: resp.transcript,
        session_summary: resp.session_summary,
        line_count: resp.line_count,
        lines: resp
            .lines
            .into_iter()
            .map(|l| TranscriptLineDto {
                text: l.text,
                timestamp_unix_ms: l.timestamp_unix_ms,
            })
            .collect(),
    })
}

#[tauri::command]
async fn save_text_file(content: String, default_name: String) -> Result<Option<String>, String> {
    let name = default_name.trim().to_string();
    if name.is_empty() {
        return Err("default file name is empty".into());
    }
    let path = tauri::async_runtime::spawn_blocking(move || {
        rfd::FileDialog::new()
            .set_file_name(&name)
            .add_filter("Text", &["txt"])
            .add_filter("Markdown", &["md"])
            .save_file()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(path) = path else {
        return Ok(None);
    };
    std::fs::write(&path, content.as_bytes()).map_err(|e| e.to_string())?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
async fn clear_listening_session(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut client = connect(state.grpc_port())
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

async fn start_capture_services(app: &AppHandle, state: &AppState) -> Result<(), String> {
    *state.capturing.lock().await = true;
    {
        let mut o = state.overlay.lock().await;
        *o = OverlayMirror::default();
    }
    start_transcript_stream(app.clone(), state.clone()).await;
    let mut client = connect(state.grpc_port())
        .await
        .map_err(|e| e.to_string())?;
    client
        .start_capture(StartCaptureRequest {})
        .await
        .map_err(|e| e.to_string())?;
    let _ = app.emit("capture-started", ());
    Ok(())
}

async fn auto_start_capture(app: AppHandle) {
    for attempt in 0..6u32 {
        sleep(Duration::from_millis(500 + u64::from(attempt) * 500)).await;
        let state = app.state::<AppState>();
        if *state.capturing.lock().await {
            return;
        }
        match start_capture_services(&app, state.inner()).await {
            Ok(()) => {
                info!("capture auto-started");
                return;
            }
            Err(e) => warn!(attempt, error = %e, "auto-start capture failed, retrying"),
        }
    }
}

#[tauri::command]
async fn toggle_capture(app: AppHandle) -> Result<bool, String> {
    let state = app.state::<AppState>();
    let capturing = *state.capturing.lock().await;
    let pending = if capturing { "stopping" } else { "starting" };
    let _ = app.emit("capture-toggle-pending", pending);

    let mut client = connect(state.grpc_port())
        .await
        .map_err(|e| {
            let msg = e.to_string();
            let _ = app.emit("capture-toggle-failed", &msg);
            msg
        })?;

    if capturing {
        if let Err(e) = client
            .stop_capture(StopCaptureRequest {})
            .await
            .map_err(|e| e.to_string())
        {
            let _ = app.emit("capture-toggle-failed", &e);
            return Err(e);
        }
        *state.capturing.lock().await = false;
        stop_transcript_stream(state.inner()).await;
        let _ = app.emit("capture-stopped", ());
        Ok(false)
    } else if let Err(e) = start_capture_services(&app, state.inner()).await {
        let _ = app.emit("capture-toggle-failed", &e);
        Err(e)
    } else {
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
    let port = state.grpc_port();
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
    let foundry_endpoint = config.foundry_endpoint.clone();
    let foundry_api_key = config.foundry_api_key.clone();
    let foundry_deployment = config.foundry_deployment.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        if rt.block_on(connect(port)).is_ok() {
            return;
        }
        if let Some(path) = find_service_binary() {
            info!(path = %path.display(), "spawning themis-service");
            let mut cmd = std::process::Command::new(path);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }
            if let Some(dir) = themis_core::find_dotenv_directory() {
                cmd.current_dir(dir);
            }
            if let Some(v) = foundry_endpoint {
                cmd.env("FOUNDRY_ENDPOINT", v);
            }
            if let Some(v) = foundry_api_key {
                cmd.env("FOUNDRY_API_KEY", v);
            }
            if let Some(v) = foundry_deployment {
                cmd.env("FOUNDRY_DEPLOYMENT", v);
            }
            let _ = cmd.spawn();
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
    let hotkey_wake = if cfg!(target_os = "macos") {
        "Command+Shift+KeyO"
    } else {
        "Ctrl+Shift+KeyO"
    };
    let hotkey_mini = if cfg!(target_os = "macos") {
        "Command+Shift+KeyM"
    } else {
        "Ctrl+Shift+KeyM"
    };
    let hotkey_topmost = if cfg!(target_os = "macos") {
        "Command+Shift+KeyP"
    } else {
        "Ctrl+Shift+KeyP"
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
    let sc_wake: tauri_plugin_global_shortcut::Shortcut = hotkey_wake
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_wake}: {e}"));
    let sc_mini: tauri_plugin_global_shortcut::Shortcut = hotkey_mini
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_mini}: {e}"));
    let sc_topmost: tauri_plugin_global_shortcut::Shortcut = hotkey_topmost
        .parse()
        .unwrap_or_else(|e| panic!("invalid hotkey {hotkey_topmost}: {e}"));

    let ui_state = Arc::new(OverlayUiState::new());

    tauri::Builder::default()
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle().clone();
                match window.label() {
                    "diagnose" => {
                        handle_aux_window_close_requested(&app, "diagnose", "diagnose-visibility");
                        api.prevent_close();
                    }
                    "settings" => {
                        handle_aux_window_close_requested(&app, "settings", "settings-visibility");
                        api.prevent_close();
                    }
                    _ => {}
                }
            }
        })
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
                    hotkey_wake,
                    hotkey_mini,
                    hotkey_topmost,
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
                        } else if shortcut == &sc_wake {
                            let _ = wake_overlay(app, true);
                        } else if shortcut == &sc_mini {
                            let state = app.state::<AppState>();
                            let _ = toggle_mini_mode(app, &state.mini_mode);
                        } else if shortcut == &sc_topmost {
                            let s = ui_state.toggle_always_on_top();
                            let _ = apply_overlay_ui(app, &s);
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
            let show_i = MenuItem::with_id(
                app,
                "show",
                if cfg!(target_os = "macos") {
                    "Show overlay (Cmd+Shift+O)"
                } else {
                    "Show overlay (Ctrl+Shift+O)"
                },
                true,
                None::<&str>,
            )?;
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
                apply_overlay_transparency(&w);
            }
            #[cfg(target_os = "macos")]
            {
                set_accessory_activation_policy();
                install_panel_app(app.handle().clone());
            }
            #[cfg(all(not(target_os = "macos"), not(windows)))]
            if let Some(mini) = app.get_webview_window("mini") {
                apply_overlay_transparency(&mini);
                set_mini_circular_clip(&mini, true);
            }

            let app_for_panel = app.handle().clone();
            app.handle().listen("mini-panel-clicked", move |_event| {
                let state = app_for_panel.state::<AppState>();
                let _ = toggle_mini_mode(&app_for_panel, &state.mini_mode);
            });

            let icon = app
                .default_window_icon()
                .cloned()
                .expect("default window icon");

            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        let _ = wake_overlay(app, true);
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

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                auto_start_capture(app_handle).await;
            });

            Ok(())
        }
        })
        .manage(ui_state)
        .manage(AppState {
            config: Arc::new(std::sync::Mutex::new(config.clone())),
            capturing: Arc::new(Mutex::new(false)),
            stream_task: Arc::new(Mutex::new(None)),
            overlay: Arc::new(Mutex::new(OverlayMirror::default())),
            mini_mode: Arc::new(std::sync::Mutex::new(MiniModeState::default())),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_config_crosscheck,
            toggle_capture,
            get_diagnostics,
            toggle_diagnose_window,
            is_diagnose_visible,
            toggle_settings_window,
            is_settings_visible,
            get_env_settings,
            save_env_settings,
            reload_env_settings,
            get_overlay_ui,
            get_insight_settings,
            set_insight_localize,
            adjust_overlay_opacity,
            adjust_overlay_font_scale,
            reset_overlay_font_scale,
            cycle_overlay_theme,
            toggle_overlay_adaptive,
            toggle_overlay_always_on_top,
            clear_listening_session,
            list_window_presets,
            apply_window_preset,
            toggle_overlay_mini_mode,
            is_overlay_mini_mode,
            expand_insight,
            get_session_export,
            save_text_file,
            toggle_overlay_visibility,
            is_overlay_visible,
            wake_overlay_window,
            hide_overlay_window,
            quit_app
        ])
        .run(tauri::generate_context!())
        .expect("error running Themis tray");
}
