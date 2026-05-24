import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

const overlayEl = document.getElementById("overlay");
const dragHandle = document.getElementById("drag-handle");
const themeBadgeEl = document.getElementById("theme-badge");
const statusEl = document.getElementById("status");
const scrollEl = document.getElementById("transcript-scroll");
const transcriptEl = document.getElementById("transcript");
const scrollLatestBtn = document.getElementById("scroll-latest");
const insightsEmptyEl = document.getElementById("insights-empty");
const insightsKeywordsSec = document.getElementById("insights-keywords");
const insightsKeywordsList = document.getElementById("insights-keywords-list");
const insightsTermsSec = document.getElementById("insights-terms");
const insightsTermsList = document.getElementById("insights-terms-list");
const insightsQuestionsSec = document.getElementById("insights-questions");
const insightsQuestionsList = document.getElementById("insights-questions-list");
const contentSplitEl = document.getElementById("content-split");
const splitDividerEl = document.getElementById("split-divider");
const insightsPanelEl = document.getElementById("insights-panel");

/** @type {string[]} Final lines (one per Azure REST phrase). */
let committedLines = [];
/** @type {Map<string, object|null>} line text → latest insights */
const lineInsights = new Map();
/** Latest partial hypothesis while speaking. */
let partialText = "";

/** User scrolled up — pause auto-follow until they click Latest or scroll to bottom. */
let followLatest = true;

/** Minimum visible time for each term/question card (ms). */
const INSIGHT_DWELL_MS = 10_000;

let termSeq = 0;
let questionSeq = 0;
/** @type {Array<{id: string, seq: number, addedAt: number, expiresAt: number, pinned: boolean, term: string, explanation: string}>} */
const termEntries = [];
/** @type {Array<{id: string, seq: number, addedAt: number, expiresAt: number, pinned: boolean, question: string, answer: string}>} */
const questionEntries = [];
/** @type {ReturnType<typeof setInterval> | null} */
let insightPruneTimer = null;

const SCROLL_BOTTOM_THRESHOLD = 48;

const SPLIT_WIDTH_STORAGE_KEY = "themis-insights-panel-width";
const INSIGHTS_PANEL_MIN = 120;
const TRANSCRIPT_PANEL_MIN = 100;
const SPLIT_DIVIDER_WIDTH = 8;

/** Programmatic drag avoids Windows WM_NCHITTEST fighting resize after data-tauri-drag-region. */
function setupWindowDrag() {
  dragHandle.addEventListener("mousedown", async (e) => {
    if (e.button !== 0) return;
    if (e.target.closest("button, a, input, select, textarea")) return;
    e.preventDefault();
    try {
      await getCurrentWindow().startDragging();
    } catch {
      /* browser preview */
    }
  });
}

setupWindowDrag();

function clampInsightsWidth(widthPx) {
  if (!contentSplitEl || !insightsPanelEl) return widthPx;
  const max = contentSplitEl.clientWidth - TRANSCRIPT_PANEL_MIN - SPLIT_DIVIDER_WIDTH;
  return Math.round(Math.max(INSIGHTS_PANEL_MIN, Math.min(widthPx, max)));
}

function applyInsightsWidth(widthPx) {
  if (!insightsPanelEl) return;
  const clamped = clampInsightsWidth(widthPx);
  insightsPanelEl.style.flex = `0 0 ${clamped}px`;
  insightsPanelEl.style.width = `${clamped}px`;
  insightsPanelEl.style.maxWidth = "none";
}

