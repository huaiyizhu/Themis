/**
 * Responsive header toolbar: max 2 rows; spill optional controls into ⋯ when tight.
 * Pin (置顶), 浮, capture, exports, clear, hide, mode, size strip never overflow.
 */
import { dismissTooltip } from "./tooltips.js";

/** Last-resort overflow when ⋯ would still be clipped (window extremely narrow). */
const DESPERATE_OVERFLOW = ["clear-session", "export-insights", "export-transcript"];

/** First moved = lowest on-screen priority (tune row first, then window utilities). */
const OVERFLOW_ORDER = [
  "scroll-latest",
  "font-segment",
  "opacity-segment",
  "toggle-localize",
  "quit-app",
  "toggle-settings",
  "toggle-diagnose",
];

const SLOT_SELECTORS = {
  tools: '[data-toolbar-slot="tools"]',
  window: '[data-toolbar-slot="window"]',
  tune: '[data-toolbar-slot="tune"]',
};

/** @type {HTMLElement | null} */
let overflowMenuHome = null;

function syncOverflowMenuTheme() {
  const menu = overflowMenuEl();
  const overlay = document.getElementById("overlay");
  if (!menu || !overlay) return;
  for (const cls of [...menu.classList]) {
    if (cls.startsWith("theme-")) menu.classList.remove(cls);
  }
  for (const cls of overlay.classList) {
    if (cls.startsWith("theme-")) menu.classList.add(cls);
  }
}

function mountOverflowMenu() {
  const menu = overflowMenuEl();
  if (!menu || menu.parentElement === document.body) return;
  overflowMenuHome = menu.parentElement;
  document.body.appendChild(menu);
  syncOverflowMenuTheme();
}

function dockOverflowMenu() {
  const menu = overflowMenuEl();
  if (!menu || !overflowMenuHome || overflowMenuHome.contains(menu)) return;
  overflowMenuHome.appendChild(menu);
}

function overflowMenuEl() {
  return document.getElementById("header-overflow-menu");
}

function overflowToggleEl() {
  return document.getElementById("header-overflow-toggle");
}

function rowMainEl() {
  return document.querySelector(".header-row-main");
}

function rowTuneEl() {
  return document.querySelector(".header-row-tune");
}

function isElementClipped(el) {
  if (!el || el.classList.contains("hidden")) return false;
  const rect = el.getBoundingClientRect();
  if (rect.width < 2 && rect.height < 2) return true;

  let node = el.parentElement;
  while (node) {
    if (node.classList?.contains("header-overflow-menu")) return false;
    const style = getComputedStyle(node);
    const clips =
      style.overflow === "hidden" ||
      style.overflow === "clip" ||
      style.overflowX === "hidden" ||
      style.overflowX === "clip" ||
      style.overflowY === "hidden" ||
      style.overflowY === "clip";
    if (clips) {
      const parentRect = node.getBoundingClientRect();
      if (
        rect.right > parentRect.right + 0.5 ||
        rect.left < parentRect.left - 0.5 ||
        rect.bottom > parentRect.bottom + 0.5 ||
        rect.top < parentRect.top - 0.5
      ) {
        return true;
      }
    }
    if (node.classList.contains("header-row-main") || node.classList.contains("header-row-tune")) {
      break;
    }
    node = node.parentElement;
  }
  return false;
}

function allOverflowIds() {
  return [...OVERFLOW_ORDER, ...DESPERATE_OVERFLOW];
}

function hasClippedOverflowInToolbar() {
  for (const id of allOverflowIds()) {
    const el = document.getElementById(id);
    const slot = slotForId(id);
    if (!el || el.classList.contains("hidden") || !slot?.contains(el)) continue;
    if (isElementClipped(el)) return true;
  }
  return false;
}

function isMainRowOverflowing() {
  const row = rowMainEl();
  if (!row) return false;
  const rowRect = row.getBoundingClientRect();
  for (const child of row.children) {
    if (child.classList.contains("toolbar-spacer")) continue;
    if (child.classList.contains("toolbar-cluster-pinned-end")) continue;
    const rect = child.getBoundingClientRect();
    if (rect.right > rowRect.right + 1 || rect.left < rowRect.left - 1) {
      return true;
    }
  }
  return isOverflowToggleClipped();
}

