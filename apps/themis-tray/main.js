import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const statusEl = document.getElementById("status");
const transcriptEl = document.getElementById("transcript");
const feedbackEl = document.getElementById("feedback");

async function refreshStatus() {
  try {
    const s = await invoke("get_status");
    statusEl.textContent = `Status: ${s.state} — ${s.message}`;
    if (s.state === "capturing" && !transcriptEl.textContent.trim()) {
      transcriptEl.textContent =
        "Capturing… play system audio or speech. Text appears here when STT returns.";
      transcriptEl.style.opacity = "0.6";
    }
  } catch (e) {
    statusEl.textContent = `Service offline (${e})`;
  }
}

listen("transcript", (event) => {
  const { text, is_final, feedback } = event.payload;
  transcriptEl.textContent = text;
  transcriptEl.style.opacity = is_final ? "1" : "0.7";
  if (feedback) {
    feedbackEl.textContent = feedback;
  }
});

refreshStatus();
setInterval(refreshStatus, 5000);
