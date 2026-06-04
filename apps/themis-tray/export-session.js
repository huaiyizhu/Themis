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
 * @param {Array<{termEntries?: object[], questionEntries?: object[]}>} opts
 */
export function buildInsightsExportText({ termEntries = [], questionEntries = [] } = {}) {
  const terms = dedupeEntriesByKey(termEntries, (item) => normalizeTermKey(item.term)).sort(
    (a, b) => (a.addedAt || 0) - (b.addedAt || 0),
  );
  const questions = dedupeEntriesByKey(questionEntries, (item) =>
    normalizeQuestionKey(item.question),
  ).sort((a, b) => (a.addedAt || 0) - (b.addedAt || 0));

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
