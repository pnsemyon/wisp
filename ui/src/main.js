// Wisp UI — vanilla JS, no build step. Talks to the Tauri v2 backend via
// `window.__TAURI__.core.invoke`.

const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);

/** @type {{profiles: any[], activeProfileId: string|null, split: {mode:string, rules:any[]}, settings: any, lastStatus: any}} */
const state = {
  profiles: [],
  activeProfileId: null,
  split: { mode: "off", rules: [] },
  settings: null,
  lastStatus: { state: "stopped" },
};

let statusTimer = null;
let logsTimer = null;
let connectPending = false;

// ---------- helpers ----------

function formatBytes(bytes) {
  if (!bytes || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let value = bytes;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i += 1;
  }
  return `${value.toFixed(value < 10 && i > 0 ? 1 : 0)} ${units[i]}`;
}

function formatSpeed(bytesPerSec) {
  return `${formatBytes(bytesPerSec)}/s`;
}

function tagsOf(profile) {
  if (!profile || !Array.isArray(profile.outbounds)) return [];
  return profile.outbounds.map((o) => o && o.tag).filter((t) => typeof t === "string" && t.length > 0);
}

function showError(message) {
  // Minimal, non-blocking error surface. Kept intentionally simple (no
  // external toast lib) per the "no CDN/network deps" constraint.
  console.error(message);
  window.alert(message);
}

// ---------- profiles ----------

const profileSelect = document.getElementById("profile-select");
const serverSelect = document.getElementById("server-select");

async function loadProfiles() {
  state.profiles = await invoke("list_profiles");
  renderProfileSelect();
}

function activeProfile() {
  return state.profiles.find((p) => p.id === state.activeProfileId) || null;
}

function renderProfileSelect() {
  profileSelect.innerHTML = "";
  if (state.profiles.length === 0) {
    const opt = document.createElement("option");
    opt.value = "";
    opt.textContent = "No profiles yet";
    profileSelect.appendChild(opt);
    profileSelect.value = "";
    renderServerSelect();
    return;
  }

  for (const profile of state.profiles) {
    const opt = document.createElement("option");
    opt.value = profile.id;
    opt.textContent = profile.name;
    profileSelect.appendChild(opt);
  }

  if (!state.activeProfileId || !state.profiles.some((p) => p.id === state.activeProfileId)) {
    state.activeProfileId = state.profiles[0].id;
  }
  profileSelect.value = state.activeProfileId;
  renderServerSelect();
}

function renderServerSelect() {
  serverSelect.innerHTML = "";
  const profile = activeProfile();
  const tags = tagsOf(profile);

  if (tags.length === 0) {
    const opt = document.createElement("option");
    opt.value = "";
    opt.textContent = "—";
    serverSelect.appendChild(opt);
    return;
  }

  for (const tag of tags) {
    const opt = document.createElement("option");
    opt.value = tag;
    opt.textContent = tag;
    serverSelect.appendChild(opt);
  }
  serverSelect.value = profile.active_tag && tags.includes(profile.active_tag) ? profile.active_tag : tags[0];
}

profileSelect.addEventListener("change", async () => {
  const id = profileSelect.value;
  if (!id) return;
  try {
    await invoke("set_active_profile", { id });
    state.activeProfileId = id;
    renderServerSelect();
  } catch (err) {
    showError(String(err));
  }
});

serverSelect.addEventListener("change", async () => {
  const tag = serverSelect.value;
  if (!tag) return;
  try {
    await invoke("switch_outbound", { tag });
    const profile = activeProfile();
    if (profile) profile.active_tag = tag;
  } catch (err) {
    showError(String(err));
  }
});

document.getElementById("btn-delete-profile").addEventListener("click", async () => {
  const id = profileSelect.value;
  if (!id) return;
  if (!window.confirm("Delete this profile?")) return;
  try {
    await invoke("delete_profile", { id });
    if (state.activeProfileId === id) state.activeProfileId = null;
    await loadProfiles();
  } catch (err) {
    showError(String(err));
  }
});