function isOverflowToggleClipped() {
  const row = rowMainEl();
  const toggle = overflowToggleEl();
  if (!row || !toggle || toggle.classList.contains("hidden")) return false;
  const rowRect = row.getBoundingClientRect();
  const rect = toggle.getBoundingClientRect();
  return rect.right > rowRect.right + 0.5 || rect.left < rowRect.left - 0.5 || rect.width < 8;
}

function toolbarNeedsOverflowMenu() {
  return toolbarOverflowing() || isOverflowToggleClipped() || hasClippedOverflowInToolbar();
}

/** Size preset strip scrolls internally — don't use row scrollWidth. */
function isTuneRowOverflowing() {
  const row = rowTuneEl();
  const tuneItems = document.querySelector(SLOT_SELECTORS.tune);
  const sizeBar = document.getElementById("size-presets");
  if (!row || !tuneItems || !row.contains(tuneItems)) return false;

  const rowRect = row.getBoundingClientRect();
  const tuneRect = tuneItems.getBoundingClientRect();
  if (!sizeBar) {
    return tuneRect.right > rowRect.right + 1;
  }
  const sizeRect = sizeBar.getBoundingClientRect();
  return tuneRect.right > sizeRect.left + 1 || sizeRect.right > rowRect.right + 1;
}

function toolbarOverflowing() {
  return isMainRowOverflowing() || isTuneRowOverflowing();
}

function slotForId(id) {
  if (id === "scroll-latest") {
    return document.querySelector(SLOT_SELECTORS.tools);
  }
  if (id === "toggle-localize" || id === "opacity-segment" || id === "font-segment") {
    return document.querySelector(SLOT_SELECTORS.tune);
  }
  if (id === "quit-app") {
    return document.querySelector(SLOT_SELECTORS.window);
  }
  if (id === "clear-session" || id === "export-insights" || id === "export-transcript") {
    return document.querySelector(".toolbar-cluster-session");
  }
  return document.querySelector(SLOT_SELECTORS.tools);
}

function restoreAll() {
  const menu = overflowMenuEl();
  if (!menu) return;

  dockOverflowMenu();

  for (const id of [...OVERFLOW_ORDER, ...DESPERATE_OVERFLOW]) {
    const el = document.getElementById(id);
    const slot = slotForId(id);
    if (!el || !slot || slot.contains(el)) continue;
    slot.appendChild(el);
  }
  menu.replaceChildren();
  updateOverflowToggle();
}

function isTuneSlotId(id) {
  return id === "toggle-localize" || id === "opacity-segment" || id === "font-segment";
}

function shouldMoveToOverflow(id, el) {
  if (!el || el.classList.contains("hidden")) return false;
  const slot = slotForId(id);
  if (!slot?.contains(el)) return false;
  if (isElementClipped(el)) return true;
  if (isTuneSlotId(id)) return isTuneRowOverflowing();
  if (DESPERATE_OVERFLOW.includes(id)) {
    return isMainRowOverflowing() || isOverflowToggleClipped();
  }
  return isMainRowOverflowing();
}

function firstVisibleCandidateInToolbar() {
  for (const id of OVERFLOW_ORDER) {
    const el = document.getElementById(id);
    if (shouldMoveToOverflow(id, el)) return el;
  }

  if (toolbarNeedsOverflowMenu()) {
    for (const id of DESPERATE_OVERFLOW) {
      const el = document.getElementById(id);
      if (shouldMoveToOverflow(id, el)) return el;
    }
  }
  return null;
}

function applyOverflowPass() {
  const menu = overflowMenuEl();
  if (!menu) return;

  let guard = 0;
  const maxPasses = OVERFLOW_ORDER.length + DESPERATE_OVERFLOW.length + 4;
  while (toolbarNeedsOverflowMenu() && guard++ < maxPasses) {
    const el = firstVisibleCandidateInToolbar();
    if (!el) break;
    menu.prepend(el);
  }
  updateOverflowToggle();
}

