import { invoke } from "@tauri-apps/api/core";
import { setupAuxWindowCloseHandler } from "./aux-window-close.js";
import { applyConfigStatusEl } from "./config-status.js";

const overlayEl = document.getElementById("overlay-text");
const overlayMetaEl = document.getElementById("overlay-meta");
const configCrosscheckEl = document.getElementById("config-crosscheck");
const analysisMetaEl = document.getElementById("analysis-meta");
const summaryEl = document.getElementById("summary");
const recordsEl = document.getElementById("records");
const latestPhraseEl = document.getElementById("latest-phrase");
const latestHeuristicEl = document.getElementById("latest-heuristic");
const latestLlmEl = document.getElementById("latest-llm");
const latestMergedEl = document.getElementById("latest-merged");
const analysisRecordsEl = document.getElementById("analysis-records");

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

function escapeHtml(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function renderInsightBlock(el, ins, emptyLabel) {
  if (!ins || (!ins.keywords?.length && !ins.terms?.length && !ins.questions?.length)) {
    el.innerHTML = `<span class="empty">${emptyLabel}</span>`;
    return;
  }
  const parts = [];
  if (ins.keywords?.length) {
    parts.push(
      `<div class="ib-row"><span class="ib-label">Keywords</span> ${ins.keywords.map((k) => `<span class="tag">${escapeHtml(k)}</span>`).join(" ")}</div>`
    );
  }
  for (const t of ins.terms || []) {
    parts.push(
      `<div class="ib-card"><div class="ib-term">${escapeHtml(t.term)}</div><div>${escapeHtml(t.explanation)}</div></div>`
    );
  }
  for (const q of ins.questions || []) {
    parts.push(
      `<div class="ib-card"><div class="ib-q">${escapeHtml(q.question)}</div><div class="ib-a">${escapeHtml(q.answer)}</div></div>`
    );
  }
  el.innerHTML = parts.join("");
}

function compactInsightSummary(ins) {
  if (!ins) return "—";
  const bits = [];
  if (ins.keywords?.length) bits.push(`kw:${ins.keywords.length}`);
  if (ins.terms?.length) bits.push(`term:${ins.terms.length}`);
  if (ins.questions?.length) bits.push(`q:${ins.questions.length}`);
  return bits.length ? bits.join(" ") : "empty";
}

function renderSummary(s) {
  if (!s || !s.count) {
    summaryEl.innerHTML =
      '<span class="empty">No final phrases yet — start capture and speak.</span>';
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
    td.textContent = "No STT latency records yet.";
    tr.appendChild(td);
    recordsEl.appendChild(tr);
    return;
  }

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
}

function renderAnalysisRecords(records) {
  analysisRecordsEl.replaceChildren();
  if (!records?.length) {
    const tr = document.createElement("tr");
    const td = document.createElement("td");
    td.colSpan = 7;
    td.className = "empty";
    td.textContent = "No analysis yet (wait for a final phrase after capture).";
    tr.appendChild(td);
    analysisRecordsEl.appendChild(tr);
    return;
  }

  const rows = [...records].reverse().slice(0, 30);
  for (const r of rows) {
    const tr = document.createElement("tr");
    tr.innerHTML = `
      <td class="text">${escapeHtml(r.text || "")}</td>
      <td class="compact">${escapeHtml(compactInsightSummary(r.heuristic))}</td>
      <td class="compact">${escapeHtml(r.llm ? compactInsightSummary(r.llm) : "—")}</td>
      <td class="compact">${escapeHtml(compactInsightSummary(r.merged))}</td>
      <td class="compact">${escapeHtml(r.llm_status || "")}</td>
      <td class="num">${fmtMs(r.heuristic_ms)}</td>
      <td class="num">${r.llm_ms != null ? fmtMs(r.llm_ms) : "—"}</td>
    `;
    analysisRecordsEl.appendChild(tr);
  }
}

function renderLatestAnalysis(records, summary) {
  if (!records?.length) {
    latestPhraseEl.textContent = "—";
    renderInsightBlock(latestHeuristicEl, null, "No heuristic output yet");
    renderInsightBlock(latestLlmEl, null, summary?.llm_configured ? "Waiting for LLM…" : "LLM not configured (FOUNDRY_*)");
    renderInsightBlock(latestMergedEl, null, "—");
    return;
  }
  const latest = records[records.length - 1];
  latestPhraseEl.textContent = latest.text || "—";
  renderInsightBlock(latestHeuristicEl, latest.heuristic, "Heuristic produced nothing");
  renderInsightBlock(
    latestLlmEl,
    latest.llm,
    latest.llm_configured
      ? `LLM: ${latest.llm_status || "no output"}`
      : "LLM disabled — set FOUNDRY_* in .env"
  );
  renderInsightBlock(latestMergedEl, latest.merged, "Nothing sent to overlay");
}

async function refresh() {
  try {
    const d = await invoke("get_diagnostics");
    overlayEl.textContent = d.overlay_display || "—";
    const parts = [];
    if (d.partial?.trim()) parts.push(`STT partial: ${d.partial.trim()}`);
    if (d.last_ui_latency_ms != null) {
      parts.push(`last UI delay: ${fmtMs(d.last_ui_latency_ms)}`);
    }
    overlayMetaEl.textContent = parts.join(" · ") || " ";

    const aSum = d.analysis_summary || {};
    const llmLine = aSum.llm_configured
      ? `LLM runtime active · last: ${aSum.last_llm_status || "?"} · ${aSum.count || 0} phrases`
      : `LLM runtime inactive (heuristic only) · ${aSum.count || 0} phrases`;
    analysisMetaEl.textContent = llmLine;

    applyConfigStatusEl(configCrosscheckEl, d.config);

    renderSummary(d.summary);
    renderRecords(d.records);
    renderLatestAnalysis(d.analysis_records, d.analysis_summary);
    renderAnalysisRecords(d.analysis_records);
  } catch (e) {
    overlayEl.textContent = `Error: ${e}`;
    summaryEl.innerHTML = "";
    recordsEl.replaceChildren();
    analysisRecordsEl.replaceChildren();
    try {
      const config = await invoke("get_config_crosscheck");
      applyConfigStatusEl(configCrosscheckEl, config);
    } catch {
      applyConfigStatusEl(configCrosscheckEl, null);
    }
  }
}

setupAuxWindowCloseHandler().catch(() => {});

refresh();
setInterval(refresh, 1000);
