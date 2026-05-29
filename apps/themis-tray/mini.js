import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const floaterEl = document.getElementById("floater");

/** @type {{ dragging: boolean, x: number, y: number, id: number } | null} */
let press = null;

function clearPress() {
  press = null;
}

async function finishPress(e) {
  if (!press) return;
  if (e.pointerId !== undefined && e.pointerId !== press.id) return;
  const moved =
    press.dragging || Math.hypot(e.screenX - press.x, e.screenY - press.y) > 6;
  clearPress();
  if (!moved) {
    try {
      await invoke("toggle_overlay_mini_mode");
    } catch (err) {
      floaterEl.title = String(err);
    }
  }
}

if (floaterEl) {
  floaterEl.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    press = { dragging: false, x: e.screenX, y: e.screenY, id: e.pointerId };
  });

  floaterEl.addEventListener("pointermove", (e) => {
    if (!press || e.pointerId !== press.id || press.dragging) return;
    if (Math.hypot(e.screenX - press.x, e.screenY - press.y) > 6) {
      press.dragging = true;
      getCurrentWindow().startDragging().catch(() => {});
    }
  });

  floaterEl.addEventListener("keydown", async (e) => {
    if (e.key !== "Enter" && e.key !== " ") return;
    e.preventDefault();
    try {
      await invoke("toggle_overlay_mini_mode");
    } catch (err) {
      floaterEl.title = String(err);
    }
  });

  window.addEventListener("pointerup", finishPress);
  window.addEventListener("pointercancel", clearPress);
  requestAnimationFrame(() => floaterEl.focus());
}