function initSplitDivider() {
  if (!contentSplitEl || !splitDividerEl || !insightsPanelEl) return;

  const saved = localStorage.getItem(SPLIT_WIDTH_STORAGE_KEY);
  if (saved) {
    const parsed = Number(saved);
    if (Number.isFinite(parsed) && parsed > 0) {
      applyInsightsWidth(parsed);
    }
  }

  let dragging = false;

  splitDividerEl.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    dragging = true;
    splitDividerEl.classList.add("is-dragging");
    document.body.classList.add("split-dragging");
  });

  window.addEventListener("mousemove", (e) => {
    if (!dragging) return;
    const rect = contentSplitEl.getBoundingClientRect();
    applyInsightsWidth(rect.right - e.clientX);
  });

  const stopDrag = () => {
    if (!dragging) return;
    dragging = false;
    splitDividerEl.classList.remove("is-dragging");
    document.body.classList.remove("split-dragging");
    localStorage.setItem(SPLIT_WIDTH_STORAGE_KEY, String(insightsPanelEl.offsetWidth));
  };

  window.addEventListener("mouseup", stopDrag);
  window.addEventListener("blur", stopDrag);

  window.addEventListener("resize", () => {
    if (insightsPanelEl.offsetWidth > 0) {
      applyInsightsWidth(insightsPanelEl.offsetWidth);
    }
  });
}

initSplitDivider();

function isNearBottom() {
  return (
    scrollEl.scrollHeight - scrollEl.scrollTop - scrollEl.clientHeight <
    SCROLL_BOTTOM_THRESHOLD
  );
}

function scrollToLatest(smooth = true) {
  followLatest = true;
  scrollLatestBtn.classList.add("hidden");
  if (smooth) {
    scrollEl.scrollTo({ top: scrollEl.scrollHeight, behavior: "smooth" });
  } else {
    scrollEl.scrollTop = scrollEl.scrollHeight;
  }
}

scrollEl.addEventListener(
  "scroll",
  () => {
    if (isNearBottom()) {
      followLatest = true;
      scrollLatestBtn.classList.add("hidden");
    } else {
      followLatest = false;
      scrollLatestBtn.classList.remove("hidden");
    }
  },
  { passive: true }
);

scrollLatestBtn.addEventListener("click", () => scrollToLatest(true));

function renderTranscript() {
  transcriptEl.classList.remove("is-placeholder", "is-partial");
  transcriptEl.replaceChildren();

  if (committedLines.length === 0 && !partialText) {
    return;
  }

  for (const line of committedLines) {
    const wrap = document.createElement("span");
    wrap.className = "line-final line-with-tags";
    const text = document.createElement("span");
    text.textContent = line;
    wrap.appendChild(text);
    const ins = lineInsights.get(line);
    if (ins?.keywords?.length) {
      const kw = document.createElement("span");
      kw.className = "line-kw";
      kw.textContent = ins.keywords.slice(0, 4).join(" · ");
      kw.title = "Keywords";
      wrap.appendChild(kw);
    }
    transcriptEl.appendChild(wrap);
  }

  if (partialText) {
    const el = document.createElement("span");
    el.className = "line-partial";
    el.textContent = partialText;
    transcriptEl.appendChild(el);
    transcriptEl.classList.add("is-partial");
  }

  requestAnimationFrame(() => {
    if (followLatest) {
      scrollToLatest(false);
    }
  });
}

function setPlaceholder(text) {
  transcriptEl.replaceChildren();
  transcriptEl.classList.add("is-placeholder");
  transcriptEl.classList.remove("is-partial");
  transcriptEl.textContent = text;
  requestAnimationFrame(() => scrollToLatest(false));
}

async function refreshStatus() {
  try {
    const s = await invoke("get_status");
    const short =
      s.state === "capturing"
        ? `● ${s.state} · ${s.capture_mode || "?"} · peak ${s.audio_peak ?? 0}`
        : `● ${s.state}`;
    statusEl.textContent = short;
    statusEl.title = s.message || "";

    if (s.state === "capturing" && committedLines.length === 0 && !partialText) {
      let hint = "Listening… new lines appear below; view scrolls to latest.";
      if (s.message?.includes("no loopback signal")) {
        hint = s.message;
      } else if (s.message?.includes("signal=strong")) {
        hint = "Capture OK. Transcript scrolls here (~2s per phrase)…";
      } else if (s.message?.includes("signal=ok")) {
        hint = "Capture OK. Waiting for speech…";
      }
      setPlaceholder(hint);
    }
  } catch (e) {
    statusEl.textContent = `Service offline`;
    statusEl.title = String(e);
  }
}

