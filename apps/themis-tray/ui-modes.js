/**
 * Meeting / glance UI modes — focused insight rendering.
 */
import { invoke } from "@tauri-apps/api/core";
import { dismissTooltip, setTip } from "./tooltips.js";

export const UI_MODE_STORAGE_KEY = "themis-ui-mode";
export const SUMMARY_COLLAPSED_KEY = "themis-summary-collapsed";
export const PINNED_COLLAPSED_KEY = "themis-pinned-collapsed";

/** @type {Set<string>} */
const dismissedTermKeys = new Set();

/** @type {Set<string>} */
const dismissedQuestionKeys = new Set();

export function normalizeTermKey(term) {
  return String(term || "")
    .trim()
    .toLowerCase();
}

export function normalizeQuestionKey(question) {
  return String(question || "").trim();
}

export function clearDismissedTerms() {
  dismissedTermKeys.clear();
  dismissedQuestionKeys.clear();
}

export function isTermDismissed(term) {
  return dismissedTermKeys.has(normalizeTermKey(term));
}

export function dismissTermKey(term) {
  dismissedTermKeys.add(normalizeTermKey(term));
}

export function isQuestionDismissed(question) {
  return dismissedQuestionKeys.has(normalizeQuestionKey(question));
}

export function dismissQuestionKey(question) {
  dismissedQuestionKeys.add(normalizeQuestionKey(question));
}

export function loadUiMode() {
  const m = localStorage.getItem(UI_MODE_STORAGE_KEY);
  return m === "glance" ? "glance" : "meeting";
}

export function applyUiMode(mode) {
  document.body.classList.remove("ui-mode-meeting", "ui-mode-glance");
  document.body.classList.add(`ui-mode-${mode}`);
  for (const btn of document.querySelectorAll(".ui-mode-btn")) {
    const active = btn.dataset.uiMode === mode;
    btn.classList.toggle("is-active", active);
    btn.setAttribute("aria-pressed", active ? "true" : "false");
  }
  const glancePanel = document.getElementById("glance-panel");
  if (glancePanel) glancePanel.hidden = mode !== "glance";
}

/**
 * @param {(mode: string) => void} [onChange]
 */
export function initUiModeSwitch(onChange) {
  const mode = loadUiMode();
  applyUiMode(mode);
  for (const btn of document.querySelectorAll(".ui-mode-btn")) {
    btn.addEventListener("click", () => {
      const next = btn.dataset.uiMode;
      if (!next || next === loadUiMode()) return;
      localStorage.setItem(UI_MODE_STORAGE_KEY, next);
      applyUiMode(next);
      onChange?.(next);
    });
  }
  return mode;
}

export function initHeaderOverflow() {
  const toggle = document.getElementById("header-overflow-toggle");
  const menu = document.getElementById("header-overflow-menu");
  if (!toggle || !menu) return;

  const close = () => {
    menu.classList.add("hidden");
    toggle.setAttribute("aria-expanded", "false");
    dismissTooltip();
  };

  toggle.addEventListener("click", (e) => {
    e.stopPropagation();
    const open = menu.classList.toggle("hidden");
    toggle.setAttribute("aria-expanded", open ? "false" : "true");
  });

  document.addEventListener("click", (e) => {
    if (!toggle.contains(e.target) && !menu.contains(e.target)) close();
  });
}

export function isSummaryCollapsed() {
  return document.getElementById("summary-panel")?.classList.contains("is-collapsed") ?? true;
}

export function initSummaryCollapse() {
  const panel = document.getElementById("summary-panel");
  const toggleBtn = document.getElementById("summary-toggle");
  if (!panel || !toggleBtn) return;

  try {
    const collapsed = localStorage.getItem(SUMMARY_COLLAPSED_KEY) !== "0";
    panel.classList.toggle("is-collapsed", collapsed);
    toggleBtn.setAttribute("aria-expanded", collapsed ? "false" : "true");
  } catch {
    panel.classList.add("is-collapsed");
  }

  toggleBtn.addEventListener("click", () => {
    const collapsed = panel.classList.toggle("is-collapsed");
    toggleBtn.setAttribute("aria-expanded", collapsed ? "false" : "true");
    try {
      localStorage.setItem(SUMMARY_COLLAPSED_KEY, collapsed ? "1" : "0");
    } catch {
      /* ignore */
    }
  });
}

