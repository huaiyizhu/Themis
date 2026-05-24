import { invoke } from "@tauri-apps/api/core";

const overlayEl = document.getElementById("overlay-text");
const overlayMetaEl = document.getElementById("overlay-meta");
const summaryEl = document.getElementById("summary");
const recordsEl = document.getElementById("records");

function fmtMs(n) {
  if (n == null || Number.isNaN(n)) return "—";
  return `${Math.round(n)} ms`;
}

function uiLatencyMs(emittedUnixMs, receivedUnixMs) {
  if (!emittedUnixMs) return null;
  const ref = receivedUnixMs ?? Date.now();
  const d = ref - emittedUnixMs;
  return d >= 0 && d < 120_000 ? d : null;
}

function renderSummary(s) {
  if (!s || !s.count) {
    summaryEl.innerHTML = '<span class="empty">No final phrases yet — start capture and speak.</span>';
    return;
  }
  const items = [
    ["Phrases", s.count],
    ["Avg Azure", fmtMs(s.avg_azure_ms)],
    ["Avg E2E", fmtMs(s.avg_e2e_ms)],
    ["Max E2E", fmtMs(s.max_e2e_ms)],
    ["Last Azure", fmtMs(s.last_azure_ms)],
  ];
  summaryEl.replaceChildren();
  for (const [label, value] of items) {
    const div = document.createElement("div");
    div.className = "stat";
    div.innerHTML = `<span class="label">${label}</span><span class="value">${value}</span>`;
    summaryEl.appendChild(div);
  }
}

function renderRecords(records) {
  recordsEl.replaceChildren();
  if (!records?.length) {
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = 7;
    td.className = "empty";
    td.textContent = "No latency records yet.";
    tr.appendChild(td);
    recordsEl.appendChild(tr);
    return;
  }

  const now = Date.now();
  const rows = [...records].reverse().slice(0, 40);
  for (const r of rows) {
    const b = r.breakdown || {};
    const tr = document.createElement("tr");
    const ui = uiLatencyMs(r.emitted_unix_ms, r.received_unix_ms);
    tr.innerHTML = `
      <td class="text">${escapeHtml(r.text || "")}</td>
      <td class="num">${fmtMs(b.buffer_ms)}</td>
      <td class="num">${fmtMs(b.azure_ms)}</td>
      <td class="num">${fmtMs(b.stt_wall_ms)}</td>
      <td class="num">${fmtMs(b.estimated_e2e_ms)}</td>
      <td class="num">${ui != null ? fmtMs(ui) : "—"}</td>
      <td>${escapeHtml(b.language || "")}</td>
    `;
    recordsEl.appendChild(tr);
  }
  void now;
}

function escapeHtml(s) {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

async function refresh() {
  try {
    const d = await invoke("get_diagnostics");
    overlayEl.textContent = d.overlay_display || "—";
    const parts = [];
    if (d.partial?.trim()) parts.push(`partial: ${d.partial.trim()}`);
    if (d.last_ui_latency_ms != null) {
      parts.push(`last UI delay: ${fmtMs(d.last_ui_latency_ms)}`);
    }
    if (d.service_online === false) {
      parts.push("service offline");
    }
    overlayMetaEl.textContent = parts.join(" · ") || " ";
    renderSummary(d.summary);
    renderRecords(d.records);
  } catch (e) {
    overlayEl.textContent = `Error: ${e}`;
    summaryEl.innerHTML = "";
    recordsEl.replaceChildren();
  }
}

refresh();
setInterval(refresh, 1000);
