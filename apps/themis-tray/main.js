import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { applyConfigStatusEl, listMissingConfigItems } from "./config-status.js";
import {
  applyCaptureStatusEl,
  applyCaptureStatusPending,
} from "./capture-status.js";
import { initTooltips, setTip, tipWithHotkey, hotkey } from "./tooltips.js";

const overlayEl = document.getElementById("overlay");
const dragHandle = document.getElementById("drag-handle");
const themeBadgeEl = document.getElementById("theme-badge");
const statusEl = document.getElementById("status");
const configStatusEl = document.getElementById("config-status");
const scrollEl = document.getElementById("transcript-scroll");
const transcriptEl = document.getElementById("transcript");
const scrollLatestBtn = document.getElementById("scroll-latest");
const clearSessionBtn = document.getElementById("clear-session");
const toggleCaptureBtn = document.getElementById("toggle-capture");
const toggleDiagnoseBtn = document.getElementById("toggle-diagnose");
const toggleSettingsBtn = document.getElementById("toggle-settings");
const toggleLocalizeBtn = document.getElementById("toggle-localize");
const toggleMiniBtn = document.getElementById("toggle-mini");
const hideOverlayBtn = document.getElementById("hide-overlay");
const quitAppBtn = document.getElementById("quit-app");
const miniFloaterEl = document.getElementById("mini-floater");
const sizePresetsEl = document.getElementById("size-presets");
const sizeToggleBtn = document.getElementById("size-toggle");
const sizeMenuEl = document.getElementById("size-menu");
const toggleTranscriptBtn = document.getElementById("toggle-transcript");
const opacityDownBtn = document.getElementById("opacity-down");
const opacityUpBtn = document.getElementById("opacity-up");
const fontDownBtn = document.getElementById("font-down");
const fontUpBtn = document.getElementById("font-up");
const fontResetBtn = document.getElementById("font-reset");

const FONT_SCALE_MIN = 0.75;
const FONT_SCALE_MAX = 1.5;
const OPACITY_MIN = 0.35;
const OPACITY_MAX = 1;
const OPACITY_STEP = 0.05;
const FONT_SCALE_STEP = 0.1;

initTooltips();

function initHeaderTips() {
  setTip(sizeToggleBtn, "窗口尺寸预设");
  setTip(clearSessionBtn, "清空字幕、总结与洞察，从零继续监听");
  setTip(hideOverlayBtn, tipWithHotkey("隐藏窗口（捕捉继续，托盘可再次打开）", "O"));
  setTip(quitAppBtn, tipWithHotkey("退出 Themis（停止托盘与捕捉）", "Q"));
  setTip(scrollLatestBtn, "跳转到最新字幕");
  setTip(toggleMiniBtn, tipWithHotkey("最小化为桌面浮标，全屏应用上仍可见", "M"));
  setTip(opacityDownBtn, tipWithHotkey("降低浮层透明度", "["));
  setTip(opacityUpBtn, tipWithHotkey("提高浮层透明度", "]"));
  setTip(fontDownBtn, tipWithHotkey("缩小字号", "−"));
  setTip(fontUpBtn, tipWithHotkey("放大字号", "+"));
  setTip(fontResetBtn, tipWithHotkey("重置字号为 100%", "0"));
  setTip(
    miniFloaterEl,
    `拖动移动 · 点击恢复 · ${hotkey("M")} 浮标 / ${hotkey("O")} 唤醒`,
  );
  setTip(document.getElementById("middle-divider"), "拖动调整 Questions / Terms 宽度");
  setTip(document.getElementById("stack-divider"), "拖动调整字幕区高度");
}

initHeaderTips();

const WINDOW_PRESET_STORAGE_KEY = "themis-window-preset";
const TRANSCRIPT_VISIBLE_STORAGE_KEY = "themis-transcript-visible";

/** Whether the live transcript column is shown. */
let transcriptVisible = true;
const insightsEmptyEl = document.getElementById("insights-empty");
const insightsKeywordsSec = document.getElementById("insights-keywords");
const insightsKeywordsList = document.getElementById("insights-keywords-list");
const keywordsToggleBtn = document.getElementById("keywords-toggle");
const keywordsCountEl = document.getElementById("keywords-count");
const KEYWORDS_COLLAPSED_STORAGE_KEY = "themis-keywords-collapsed";
const insightsTermsSec = document.getElementById("insights-terms");
const insightsTermsList = document.getElementById("insights-terms-list");
const questionsEmptyEl = document.getElementById("questions-empty");
const questionsListEl = document.getElementById("questions-list");
const questionsPanelEl = document.getElementById("questions-panel");
const layoutBodyEl = document.getElementById("layout-body");
const middleRowEl = document.getElementById("middle-row");
const middleDividerEl = document.getElementById("middle-divider");
const stackDividerEl = document.getElementById("stack-divider");
const transcriptBlockEl = document.getElementById("transcript-block");
const transcriptSectionEl = document.getElementById("transcript-section");
const insightsPanelEl = document.getElementById("insights-panel");
const summaryEmptyEl = document.getElementById("summary-empty");
const summaryTextEl = document.getElementById("summary-text");

/** @type {string[]} Final lines (one per Azure REST phrase). */
let committedLines = [];
/** @type {Map<string, object|null>} line text → latest insights */
const lineInsights = new Map();
/** Latest partial hypothesis while speaking. */
let partialText = "";

