import { getCurrentWindow } from "@tauri-apps/api/window";

/** Hide instead of destroy when user clicks the native close button (matches Rust handler). */
export async function setupAuxWindowCloseHandler() {
  const win = getCurrentWindow();
  await win.onCloseRequested(async (event) => {
    event.preventDefault();
    await win.hide();
  });
}
