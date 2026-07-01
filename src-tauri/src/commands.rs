//! Tauri commands exposed to the UI. Every command returns `Result<T, String>`
//! so the webview gets a readable error message instead of a panic.

use tauri::State;
use tracing::{debug, error, info, warn};

use wisp_core::{build_config, BuildSettings, Profile, SplitConfig, SplitMode, SplitRule};
use wisp_engine::{Engine, EngineStatus, TrafficStats};

use crate::state::{build_engine, AppState, Settings};

/// Import a profile from pasted sing-box JSON or share link(s) and add it to
/// the profile list. If the derived id collides with an existing profile, a
/// numeric suffix is appended (mirrors `wisp_core::profile::unique_id`,
/// which isn't public).
#[tauri::command]
pub async fn import_profile(state: State<'_, AppState>, text: String) -> Result<Profile, String> {
    info!(input_len = text.len(), "import_profile: importing");
    let mut profile = match wisp_core::import(&text) {
        Ok(profile) => profile,
        Err(err) => {
            error!(%err, "import_profile: parse failed");
            return Err(err.to_string());
        }
    };

    let mut inner = state.inner.lock().await;
    dedupe_id(&mut profile, &inner.profiles);
    inner.profiles.push(profile.clone());
    if let Err(err) = state.save_profiles(&inner.profiles, &inner.active_profile) {
        error!(%err, "import_profile: save failed");
        return Err(err);
    }
    info!(profile_id = %profile.id, profile_name = %profile.name, outbounds = profile.outbounds.len(), "import_profile: imported");
    Ok(profile)
}

fn dedupe_id(profile: &mut Profile, existing: &[Profile]) {
    if !existing.iter().any(|p| p.id == profile.id) {
        return;
    }
    let base = profile.id.clone();
    let mut idx = 1;
    loop {
        let candidate = format!("{base}-{idx}");
        if !existing.iter().any(|p| p.id == candidate) {
            profile.id = candidate;
            return;
        }
        idx += 1;
    }
}

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<Profile>, String> {
    let inner = state.inner.lock().await;
    Ok(inner.profiles.clone())
}

#[tauri::command]
pub async fn delete_profile(state: State<'_, AppState>, id: String) -> Result<(), String> {
    info!(profile_id = %id, "delete_profile: deleting");
    let mut inner = state.inner.lock().await;
    inner.profiles.retain(|p| p.id != id);
    if inner.active_profile.as_deref() == Some(id.as_str()) {
        inner.active_profile = None;
    }
    let result = state.save_profiles(&inner.profiles, &inner.active_profile);
    if let Err(err) = &result {
        error!(profile_id = %id, %err, "delete_profile: save failed");
    }
    result
}

#[tauri::command]
pub async fn set_active_profile(state: State<'_, AppState>, id: String) -> Result<(), String> {
    info!(profile_id = %id, "set_active_profile: activating");
    let mut inner = state.inner.lock().await;
    if !inner.profiles.iter().any(|p| p.id == id) {
        error!(profile_id = %id, "set_active_profile: no such profile");
        return Err(format!("no such profile: {id}"));
    }
    inner.active_profile = Some(id);
    let result = state.save_profiles(&inner.profiles, &inner.active_profile);
    if let Err(err) = &result {
        error!(%err, "set_active_profile: save failed");
    }
    result
}