/** User scrolled up — pause auto-follow until they click Latest or scroll to bottom. */
let followLatest = true;

/** Minimum visible time for each term/question card (ms); loaded from .env via tray. */
let insightDwellMs = 20_000;
/** Whether term/Q&A explanations are localized to Chinese on the service side. */
let insightLocalizeZh = true;

let termSeq = 0;
let questionSeq = 0;
/** @type {Array<{id: string, seq: number, addedAt: number, expiresAt: number, pinned: boolean, userPinned: boolean, term: string, explanation: string, detailText?: string, detailExpanded?: boolean, detailLoading?: boolean, detailError?: string}>} */
const termEntries = [];
/** @type {Array<{id: string, seq: number, addedAt: number, expiresAt: number, pinned: boolean, userPinned: boolean, question: string, answer: string, detailText?: string, detailExpanded?: boolean, detailLoading?: boolean, detailError?: string}>} */
const questionEntries = [];
/** @type {ReturnType<typeof setInterval> | null} */
let insightPruneTimer = null;

const SCROLL_BOTTOM_THRESHOLD = 48;

const TRANSCRIPT_HEIGHT_STORAGE_KEY = "themis-transcript-panel-height";
const MIDDLE_WIDTH_STORAGE_KEY = "themis-questions-panel-width";
const TRANSCRIPT_PANEL_MIN = 80;
const MIDDLE_ROW_MIN = 120;
const QUESTIONS_PANEL_MIN = 160;
const INSIGHTS_PANEL_MIN = 120;
const STACK_DIVIDER_HEIGHT = 8;
const MIDDLE_DIVIDER_WIDTH = 8;

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

function applyMiniMode(active) {
  document.body.classList.toggle("is-mini-mode", active);
  document.documentElement.classList.toggle("is-mini-mode", active);
  miniFloaterEl?.classList.toggle("hidden", !active);
  if (active) {
    document.documentElement.style.background = "transparent";
    document.body.style.background = "transparent";
  } else {
    document.documentElement.style.background = "";
    document.body.style.background = "";
  }
}

async function syncMiniMode() {
  try {
    const active = await invoke("is_overlay_mini_mode");
    applyMiniMode(Boolean(active));
  } catch {
    /* not in tauri shell */
  }
}

function setupMiniFloater() {
  if (!miniFloaterEl) return;

  /** @type {{ dragging: boolean, x: number, y: number, id: number } | null} */
  let press = null;

  const clearPress = () => {
    press = null;
  };

  const finishPress = async (e) => {
    if (!press) return;
    if (e.pointerId !== undefined && e.pointerId !== press.id) return;
    const moved =
      press.dragging || Math.hypot(e.screenX - press.x, e.screenY - press.y) > 6;
    clearPress();
    if (!moved) {
      try {
        await invoke("toggle_overlay_mini_mode");
      } catch (err) {
        if (statusEl) setTip(statusEl, String(err));
      }
    }
  };

  miniFloaterEl.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    press = { dragging: false, x: e.screenX, y: e.screenY, id: e.pointerId };
  });

  miniFloaterEl.addEventListener("pointermove", (e) => {
    if (!press || e.pointerId !== press.id || press.dragging) return;
    if (Math.hypot(e.screenX - press.x, e.screenY - press.y) > 6) {
      press.dragging = true;
      getCurrentWindow().startDragging().catch(() => {});
    }
  });

  window.addEventListener("pointerup", finishPress);
  window.addEventListener("pointercancel", clearPress);

  miniFloaterEl.addEventListener("keydown", async (e) => {
    if (e.key !== "Enter" && e.key !== " ") return;
    e.preventDefault();
    try {
      await invoke("toggle_overlay_mini_mode");
    } catch (err) {
      if (statusEl) setTip(statusEl, String(err));
    }
  });
}

setupMiniFloater();

toggleMiniBtn?.addEventListener("click", async () => {
  try {
    await invoke("toggle_overlay_mini_mode");
  } catch (e) {
    setTip(statusEl, String(e));
  }
});

hideOverlayBtn?.addEventListener("click", async () => {
  try {
    await invoke("hide_overlay_window");
  } catch (e) {
    setTip(statusEl, String(e));
  }
});

quitAppBtn?.addEventListener("click", async () => {
  try {
    await invoke("quit_app");
  } catch (e) {
    setTip(statusEl, String(e));
  }
});

async function adjustOpacity(delta) {
  try {
    await invoke("adjust_overlay_opacity", { delta });
  } catch (e) {
    setTip(statusEl, String(e));
  }
}

opacityDownBtn?.addEventListener("click", () => adjustOpacity(-OPACITY_STEP));
opacityUpBtn?.addEventListener("click", () => adjustOpacity(OPACITY_STEP));

async function adjustFontScale(delta) {
  try {
    await invoke("adjust_overlay_font_scale", { delta });
  } catch (e) {
    setTip(statusEl, String(e));
  }
}

async function resetFontScale() {
  try {
    await invoke("reset_overlay_font_scale");
  } catch (e) {
    setTip(statusEl, String(e));
  }
}

fontDownBtn?.addEventListener("click", () => adjustFontScale(-FONT_SCALE_STEP));
fontUpBtn?.addEventListener("click", () => adjustFontScale(FONT_SCALE_STEP));
fontResetBtn?.addEventListener("click", () => resetFontScale());

