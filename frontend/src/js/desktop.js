import { $ } from "./dom.js";
import {
  applyKeyInputs,
  loadPersistedConfig,
  savePersistedDesktopConfig,
  savePersistedBrowserStoredConfig,
} from "./config.js";
import { state } from "./state.js";

export function showDesktopUi() {
  $("open-output-btn").classList.remove("hidden");
}

export function setDesktopBusy(message = "") {
  const targetIds = ["desktop-setup-error"];
  for (const id of targetIds) {
    const el = $(id);
    if (!el) {
      continue;
    }
    if (message) {
      el.textContent = message;
      el.classList.remove("hidden");
    } else {
      el.textContent = "";
      el.classList.add("hidden");
    }
  }
}

export function openSetupDialog() {
  $("desktop-setup-dialog").showModal();
}

export function closeSetupDialog() {
  if ($("desktop-setup-dialog").open) {
    $("desktop-setup-dialog").close();
  }
}

export async function bootstrapDesktop(initialConfig = null) {
  state.desktopMode = true;
  showDesktopUi();
  const payload = initialConfig || await loadPersistedConfig();
  state.developerConfig = payload.developerConfig || {};
  applyKeyInputs(payload.browserConfig || {});
  state.desktopConfigured = !!payload.firstRunCompleted;
  if (!state.desktopConfigured) {
    openSetupDialog();
  } else {
    closeSetupDialog();
  }
}

export async function saveDesktopConfig(mineruToken, modelApiKey, afterSave, extraBrowserConfig = {}) {
  let markConfigured = false;
  let nextBrowserConfig = {
    ...extraBrowserConfig,
    mineruToken,
    modelApiKey,
  };
  let callback = afterSave;
  if (typeof mineruToken === "object" && mineruToken !== null) {
    nextBrowserConfig = { ...(mineruToken.browserConfig || mineruToken) };
    markConfigured = !!mineruToken.markConfigured;
    callback = typeof modelApiKey === "function" ? modelApiKey : afterSave;
  }
  let persisted = await savePersistedBrowserStoredConfig({
    ...nextBrowserConfig,
  });
  state.developerConfig = persisted.developerConfig || state.developerConfig;
  applyKeyInputs(persisted.browserConfig || {});
  if (callback) {
    await callback();
  }
  if (markConfigured && !persisted.firstRunCompleted) {
    persisted = await savePersistedDesktopConfig({ firstRunCompleted: true });
  }
  state.developerConfig = persisted.developerConfig || state.developerConfig;
  applyKeyInputs(persisted.browserConfig || {});
  state.desktopConfigured = !!persisted.firstRunCompleted;
  if (state.desktopConfigured) {
    closeSetupDialog();
    $("error-box").textContent = "-";
  }
  return persisted;
}