export function expandPinnedPanel() {
  const panel = document.getElementById("pinned-panel");
  const toggleBtn = document.getElementById("pinned-toggle");
  if (!panel || !toggleBtn) return;
  panel.classList.remove("is-collapsed");
  toggleBtn.setAttribute("aria-expanded", "true");
  try {
    localStorage.setItem(PINNED_COLLAPSED_KEY, "0");
  } catch {
    /* ignore */
  }
}

export function initPinnedCollapse() {
  const panel = document.getElementById("pinned-panel");
  const toggleBtn = document.getElementById("pinned-toggle");
  if (!panel || !toggleBtn) return;

  try {
    const collapsed = localStorage.getItem(PINNED_COLLAPSED_KEY) !== "0";
    panel.classList.toggle("is-collapsed", collapsed);
    toggleBtn.setAttribute("aria-expanded", collapsed ? "false" : "true");
  } catch {
    panel.classList.add("is-collapsed");
  }

  toggleBtn.addEventListener("click", () => {
    const collapsed = panel.classList.toggle("is-collapsed");
    toggleBtn.setAttribute("aria-expanded", collapsed ? "false" : "true");
    document.body.classList.toggle("has-pinned-open", panel.classList.contains("has-pins") && !collapsed);
    try {
      localStorage.setItem(PINNED_COLLAPSED_KEY, collapsed ? "1" : "0");
    } catch {
      /* ignore */
    }
  });
}

export function setSummaryHint(text) {
  const el = document.getElementById("summary-hint");
  if (el) el.textContent = text;
}