listen("mini-mode-changed", (event) => {
  applyMiniMode(Boolean(event.payload));
});

function clampTranscriptHeight(heightPx) {
  if (!layoutBodyEl || !transcriptBlockEl) return heightPx;
  const total = layoutBodyEl.clientHeight;
  const max = total - MIDDLE_ROW_MIN;
  return Math.round(Math.max(TRANSCRIPT_PANEL_MIN, Math.min(heightPx, max)));
}

function applyTranscriptHeight(heightPx) {
  if (!transcriptBlockEl) return;
  const clamped = clampTranscriptHeight(heightPx);
  transcriptBlockEl.style.flex = `0 0 ${clamped}px`;
  transcriptBlockEl.style.height = `${clamped}px`;
  transcriptBlockEl.style.maxHeight = "none";
}

function clampQuestionsWidth(widthPx) {
  if (!middleRowEl || !questionsPanelEl) return widthPx;
  const total = middleRowEl.clientWidth;
  const max = total - INSIGHTS_PANEL_MIN - MIDDLE_DIVIDER_WIDTH;
  return Math.round(Math.max(QUESTIONS_PANEL_MIN, Math.min(widthPx, max)));
}

function applyQuestionsWidth(widthPx) {
  if (!questionsPanelEl) return;
  const clamped = clampQuestionsWidth(widthPx);
  questionsPanelEl.style.flex = `0 0 ${clamped}px`;
  questionsPanelEl.style.width = `${clamped}px`;
  questionsPanelEl.style.maxWidth = "none";
}

function initStackDivider() {
  if (!layoutBodyEl || !stackDividerEl || !transcriptBlockEl) return;

  const saved = localStorage.getItem(TRANSCRIPT_HEIGHT_STORAGE_KEY);
  if (saved) {
    const parsed = Number(saved);
    if (Number.isFinite(parsed) && parsed > 0) {
      applyTranscriptHeight(parsed);
    }
  } else if (layoutBodyEl.clientHeight > 0) {
    applyTranscriptHeight(Math.round(layoutBodyEl.clientHeight * 0.35));
  }

  let dragging = false;

  stackDividerEl.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    dragging = true;
    stackDividerEl.classList.add("is-dragging");
    document.body.classList.add("stack-dragging");
  });

  window.addEventListener("mousemove", (e) => {
    if (!dragging) return;
    const rect = layoutBodyEl.getBoundingClientRect();
    applyTranscriptHeight(rect.bottom - e.clientY);
  });

  const stopDrag = () => {
    if (!dragging) return;
    dragging = false;
    stackDividerEl.classList.remove("is-dragging");
    document.body.classList.remove("stack-dragging");
    localStorage.setItem(TRANSCRIPT_HEIGHT_STORAGE_KEY, String(transcriptBlockEl.offsetHeight));
  };

  window.addEventListener("mouseup", stopDrag);
  window.addEventListener("blur", stopDrag);

  window.addEventListener("resize", () => {
    if (transcriptBlockEl.offsetHeight > 0) {
      applyTranscriptHeight(transcriptBlockEl.offsetHeight);
    } else if (layoutBodyEl.clientHeight > 0) {
      applyTranscriptHeight(Math.round(layoutBodyEl.clientHeight * 0.35));
    }
  });
}

function initMiddleDivider() {
  if (!middleRowEl || !middleDividerEl || !questionsPanelEl) return;

  const saved = localStorage.getItem(MIDDLE_WIDTH_STORAGE_KEY);
  if (saved) {
    const parsed = Number(saved);
    if (Number.isFinite(parsed) && parsed > 0) {
      applyQuestionsWidth(parsed);
    }
  } else if (middleRowEl.clientWidth > 0) {
    applyQuestionsWidth(Math.round(middleRowEl.clientWidth * 0.5));
  }

  let dragging = false;

  middleDividerEl.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    dragging = true;
    middleDividerEl.classList.add("is-dragging");
    document.body.classList.add("middle-dragging");
  });

  window.addEventListener("mousemove", (e) => {
    if (!dragging) return;
    const rect = middleRowEl.getBoundingClientRect();
    applyQuestionsWidth(e.clientX - rect.left);
  });

  const stopDrag = () => {
    if (!dragging) return;
    dragging = false;
    middleDividerEl.classList.remove("is-dragging");
    document.body.classList.remove("middle-dragging");
    localStorage.setItem(MIDDLE_WIDTH_STORAGE_KEY, String(questionsPanelEl.offsetWidth));
  };

  window.addEventListener("mouseup", stopDrag);
  window.addEventListener("blur", stopDrag);

  window.addEventListener("resize", () => {
    if (questionsPanelEl.offsetWidth > 0) {
      applyQuestionsWidth(questionsPanelEl.offsetWidth);
    } else if (middleRowEl.clientWidth > 0) {
      applyQuestionsWidth(Math.round(middleRowEl.clientWidth * 0.5));
    }
  });
}

initStackDivider();
initMiddleDivider();

function closeSizeMenu() {
  sizeMenuEl?.classList.add("hidden");
  sizeToggleBtn?.setAttribute("aria-expanded", "false");
}

function markActiveSizePreset(presetId) {
  if (!sizeMenuEl) return;
  for (const btn of sizeMenuEl.querySelectorAll("[data-preset]")) {
    btn.classList.toggle("is-active", btn.dataset.preset === presetId);
  }
}