// ---------- import modal ----------

const importModal = document.getElementById("import-modal");
const importText = document.getElementById("import-text");
const importError = document.getElementById("import-error");

document.getElementById("btn-import").addEventListener("click", () => {
  importText.value = "";
  importError.classList.add("hidden");
  importModal.classList.remove("hidden");
});

document.getElementById("btn-import-cancel").addEventListener("click", () => {
  importModal.classList.add("hidden");
});

document.getElementById("btn-import-confirm").addEventListener("click", async () => {
  const text = importText.value.trim();
  if (!text) {
    importError.textContent = "Paste something first.";
    importError.classList.remove("hidden");
    return;
  }
  try {
    const profile = await invoke("import_profile", { text });
    importModal.classList.add("hidden");
    await loadProfiles();
    state.activeProfileId = profile.id;
    await invoke("set_active_profile", { id: profile.id });
    renderProfileSelect();
  } catch (err) {
    importError.textContent = String(err);
    importError.classList.remove("hidden");
  }
});

// ---------- connect / status / traffic ----------

const statusPill = document.getElementById("status-pill");
const statusText = document.getElementById("status-text");
const connectBtn = document.getElementById("btn-connect");
const connectLabel = document.getElementById("connect-label");

function applyStatus(status) {
  state.lastStatus = status;
  const s = (status.state || "stopped").toLowerCase();
  statusPill.dataset.state = s;
  statusText.textContent = s.charAt(0).toUpperCase() + s.slice(1);

  const connected = s === "running" || s === "starting";
  connectBtn.dataset.connected = String(connected);
  if (!connectPending) {
    connectLabel.textContent = s === "running" ? "Disconnect" : s === "starting" ? "Connecting…" : "Connect";
  }

  if (s !== "running") {
    document.getElementById("down-speed").textContent = formatSpeed(0);
    document.getElementById("up-speed").textContent = formatSpeed(0);
  }
}

async function pollStatus() {
  try {
    const status = await invoke("status");
    applyStatus(status);
    if ((status.state || "").toLowerCase() === "running") {
      const traffic = await invoke("traffic");
      document.getElementById("down-speed").textContent = formatSpeed(traffic.down_speed);
      document.getElementById("up-speed").textContent = formatSpeed(traffic.up_speed);
      document.getElementById("down-total").textContent = formatBytes(traffic.down_bytes);
      document.getElementById("up-total").textContent = formatBytes(traffic.up_bytes);
    }
  } catch (err) {
    console.error("status poll failed", err);
  }
}

function startPolling() {
  if (statusTimer) clearInterval(statusTimer);
  statusTimer = setInterval(pollStatus, 1500);
  pollStatus();
}

connectBtn.addEventListener("click", async () => {
  if (connectPending) return;
  const currentlyConnected = connectBtn.dataset.connected === "true";
  connectPending = true;
  connectBtn.dataset.pending = "true";
  connectLabel.textContent = currentlyConnected ? "Disconnecting…" : "Connecting…";
  try {
    if (currentlyConnected) {
      await invoke("disconnect");
    } else {
      if (!state.activeProfileId) {
        showError("Import and select a profile first.");
        return;
      }
      const status = await invoke("connect");
      applyStatus(status);
    }
  } catch (err) {
    showError(String(err));
  } finally {
    connectPending = false;
    connectBtn.dataset.pending = "false";
    await pollStatus();
  }
});

// ---------- split tunneling ----------

const splitRulesEl = document.getElementById("split-rules");
const splitRulesEmpty = document.getElementById("split-rules-empty");

// Human-readable labels for each `SplitRule` kind (matches wisp-core's
// serde-tagged `{"kind":"...","value":"..."}` shape).
const RULE_KIND_LABELS = {
  process: "App",
  process_path: "Path",
  process_path_regex: "Path regex",
  domain_suffix: "Domain",
  domain_regex: "Domain regex",
  ip_cidr: "IP",
  preset: "Preset",
};