export function formatSummaryTime() {
  return new Date().toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

export function setConfigOkClass(config) {
  const tray = config?.tray;
  const service = config?.service;
  const ok =
    tray?.stt_configured &&
    tray?.llm_configured &&
    service?.stt_configured &&
    service?.llm_configured &&
    config?.in_sync !== false;
  document.body.classList.toggle("config-ok", Boolean(ok));
}

function visibleTerms(termEntries) {
  return termEntries.filter((e) => !isTermDismissed(e.term));
}

/** 实时主区域：未固定、未「知道了」的条目 */
function liveTerms(termEntries) {
  return visibleTerms(termEntries).filter((e) => !e.userPinned);
}

function liveQuestions(questionEntries) {
  return questionEntries.filter(
    (e) => !e.userPinned && !isQuestionDismissed(e.question),
  );
}

function pinnedQuestions(questionEntries) {
  return questionEntries.filter(
    (e) => e.userPinned && !isQuestionDismissed(e.question),
  );
}

function countLiveTerms(termEntries) {
  return liveTerms(termEntries).length;
}

function countPinnedTerms(termEntries) {
  return termEntries.filter((e) => e.userPinned && !isTermDismissed(e.term)).length;
}

/** @param {object} ctx */
function updateInsightPanelCounts(ctx) {
  const qLive = liveQuestions(ctx.questionEntries).length;
  const qPinned = pinnedQuestions(ctx.questionEntries).length;
  const tLive = countLiveTerms(ctx.termEntries);
  const tPinned = countPinnedTerms(ctx.termEntries);

  const questionsCountEl = document.getElementById("questions-count");
  const termsCountEl = document.getElementById("terms-count");
  const pinnedCountsEl = document.getElementById("pinned-counts");

  if (questionsCountEl) {
    questionsCountEl.textContent = String(qLive);
    questionsCountEl.setAttribute(
      "aria-label",
      qPinned > 0 ? `当前问题 ${qLive} 条，另有 ${qPinned} 条已固定` : `当前问题 ${qLive} 条`,
    );
  }
  if (termsCountEl) {
    termsCountEl.textContent = String(tLive);
    termsCountEl.setAttribute(
      "aria-label",
      tPinned > 0 ? `术语 ${tLive} 条，另有 ${tPinned} 条已固定` : `术语 ${tLive} 条`,
    );
  }
  if (pinnedCountsEl) {
    if (qPinned === 0 && tPinned === 0) {
      pinnedCountsEl.textContent = "";
      pinnedCountsEl.setAttribute("aria-label", "暂无已固定");
    } else {
      pinnedCountsEl.textContent = ` · 问题 ${qPinned} · 术语 ${tPinned}`;
      pinnedCountsEl.setAttribute(
        "aria-label",
        `已固定：问题 ${qPinned} 条，术语 ${tPinned} 条`,
      );
    }
  }
}

function primaryEntries(entries, limit) {
  const sorted = [...entries].sort((a, b) => b.seq - a.seq);
  return sorted.slice(0, limit);
}

/** @param {object[]} entries @param {string} id */
function promoteEntryToTop(entries, id) {
  const item = entries.find((e) => e.id === id);
  if (!item) return;
  const maxSeq = entries.reduce((m, e) => Math.max(m, e.seq), 0);
  item.seq = maxSeq + 1;
}

/**
 * @param {object[]} entries
 * @param {string} id
 * @param {string} kind
 */
export function toggleUserPin(entries, id, kind, _dwellMs = 20_000) {
  const item = entries.find((e) => e.id === id);
  if (!item) return false;
  if (item.userPinned) {
    item.userPinned = false;
    item.pinned = false;
  } else {
    item.userPinned = true;
    item.pinned = true;
    expandPinnedPanel();
  }
  return true;
}

function copyText(text) {
  const t = String(text || "").trim();
  if (!t) return;
  navigator.clipboard?.writeText(t).catch(() => {});
}

/** @param {object} item @param {"term"|"question"} kind */
function buildInsightCopyText(item, kind) {
  const detail = String(item.detailText ?? "").trim();
  if (kind === "term") {
    const body =
      item.termLevel === "advanced" && item.advancedText
        ? item.advancedText
        : item.explanation;
    const parts = [`${item.term}\n${body}`];
    if (detail) parts.push(`\n更详细\n${detail}`);
    return parts.join("\n");
  }
  const parts = [`Q: ${item.question}\nA: ${item.answer}`];
  if (detail) parts.push(`\n回答思路\n${detail}`);
  return parts.join("\n");
}

/**
 * @param {HTMLElement} card
 * @param {object} item
 * @param {"term"|"question"} kind
 */
function appendDetailBlock(card, item, kind) {
  if (!item.detailExpanded) return;
  const detail = document.createElement("div");
  detail.className = "insight-detail";
  if (item.detailLoading) {
    detail.classList.add("loading");
    detail.textContent = kind === "question" ? "正在生成回答思路…" : "正在加载…";
  } else if (item.detailError) {
    detail.classList.add("error");
    detail.textContent = item.detailError;
  } else if (item.detailText) {
    detail.textContent = item.detailText;
  }
  card.appendChild(detail);
}

/**
 * @param {HTMLElement} actions
 * @param {object} item
 * @param {"term"|"question"} kind
 * @param {object} ctx
 * @param {{ inPinnedPanel?: boolean }} [opts]
 */
function appendActionButtons(actions, item, kind, ctx, opts = {}) {
  const rerender = () => ctx.rerender();
  const { inPinnedPanel = false } = opts;
  if (kind === "question") {
    const approachBtn = document.createElement("button");
    approachBtn.type = "button";
    approachBtn.className = "insight-more-btn";
    approachBtn.dataset.id = item.id;
    approachBtn.dataset.action = "approach";
    if (item.detailLoading) {
      approachBtn.textContent = "加载中…";
      approachBtn.disabled = true;
    } else if (item.detailExpanded) {
      approachBtn.textContent = "收起思路";
    } else {
      approachBtn.textContent = "回答思路";
    }
    setTip(approachBtn, "如何组织回答：结构、要点、示范开场");
    actions.appendChild(approachBtn);
  } else {
    const levelWrap = document.createElement("div");
    levelWrap.className = "insight-level-switch";
    const basicBtn = document.createElement("button");
    basicBtn.type = "button";
    basicBtn.className = "insight-level-btn";
    basicBtn.dataset.id = item.id;
    basicBtn.dataset.level = "basic";
    basicBtn.textContent = "扫盲";
    const advBtn = document.createElement("button");
    advBtn.type = "button";
    advBtn.className = "insight-level-btn";
    advBtn.dataset.id = item.id;
    advBtn.dataset.level = "advanced";
    advBtn.textContent = "进阶";
    const level = item.termLevel || "basic";
    basicBtn.classList.toggle("is-active", level === "basic");
    advBtn.classList.toggle("is-active", level === "advanced");
    if (item.advancedLoading) advBtn.disabled = true;
    setTip(basicBtn, "扫盲：听写时自动生成的简短解释，适合快速听懂");
    setTip(
      advBtn,
      item.advancedText
        ? "进阶：更深入的原理与对比（已缓存，可反复切换）"
        : "进阶：向 LLM 请求更技术向的说明（需已配置 Foundry）",
    );
    levelWrap.append(basicBtn, advBtn);
    actions.appendChild(levelWrap);

    const moreBtn = document.createElement("button");
    moreBtn.type = "button";
    moreBtn.className = "insight-more-btn";
    moreBtn.dataset.id = item.id;
    moreBtn.dataset.action = "more";
    moreBtn.textContent = item.detailExpanded ? "收起" : "更详细";
    actions.appendChild(moreBtn);
  }

  const copyBtn = document.createElement("button");
  copyBtn.type = "button";
  copyBtn.className = "insight-more-btn";
  copyBtn.textContent = "复制";
  copyBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    copyText(buildInsightCopyText(item, kind));
  });
  actions.appendChild(copyBtn);

  const pinBtn = document.createElement("button");
  pinBtn.type = "button";
  pinBtn.className = "insight-more-btn insight-pin-btn";
  pinBtn.dataset.id = item.id;
  pinBtn.dataset.action = "pin";
  pinBtn.dataset.kind = kind;
  pinBtn.textContent = item.userPinned ? "取消固定" : "📌 固定";
  setTip(
    pinBtn,
    item.userPinned
      ? "从底部「已固定」栏移除，恢复自动轮换"
      : "加入底部「已固定」栏，本场一直保留、可随时回看",
  );
  actions.appendChild(pinBtn);

  if (inPinnedPanel) {
    const promoteBtn = document.createElement("button");
    promoteBtn.type = "button";
    promoteBtn.className = "insight-more-btn";
    promoteBtn.dataset.id = item.id;
    promoteBtn.dataset.kind = kind;
    promoteBtn.dataset.action = "promote";
    promoteBtn.textContent = "主区显示";
    setTip(promoteBtn, "在开会/看课主区域置顶显示此条");
    actions.appendChild(promoteBtn);
  }

  if (kind === "term") {
    const dismissBtn = document.createElement("button");
    dismissBtn.type = "button";
    dismissBtn.className = "insight-more-btn insight-dismiss-btn";
    dismissBtn.textContent = "知道了";
    setTip(dismissBtn, "已懂，本会话不再显示该术语");
    dismissBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      dismissTermKey(item.term);
      ctx.removeTerm?.(item.term);
      rerender();
    });
    actions.appendChild(dismissBtn);
  } else if (kind === "question") {
    const dismissBtn = document.createElement("button");
    dismissBtn.type = "button";
    dismissBtn.className = "insight-more-btn insight-dismiss-btn";
    dismissBtn.textContent = "忽略";
    setTip(dismissBtn, "从「当前问题」列表移除；本场同问句不再显示");
    dismissBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      dismissQuestionKey(item.question);
      ctx.removeQuestion?.(item.question);
      rerender();
    });
    actions.appendChild(dismissBtn);
  }
}