async function applyWindowPreset(presetId) {
  try {
    const applied = await invoke("apply_window_preset", { preset: presetId });
    localStorage.setItem(WINDOW_PRESET_STORAGE_KEY, applied);
    markActiveSizePreset(applied);
    closeSizeMenu();
  } catch (e) {
    setTip(statusEl, String(e));
  }
}

async function initSizePresets() {
  if (!sizeMenuEl || !sizeToggleBtn) return;
  let presets = [];
  try {
    presets = await invoke("list_window_presets");
  } catch {
    return;
  }

  sizeMenuEl.replaceChildren();
  for (const p of presets) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "size-menu-item";
    btn.dataset.preset = p.id;
    btn.setAttribute("role", "menuitem");
    setTip(
      btn,
      p.fullscreen
        ? "全屏"
        : p.id === "center-third"
          ? "宽 1/3 屏 × 工作区全高，水平居中"
          : p.id === "current-screen"
            ? "铺满当前显示器工作区（保留菜单栏/程序坞区域）"
            : `${p.width}×${p.height}`,
    );
    btn.textContent =
      p.fullscreen || p.id === "center-third" || p.id === "current-screen"
        ? p.label
        : `${p.label} ${p.width}×${p.height}`;
    btn.addEventListener("click", (e) => {
      e.stopPropagation();
      applyWindowPreset(p.id);
    });
    sizeMenuEl.appendChild(btn);
  }

  sizeToggleBtn.addEventListener("click", (e) => {
    e.stopPropagation();
    const open = sizeMenuEl.classList.toggle("hidden");
    sizeToggleBtn.setAttribute("aria-expanded", open ? "false" : "true");
  });

  document.addEventListener("click", (e) => {
    if (!sizePresetsEl?.contains(e.target)) {
      closeSizeMenu();
    }
  });

  let saved = localStorage.getItem(WINDOW_PRESET_STORAGE_KEY) || "center-third";
  if (saved === "center-quarter") {
    saved = "center-third";
    localStorage.setItem(WINDOW_PRESET_STORAGE_KEY, saved);
  }
  markActiveSizePreset(saved);
  try {
    await invoke("apply_window_preset", { preset: saved });
  } catch {
    /* browser preview */
  }
}

initSizePresets();

function applyTranscriptVisible(visible) {
  transcriptVisible = visible;
  layoutBodyEl?.classList.toggle("transcript-hidden", !visible);
  if (toggleTranscriptBtn) {
    toggleTranscriptBtn.textContent = visible ? "▾" : "▴";
    setTip(
      toggleTranscriptBtn,
      visible
        ? tipWithHotkey("隐藏实时字幕", "H")
        : tipWithHotkey("显示实时字幕", "H"),
    );
    toggleTranscriptBtn.setAttribute("aria-label", visible ? "隐藏实时字幕" : "显示实时字幕");
    toggleTranscriptBtn.setAttribute("aria-pressed", visible ? "false" : "true");
  }
  if (!visible) {
    scrollLatestBtn?.classList.add("hidden");
    if (transcriptBlockEl) {
      transcriptBlockEl.style.flex = "";
      transcriptBlockEl.style.height = "";
    }
  } else {
    if (followLatest) {
      scrollLatestBtn?.classList.add("hidden");
    } else {
      scrollLatestBtn?.classList.remove("hidden");
    }
    const saved = localStorage.getItem(TRANSCRIPT_HEIGHT_STORAGE_KEY);
    if (saved) {
      const parsed = Number(saved);
      if (Number.isFinite(parsed) && parsed > 0) {
        applyTranscriptHeight(parsed);
      }
    } else if (layoutBodyEl?.clientHeight > 0) {
      applyTranscriptHeight(Math.round(layoutBodyEl.clientHeight * 0.35));
    }
  }
  localStorage.setItem(TRANSCRIPT_VISIBLE_STORAGE_KEY, visible ? "1" : "0");
}

function toggleTranscriptPanel() {
  applyTranscriptVisible(!transcriptVisible);
}

applyTranscriptVisible(localStorage.getItem(TRANSCRIPT_VISIBLE_STORAGE_KEY) !== "0");

toggleTranscriptBtn?.addEventListener("click", (e) => {
  e.stopPropagation();
  closeSizeMenu();
  toggleTranscriptPanel();
});

listen("toggle-transcript-panel", () => {
  toggleTranscriptPanel();
});

listen("overlay-woken", () => {
  const pulseTarget =
    document.body.classList.contains("is-mini-mode") ? miniFloaterEl : overlayEl;
  if (!pulseTarget) return;
  pulseTarget.classList.remove("overlay-wake-pulse");
  void pulseTarget.offsetWidth;
  pulseTarget.classList.add("overlay-wake-pulse");
  window.setTimeout(() => pulseTarget.classList.remove("overlay-wake-pulse"), 1300);
});

listen("window-preset-applied", (event) => {
  const presetId = event.payload;
  if (!presetId) return;
  localStorage.setItem(WINDOW_PRESET_STORAGE_KEY, presetId);
  markActiveSizePreset(presetId);
});

function renderSessionSummary(summary) {
  const text = String(summary ?? "").trim();
  if (!text) {
    resetSessionSummary();
    return;
  }
  summaryEmptyEl.classList.add("hidden");
  summaryTextEl.classList.remove("hidden");
  summaryTextEl.textContent = text;
}

