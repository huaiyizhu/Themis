import { invoke } from "@tauri-apps/api/core";
import { buildInsightExportText, normalizeQuestionKey, normalizeTermKey } from "./ui-modes.js";

function pad2(n) {
  return String(n).padStart(2, "0");
}

export function formatExportTimestamp(date = new Date()) {
  return date.toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

export function defaultExportFileName(prefix) {
  const d = new Date();
  const stamp = `${d.getFullYear()}${pad2(d.getMonth() + 1)}${pad2(d.getDate())}-${pad2(d.getHours())}${pad2(d.getMinutes())}${pad2(d.getSeconds())}`;
  return `${prefix}-${stamp}.txt`;
}

/** @param {object[]} entries @param {(item: object) => string} keyFn */
function dedupeEntriesByKey(entries, keyFn) {
  const map = new Map();
  for (const item of entries) {
    const key = keyFn(item);
    if (!key) continue;
    const prev = map.get(key);
    if (!prev || item.seq > prev.seq) {
      map.set(key, item);
    }
  }
  return [...map.values()].sort((a, b) => a.seq - b.seq);
}

/**
 * @param {{ transcript?: string, summary?: string, partial?: string }} opts
 */
export function buildTranscriptExportText({ transcript = "", summary = "", partial = "" } = {}) {
  const parts = ["# Themis 会话字幕", `导出时间: ${formatExportTimestamp()}`, ""];
  const body = String(transcript || "").trim();
  const partialText = String(partial || "").trim();

  if (!body && !partialText) {
    parts.push("（暂无字幕）");
  } else {
    if (body) parts.push(body);
    if (partialText) {
      if (body) parts.push("");
      parts.push(`[进行中] ${partialText}`);
    }
  }

  const summaryText = String(summary || "").trim();
  if (summaryText) {
    parts.push("", "---", "", "## 会话摘要", "", summaryText);
  }

  return parts.join("\n");
}

/**
 * @param {{ termEntries?: object[], questionEntries?: object[] }} opts
 */
export function buildInsightsExportText({ termEntries = [], questionEntries = [] } = {}) {
  const terms = dedupeEntriesByKey(termEntries, (item) => normalizeTermKey(item.term));
  const questions = dedupeEntriesByKey(questionEntries, (item) =>
    normalizeQuestionKey(item.question),
  );

  const parts = ["# Themis 问题与术语", `导出时间: ${formatExportTimestamp()}`, ""];

  parts.push(`## 术语 (${terms.length})`, "");
  if (terms.length === 0) {
    parts.push("（暂无术语）", "");
  } else {
    for (const item of terms) {
      parts.push(buildInsightExportText(item, "term"));
      parts.push("");
    }
  }

  parts.push(`## 问题 (${questions.length})`, "");
  if (questions.length === 0) {
    parts.push("（暂无问题）");
  } else {
    for (const item of questions) {
      parts.push(buildInsightExportText(item, "question"));
      parts.push("");
    }
  }

  return parts.join("\n").trimEnd();
}

/**
 * @param {string} content
 * @param {string} defaultName
 */
export async function saveExportText(content, defaultName) {
  const text = String(content || "").trim();
  if (!text) {
    return { ok: false, reason: "empty" };
  }
  try {
    const path = await invoke("save_text_file", { content: text, defaultName });
    if (!path) {
      return { ok: false, reason: "cancelled" };
    }
    return { ok: true, path };
  } catch (e) {
    return { ok: false, reason: String(e) };
  }
}

/** @param {string} content */
export async function copyExportText(content) {
  const text = String(content || "").trim();
  if (!text) return false;
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

/**
 * Prefer backend transcript when longer; fall back to local committed lines.
 * @param {{ committedLines?: string[], partialText?: string, summaryText?: string }} local
 */
export async function resolveTranscriptForExport(local) {
  let transcript = (local.committedLines || []).join("\n").trim();
  let summary = String(local.summaryText || "").trim();
  let partial = String(local.partialText || "").trim();

  try {
    const remote = await invoke("get_session_export");
    const remoteTranscript = String(remote?.transcript || "").trim();
    const remoteSummary = String(remote?.session_summary || "").trim();
    if (remoteTranscript.length > transcript.length) {
      transcript = remoteTranscript;
    }
    if (remoteSummary) {
      summary = remoteSummary;
    }
  } catch {
    /* service offline — use local state */
  }

  return { transcript, summary, partial };
}
