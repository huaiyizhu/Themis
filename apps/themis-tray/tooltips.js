/** Custom tooltips — native `title` popups flicker/disappear in Tauri/WebView2 overlay windows. */

let tipBubble = null;
let tipTarget = null;
let showTimer = null;

function ensureBubble() {
  if (!tipBubble) {
    tipBubble = document.createElement("div");
    tipBubble.className = "themis-tip";
    tipBubble.setAttribute("role", "tooltip");
    tipBubble.hidden = true;
    document.body.appendChild(tipBubble);
  }
  return tipBubble;
}

export function modKey() {
  return /Mac|iPhone|iPad|iPod/i.test(navigator.platform) ? "Cmd" : "Ctrl";
}

/** @param {string} key Single key letter/symbol (e.g. "T", "H", "−") */
export function hotkey(key, { shift = true } = {}) {
  const mod = modKey();
  return shift ? `${mod}+Shift+${key}` : `${mod}+${key}`;
}

export function tipWithHotkey(description, key, opts) {
  if (!key) return description;
  return `${description} (${hotkey(key, opts)})`;
}

export function setTip(el, text) {
  if (!el) return;
  const value = text == null ? "" : String(text).trim();
  if (value) {
    el.setAttribute("data-tip", value);
  } else {
    el.removeAttribute("data-tip");
  }
  el.removeAttribute("title");
  if (el === tipTarget && tipBubble) {
    if (value) {
      tipBubble.textContent = value;
      positionTip(el, tipBubble);
    } else {
      hideTip();
    }
  }
}

export function dismissTooltip() {
  hideTip();
}

function hideTip() {
  clearTimeout(showTimer);
  showTimer = null;
  if (tipBubble) {
    tipBubble.hidden = true;
  }
  tipTarget = null;
}

function showTip(el) {
  const text = el.getAttribute("data-tip");
  if (!text) {
    hideTip();
    return;
  }
  tipTarget = el;
  const bubble = ensureBubble();
  bubble.textContent = text;
  bubble.hidden = false;
  positionTip(el, bubble);
}

function positionTip(el, bubble) {
  const margin = 8;
  const rect = el.getBoundingClientRect();
  bubble.style.left = "0";
  bubble.style.top = "0";
  bubble.hidden = false;
  const bw = bubble.offsetWidth;
  const bh = bubble.offsetHeight;

  bubble.classList.remove("is-below", "is-menu-side-left", "is-menu-side-right");

  const overflowMenu = el.closest?.("#header-overflow-menu");
  if (overflowMenu) {
    const menuRect = overflowMenu.getBoundingClientRect();
    const halfH = bh / 2;
    let top = rect.top + rect.height / 2;
    top = Math.max(margin + halfH, Math.min(top, window.innerHeight - margin - halfH));

    const leftAnchor = menuRect.left - margin;
    if (leftAnchor - bw >= margin) {
      bubble.classList.add("is-menu-side-left");
      bubble.style.left = `${leftAnchor}px`;
      bubble.style.top = `${top}px`;
      return;
    }

    bubble.classList.add("is-menu-side-right");
    bubble.style.left = `${menuRect.right + margin}px`;
    bubble.style.top = `${top}px`;
    return;
  }

  let top = rect.top - margin - bh;
  let left = rect.left + rect.width / 2;

  bubble.classList.toggle("is-below", top < margin);
  if (top < margin) {
    top = rect.bottom + margin;
  }

  const half = bw / 2;
  left = Math.max(margin + half, Math.min(left, window.innerWidth - margin - half));

  bubble.style.left = `${left}px`;
  bubble.style.top = `${top}px`;
}

function scheduleShow(el) {
  clearTimeout(showTimer);
  showTimer = setTimeout(() => {
    showTimer = null;
    showTip(el);
  }, 220);
}

export function migrateTitleToDataTip(root = document) {
  root.querySelectorAll("[title]").forEach((el) => {
    const title = el.getAttribute("title")?.trim();
    if (title && !el.hasAttribute("data-tip")) {
      el.setAttribute("data-tip", title);
    }
    el.removeAttribute("title");
  });
}

export function initTooltips() {
  migrateTitleToDataTip(document);

  document.addEventListener(
    "mouseover",
    (e) => {
      const el = e.target.closest?.("[data-tip]");
      if (!el) {
        if (tipTarget) hideTip();
        return;
      }
      if (el === tipTarget) return;
      if (tipTarget) hideTip();
      scheduleShow(el);
    },
    true,
  );

  document.addEventListener(
    "mouseout",
    (e) => {
      const el = e.target.closest?.("[data-tip]");
      if (!el || el !== tipTarget) return;
      const related = e.relatedTarget;
      if (related && el.contains(related)) return;
      clearTimeout(showTimer);
      hideTip();
    },
    true,
  );

  window.addEventListener(
    "scroll",
    () => {
      if (tipTarget) {
        positionTip(tipTarget, ensureBubble());
      }
    },
    true,
  );
  window.addEventListener("blur", hideTip);
  window.addEventListener("resize", hideTip);
}