function resetSessionSummary() {
  summaryEmptyEl.classList.remove("hidden");
  summaryTextEl.classList.add("hidden");
  summaryTextEl.textContent = "";
}

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
      setTip(kw, "Keywords");
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

async function refreshConfigStatus(configFromStatus) {
  try {
    const config =
      configFromStatus ?? (await invoke("get_config_crosscheck"));
    applyConfigStatusEl(configStatusEl, config);
  } catch {
    applyConfigStatusEl(configStatusEl, null);
  }
}

/** @type {"starting" | "stopping" | null} */
let capturePending = null;

function beginCapturePending(action) {
  if (capturePending === action) return;
  capturePending = action;
  applyCaptureStatusPending(statusEl, action);
  updateCaptureButtonPending(action);
  if (action === "starting") {
    setPlaceholder("Starting capture…");
  }
}

function endCapturePending() {
  capturePending = null;
  toggleCaptureBtn?.classList.remove("is-busy");
}

async function refreshStatus() {
  if (capturePending) return;
  try {
    const s = await invoke("get_status");
    applyCaptureStatusEl(statusEl, s);
    applyConfigStatusEl(configStatusEl, s.config);
    updateCaptureButton(s.state === "capturing");

    if (s.state === "idle") {
      const missing = listMissingConfigItems(s.config?.tray);
      const azureMissing = missing.filter((m) => m.startsWith("Azure"));
      if (azureMissing.length) {
        setPlaceholder(
          `未配置：${azureMissing.join("；")}。请点击标题栏「配置」填写并保存（无需先复制 .env）。准备好后按 Ctrl+Shift+T / Cmd+Shift+T 开始采集。`,
        );
      } else {
        setPlaceholder(
          "Service online — press Cmd+Shift+T (macOS) or Ctrl+Shift+T (Windows) to start capture.",
        );
      }
    } else if (s.state === "capturing" && committedLines.length === 0 && !partialText) {
      let hint = "Listening… new lines appear below; view scrolls to latest.";
      if (s.message?.includes("no loopback signal") || s.message?.includes("signal=silent")) {
        hint =
          "No audio signal (peak low). macOS: route output to BlackHole and set Input=BlackHole; allow Microphone permission.";
      } else if (s.message?.includes("signal=strong") || s.message?.includes("signal=ok")) {
        hint = "Capture OK (see status peak/frames). Transcript appears here every ~2s when speech is detected…";
      } else if ((s.audio_peak ?? 0) === 0 && (s.audio_frames ?? 0) === 0) {
        hint = "Capturing but no frames yet — check input device in status tooltip.";
      }
      setPlaceholder(hint);
    }
  } catch (e) {
    applyCaptureStatusEl(statusEl, { offline: true, error: String(e) });
    await refreshConfigStatus();
  }
}

function updateCaptureButton(capturing) {
  if (!toggleCaptureBtn) return;
  toggleCaptureBtn.classList.remove("is-busy");
  toggleCaptureBtn.disabled = false;
  toggleCaptureBtn.classList.toggle("is-capturing", capturing);
  toggleCaptureBtn.textContent = capturing ? "停止" : "捕捉";
  setTip(
    toggleCaptureBtn,
    capturing
      ? tipWithHotkey("停止系统音频捕捉", "T")
      : tipWithHotkey("开始系统音频捕捉", "T"),
  );
  toggleCaptureBtn.setAttribute("aria-pressed", capturing ? "true" : "false");
}

/** @param {"starting" | "stopping"} action */
function updateCaptureButtonPending(action) {
  if (!toggleCaptureBtn) return;
  toggleCaptureBtn.classList.add("is-busy");
  toggleCaptureBtn.disabled = true;
  toggleCaptureBtn.classList.toggle("is-capturing", action === "stopping");
  toggleCaptureBtn.textContent = action === "stopping" ? "停止中…" : "启动中…";
  setTip(
    toggleCaptureBtn,
    action === "stopping" ? "正在停止捕捉…" : "正在启动捕捉…",
  );
  toggleCaptureBtn.setAttribute(
    "aria-pressed",
    action === "stopping" ? "true" : "false",
  );
}

function updateDiagnoseButton(open) {
  if (!toggleDiagnoseBtn) return;
  toggleDiagnoseBtn.classList.toggle("is-open", open);
  setTip(
    toggleDiagnoseBtn,
    open ? tipWithHotkey("关闭诊断窗口", "D") : tipWithHotkey("打开诊断窗口", "D"),
  );
  toggleDiagnoseBtn.setAttribute("aria-pressed", open ? "true" : "false");
}

async function syncDiagnoseButton() {
  try {
    const open = await invoke("is_diagnose_visible");
    updateDiagnoseButton(Boolean(open));
  } catch {
    /* not in tauri shell */
  }
}

function updateSettingsButton(open) {
  if (!toggleSettingsBtn) return;
  toggleSettingsBtn.classList.toggle("is-open", open);
  setTip(
    toggleSettingsBtn,
    open ? "关闭配置窗口" : "配置 Azure Speech / Foundry（保存后自动写入 .env）",
  );
  toggleSettingsBtn.setAttribute("aria-pressed", open ? "true" : "false");
}

