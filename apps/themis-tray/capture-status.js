/** Colored capture state for overlay status line. */

import { setTip } from "./tooltips.js";

const CAP_STATE_CLASSES = [
  "cap-state-capturing",
  "cap-state-idle",
  "cap-state-error",
  "cap-state-failed",
  "cap-state-offline",
  "cap-state-connecting",
  "cap-state-warning",
  "cap-state-starting",
  "cap-state-stopping",
];

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function signalLevel(peak, frames) {
  const p = Number(peak) || 0;
  const f = Number(frames) || 0;
  if (p >= 2000) return "strong";
  if (p >= 200) return "ok";
  if (f > 0) return "quiet";
  return "silent";
}

function isFailedMessage(message) {
  if (!message) return false;
  const m = message.toLowerCase();
  return (
    m.includes("failed") ||
    m.includes("error") ||
    m.includes("cannot") ||
    m.includes("unable")
  );
}

function resolveVisualState(status) {
  const state = String(status?.state || "idle").toLowerCase();
  if (state === "error") return "error";
  if (state === "capturing") {
    const level = signalLevel(status.audio_peak, status.audio_frames);
    return level === "silent" || level === "quiet" ? "warning" : "capturing";
  }
  if (state === "idle" && isFailedMessage(status?.message)) return "failed";
  return state === "capturing" ? "capturing" : "idle";
}

function stateLabelClass(visualState, rawState) {
  if (visualState === "failed") return "cap-failed";
  if (visualState === "warning") return "cap-capturing";
  const state = String(rawState || "idle").toLowerCase();
  if (state === "error") return "cap-error";
  if (state === "capturing") return "cap-capturing";
  return "cap-idle";
}

function peakClass(level) {
  return `cap-peak-${level}`;
}

function renderDot() {
  return '<span class="cap-dot" aria-hidden="true"></span>';
}

function renderSep() {
  return '<span class="cap-sep">·</span>';
}

/**
 * @param {{ state?: string, capture_mode?: string, audio_peak?: number, audio_frames?: number, message?: string }} status
 */
export function renderCaptureStatus(status) {
  const rawState = String(status?.state || "idle").toLowerCase();
  const visualState = resolveVisualState(status);
  const labelClass = stateLabelClass(visualState, rawState);
  const labelText =
    visualState === "failed" ? "failed to capture" : rawState || "idle";

  let html = `${renderDot()}<span class="cap-state-label ${labelClass}">${escapeHtml(labelText)}</span>`;

  if (rawState === "capturing") {
    const mode = status.capture_mode?.trim() || "?";
    const peak = status.audio_peak ?? 0;
    const frames = status.audio_frames ?? 0;
    const level = signalLevel(peak, frames);
    html += `${renderSep()}<span class="cap-mode">${escapeHtml(mode)}</span>`;
    html += `${renderSep()}<span class="cap-metric">peak <span class="${peakClass(level)}">${escapeHtml(peak)}</span></span>`;
    html += `${renderSep()}<span class="cap-metric">frames ${escapeHtml(frames)}</span>`;
    html += `${renderSep()}<span class="cap-signal cap-signal-${level}">signal ${level}</span>`;
  }

  return html;
}

export function renderCaptureStatusOffline(error) {
  return `${renderDot()}<span class="cap-state-label cap-offline">service offline</span>`;
}

export function renderCaptureStatusConnecting() {
  return `${renderDot()}<span class="cap-state-label cap-connecting">connecting…</span>`;
}

/**
 * @param {"starting" | "stopping"} action
 */
export function renderCaptureStatusPending(action) {
  const label = action === "stopping" ? "stopping capture…" : "starting capture…";
  const labelClass = action === "stopping" ? "cap-stopping" : "cap-starting";
  return `${renderDot()}<span class="cap-state-label ${labelClass}">${label}</span>`;
}

/**
 * @param {HTMLElement | null} el
 * @param {"starting" | "stopping"} action
 */
export function applyCaptureStatusPending(el, action) {
  if (!el) return;
  el.classList.remove(...CAP_STATE_CLASSES);
  el.innerHTML = renderCaptureStatusPending(action);
  el.classList.add(action === "stopping" ? "cap-state-stopping" : "cap-state-starting");
  setTip(
    el,
    action === "stopping"
      ? "Stopping capture — waiting for service…"
      : "Starting capture — waiting for service…",
  );
}

/**
 * @param {HTMLElement | null} el
 * @param {{ state?: string, message?: string, capture_detail?: string, capture_mode?: string, audio_peak?: number, audio_frames?: number } | { offline: true, error?: string } | null | undefined} payload
 */
export function applyCaptureStatusEl(el, payload) {
  if (!el) return;

  el.classList.remove(...CAP_STATE_CLASSES);

  if (!payload) {
    el.innerHTML = renderCaptureStatusConnecting();
    el.classList.add("cap-state-connecting");
    return;
  }

  if (payload.offline) {
    el.innerHTML = renderCaptureStatusOffline(payload.error);
    el.classList.add("cap-state-offline");
    setTip(el, payload.error || "Service offline");
    return;
  }

  const visualState = resolveVisualState(payload);
  el.innerHTML = renderCaptureStatus(payload);
  el.classList.add(`cap-state-${visualState}`);

  const detail = payload.capture_detail ? `${payload.capture_detail}\n` : "";
  setTip(el, `${detail}${payload.message || ""}`.trim());
}