// Friendly names for preset rules (a single `{kind:"preset",value:"<id>"}`
// entry that expands to many concrete rules in the engine). New presets get an
// entry here plus one in wisp-core's `presets::preset_label`.
const PRESET_LABELS = {
  valve: "Valve / Steam games (Dota 2, CS, Steam)",
};

// Example placeholders shown in the "add rule" value input, per kind.
const RULE_KIND_PLACEHOLDERS = {
  process: "chrome.exe",
  process_path_regex: "C:\\\\Games\\\\.*",
  domain_suffix: "example.com",
  domain_regex: "^ads\\.",
  ip_cidr: "1.2.3.0/24",
};

function ruleLabel(rule) {
  const kind = RULE_KIND_LABELS[rule.kind] || rule.kind;
  const value =
    rule.kind === "preset"
      ? PRESET_LABELS[rule.value] || rule.value
      : rule.value;
  return { kind, value };
}

async function loadSplit() {
  state.split = await invoke("get_split");
  renderSplit();
}

function renderSplit() {
  const modeInputs = document.querySelectorAll('input[name="split-mode"]');
  modeInputs.forEach((input) => {
    input.checked = input.value === state.split.mode;
  });

  splitRulesEl.innerHTML = "";
  if (!state.split.rules || state.split.rules.length === 0) {
    splitRulesEl.appendChild(splitRulesEmpty);
    return;
  }

  for (const rule of state.split.rules) {
    const { kind, value } = ruleLabel(rule);
    const row = document.createElement("div");
    row.className = "split-rule";
    if (rule.kind === "preset") row.classList.add("split-rule-preset");
    const label = document.createElement("span");
    const kindSpan = document.createElement("span");
    kindSpan.className = "rule-kind";
    kindSpan.textContent = kind;
    label.appendChild(kindSpan);
    label.appendChild(document.createTextNode(value));
    row.appendChild(label);
    const removeBtn = document.createElement("button");
    removeBtn.className = "rule-remove";
    removeBtn.textContent = "✕";
    removeBtn.addEventListener("click", async () => {
      try {
        await invoke("remove_split_rule", { rule });
        await loadSplit();
      } catch (err) {
        showError(String(err));
      }
    });
    row.appendChild(removeBtn);
    splitRulesEl.appendChild(row);
  }
}

document.querySelectorAll('input[name="split-mode"]').forEach((input) => {
  input.addEventListener("change", async () => {
    if (!input.checked) return;
    try {
      await invoke("set_split_mode", { mode: input.value });
      state.split.mode = input.value;
    } catch (err) {
      showError(String(err));
    }
  });
});

// ---------- add rule (kind + value) ----------

const selectRuleKind = document.getElementById("select-rule-kind");
const inputRuleValue = document.getElementById("input-rule-value");

function updateRuleValuePlaceholder() {
  inputRuleValue.placeholder = RULE_KIND_PLACEHOLDERS[selectRuleKind.value] || "";
}
selectRuleKind.addEventListener("change", updateRuleValuePlaceholder);
updateRuleValuePlaceholder();

document.getElementById("btn-add-rule").addEventListener("click", async () => {
  const kind = selectRuleKind.value;
  const value = inputRuleValue.value.trim();
  if (!value) return;
  try {
    await invoke("add_split_rule", { rule: { kind, value } });
    inputRuleValue.value = "";
    await loadSplit();
  } catch (err) {
    showError(String(err));
  }
});

// ---------- Valve/Steam gaming preset ----------

document.getElementById("btn-valve-preset").addEventListener("click", async () => {
  try {
    state.split = await invoke("add_valve_preset");
    renderSplit();
  } catch (err) {
    showError(String(err));
  }
});

// ---------- export / import split config ----------