async function syncSettingsButton() {
  try {
    const open = await invoke("is_settings_visible");
    updateSettingsButton(Boolean(open));
  } catch {
    /* not in tauri shell */
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
  if (!prev || prev.userPinned) return;
  prev.pinned = false;
  prev.expiresAt = now + insightDwellMs;
}

function toggleEntryPin(entries, id) {
  const item = entries.find((e) => e.id === id);
  if (!item) return false;
  if (item.pinned) {
    item.pinned = false;
    item.userPinned = false;
    item.expiresAt = Date.now() + insightDwellMs;
  } else {
    item.pinned = true;
    item.userPinned = true;
  }
  return true;
}

function setupInsightCardPin() {
  questionsListEl.addEventListener("click", (e) => {
    const moreBtn = e.target.closest(".insight-more-btn");
    if (moreBtn?.dataset.id) {
      e.stopPropagation();
      toggleInsightDetail("question", moreBtn.dataset.id);
      return;
    }
    const card = e.target.closest(".question-card");
    if (!card?.dataset.id) return;
    if (toggleEntryPin(questionEntries, card.dataset.id)) {
      renderInsightPanels();
    }
  });
  insightsTermsList.addEventListener("click", (e) => {
    const moreBtn = e.target.closest(".insight-more-btn");
    if (moreBtn?.dataset.id) {
      e.stopPropagation();
      toggleInsightDetail("term", moreBtn.dataset.id);
      return;
    }
    const card = e.target.closest(".insight-card");
    if (!card?.dataset.id) return;
    if (toggleEntryPin(termEntries, card.dataset.id)) {
      renderInsightPanels();
    }
  });
}

setupInsightCardPin();

function pruneEntryList(entries) {
  const now = Date.now();
  let changed = false;
  if (entries.length > 0) {
    const head = entries[0];
    if (!head.pinned && now - head.addedAt >= insightDwellMs) {
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
    renderInsightPanels();
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
      expiresAt: now + insightDwellMs,
      pinned: false,
      userPinned: false,
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
      expiresAt: now + insightDwellMs,
      pinned: false,
      userPinned: false,
      question: q.question,
      answer: q.answer,
    });
    added = true;
  }
  return added;
}

function sortInsightEntries(entries) {
  return [...entries].sort((a, b) => {
    if (a.pinned !== b.pinned) return a.pinned ? -1 : 1;
    return b.seq - a.seq;
  });
}

function appendInsightMoreSection(card, item) {
  const actions = document.createElement("div");
  actions.className = "insight-actions";
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "insight-more-btn";
  btn.dataset.id = item.id;
  if (item.detailLoading) {
    btn.textContent = "加载中…";
    btn.disabled = true;
  } else if (item.detailExpanded) {
    btn.textContent = "收起";
  } else {
    btn.textContent = "更详细";
  }
  actions.appendChild(btn);
  card.appendChild(actions);

  if (item.detailExpanded) {
    const detail = document.createElement("div");
    detail.className = "insight-detail";
    if (item.detailLoading) {
      detail.classList.add("loading");
      detail.textContent = "正在生成详细说明…";
    } else if (item.detailError) {
      detail.classList.add("error");
      detail.textContent = item.detailError;
    } else if (item.detailText) {
      detail.textContent = item.detailText;
    }
    card.appendChild(detail);
  }
}

async function toggleInsightDetail(kind, id) {
  const entries = kind === "term" ? termEntries : questionEntries;
  const item = entries.find((e) => e.id === id);
  if (!item) return;

  if (item.detailExpanded) {
    item.detailExpanded = false;
    renderInsightPanels();
    return;
  }

  if (item.detailText) {
    item.detailExpanded = true;
    renderInsightPanels();
    return;
  }

  if (item.detailLoading) return;

  item.detailLoading = true;
  item.detailExpanded = true;
  item.detailError = undefined;
  renderInsightPanels();

  const subject = kind === "term" ? item.term : item.question;
  const brief = kind === "term" ? item.explanation : item.answer;

  try {
    item.detailText = await invoke("expand_insight", { kind, subject, brief });
  } catch (err) {
    item.detailError = String(err);
  } finally {
    item.detailLoading = false;
    renderInsightPanels();
  }
}