/**
 * @param {object} ctx
 */
export function renderMeetingPanels(ctx) {
  const {
    termEntries,
    questionEntries,
    questionsListEl,
    insightsTermsList,
    questionsEmptyEl,
    insightsEmptyEl,
    formatInsightTime,
  } = ctx;

  const terms = liveTerms(termEntries);
  const qPrimary = primaryEntries(liveQuestions(questionEntries), Number.POSITIVE_INFINITY);
  const tShow = primaryEntries(terms, Number.POSITIVE_INFINITY);

  updateInsightPanelCounts(ctx);

  questionsEmptyEl?.classList.toggle("hidden", qPrimary.length > 0);
  insightsEmptyEl?.classList.toggle("hidden", tShow.length > 0);

  if (questionsListEl) {
    questionsListEl.replaceChildren();
    for (const item of qPrimary) {
      const card = document.createElement("div");
      card.className = "question-card";
      if (item.userPinned) card.classList.add("is-pinned");
      card.dataset.id = item.id;

      const q = document.createElement("div");
      q.className = "q";
      q.textContent = item.question;
      const a = document.createElement("div");
      a.className = "a";
      a.textContent = item.answer;

      const actions = document.createElement("div");
      actions.className = "insight-actions-row";
      appendActionButtons(actions, item, "question", ctx);

      card.append(q, a, actions);
      appendDetailBlock(card, item, "question");
      questionsListEl.appendChild(card);
    }
  }

  if (insightsTermsList) {
    insightsTermsList.replaceChildren();
    tShow.forEach((item, index) => {
      const card = document.createElement("div");
      card.className = "insight-card";
      if (item.userPinned) card.classList.add("is-pinned");
      if (index === 1) card.classList.add("is-secondary");
      card.dataset.id = item.id;

      const term = document.createElement("div");
      term.className = "term";
      term.textContent = item.term;
      const body = document.createElement("div");
      body.className = "insight-body";
      body.textContent = termBodyText(item);
      if (item.termLevel === "advanced" && item.advancedError) {
        body.classList.add("error");
      }

      const actions = document.createElement("div");
      actions.className = "insight-actions-row";
      appendActionButtons(actions, item, "term", ctx);

      card.append(term, body, actions);
      appendDetailBlock(card, item, "term");
      insightsTermsList.appendChild(card);
    });
  }
}

