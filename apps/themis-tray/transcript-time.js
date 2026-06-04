function pad2(n) {
  return String(n).padStart(2, "0");
}

/** Elapsed time from session start, e.g. 00:15 */
export function formatRelativeTranscriptTime(atMs, sessionStartMs) {
  if (!atMs || !sessionStartMs) return "";
  const sec = Math.max(0, Math.floor((atMs - sessionStartMs) / 1000));
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  return `${pad2(m)}:${pad2(s)}`;
}

/** Absolute local time for export, e.g. 2026/06/04 14:30:05 */
export function formatAbsoluteTranscriptTime(atMs) {
  if (!atMs) return "";
  return new Date(atMs).toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

/**
 * @param {Array<{text: string, atMs: number}>} lines
 * @param {{ partialText?: string, partialAtMs?: number | null, summary?: string }} opts
 */
export function buildTimestampedTranscriptExportText(lines, { partialText = "", partialAtMs = null, summary = "" } = {}) {
  const parts = ["# Themis 会话字幕", `导出时间: ${formatAbsoluteTranscriptTime(Date.now())}`, ""];

  if (!lines?.length && !partialText) {
    parts.push("（暂无字幕）");
  } else {
    for (const line of lines) {
      const stamp = formatAbsoluteTranscriptTime(line.atMs);
      parts.push(`[${stamp}] ${line.text}`);
    }
    const partial = String(partialText || "").trim();
    if (partial) {
      const stamp = partialAtMs ? formatAbsoluteTranscriptTime(partialAtMs) : "进行中";
      parts.push(`[${stamp}] ${partial}${partialAtMs ? "" : " …"}`);
    }
  }

  const summaryText = String(summary || "").trim();
  if (summaryText) {
    parts.push("", "---", "", "## 会话摘要", "", summaryText);
  }

  return parts.join("\n");
}

/** @param {Array<{text: string, atMs: number}>} localLines */
export function mergeExportLines(localLines, remoteLines) {
  const byText = new Map();
  for (const line of localLines || []) {
    if (line?.text) byText.set(line.text, line);
  }
  for (const line of remoteLines || []) {
    const text = String(line?.text || "").trim();
    if (!text) continue;
    const atMs = Number(line.timestamp_unix_ms ?? line.atMs ?? 0);
    const existing = byText.get(text);
    if (!existing || (atMs > 0 && (!existing.atMs || atMs < existing.atMs))) {
      byText.set(text, { text, atMs: atMs || existing?.atMs || 0 });
    }
  }
  return [...byText.values()].sort((a, b) => a.atMs - b.atMs);
}
