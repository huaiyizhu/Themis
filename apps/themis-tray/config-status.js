import { setTip } from "./tooltips.js";

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
  return dep || "已配置";
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

/** @param {object} tray */
export function listMissingConfigItems(tray) {
  if (!tray) return [];
  /** @type {string[]} */
  const missing = [];
  if (!tray.stt_configured) {
    missing.push("Azure Speech（AZURE_SPEECH_KEY、AZURE_SPEECH_REGION）");
  }
  if (!tray.llm_configured) {
    missing.push("Azure OpenAI Insights（FOUNDRY_ENDPOINT、FOUNDRY_API_KEY）");
  }
  if (tray.analysis_enabled === false) {
    missing.push("Insights 已关闭（THEMIS_ANALYSIS_ENABLED=false）");
  }
  return missing;
}

export function isAllOk(config) {
  if (!config?.tray || !config.service || !config.in_sync) return false;
  const { tray, service } = config;
  return (
    tray.stt_configured &&
    tray.llm_configured &&
    service.stt_configured &&
    service.llm_configured
  );
}

function renderMissingHint(tray) {
  const missing = listMissingConfigItems(tray).filter((m) =>
    m.startsWith("Azure") || m.startsWith("Insights 已关闭"),
  );
  if (!missing.length) return "";
  return `<span class="cfg-hint">· 未配置：${escapeHtml(missing.join("；"))} · 点击标题栏「<strong>配置</strong>」填写并保存</span>`;
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
  } else {
    html += renderMissingHint(tray);
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
  } else {
    const missing = listMissingConfigItems(tray).filter((m) => m.startsWith("Azure"));
    if (missing.length) {
      line += ` · 未配置：${missing.join("；")} · 请用「配置」保存`;
    }
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
  const missing = listMissingConfigItems(config.tray);
  if (missing.length) {
    lines.push(`Missing: ${missing.join("; ")}`);
    lines.push("Tip: use overlay 配置 button — .env is created on save (no manual copy required).");
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
  setTip(el, configCrossCheckTitle(config));
  el.classList.toggle("config-mismatch", Boolean(config?.service && !config.in_sync));
  el.classList.toggle("config-offline", !config?.service);
  el.classList.toggle("config-all-ok", isAllOk(config));
  el.classList.toggle(
    "config-incomplete",
    Boolean(config?.tray && listMissingConfigItems(config.tray).some((m) => m.startsWith("Azure"))),
  );
}
