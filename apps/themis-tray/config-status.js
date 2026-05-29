/** Format tray ↔ service STT/LLM config cross-check for overlay & diagnose UI. */

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function sttDetail(side) {
  if (!side.stt_configured) {
    return side.stt_mode === "mock" ? "mock" : "未配置";
  }
  const region = side.speech_region?.trim();
  return region ? `${side.stt_mode}/${region}` : side.stt_mode;
}

function llmDetail(side) {
  if (!side.llm_configured) return "未配置";
  const dep = side.foundry_deployment?.trim();
  return dep || "configured";
}

function statusMark(ok) {
  const cls = ok ? "cfg-ok" : "cfg-fail";
  const sym = ok ? "✓" : "✗";
  return `<span class="${cls}" aria-hidden="true">${sym}</span>`;
}

function renderSide(label, configured, detail, offline) {
  if (offline) {
    return `<span class="cfg-side cfg-side-offline">${escapeHtml(label)} <span class="cfg-offline-tag">offline</span></span>`;
  }
  return `<span class="cfg-side">${escapeHtml(label)} ${statusMark(configured)} <span class="cfg-detail">${escapeHtml(detail)}</span></span>`;
}

function renderChip(kind, kindClass, trayConfigured, serviceConfigured, trayDetail, serviceDetail, serviceOffline) {
  const envSide = renderSide(".env", trayConfigured, trayDetail, false);
  const svcSide = renderSide("服务", serviceConfigured, serviceDetail, serviceOffline);
  return `<span class="cfg-chip ${kindClass}"><span class="cfg-kind">${kind}</span>${envSide}<span class="cfg-dot">·</span>${svcSide}</span>`;
}

function isAllOk(config) {
  if (!config?.tray || !config.service || !config.in_sync) return false;
  const { tray, service } = config;
  return (
    tray.stt_configured &&
    tray.llm_configured &&
    service.stt_configured &&
    service.llm_configured
  );
}

/**
 * @param {{ tray: object, service?: object | null, in_sync: boolean } | null | undefined} config
 */
export function renderConfigCrossCheck(config) {
  if (!config?.tray) return `<span class="cfg-offline-tag">配置检查：—</span>`;

  const { tray, service, in_sync } = config;
  const serviceOffline = !service;
  const sttChip = renderChip(
    "STT",
    "cfg-stt",
    tray.stt_configured,
    service?.stt_configured ?? false,
    sttDetail(tray),
    service ? sttDetail(service) : "",
    serviceOffline,
  );
  const llmChip = renderChip(
    "LLM",
    "cfg-llm",
    tray.llm_configured,
    service?.llm_configured ?? false,
    llmDetail(tray),
    service ? llmDetail(service) : "",
    serviceOffline,
  );
  let html = `${sttChip}<span class="cfg-pipe">│</span>${llmChip}`;

  if (service && !in_sync) {
    html += `<span class="cfg-warn">· ⚠ 不一致，请 restart 服务</span>`;
  }
  return html;
}

/**
 * Plain-text fallback (tooltips, logs).
 * @param {{ tray: object, service?: object | null, in_sync: boolean } | null | undefined} config
 */
export function formatConfigCrossCheck(config) {
  if (!config?.tray) return "配置检查：—";
  const { tray, service, in_sync } = config;
  const mark = (ok) => (ok ? "✓" : "✗");
  const sttTray = `STT .env ${mark(tray.stt_configured)} ${sttDetail(tray)}`;
  const sttSvc = service
    ? `服务 ${mark(service.stt_configured)} ${sttDetail(service)}`
    : "服务 — offline";
  const llmTray = `LLM .env ${mark(tray.llm_configured)} ${llmDetail(tray)}`;
  const llmSvc = service
    ? `服务 ${mark(service.llm_configured)} ${llmDetail(service)}`
    : "服务 — offline";
  let line = `${sttTray} · ${sttSvc} │ ${llmTray} · ${llmSvc}`;
  if (service && !in_sync) {
    line += " · ⚠ 不一致，请 restart 服务";
  }
  return line;
}

/**
 * @param {{ tray: object, service?: object | null, in_sync: boolean } | null | undefined} config
 */
export function configCrossCheckTitle(config) {
  if (!config?.tray) return "";
  const lines = [
    "交叉验证：.env（托盘读取）vs themis-service（运行时）",
    `STT .env: ${config.tray.stt_configured ? "OK" : "missing"} (${sttDetail(config.tray)})`,
  ];
  if (config.service) {
    lines.push(
      `STT service: ${config.service.stt_configured ? "OK" : "missing"} (${sttDetail(config.service)})`,
      `LLM .env: ${config.tray.llm_configured ? "OK" : "missing"} (${llmDetail(config.tray)})`,
      `LLM service: ${config.service.llm_configured ? "OK" : "missing"} (${llmDetail(config.service)})`,
      `In sync: ${config.in_sync ? "yes" : "NO — run ./scripts/themis.sh restart"}`
    );
  } else {
    lines.push("Service offline — start/restart themis-service");
  }
  if (!config.tray.analysis_enabled) {
    lines.push("THEMIS_ANALYSIS_ENABLED=false (Insights disabled)");
  }
  return lines.join("\n");
}

/**
 * @param {HTMLElement | null} el
 * @param {{ tray: object, service?: object | null, in_sync: boolean } | null | undefined} config
 */
export function applyConfigStatusEl(el, config) {
  if (!el) return;
  el.innerHTML = renderConfigCrossCheck(config);
  el.title = configCrossCheckTitle(config);
  el.classList.toggle("config-mismatch", Boolean(config?.service && !config.in_sync));
  el.classList.toggle("config-offline", !config?.service);
  el.classList.toggle("config-all-ok", isAllOk(config));
}
