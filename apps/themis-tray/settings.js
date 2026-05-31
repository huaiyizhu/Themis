import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { applyConfigStatusEl } from "./config-status.js";

const form = document.getElementById("settings-form");
const envPathEl = document.getElementById("env-path");
const saveStatusEl = document.getElementById("save-status");
const configEl = document.getElementById("config-crosscheck");
const saveBtn = document.getElementById("save-btn");
const reloadBtn = document.getElementById("reload-btn");

const FIELD_NAMES = [
  "azure_speech_key",
  "azure_speech_region",
  "azure_speech_language",
  "azure_speech_mode",
  "themis_stt_fixup",
  "azure_speech_corrections",
  "foundry_endpoint",
  "foundry_api_key",
  "foundry_deployment",
  "themis_analysis_enabled",
  "themis_insight_dwell_secs",
  "themis_session_summary_interval_secs",
  "themis_audio_capture_mode",
  "themis_audio_output_device",
  "themis_audio_input_device",
  "themis_audio_gain_max",
  "themis_grpc_port",
  "themis_log_level",
  "themis_use_mock_speech",
];

function setStatus(text, ok) {
  if (!saveStatusEl) return;
  saveStatusEl.textContent = text;
  saveStatusEl.classList.remove("ok", "err");
  if (text) saveStatusEl.classList.add(ok ? "ok" : "err");
}

function fillForm(settings) {
  for (const name of FIELD_NAMES) {
    const el = form?.elements.namedItem(name);
    if (el && "value" in el) {
      el.value = settings[name] ?? "";
    }
  }
}

function readForm() {
  /** @type {Record<string, string>} */
  const settings = {};
  for (const name of FIELD_NAMES) {
    const el = form?.elements.namedItem(name);
    settings[name] = el && "value" in el ? String(el.value).trim() : "";
  }
  return settings;
}

async function loadSettings() {
  try {
    const data = await invoke("get_env_settings");
    envPathEl.textContent = data.exists
      ? `.env 路径：${data.path}`
      : `尚无 .env，保存后将写入：${data.path}`;
    fillForm(data.settings);
    setStatus("", true);
    const cross = await invoke("get_config_crosscheck");
    applyConfigStatusEl(configEl, cross);
  } catch (e) {
    setStatus(String(e), false);
  }
}

async function onSave(e) {
  e.preventDefault();
  saveBtn.disabled = true;
  reloadBtn.disabled = true;
  setStatus("保存中…", true);
  try {
    const result = await invoke("save_env_settings", { settings: readForm() });
    setStatus(result.message ?? "已保存", true);
    applyConfigStatusEl(configEl, result.config);
    envPathEl.textContent = `.env 路径：${result.path}`;
  } catch (err) {
    setStatus(String(err), false);
  } finally {
    saveBtn.disabled = false;
    reloadBtn.disabled = false;
  }
}

async function onReloadDisk() {
  saveBtn.disabled = true;
  reloadBtn.disabled = true;
  setStatus("重新加载中…", true);
  try {
    const result = await invoke("reload_env_settings");
    fillForm((await invoke("get_env_settings")).settings);
    setStatus(result.message ?? "已重新加载", true);
    applyConfigStatusEl(configEl, result.config);
  } catch (err) {
    setStatus(String(err), false);
  } finally {
    saveBtn.disabled = false;
    reloadBtn.disabled = false;
  }
}

form?.addEventListener("submit", onSave);
reloadBtn?.addEventListener("click", onReloadDisk);
listen("env-settings-saved", () => loadSettings());
loadSettings();