/** 看课模式：把「上一词」换到主卡位置（按 seq 降序，seq 大者为主卡） */
function swapGlancePrimary(ctx, mainId, prevId) {
  const mainE = ctx.termEntries.find((e) => e.id === mainId);
  const prevE = ctx.termEntries.find((e) => e.id === prevId);
  if (!mainE || !prevE || mainE.id === prevE.id) return;
  const tmp = mainE.seq;
  mainE.seq = prevE.seq;
  prevE.seq = tmp;
  ctx.rerender();
}

/**
 * @param {object} ctx
 */
export function renderGlancePanel(ctx) {
  const { termEntries, insightsTermsList, insightsEmptyEl } = ctx;
  const primary = document.getElementById("glance-primary");
  const prevRow = document.getElementById("glance-prev");
  if (!primary) return;

  const terms = liveTerms(termEntries);
  const sorted = [...terms].sort((a, b) => b.seq - a.seq);

  if (sorted.length === 0) {
    primary.replaceChildren();
    const p = document.createElement("p");
    p.className = "glance-empty";
    p.textContent = "听写识别到术语后会显示在此";
    primary.appendChild(p);
    prevRow?.classList.add("hidden");
    insightsEmptyEl?.classList.remove("hidden");
    return;
  }

  insightsEmptyEl?.classList.add("hidden");
  const main = sorted[0];
  const prev = sorted[1];

  primary.replaceChildren();
  const term = document.createElement("div");
  term.className = "term";
  term.textContent = main.term;
  const body = document.createElement("div");
  body.className = "insight-body";
  body.textContent = termBodyText(main);
  if (main.termLevel === "advanced" && main.advancedError) {
    body.classList.add("error");
  }
  const actions = document.createElement("div");
  actions.className = "insight-actions-row";
  appendActionButtons(actions, main, "term", ctx);
  primary.append(term, body, actions);
  appendDetailBlock(primary, main, "term");

  if (prev && prevRow) {
    prevRow.classList.remove("hidden");
    prevRow.dataset.swapMain = main.id;
    prevRow.dataset.swapPrev = prev.id;
    prevRow.textContent = `上一词：${prev.term} · ${prev.explanation.slice(0, 48)}${prev.explanation.length > 48 ? "…" : ""}`;
    setTip(prevRow, `点击将「${prev.term}」切换为主显示`);
  } else if (prevRow) {
    prevRow.classList.add("hidden");
    delete prevRow.dataset.swapMain;
    delete prevRow.dataset.swapPrev;
  }
}

