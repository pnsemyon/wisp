//! Tauri commands exposed to the UI. Every command returns `Result<T, String>`
//! so the webview gets a readable error message instead of a panic.

use tauri::State;

use wisp_core::{build_config, BuildSettings, Profile, SplitConfig, SplitMode, SplitRule};
use wisp_engine::{Engine, EngineStatus, TrafficStats};

use crate::state::{build_engine, AppState, Settings};

/// Import a profile from pasted sing-box JSON or share link(s) and add it to
/// the profile list. If the derived id collides with an existing profile, a
/// numeric suffix is appended (mirrors `wisp_core::profile::unique_id`,
/// which isn't public).
#[tauri::command]
pub async fn import_profile(state: State<'_, AppState>, text: String) -> Result<Profile, String> {
    let mut profile = wisp_core::import(&text).map_err(|e| e.to_string())?;

    let mut inner = state.inner.lock().await;
    dedupe_id(&mut profile, &inner.profiles);
    inner.profiles.push(profile.clone());
    state.save_profiles(&inner.profiles, &inner.active_profile)?;
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
    let mut inner = state.inner.lock().await;
    inner.profiles.retain(|p| p.id != id);
    if inner.active_profile.as_deref() == Some(id.as_str()) {
        inner.active_profile = None;
    }
    state.save_profiles(&inner.profiles, &inner.active_profile)
}

#[tauri::command]
pub async fn set_active_profile(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut inner = state.inner.lock().await;
    if !inner.profiles.iter().any(|p| p.id == id) {
        return Err(format!("no such profile: {id}"));
    }
    inner.active_profile = Some(id);
    state.save_profiles(&inner.profiles, &inner.active_profile)
}

/// Build a fresh sing-box config from the active profile + split + settings,
/// (re)construct the engine so it always reflects the latest Clash API
/// port/secret, and start it.
#[tauri::command]
pub async fn connect(state: State<'_, AppState>) -> Result<EngineStatus, String> {
    let mut inner = state.inner.lock().await;

    let active_id = inner
        .active_profile
        .clone()
        .ok_or_else(|| "no active profile selected".to_string())?;
    let profile = inner
        .profiles
        .iter()
        .find(|p| p.id == active_id)
        .cloned()
        .ok_or_else(|| "active profile no longer exists".to_string())?;

    let build_settings = BuildSettings {
        mtu: inner.settings.mtu,
        clash_secret: inner.settings.clash_secret.clone(),
        clash_port: inner.settings.clash_port,
        socks_port: None,
    };
    let config = build_config(&profile, &inner.split, &build_settings).map_err(|e| e.to_string())?;

    // Rebuilding the engine below always picks up the latest Clash API
    // port/secret from settings, but if a previous engine instance is still
    // running, stop it first so we don't leak an orphaned sing-box process.
    let previous_state = inner.engine.status().await.state;
    if !matches!(previous_state, wisp_engine::EngineState::Stopped) {
        let _ = inner.engine.stop().await;
    }

    let engine = std::sync::Arc::new(build_engine(&state.config_dir, &inner.settings));
    engine.start(config).await.map_err(|e| e.to_string())?;
    inner.engine = engine;

    Ok(inner.engine.status().await)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<(), String> {
    let inner = state.inner.lock().await;
    inner.engine.stop().await.map_err(|e| e.to_string())
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
    let mut inner = state.inner.lock().await;
    inner.engine.switch(&tag).await.map_err(|e| e.to_string())?;

    if let Some(active_id) = inner.active_profile.clone() {
        if let Some(profile) = inner.profiles.iter_mut().find(|p| p.id == active_id) {
            profile.active_tag = Some(tag);
        }
        state.save_profiles(&inner.profiles, &inner.active_profile)?;
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
    let mut inner = state.inner.lock().await;
    inner.split.mode = mode;
    state.save_split(&inner.split)
}

#[tauri::command]
pub async fn add_split_rule(state: State<'_, AppState>, rule: SplitRule) -> Result<(), String> {
    let mut inner = state.inner.lock().await;
    if !inner.split.rules.contains(&rule) {
        inner.split.rules.push(rule);
    }
    state.save_split(&inner.split)
}

#[tauri::command]
pub async fn remove_split_rule(state: State<'_, AppState>, rule: SplitRule) -> Result<(), String> {
    let mut inner = state.inner.lock().await;
    inner.split.rules.retain(|r| r != &rule);
    state.save_split(&inner.split)
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
    let mut inner = state.inner.lock().await;
    inner.settings = settings;

    // Only rebuild the engine handle (picking up the new Clash API
    // port/secret/mtu) if nothing is running: rebuilding while connected
    // would point control commands (status/stats/switch) at a port the live
    // sing-box process isn't actually listening on. If it's currently
    // running, the new settings simply take effect on the next `connect`.
    let current_state = inner.engine.status().await.state;
    if matches!(current_state, wisp_engine::EngineState::Stopped) {
        inner.engine = std::sync::Arc::new(build_engine(&state.config_dir, &inner.settings));
    }

    state.save_settings(&inner.settings)
}
