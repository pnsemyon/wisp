//! Wisp desktop app: Tauri v2 backend. Wires up persisted app state, the
//! sing-box engine, all `#[tauri::command]`s, and a system tray so the
//! window can be hidden instead of closed.

pub mod commands;
mod elevation;
pub mod state;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};
use tracing_subscriber::prelude::*;

use state::AppState;

/// Default `tracing` filter used when neither `RUST_LOG` nor `WISP_LOG` is
/// set: info-level everywhere, debug for our own crates.
const DEFAULT_LOG_FILTER: &str = "info,wisp_core=debug,wisp_engine=debug,wisp_app=debug";

/// Install a `tracing` subscriber that writes to both stderr (useful for
/// `cargo tauri dev`) and a daily-rotating log file under
/// `%LOCALAPPDATA%\Wisp\logs\wisp.log` (or the platform equivalent of
/// `dirs::data_local_dir()`), since this app is built with
/// `windows_subsystem = "windows"` and otherwise has no console for stderr
/// to go to on an installed machine.
///
/// Returns the `WorkerGuard` for the non-blocking file writer, which must be
/// kept alive for the duration of `run()` or buffered log lines can be lost
/// on shutdown. Returns `None` only if the log directory couldn't be
/// resolved/created, in which case logging still works via stderr.
fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let filter_directives = std::env::var("RUST_LOG")
        .or_else(|_| std::env::var("WISP_LOG"))
        .unwrap_or_else(|_| DEFAULT_LOG_FILTER.to_string());
    let make_filter = || {
        tracing_subscriber::EnvFilter::try_new(&filter_directives)
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(DEFAULT_LOG_FILTER))
    };

    let stderr_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_line_number(true)
        .with_writer(std::io::stderr)
        .with_filter(make_filter());

    let log_dir = dirs::data_local_dir().map(|dir| dir.join("Wisp").join("logs"));
    let (file_layer, guard, log_file_path) = match log_dir {
        Some(dir) => match std::fs::create_dir_all(&dir) {
            Ok(()) => {
                let file_appender = tracing_appender::rolling::daily(&dir, "wisp.log");
                let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
                let layer = tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_target(true)
                    .with_line_number(true)
                    .with_writer(non_blocking)
                    .with_filter(make_filter());
                (Some(layer), Some(guard), Some(dir.join("wisp.log")))
            }
            Err(err) => {
                eprintln!("wisp: could not create log dir {}: {err}", dir.display());
                (None, None, None)
            }
        },
        None => {
            eprintln!("wisp: could not resolve local data dir for logging");
            (None, None, None)
        }
    };

    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer)
        .init();

    tracing::info!(
        app = "Wisp",
        version = env!("CARGO_PKG_VERSION"),
        log_filter = %filter_directives,
        log_file = ?log_file_path,
        "startup: logging initialized"
    );
    if log_file_path.is_none() {
        tracing::warn!("startup: file logging is unavailable; only stderr logging is active");
    }

    guard
}

pub fn run() {
    let _log_guard = init_logging();

    // Creating the TUN adapter requires admin rights. On Windows, if we're
    // not elevated, this relaunches the app elevated (UAC prompt) and
    // returns `false` so this (non-elevated) instance can exit quietly. This
    // is a no-op (always `true`) on other platforms.
    if !elevation::ensure_elevated() {
        tracing::info!("startup: not elevated, relaunching elevated and exiting this instance");
        return;
    }

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            // Tell wisp-engine where Tauri unpacked the bundled sing-box.exe +
            // wintun.dll (see `bundle.resources` in tauri.conf.json), so
            // `locate_resources()` finds them in the installed app.
            match app.path().resource_dir() {
                Ok(res_dir) => {
                    tracing::info!(resource_dir = %res_dir.display(), "startup: resolved resource dir");
                    std::env::set_var("WISP_RESOURCE_DIR", res_dir);
                }
                Err(err) => {
                    tracing::warn!(%err, "startup: could not resolve resource dir; falling back to default search paths");
                }
            }

            let config_dir = app
                .path()
                .app_config_dir()
                .map_err(|e| format!("could not resolve app config dir: {e}"))?;
            tracing::info!(config_dir = %config_dir.display(), "startup: resolved app config dir");
            let state = AppState::new(config_dir)?;
            app.manage(state);

            let connect_item = MenuItem::with_id(app, "connect", "Connect", true, None::<&str>)?;
            let disconnect_item = MenuItem::with_id(app, "disconnect", "Disconnect", true, None::<&str>)?;
            let show_item = MenuItem::with_id(app, "show", "Show Wisp", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let tray_menu = Menu::with_items(
                app,
                &[&connect_item, &disconnect_item, &show_item, &quit_item],
            )?;

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .tooltip("Wisp");
            if let Some(icon) = app.default_window_icon().cloned() {
                tray_builder = tray_builder.icon(icon);
            }

            tray_builder
                .on_menu_event(|app, event| match event.id().0.as_str() {
                    "connect" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app.state::<AppState>();
                            if let Err(err) = commands::connect(state).await {
                                tracing::error!("tray connect failed: {err}");
                            }
                        });
                    }
                    "disconnect" => {
                        let app = app.clone();
                        tauri::async_runtime::spawn(async move {
                            let state = app.state::<AppState>();
                            if let Err(err) = commands::disconnect(state).await {
                                tracing::error!("tray disconnect failed: {err}");
                            }
                        });
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the main window just hides it to the tray; the app
            // keeps running (and the tunnel stays up) until "Quit".
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::import_profile,
            commands::list_profiles,
            commands::delete_profile,
            commands::set_active_profile,
            commands::connect,
            commands::disconnect,
            commands::status,
            commands::traffic,
            commands::logs,
            commands::switch_outbound,
            commands::get_split,
            commands::set_split_mode,
            commands::add_split_rule,
            commands::remove_split_rule,
            commands::export_split,
            commands::import_split,
            commands::add_valve_preset,
            commands::list_running_processes,
            commands::get_settings,
            commands::set_settings,
        ])
        .run(tauri::generate_context!());

    if let Err(err) = result {
        tracing::error!("tauri application error: {err}");
    }
}