/**
 * @param {object} item
 * @param {object} ctx
 */
function buildPinnedTermCard(item, ctx) {
  const card = document.createElement("div");
  card.className = "insight-card pinned-card is-pinned";
  card.dataset.id = item.id;
  card.dataset.kind = "term";

  const term = document.createElement("div");
  term.className = "term";
  term.textContent = item.term;
  const body = document.createElement("div");
  body.className = "insight-body";
  body.textContent = termBodyText(item);
  if (item.termLevel === "advanced" && item.advancedError) {
    body.classList.add("error");
  }

  const actions = document.createElement("div");
  actions.className = "insight-actions-row";
  appendActionButtons(actions, item, "term", ctx, { inPinnedPanel: true });

  card.append(term, body, actions);
  appendDetailBlock(card, item, "term");
  return card;
}

/**
 * @param {object} item
 * @param {object} ctx
 */
function buildPinnedQuestionCard(item, ctx) {
  const card = document.createElement("div");
  card.className = "question-card pinned-card is-pinned";
  card.dataset.id = item.id;
  card.dataset.kind = "question";

  const q = document.createElement("div");
  q.className = "q";
  q.textContent = item.question;
  const a = document.createElement("div");
  a.className = "a";
  a.textContent = item.answer;

  const actions = document.createElement("div");
  actions.className = "insight-actions-row";
  appendActionButtons(actions, item, "question", ctx, { inPinnedPanel: true });

  card.append(q, a, actions);
  appendDetailBlock(card, item, "question");
  return card;
}

/**
 * @param {object} ctx
 */
export function renderPinnedPanel(ctx) {
  const panel = document.getElementById("pinned-panel");
  const list = document.getElementById("pinned-list");
  const empty = document.getElementById("pinned-empty");
  const hint = document.getElementById("pinned-hint");
  if (!list) return;

  const pinnedTerms = ctx.termEntries.filter(
    (e) => e.userPinned && !isTermDismissed(e.term),
  );
  const pinnedQuestions = ctx.questionEntries.filter(
    (e) => e.userPinned && !isQuestionDismissed(e.question),
  );
  const total = pinnedTerms.length + pinnedQuestions.length;

  updateInsightPanelCounts(ctx);
  const expanded = panel && !panel.classList.contains("is-collapsed");

  if (hint) {
    hint.textContent =
      total > 0
        ? `${total} 条 · 扫盲/进阶 · 复制 · 更详细/回答思路`
        : "固定术语或问题，便于当场查看与会后回看";
  }

  empty?.classList.toggle("hidden", total > 0);
  list.replaceChildren();

  for (const item of pinnedTerms) {
    list.appendChild(buildPinnedTermCard(item, ctx));
  }
  for (const item of pinnedQuestions) {
    list.appendChild(buildPinnedQuestionCard(item, ctx));
  }

  panel?.classList.toggle("has-pins", total > 0);
  document.body.classList.toggle("has-pinned-open", total > 0 && expanded);
}

/**
 * @param {object} ctx
 */
export function renderInsightPanels(ctx) {
  renderPinnedPanel(ctx);
  if (loadUiMode() === "glance") {
    renderGlancePanel(ctx);
    return;
  }
  renderMeetingPanels(ctx);
}

/**
 * @param {object} item
 * @param {() => void} rerender
 */
/** @param {object} item */
function termBodyText(item) {
  if (item.termLevel !== "advanced") return item.explanation;
  if (item.advancedLoading) return "正在加载进阶说明…";
  if (item.advancedError) {
    return `进阶说明未能生成：${item.advancedError}（可点「扫盲」看简版，或稍后重试「进阶」）`;
  }
  if (item.advancedText?.trim()) return item.advancedText.trim();
  return "进阶说明为空，请重试「进阶」或点「扫盲」。";
}

