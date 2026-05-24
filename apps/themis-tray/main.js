import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const statusEl = document.getElementById("status");
const scrollEl = document.getElementById("transcript-scroll");
const transcriptEl = document.getElementById("transcript");
const feedbackEl = document.getElementById("feedback");
const scrollLatestBtn = document.getElementById("scroll-latest");

/** @type {string[]} Final lines (one per Azure REST phrase). */
let committedLines = [];
/** Latest partial hypothesis while speaking. */
let partialText = "";

/** User scrolled up — pause auto-follow until they click Latest or scroll to bottom. */
let followLatest = true;

const SCROLL_BOTTOM_THRESHOLD = 48;

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
    const el = document.createElement("span");
    el.className = "line-final";
    el.textContent = line;
    transcriptEl.appendChild(el);
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

function isSystemMessage(text) {
  return (
    /^Azure (REST|streaming|auto-language)/i.test(text) ||
    text.includes("connected…") ||
    text.includes("transcribing every") ||
    text.includes("picking best match")
  );
}

listen("transcript", (event) => {
  const { text, is_final, feedback } = event.payload;
  if (!text || (text.startsWith("(") && text.includes("未识别"))) {
    return;
  }
  if (isSystemMessage(text)) {
    return;
  }

  if (is_final) {
    const trimmed = text.trim();
    if (trimmed) {
      committedLines.push(trimmed);
      partialText = "";
    }
  } else {
    partialText = text.trim();
  }

  renderTranscript();

  if (feedback) {
    feedbackEl.textContent = feedback;
  }
});

listen("capture-stopped", () => {
  partialText = "";
  renderTranscript();
});

listen("capture-started", () => {
  committedLines = [];
  partialText = "";
  followLatest = true;
  scrollLatestBtn.classList.add("hidden");
  setPlaceholder("Capture started — transcript builds below…");
});

refreshStatus();
setInterval(refreshStatus, 5000);