/// Build a fresh sing-box config from the active profile + split + settings,
/// (re)construct the engine so it always reflects the latest Clash API
/// port/secret, and start it.
#[tauri::command]
pub async fn connect(state: State<'_, AppState>) -> Result<EngineStatus, String> {
    let mut inner = state.inner.lock().await;

    let active_id = match inner.active_profile.clone() {
        Some(id) => id,
        None => {
            error!("connect: no active profile selected");
            return Err("no active profile selected".to_string());
        }
    };
    let profile = match inner.profiles.iter().find(|p| p.id == active_id).cloned() {
        Some(profile) => profile,
        None => {
            error!(profile_id = %active_id, "connect: active profile no longer exists");
            return Err("active profile no longer exists".to_string());
        }
    };

    info!(
        profile_id = %profile.id,
        profile_name = %profile.name,
        split_mode = ?inner.split.mode,
        rule_count = inner.split.rules.len(),
        mtu = inner.settings.mtu,
        clash_port = inner.settings.clash_port,
        "connect: starting"
    );

    let build_settings = BuildSettings {
        mtu: inner.settings.mtu,
        clash_secret: inner.settings.clash_secret.clone(),
        clash_port: inner.settings.clash_port,
        socks_port: None,
        log_level: inner.settings.log_level.clone(),
    };
    let config = match build_config(&profile, &inner.split, &build_settings) {
        Ok(config) => config,
        Err(err) => {
            error!(%err, "connect: build_config failed");
            return Err(err.to_string());
        }
    };
    if let Ok(serialized) = serde_json::to_string(&config) {
        debug!(config_bytes = serialized.len(), "connect: generated config");
    }

    match wisp_engine::locate_resources() {
        Ok(resources) => debug!(
            binary = %resources.singbox.display(),
            wintun_dir = %resources.wintun_dir.display(),
            "connect: located sing-box resources"
        ),
        Err(err) => warn!(%err, "connect: could not locate sing-box resources"),
    }

    // Rebuilding the engine below always picks up the latest Clash API
    // port/secret from settings, but if a previous engine instance is still
    // running, stop it first so we don't leak an orphaned sing-box process.
    let previous_state = inner.engine.status().await.state;
    if !matches!(previous_state, wisp_engine::EngineState::Stopped) {
        debug!(
            ?previous_state,
            "connect: stopping previous engine instance first"
        );
        if let Err(err) = inner.engine.stop().await {
            warn!(%err, "connect: stopping previous engine instance failed");
        }
    }

    let engine = std::sync::Arc::new(build_engine(&state.config_dir, &inner.settings));
    if let Err(err) = engine.start(config).await {
        error!(%err, "connect: engine start failed");
        return Err(err.to_string());
    }
    inner.engine = engine;

    let status = inner.engine.status().await;
    info!(?status, "connect: succeeded");
    Ok(status)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<(), String> {
    info!("disconnect: stopping engine");
    let inner = state.inner.lock().await;
    let result = inner.engine.stop().await.map_err(|e| e.to_string());
    if let Err(err) = &result {
        error!(%err, "disconnect: failed");
    }
    result
}

#[tauri::command]
pub async fn status(state: State<'_, AppState>) -> Result<EngineStatus, String> {
    let inner = state.inner.lock().await;
    Ok(inner.engine.status().await)
}

#[tauri::command]
pub async fn traffic(state: State<'_, AppState>) -> Result<TrafficStats, String> {
    let inner = state.inner.lock().await;
    inner.engine.stats().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn logs(state: State<'_, AppState>, n: usize) -> Result<Vec<String>, String> {
    let inner = state.inner.lock().await;
    Ok(inner.engine.logs(n).await)
}

#[tauri::command]
pub async fn switch_outbound(state: State<'_, AppState>, tag: String) -> Result<(), String> {
    info!(tag = %tag, "switch_outbound: switching");
    let mut inner = state.inner.lock().await;
    if let Err(err) = inner.engine.switch(&tag).await {
        error!(tag = %tag, %err, "switch_outbound: engine switch failed");
        return Err(err.to_string());
    }

    if let Some(active_id) = inner.active_profile.clone() {
        if let Some(profile) = inner.profiles.iter_mut().find(|p| p.id == active_id) {
            profile.active_tag = Some(tag);
        }
        if let Err(err) = state.save_profiles(&inner.profiles, &inner.active_profile) {
            error!(%err, "switch_outbound: save failed");
            return Err(err);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_split(state: State<'_, AppState>) -> Result<SplitConfig, String> {
    let inner = state.inner.lock().await;
    Ok(inner.split.clone())
}

#[tauri::command]
pub async fn set_split_mode(state: State<'_, AppState>, mode: SplitMode) -> Result<(), String> {
    info!(mode = ?mode, "set_split_mode: updating");
    {
        let mut inner = state.inner.lock().await;
        inner.split.mode = mode;
        if let Err(err) = state.save_split(&inner.split) {
            error!(%err, "set_split_mode: save failed");
            return Err(err);
        }
    }
    apply_and_maybe_reconnect(state).await
}

#[tauri::command]
pub async fn add_split_rule(state: State<'_, AppState>, rule: SplitRule) -> Result<(), String> {
    info!(rule = ?rule, "add_split_rule: adding");
    {
        let mut inner = state.inner.lock().await;
        if !inner.split.rules.contains(&rule) {
            inner.split.rules.push(rule);
        }
        if let Err(err) = state.save_split(&inner.split) {
            error!(%err, "add_split_rule: save failed");
            return Err(err);
        }
    }
    apply_and_maybe_reconnect(state).await
}

#[tauri::command]
pub async fn remove_split_rule(state: State<'_, AppState>, rule: SplitRule) -> Result<(), String> {
    info!(rule = ?rule, "remove_split_rule: removing");
    {
        let mut inner = state.inner.lock().await;
        inner.split.rules.retain(|r| r != &rule);
        if let Err(err) = state.save_split(&inner.split) {
            error!(%err, "remove_split_rule: save failed");
            return Err(err);
        }
    }
    apply_and_maybe_reconnect(state).await
}

/// Write the current split-tunnel config as pretty JSON to `path`, for the
/// user to back up or share.
#[tauri::command]
pub async fn export_split(state: State<'_, AppState>, path: String) -> Result<(), String> {
    info!(path = %path, "export_split: exporting");
    let split = {
        let inner = state.inner.lock().await;
        inner.split.clone()
    };
    let text = serde_json::to_string_pretty(&split).map_err(|e| e.to_string())?;
    let result = std::fs::write(&path, text).map_err(|e| format!("failed to write {path}: {e}"));
    if let Err(err) = &result {
        error!(%err, "export_split: write failed");
    }
    result
}

/// Read a `SplitConfig` from `path`, replace the current one, persist it,
/// and (if connected) apply it immediately.
#[tauri::command]
pub async fn import_split(state: State<'_, AppState>, path: String) -> Result<SplitConfig, String> {
    info!(path = %path, "import_split: importing");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("failed to read {path}: {e}"))?;
    let split: SplitConfig = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    {
        let mut inner = state.inner.lock().await;
        inner.split = split.clone();
        if let Err(err) = state.save_split(&inner.split) {
            error!(%err, "import_split: save failed");
            return Err(err);
        }
    }
    apply_and_maybe_reconnect(state).await?;
    Ok(split)
}

/// One-click preset: add every rule from `wisp_core::valve_gaming_preset()`
/// (Valve/Steam IP ranges + domains) to the split config, switching a
/// currently-`Off` mode to `Blacklist` since the preset is only meaningful
/// as an exclude list. Applies immediately if the tunnel is connected.
#[tauri::command]
pub async fn add_valve_preset(state: State<'_, AppState>) -> Result<SplitConfig, String> {
    info!("add_valve_preset: adding Valve/Steam gaming preset rules");
    let split = {
        let mut inner = state.inner.lock().await;
        for rule in wisp_core::valve_gaming_preset() {
            if !inner.split.rules.contains(&rule) {
                inner.split.rules.push(rule);
            }
        }
        if matches!(inner.split.mode, SplitMode::Off) {
            inner.split.mode = SplitMode::Blacklist;
        }
        if let Err(err) = state.save_split(&inner.split) {
            error!(%err, "add_valve_preset: save failed");
            return Err(err);
        }
        inner.split.clone()
    };
    apply_and_maybe_reconnect(state).await?;
    Ok(split)
}

/// Unique process executable names currently running, sorted, for the "Add
/// app" split-tunnel picker. Cross-platform via `sysinfo` (on Windows this
/// lists e.g. `chrome.exe`).
#[tauri::command]
pub async fn list_running_processes() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut system = sysinfo::System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let mut names: Vec<String> = system
            .processes()
            .values()
            .filter_map(|p| p.name().to_str().map(str::to_string))
            .collect();
        names.sort();
        names.dedup();
        names
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let inner = state.inner.lock().await;
    Ok(inner.settings.clone())
}

#[tauri::command]
pub async fn set_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    info!(
        mtu = settings.mtu,
        autostart = settings.autostart,
        clash_port = settings.clash_port,
        log_level = %settings.log_level,
        "set_settings: updating"
    );
    {
        let mut inner = state.inner.lock().await;
        inner.settings = settings;

        // Only rebuild the engine handle here (picking up the new Clash API
        // port/secret/mtu) if nothing is running: rebuilding while connected
        // would point control commands (status/stats/switch) at a port the
        // live sing-box process isn't actually listening on. If it's
        // currently running, `apply_and_maybe_reconnect` below rebuilds the
        // engine (and the config) as part of the reconnect.
        let current_state = inner.engine.status().await.state;
        if matches!(current_state, wisp_engine::EngineState::Stopped) {
            debug!("set_settings: rebuilding engine handle (engine currently stopped)");
            inner.engine = std::sync::Arc::new(build_engine(&state.config_dir, &inner.settings));
        } else {
            debug!(
                ?current_state,
                "set_settings: engine running, will rebuild config and reconnect"
            );
        }

        if let Err(err) = state.save_settings(&inner.settings) {
            error!(%err, "set_settings: save failed");
            return Err(err);
        }
    }
    apply_and_maybe_reconnect(state).await
}

/// Shared by `set_settings`/`set_split_mode`/`add_split_rule`/
/// `remove_split_rule`/`import_split`: if the engine is currently running,
/// rebuild the sing-box config from the (already persisted) settings/split
/// and restart the engine so the change takes effect immediately, by
/// reusing `connect` (which itself stops any previous instance before
/// starting the new one). If the engine isn't running, this is a no-op —
/// the change simply takes effect on the next `connect`.
///
/// Callers must not hold `state.inner`'s lock when calling this: it takes
/// (and releases) the lock itself just to read the engine state, then calls
/// `connect`, which takes its own lock. Holding the guard across that call
/// would deadlock on the (non-reentrant) tokio mutex.
async fn apply_and_maybe_reconnect(state: State<'_, AppState>) -> Result<(), String> {
    let is_running = {
        let inner = state.inner.lock().await;
        matches!(
            inner.engine.status().await.state,
            wisp_engine::EngineState::Running
        )
    };
    if !is_running {
        return Ok(());
    }

    info!("apply_and_maybe_reconnect: engine running, rebuilding config and reconnecting");
    if let Err(err) = connect(state).await {
        error!(%err, "apply_and_maybe_reconnect: reconnect failed");
        return Err(err);
    }
    Ok(())
}