function restoreWhileFits() {
  const menu = overflowMenuEl();
  if (!menu) return;

  for (let i = allOverflowIds().length - 1; i >= 0; i -= 1) {
    const id = allOverflowIds()[i];
    const el = document.getElementById(id);
    const slot = slotForId(id);
    if (!el || !slot || !menu.contains(el)) continue;
    slot.appendChild(el);
    if (toolbarNeedsOverflowMenu() || isElementClipped(el)) {
      menu.appendChild(el);
    }
  }
  updateOverflowToggle();
}

export function reflowToolbarOverflow() {
  if (isOverflowMenuOpen()) return;
  restoreAll();
  applyOverflowPass();
  restoreWhileFits();
  applyOverflowPass();
}

function updateOverflowToggle() {
  const toggle = overflowToggleEl();
  const menu = overflowMenuEl();
  if (!toggle || !menu) return;
  const hasItems = menu.childElementCount > 0;
  toggle.classList.toggle("hidden", !hasItems);
  if (!hasItems) {
    closeOverflowMenu();
  }
}

function isOverflowMenuOpen() {
  const menu = overflowMenuEl();
  return Boolean(menu && !menu.classList.contains("hidden") && menu.childElementCount > 0);
}

function positionOverflowMenu() {
  const toggle = overflowToggleEl();
  const menu = overflowMenuEl();
  if (!toggle || !menu) return;
  const rect = toggle.getBoundingClientRect();
  menu.style.position = "fixed";
  menu.style.top = `${Math.round(rect.bottom + 4)}px`;
  menu.style.right = `${Math.round(window.innerWidth - rect.right)}px`;
  menu.style.left = "auto";
  menu.style.zIndex = "10000";
}

function openOverflowMenu() {
  const toggle = overflowToggleEl();
  const menu = overflowMenuEl();
  if (!toggle || !menu || menu.childElementCount === 0) return;
  dismissTooltip();
  mountOverflowMenu();
  positionOverflowMenu();
  menu.classList.remove("hidden");
  toggle.setAttribute("aria-expanded", "true");
}

function closeOverflowMenu() {
  const menu = overflowMenuEl();
  const toggle = overflowToggleEl();
  menu?.classList.add("hidden");
  toggle?.setAttribute("aria-expanded", "false");
  dismissTooltip();
  dockOverflowMenu();
}

export function initToolbarOverflow() {
  const toolbar = document.querySelector(".header-toolbar");
  const toggle = overflowToggleEl();
  const menu = overflowMenuEl();
  if (!toolbar || !toggle || !menu) return;

  toggle.addEventListener("mousedown", (e) => {
    e.stopPropagation();
    dismissTooltip();
  });

  toggle.addEventListener("click", (e) => {
    e.stopPropagation();
    e.preventDefault();
    if (menu.childElementCount === 0) return;
    if (isOverflowMenuOpen()) {
      closeOverflowMenu();
      window.requestAnimationFrame(reflowToolbarOverflow);
    } else {
      openOverflowMenu();
    }
  });

  menu.addEventListener("click", (e) => {
    e.stopPropagation();
    if (e.target.closest("button, .header-segment-step")) {
      closeOverflowMenu();
      window.requestAnimationFrame(reflowToolbarOverflow);
    }
  });

  document.addEventListener("click", (e) => {
    if (toggle.contains(e.target) || menu.contains(e.target)) return;
    if (!isOverflowMenuOpen()) return;
    closeOverflowMenu();
    window.requestAnimationFrame(reflowToolbarOverflow);
  });

  const scheduleReflow = () => {
    if (isOverflowMenuOpen()) {
      positionOverflowMenu();
      return;
    }
    window.requestAnimationFrame(reflowToolbarOverflow);
  };

  if (typeof ResizeObserver !== "undefined") {
    const ro = new ResizeObserver(scheduleReflow);
    ro.observe(toolbar);
    if (rowMainEl()) ro.observe(rowMainEl());
    if (rowTuneEl()) ro.observe(rowTuneEl());
    document.querySelectorAll(".toolbar-cluster-tools, .toolbar-cluster-session, .toolbar-cluster-window").forEach((el) => {
      ro.observe(el);
    });
  }
  window.addEventListener("resize", scheduleReflow);

  scheduleReflow();
}

export function closeToolbarOverflowMenu() {
  closeOverflowMenu();
}