function renderInsightPanels() {
  const hasTerms = termEntries.length > 0;
  const hasQuestions = questionEntries.length > 0;
  const hasKeywords = insightsKeywordsList.childElementCount > 0;

  questionsEmptyEl.classList.toggle("hidden", hasQuestions);
  questionsListEl.replaceChildren();
  for (const item of sortInsightEntries(questionEntries)) {
    const card = document.createElement("div");
    card.className = "question-card";
    if (item.pinned) card.classList.add("is-pinned");
    card.dataset.id = item.id;
    setTip(card, item.pinned ? "点击取消固定" : "点击固定（不会被自动移除）");
    const meta = document.createElement("div");
    meta.className = "insight-meta";
    const pinLabel = item.pinned ? " · 📌" : "";
    meta.textContent = `#${item.seq} · ${formatInsightTime(item.addedAt)}${pinLabel}`;
    const q = document.createElement("div");
    q.className = "q";
    q.textContent = item.question;
    const a = document.createElement("div");
    a.className = "a";
    a.textContent = item.answer;
    card.append(meta, q, a);
    appendInsightMoreSection(card, item);
    questionsListEl.appendChild(card);
  }

  if (hasTerms || hasKeywords) {
    insightsEmptyEl.classList.add("hidden");
  } else {
    insightsEmptyEl.classList.remove("hidden");
  }

  insightsTermsSec.classList.toggle("hidden", !hasTerms);
  insightsTermsList.replaceChildren();
  for (const item of sortInsightEntries(termEntries)) {
    const card = document.createElement("div");
    card.className = "insight-card";
    if (item.pinned) card.classList.add("is-pinned");
    card.dataset.id = item.id;
    setTip(card, item.pinned ? "点击取消固定" : "点击固定（不会被自动移除）");
    const meta = document.createElement("div");
    meta.className = "insight-meta";
    const pinLabel = item.pinned ? " · 📌" : "";
    meta.textContent = `#${item.seq} · ${formatInsightTime(item.addedAt)}${pinLabel}`;
    const term = document.createElement("div");
    term.className = "term";
    term.textContent = item.term;
    const body = document.createElement("div");
    body.textContent = item.explanation;
    card.append(meta, term, body);
    appendInsightMoreSection(card, item);
    insightsTermsList.appendChild(card);
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

function loadKeywordsCollapsed() {
  try {
    return localStorage.getItem(KEYWORDS_COLLAPSED_STORAGE_KEY) === "1";
  } catch {
    return false;
  }
}

function setKeywordsCollapsed(collapsed) {
  if (!insightsKeywordsSec || !keywordsToggleBtn) return;
  insightsKeywordsSec.classList.toggle("is-collapsed", collapsed);
  keywordsToggleBtn.setAttribute("aria-expanded", collapsed ? "false" : "true");
  try {
    localStorage.setItem(KEYWORDS_COLLAPSED_STORAGE_KEY, collapsed ? "1" : "0");
  } catch {
    /* ignore */
  }
}

function updateKeywordsCount(count) {
  if (!keywordsCountEl) return;
  if (count > 0) {
    keywordsCountEl.textContent = String(count);
    keywordsCountEl.classList.remove("hidden");
  } else {
    keywordsCountEl.textContent = "";
    keywordsCountEl.classList.add("hidden");
  }
}

keywordsToggleBtn?.addEventListener("click", () => {
  setKeywordsCollapsed(!insightsKeywordsSec?.classList.contains("is-collapsed"));
});

setKeywordsCollapsed(loadKeywordsCollapsed());

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
    updateKeywordsCount(insights.keywords.length);
    changed = true;
  }

  if (appendTermEntries(insights.terms)) changed = true;
  if (appendQuestionEntries(insights.questions)) changed = true;

  if (changed) {
    ensureInsightPruneTimer();
    renderInsightPanels();
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
  const { text, is_final, insights, session_summary } = event.payload;
  if (session_summary !== undefined && session_summary !== null) {
    renderSessionSummary(session_summary);
  }
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
  endCapturePending();
  updateCaptureButton(false);
  partialText = "";
  renderTranscript();
  refreshStatus();
});

function clearOverlaySession(placeholderText = "已清空，继续监听中…") {
  committedLines = [];
  partialText = "";
  lineInsights.clear();
  followLatest = true;
  scrollLatestBtn.classList.add("hidden");
  resetInsightDwellState();
  resetSessionSummary();
  questionsEmptyEl.classList.remove("hidden");
  insightsEmptyEl.classList.remove("hidden");
  insightsKeywordsSec.classList.add("hidden");
  insightsTermsSec.classList.add("hidden");
  insightsKeywordsList.replaceChildren();
  updateKeywordsCount(0);
  insightsTermsList.replaceChildren();
  questionsListEl.replaceChildren();
  setPlaceholder(placeholderText);
}

clearSessionBtn?.addEventListener("click", async () => {
  try {
    await invoke("clear_listening_session");
  } catch (e) {
    clearOverlaySession("已清空（本地）；服务未连接时仅清除界面");
    setTip(statusEl, String(e));
  }
});

listen("session-cleared", () => {
  clearOverlaySession();
});

listen("capture-started", () => {
  endCapturePending();
  updateCaptureButton(true);
  clearOverlaySession("Capture started — transcript builds below…");
  refreshStatus();
});

listen("capture-toggle-pending", (event) => {
  const action = event.payload === "stopping" ? "stopping" : "starting";
  beginCapturePending(action);
});

listen("capture-toggle-failed", (event) => {
  const wasCapturing = capturePending === "stopping";
  endCapturePending();
  applyCaptureStatusEl(statusEl, {
    state: wasCapturing ? "capturing" : "idle",
    message: String(event.payload ?? "toggle failed"),
  });
  updateCaptureButton(wasCapturing);
  setTip(statusEl, String(event.payload ?? ""));
});

toggleCaptureBtn?.addEventListener("click", async () => {
  const action =
    toggleCaptureBtn.getAttribute("aria-pressed") === "true"
      ? "stopping"
      : "starting";
  beginCapturePending(action);
  try {
    await invoke("toggle_capture");
  } catch (e) {
    const wasCapturing = action === "stopping";
    endCapturePending();
    applyCaptureStatusEl(statusEl, {
      state: wasCapturing ? "capturing" : "idle",
      message: String(e),
    });
    updateCaptureButton(wasCapturing);
    setTip(statusEl, String(e));
  }
});

toggleDiagnoseBtn?.addEventListener("click", async () => {
  try {
    const open = await invoke("toggle_diagnose_window");
    updateDiagnoseButton(Boolean(open));
  } catch (e) {
    setTip(statusEl, String(e));
  }
});

listen("diagnose-visibility", (event) => {
  updateDiagnoseButton(Boolean(event.payload));
});