export async function setTermLevel(item, level, rerender) {
  if (level === "basic") {
    item.termLevel = "basic";
    rerender();
    return;
  }
  item.termLevel = "advanced";
  item.advancedError = undefined;
  if (item.advancedText?.trim()) {
    rerender();
    return;
  }
  if (item.advancedLoading) return;
  item.advancedLoading = true;
  rerender();
  try {
    const text = await invoke("expand_insight", {
      kind: "term_advanced",
      subject: item.term,
      brief: item.explanation,
    });
    const trimmed = String(text ?? "").trim();
    if (!trimmed) {
      item.advancedError = "服务返回为空（请确认 LLM 已配置且服务在线）";
    } else {
      item.advancedText = trimmed;
    }
  } catch (err) {
    item.advancedError = String(err);
    // 保持「进阶」选中，便于用户看到失败原因而不是悄悄跳回扫盲
  } finally {
    item.advancedLoading = false;
    rerender();
  }
}

/**
 * @param {object} item
 * @param {() => void} rerender
 */
export async function toggleQuestionApproach(item, rerender) {
  if (item.detailExpanded) {
    item.detailExpanded = false;
    rerender();
    return;
  }
  if (item.detailText) {
    item.detailExpanded = true;
    rerender();
    return;
  }
  if (item.detailLoading) return;
  item.detailLoading = true;
  item.detailExpanded = true;
  item.detailError = undefined;
  rerender();
  try {
    item.detailText = await invoke("expand_insight", {
      kind: "question_approach",
      subject: item.question,
      brief: item.answer,
    });
  } catch (err) {
    item.detailError = String(err);
  } finally {
    item.detailLoading = false;
    rerender();
  }
}

/**
 * @param {HTMLElement} container
 * @param {object} ctx
 */
export function setupInsightInteractions(container, ctx) {
  if (!container) return;

  container.addEventListener("click", async (e) => {
    const prevRow = e.target.closest("#glance-prev");
    if (
      prevRow &&
      !prevRow.classList.contains("hidden") &&
      prevRow.dataset.swapMain &&
      prevRow.dataset.swapPrev
    ) {
      e.preventDefault();
      swapGlancePrimary(ctx, prevRow.dataset.swapMain, prevRow.dataset.swapPrev);
      return;
    }

    const levelBtn = e.target.closest(".insight-level-btn");
    if (levelBtn?.dataset.id) {
      e.stopPropagation();
      const item = ctx.termEntries.find((x) => x.id === levelBtn.dataset.id);
      if (!item) return;
      await setTermLevel(item, levelBtn.dataset.level, () => ctx.rerender());
      return;
    }

    const moreBtn = e.target.closest(".insight-more-btn");
    if (moreBtn?.dataset.id && moreBtn.dataset.action === "pin") {
      e.stopPropagation();
      const kind = moreBtn.dataset.kind === "question" ? "question" : "term";
      const entries =
        kind === "question" ? ctx.questionEntries : ctx.termEntries;
      if (toggleUserPin(entries, moreBtn.dataset.id, kind, ctx.insightDwellMs)) {
        ctx.rerender();
      }
      return;
    }

    if (moreBtn?.dataset.id && moreBtn.dataset.action === "promote") {
      e.stopPropagation();
      const kind = moreBtn.dataset.kind === "question" ? "question" : "term";
      const entries =
        kind === "question" ? ctx.questionEntries : ctx.termEntries;
      promoteEntryToTop(entries, moreBtn.dataset.id);
      ctx.rerender();
      return;
    }

    if (moreBtn?.dataset.id && moreBtn.dataset.action === "approach") {
      e.stopPropagation();
      const item = ctx.questionEntries.find((x) => x.id === moreBtn.dataset.id);
      if (item) await toggleQuestionApproach(item, () => ctx.rerender());
      return;
    }

    if (moreBtn?.dataset.id && moreBtn.dataset.action === "more") {
      e.stopPropagation();
      await ctx.toggleDetail("term", moreBtn.dataset.id);
      return;
    }
  });

  container.addEventListener("keydown", (e) => {
    if (e.target?.id !== "glance-prev") return;
    if (e.key !== "Enter" && e.key !== " ") return;
    const row = e.target;
    if (
      row.classList.contains("hidden") ||
      !row.dataset.swapMain ||
      !row.dataset.swapPrev
    ) {
      return;
    }
    e.preventDefault();
    swapGlancePrimary(ctx, row.dataset.swapMain, row.dataset.swapPrev);
  });
}
