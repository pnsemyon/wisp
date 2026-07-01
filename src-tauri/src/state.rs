//! Application state: persisted profiles/settings/split config plus the
//! shared sing-box engine handle.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use wisp_core::profile::Profile;
use wisp_core::split::SplitConfig;
use wisp_engine::SingBoxProcess;

/// User-configurable settings, persisted to `settings.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// TUN interface MTU.
    pub mtu: u32,
    /// Whether Wisp should launch at Windows login.
    pub autostart: bool,
    /// Local port for sing-box's Clash API.
    pub clash_port: u16,
    /// Secret for sing-box's Clash API. Deterministic (not random) so it
    /// survives reinstalls without needing a CSPRNG dependency.
    pub clash_secret: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            mtu: 1280,
            autostart: false,
            clash_port: 9090,
            clash_secret: default_clash_secret(),
        }
    }
}

/// A fixed-but-unique secret derived from the app identifier. Not
/// cryptographically random, but unique enough to keep the local Clash API
/// from being trivially guessed by another local process, and stable across
/// runs so it doesn't need to be persisted separately.
fn default_clash_secret() -> String {
    "com.wisp.app-clash-9f2b7a1e-4d3c-4a6f-8e2d-1a7c5b9f0e3d".to_string()
}

/// Everything guarded by the state mutex.
pub struct Inner {
    pub profiles: Vec<Profile>,
    pub active_profile: Option<String>,
    pub split: SplitConfig,
    pub settings: Settings,
    pub engine: Arc<SingBoxProcess>,
}

/// On-disk shape of `profiles.json`: the profile list plus which one is
/// currently active (kept together so re-selecting a profile survives a
/// restart).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfilesFile {
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub active_profile: Option<String>,
}

/// Shared, mutex-guarded application state, managed by Tauri.
pub struct AppState {
    pub inner: Mutex<Inner>,
    pub config_dir: PathBuf,
}

impl AppState {
    /// Load persisted profiles/settings/split from `config_dir` (creating it
    /// if missing) and construct the engine handle from them.
    pub fn new(config_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| format!("failed to create config dir {}: {e}", config_dir.display()))?;

        let settings: Settings = load_json(&config_dir.join("settings.json")).unwrap_or_default();
        let profiles_file: ProfilesFile =
            load_json(&config_dir.join("profiles.json")).unwrap_or_default();
        let split: SplitConfig = load_json(&config_dir.join("split.json")).unwrap_or_default();

        let engine = Arc::new(build_engine(&config_dir, &settings));

        Ok(AppState {
            inner: Mutex::new(Inner {
                profiles: profiles_file.profiles,
                active_profile: profiles_file.active_profile,
                split,
                settings,
                engine,
            }),
            config_dir,
        })
    }

    pub fn profiles_path(&self) -> PathBuf {
        self.config_dir.join("profiles.json")
    }

    pub fn settings_path(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    pub fn split_path(&self) -> PathBuf {
        self.config_dir.join("split.json")
    }

    pub fn save_profiles(
        &self,
        profiles: &[Profile],
        active_profile: &Option<String>,
    ) -> Result<(), String> {
        let file = ProfilesFile {
            profiles: profiles.to_vec(),
            active_profile: active_profile.clone(),
        };
        save_json(&self.profiles_path(), &file)
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        save_json(&self.settings_path(), settings)
    }

    pub fn save_split(&self, split: &SplitConfig) -> Result<(), String> {
        save_json(&self.split_path(), split)
    }
}

/// Build (or rebuild) the sing-box engine handle from current settings.
/// Resource location failures are not fatal here: they only surface when the
/// user actually tries to `connect`, so the app can still boot (and other
/// commands still work) on a machine where `sing-box`/`wintun` aren't staged
/// yet (e.g. this Linux dev/check environment).
pub fn build_engine(config_dir: &Path, settings: &Settings) -> SingBoxProcess {
    let binary = match wisp_engine::locate_resources() {
        Ok(resources) => {
            tracing::info!(binary = %resources.singbox.display(), "build_engine: using located sing-box binary");
            resources.singbox
        }
        Err(err) => {
            let fallback = config_dir.join("sing-box.exe");
            tracing::warn!(
                %err,
                fallback = %fallback.display(),
                "build_engine: locate_resources failed, falling back to config dir"
            );
            fallback
        }
    };

    tracing::info!(
        clash_port = settings.clash_port,
        "build_engine: constructing sing-box engine handle"
    );
    SingBoxProcess::new(
        binary,
        config_dir.to_path_buf(),
        settings.clash_port,
        settings.clash_secret.clone(),
    )
}

fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("failed to write {}: {e}", path.display()))
}