function formatInsightTime(ms) {
  return new Date(ms).toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

function ensureInsightPruneTimer() {
  if (insightPruneTimer !== null) return;
  insightPruneTimer = setInterval(pruneExpiredInsights, 500);
}

function supersedeHeadEntry(entries, now) {
  const prev = entries[0];
  if (!prev) return;
  prev.pinned = false;
  prev.expiresAt = now + INSIGHT_DWELL_MS;
}

function pruneEntryList(entries) {
  const now = Date.now();
  let changed = false;
  if (entries.length > 0) {
    const head = entries[0];
    if (!head.pinned && now - head.addedAt >= INSIGHT_DWELL_MS) {
      head.pinned = true;
      changed = true;
    }
  }
  for (let i = entries.length - 1; i >= 0; i -= 1) {
    const item = entries[i];
    if (item.pinned) continue;
    if (item.expiresAt <= now) {
      entries.splice(i, 1);
      changed = true;
    }
  }
  return changed;
}

function pruneExpiredInsights() {
  const termsChanged = pruneEntryList(termEntries);
  const questionsChanged = pruneEntryList(questionEntries);
  if (termsChanged || questionsChanged) {
    renderTermAndQuestionPanels();
  }
}

function appendTermEntries(terms) {
  if (!terms?.length) return false;
  const now = Date.now();
  let added = false;
  for (let i = terms.length - 1; i >= 0; i -= 1) {
    const t = terms[i];
    const key = String(t.term || "").trim().toLowerCase();
    if (!key) continue;
    if (termEntries.some((e) => e.term.trim().toLowerCase() === key)) continue;
    supersedeHeadEntry(termEntries, now);
    termSeq += 1;
    termEntries.unshift({
      id: `${now}-t${termSeq}`,
      seq: termSeq,
      addedAt: now,
      expiresAt: now + INSIGHT_DWELL_MS,
      pinned: false,
      term: t.term,
      explanation: t.explanation,
    });
    added = true;
  }
  return added;
}

function appendQuestionEntries(questions) {
  if (!questions?.length) return false;
  const now = Date.now();
  let added = false;
  for (let i = questions.length - 1; i >= 0; i -= 1) {
    const q = questions[i];
    const key = String(q.question || "").trim();
    if (!key) continue;
    if (questionEntries.some((e) => e.question.trim() === key)) continue;
    supersedeHeadEntry(questionEntries, now);
    questionSeq += 1;
    questionEntries.unshift({
      id: `${now}-q${questionSeq}`,
      seq: questionSeq,
      addedAt: now,
      expiresAt: now + INSIGHT_DWELL_MS,
      pinned: false,
      question: q.question,
      answer: q.answer,
    });
    added = true;
  }
  return added;
}

function renderTermAndQuestionPanels() {
  const hasTerms = termEntries.length > 0;
  const hasQuestions = questionEntries.length > 0;
  const hasKeywords = insightsKeywordsList.childElementCount > 0;

  if (hasTerms || hasQuestions || hasKeywords) {
    insightsEmptyEl.classList.add("hidden");
  } else {
    insightsEmptyEl.classList.remove("hidden");
  }

  insightsTermsSec.classList.toggle("hidden", !hasTerms);
  insightsTermsList.replaceChildren();
  for (const item of termEntries) {
    const card = document.createElement("div");
    card.className = "insight-card";
    card.dataset.id = item.id;
    const meta = document.createElement("div");
    meta.className = "insight-meta";
    meta.textContent = `#${item.seq} · ${formatInsightTime(item.addedAt)}`;
    const term = document.createElement("div");
    term.className = "term";
    term.textContent = item.term;
    const body = document.createElement("div");
    body.textContent = item.explanation;
    card.append(meta, term, body);
    insightsTermsList.appendChild(card);
  }

  insightsQuestionsSec.classList.toggle("hidden", !hasQuestions);
  insightsQuestionsList.replaceChildren();
  for (const item of questionEntries) {
    const card = document.createElement("div");
    card.className = "insight-card";
    card.dataset.id = item.id;
    const meta = document.createElement("div");
    meta.className = "insight-meta";
    meta.textContent = `#${item.seq} · ${formatInsightTime(item.addedAt)}`;
    const q = document.createElement("div");
    q.className = "q";
    q.textContent = item.question;
    const a = document.createElement("div");
    a.className = "a";
    a.textContent = item.answer;
    card.append(meta, q, a);
    insightsQuestionsList.appendChild(card);
  }
}

function resetInsightDwellState() {
  termSeq = 0;
  questionSeq = 0;
  termEntries.length = 0;
  questionEntries.length = 0;
  if (insightPruneTimer !== null) {
    clearInterval(insightPruneTimer);
    insightPruneTimer = null;
  }
}

function renderInsights(insights) {
  if (!insights || (!insights.keywords?.length && !insights.terms?.length && !insights.questions?.length)) {
    return;
  }
  let changed = false;

  if (insights.keywords?.length) {
    insightsKeywordsSec.classList.remove("hidden");
    insightsKeywordsList.replaceChildren();
    for (const kw of insights.keywords) {
      const tag = document.createElement("span");
      tag.className = "tag";
      tag.textContent = kw;
      insightsKeywordsList.appendChild(tag);
    }
    changed = true;
  }

  if (appendTermEntries(insights.terms)) changed = true;
  if (appendQuestionEntries(insights.questions)) changed = true;

  if (changed) {
    ensureInsightPruneTimer();
    renderTermAndQuestionPanels();
  }
}

function isSystemMessage(text) {
  return (
    /^Azure (REST|streaming|auto-language)/i.test(text) ||
    text.includes("connected…") ||
    text.includes("transcribing every") ||
    text.includes("picking best match")
  );
}

listen("transcript", (event) => {
  const { text, is_final, insights } = event.payload;
  if (!text || (text.startsWith("(") && text.includes("未识别"))) {
    return;
  }
  if (isSystemMessage(text)) {
    return;
  }

  const trimmed = text.trim();

  if (is_final) {
    if (insights) {
      lineInsights.set(trimmed, insights);
      renderInsights(insights);
      if (committedLines.includes(trimmed)) {
        renderTranscript();
      }
      return;
    }
    if (trimmed && !committedLines.includes(trimmed)) {
      committedLines.push(trimmed);
      partialText = "";
    }
  } else {
    partialText = trimmed;
  }

  renderTranscript();
});

listen("capture-stopped", () => {
  partialText = "";
  renderTranscript();
});

listen("capture-started", () => {
  committedLines = [];
  partialText = "";
  lineInsights.clear();
  followLatest = true;
  scrollLatestBtn.classList.add("hidden");
  resetInsightDwellState();
  insightsEmptyEl.classList.remove("hidden");
  insightsKeywordsSec.classList.add("hidden");
  insightsTermsSec.classList.add("hidden");
  insightsQuestionsSec.classList.add("hidden");
  insightsKeywordsList.replaceChildren();
  insightsTermsList.replaceChildren();
  insightsQuestionsList.replaceChildren();
  setPlaceholder("Capture started — transcript builds below…");
});

function applyOverlayUi(payload) {
  const theme = payload.effective_theme || payload.theme || "dark-glass";
  overlayEl.className = `theme-${theme}`;
  const opacity =
    typeof payload.opacity === "number"
      ? Math.min(1, Math.max(0.35, payload.opacity))
      : 0.92;
  overlayEl.style.opacity = String(opacity);
  document.body.classList.toggle("adaptive-on", Boolean(payload.adaptive));
  const short = theme.replace(/-glass$/, "").replace("high-contrast-", "hc-");
  themeBadgeEl.textContent = short;
  themeBadgeEl.title = payload.adaptive
    ? `${theme} (auto contrast on)`
    : `${theme} — Ctrl+Shift+S cycle`;
}

listen("overlay-ui", (event) => {
  applyOverlayUi(event.payload);
});

async function loadOverlayUi() {
  try {
    const s = await invoke("get_overlay_ui");
    applyOverlayUi({
      theme: s.theme,
      effective_theme: s.theme,
      adaptive: s.adaptive,
      opacity: s.opacity,
    });
  } catch {
    /* not in tauri shell */
  }
}

loadOverlayUi();
refreshStatus();
setInterval(refreshStatus, 5000);