document.getElementById("btn-export-split").addEventListener("click", async () => {
  try {
    const path = await window.__TAURI__.dialog.save({
      defaultPath: "wisp-split.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    await invoke("export_split", { path });
  } catch (err) {
    showError(String(err));
  }
});

document.getElementById("btn-import-split").addEventListener("click", async () => {
  try {
    const path = await window.__TAURI__.dialog.open({
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    state.split = await invoke("import_split", { path });
    renderSplit();
  } catch (err) {
    showError(String(err));
  }
});

// ---------- add app modal ----------

const appModal = document.getElementById("app-modal");
const appList = document.getElementById("app-list");
const appFilter = document.getElementById("app-filter");
let runningProcesses = [];

document.getElementById("btn-add-app").addEventListener("click", async () => {
  appFilter.value = "";
  appList.innerHTML = "<div class='empty-hint'>Loading…</div>";
  appModal.classList.remove("hidden");
  try {
    runningProcesses = await invoke("list_running_processes");
    renderAppList("");
  } catch (err) {
    appList.innerHTML = "";
    showError(String(err));
  }
});

document.getElementById("btn-app-cancel").addEventListener("click", () => {
  appModal.classList.add("hidden");
});

appFilter.addEventListener("input", () => renderAppList(appFilter.value.trim().toLowerCase()));

function renderAppList(filter) {
  appList.innerHTML = "";
  const filtered = filter ? runningProcesses.filter((name) => name.toLowerCase().includes(filter)) : runningProcesses;
  if (filtered.length === 0) {
    appList.innerHTML = "<div class='empty-hint'>No matching processes.</div>";
    return;
  }
  for (const name of filtered.slice(0, 200)) {
    const item = document.createElement("button");
    item.className = "pick-item";
    item.textContent = name;
    item.addEventListener("click", async () => {
      try {
        await invoke("add_split_rule", { rule: { kind: "process", value: name } });
        appModal.classList.add("hidden");
        await loadSplit();
      } catch (err) {
        showError(String(err));
      }
    });
    appList.appendChild(item);
  }
}

// ---------- settings ----------

const inputMtu = document.getElementById("input-mtu");
const inputAutostart = document.getElementById("input-autostart");
const selectLogLevel = document.getElementById("select-log-level");

async function loadSettings() {
  state.settings = await invoke("get_settings");
  inputMtu.value = state.settings.mtu;
  inputAutostart.checked = !!state.settings.autostart;
  selectLogLevel.value = state.settings.log_level || "info";
}

document.getElementById("btn-save-settings").addEventListener("click", async () => {
  if (!state.settings) return;
  const updated = {
    ...state.settings,
    mtu: Number(inputMtu.value) || 1280,
    autostart: inputAutostart.checked,
    log_level: selectLogLevel.value,
  };
  try {
    await invoke("set_settings", { settings: updated });
    state.settings = updated;
  } catch (err) {
    showError(String(err));
  }
});

// ---------- logs ----------

const logsToggle = document.getElementById("logs-toggle");
const logsPanel = document.getElementById("logs-panel");
const logsChevron = document.getElementById("logs-chevron");
const logsOutput = document.getElementById("logs-output");

async function pollLogs() {
  try {
    const lines = await invoke("logs", { n: 200 });
    logsOutput.textContent = lines.join("\n");
    logsOutput.scrollTop = logsOutput.scrollHeight;
  } catch (err) {
    console.error("logs poll failed", err);
  }
}

logsToggle.addEventListener("click", () => {
  const isHidden = logsPanel.classList.contains("hidden");
  if (isHidden) {
    logsPanel.classList.remove("hidden");
    logsChevron.classList.add("open");
    pollLogs();
    logsTimer = setInterval(pollLogs, 2000);
  } else {
    logsPanel.classList.add("hidden");
    logsChevron.classList.remove("open");
    if (logsTimer) clearInterval(logsTimer);
  }
});

// ---------- init ----------

async function init() {
  try {
    await loadProfiles();
    await loadSplit();
    await loadSettings();
  } catch (err) {
    showError(String(err));
  }
  startPolling();
}

window.addEventListener("DOMContentLoaded", init);