toggleSettingsBtn?.addEventListener("click", async () => {
  try {
    const open = await invoke("toggle_settings_window");
    updateSettingsButton(Boolean(open));
  } catch (e) {
    setTip(statusEl, String(e));
  }
});

listen("settings-visibility", (event) => {
  updateSettingsButton(Boolean(event.payload));
});

listen("env-settings-saved", () => {
  refreshConfigStatus();
});

const THEME_SHORT_LABELS = {
  "dark-glass": "dark",
  "light-glass": "light",
  "solid-dark": "solid-d",
  "solid-light": "solid-l",
  "midnight": "night",
  "slate": "slate",
  "paper": "paper",
  "cream": "cream",
  "high-contrast-dark": "hc-dark",
  "high-contrast-light": "hc-light",
  outline: "outline",
};

function themeShortLabel(theme) {
  return (
    THEME_SHORT_LABELS[theme] ||
    theme.replace(/-glass$/, "").replace("high-contrast-", "hc-").slice(0, 10)
  );
}

function applyOverlayUi(payload) {
  const theme = payload.effective_theme || payload.theme || "dark-glass";
  overlayEl.className = `theme-${theme}`;
  const opacity =
    typeof payload.opacity === "number"
      ? Math.min(OPACITY_MAX, Math.max(OPACITY_MIN, payload.opacity))
      : 0.92;
  overlayEl.style.opacity = String(opacity);
  const opacityPct = Math.round(opacity * 100);
  if (opacityDownBtn) {
    opacityDownBtn.disabled = opacity <= OPACITY_MIN + 0.001;
    setTip(opacityDownBtn, tipWithHotkey(`降低浮层透明度（当前 ${opacityPct}%）`, "["));
  }
  if (opacityUpBtn) {
    opacityUpBtn.disabled = opacity >= OPACITY_MAX - 0.001;
    setTip(opacityUpBtn, tipWithHotkey(`提高浮层透明度（当前 ${opacityPct}%）`, "]"));
  }
  const fontScale =
    typeof payload.font_scale === "number"
      ? Math.min(FONT_SCALE_MAX, Math.max(FONT_SCALE_MIN, payload.font_scale))
      : 1;
  overlayEl.style.setProperty("--font-scale", String(fontScale));
  const scalePct = Math.round(fontScale * 100);
  if (fontDownBtn) {
    fontDownBtn.disabled = fontScale <= FONT_SCALE_MIN + 0.001;
    setTip(fontDownBtn, tipWithHotkey(`缩小字号（当前 ${scalePct}%）`, "−"));
  }
  if (fontUpBtn) {
    fontUpBtn.disabled = fontScale >= FONT_SCALE_MAX - 0.001;
    setTip(fontUpBtn, tipWithHotkey(`放大字号（当前 ${scalePct}%）`, "+"));
  }
  if (fontResetBtn) {
    fontResetBtn.disabled = Math.abs(fontScale - 1) < 0.001;
    setTip(
      fontResetBtn,
      tipWithHotkey(`重置字号为 100%（当前 ${scalePct}%）`, "0"),
    );
  }
  document.body.classList.toggle("adaptive-on", Boolean(payload.adaptive));
  const saved = payload.theme && payload.theme !== theme ? ` · saved ${payload.theme}` : "";
  themeBadgeEl.textContent = themeShortLabel(theme);
  setTip(
    themeBadgeEl,
    payload.adaptive
      ? `${theme}${saved} · 自动对比度 · ${hotkey("S")} 切换样式 · ${hotkey("A")} 自适应`
      : `${theme}${saved} · ${hotkey("S")} 切换样式 · ${hotkey("A")} 自适应对比`,
  );
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
      font_scale: s.font_scale,
    });
  } catch {
    /* not in tauri shell */
  }
}

async function loadInsightSettings() {
  try {
    const s = await invoke("get_insight_settings");
    if (typeof s.insight_dwell_ms === "number" && s.insight_dwell_ms >= 5000) {
      insightDwellMs = s.insight_dwell_ms;
    }
    if (typeof s.localize_zh === "boolean") {
      updateLocalizeButton(s.localize_zh);
    }
  } catch {
    /* not in tauri shell */
  }
}

function updateLocalizeButton(localizeZh) {
  if (!toggleLocalizeBtn) return;
  insightLocalizeZh = Boolean(localizeZh);
  toggleLocalizeBtn.classList.toggle("is-localized", insightLocalizeZh);
  toggleLocalizeBtn.textContent = insightLocalizeZh ? "中文" : "原文";
  setTip(
    toggleLocalizeBtn,
    insightLocalizeZh
      ? "术语/问答说明译为中文（点击切换为原文）"
      : "保持术语/问答原文（点击切换为中文说明）",
  );
  toggleLocalizeBtn.setAttribute("aria-pressed", insightLocalizeZh ? "true" : "false");
}

toggleLocalizeBtn?.addEventListener("click", async () => {
  try {
    const s = await invoke("set_insight_localize", { localizeZh: !insightLocalizeZh });
    updateLocalizeButton(s.localize_zh);
  } catch (e) {
    setTip(statusEl, String(e));
  }
});

loadOverlayUi();
loadInsightSettings();
syncDiagnoseButton();
syncSettingsButton();
syncMiniMode();
refreshStatus();
setInterval(refreshStatus, 5000);
