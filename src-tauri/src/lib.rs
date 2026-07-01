//! Wisp desktop app: Tauri v2 backend. Wires up persisted app state, the
//! sing-box engine, all `#[tauri::command]`s, and a system tray so the
//! window can be hidden instead of closed.

pub mod commands;
mod elevation;
pub mod state;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WindowEvent};

use state::AppState;

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Creating the TUN adapter requires admin rights. On Windows, if we're
    // not elevated, this relaunches the app elevated (UAC prompt) and
    // returns `false` so this (non-elevated) instance can exit quietly. This
    // is a no-op (always `true`) on other platforms.
    if !elevation::ensure_elevated() {
        return;
    }

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            let config_dir = app
                .path()
                .app_config_dir()
                .map_err(|e| format!("could not resolve app config dir: {e}"))?;
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
            commands::list_running_processes,
            commands::get_settings,
            commands::set_settings,
        ])
        .run(tauri::generate_context!());

    if let Err(err) = result {
        tracing::error!("tauri application error: {err}");
    }
}
