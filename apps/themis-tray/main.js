import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const statusEl = document.getElementById("status");
const transcriptEl = document.getElementById("transcript");
const feedbackEl = document.getElementById("feedback");

/** Accumulated final transcript (do not replace with single-word chunks). */
let committedText = "";
/** Latest partial hypothesis while speaking. */
let partialText = "";

function renderTranscript() {
  if (partialText) {
    transcriptEl.textContent = committedText
      ? `${committedText} ${partialText}`
      : partialText;
    transcriptEl.style.opacity = "0.75";
  } else {
    transcriptEl.textContent = committedText;
    transcriptEl.style.opacity = "1";
  }
}

async function refreshStatus() {
  try {
    const s = await invoke("get_status");
    let line = `Status: ${s.state} — ${s.message}`;
    if (s.state === "capturing" && s.audio_frames !== undefined) {
      line += ` | mode=${s.capture_mode || "?"} peak=${s.audio_peak ?? 0}`;
    }
    statusEl.textContent = line;
    if (s.state === "capturing" && !committedText && !partialText) {
      let hint = "Listening… transcript builds here sentence by sentence.";
      if (s.message && s.message.includes("no loopback signal")) {
        hint = s.message;
      } else if (s.message && s.message.includes("signal=strong")) {
        hint =
          "Capture OK (signal=strong). Waiting for Azure speech (~4s per phrase in REST mode)…";
      } else if (s.message && s.message.includes("signal=ok")) {
        hint = "Capture OK. Waiting for Azure speech…";
      }
      transcriptEl.textContent = hint;
      transcriptEl.style.opacity = "0.6";
    }
  } catch (e) {
    statusEl.textContent = `Service offline (${e})`;
  }
}

/** Internal status lines from the service — show in status bar, not transcript. */
function isSystemMessage(text) {
  return (
    /^Azure (REST|streaming)/i.test(text) ||
    text.includes("connected…") ||
    text.includes("transcribing every")
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
      committedText = committedText ? `${committedText} ${trimmed}` : trimmed;
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
  committedText = "";
  partialText = "";
  renderTranscript();
});

refreshStatus();
setInterval(refreshStatus, 5000);
