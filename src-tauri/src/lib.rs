mod commands;
mod provider;
mod state;
mod storage;
mod tray;

use provider::plugin;
use provider::registry::ProviderRegistry;
use provider::types::{Currency, ProviderIcon, ProviderState, ProviderStatus};
use state::AppState;
use tauri::ActivationPolicy;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt as AutostartExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_sql::Builder::new().build())
        .manage(AppState::default())
        .setup(|app| {
            storage::ensure_dirs()?;

            // Initialize history database
            let history_db_path = dirs::home_dir()
                .ok_or_else(|| "home dir not found".to_string())
                .map_err(|e| Box::<dyn std::error::Error>::from(e))?
                .join(".costwatch")
                .join("history.db");
            let conn = rusqlite::Connection::open(&history_db_path)?;
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS provider_history (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    provider_id TEXT NOT NULL,
                    recorded_at TEXT NOT NULL,
                    value REAL NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_provider_time
                    ON provider_history(provider_id, recorded_at);"
            )?;

            let settings = storage::load_settings().unwrap_or_default();

            if settings.launch_at_login {
                let _ = app.autolaunch().enable();
            } else {
                let _ = app.autolaunch().disable();
            }

            let mut registry = ProviderRegistry::new();
            for (id, config) in provider::builtin::all_builtin_configs() {
                registry.register(id.to_string(), config)?;
            }
            for (id, config) in plugin::load_plugin_configs() {
                registry.register(id, config)?;
            }

            let configs = registry.list().clone();
            let state = app.state::<AppState>();
            state.settings.lock().unwrap().clone_from(&settings);
            *state.configs.lock().unwrap() = configs.clone();

            // Initialize providers
            let tokens = storage::load_tokens().unwrap_or_default();
            {
                let mut providers = state.providers.lock().unwrap();
                for (id, config) in &configs {
                    let is_builtin = !id.starts_with("plugin-");
                    let has_token = tokens.providers.contains_key(id);
                    providers.push(ProviderState {
                        id: id.clone(),
                        name: config.name.clone(),
                        icon: if is_builtin {
                            ProviderIcon::Builtin(config.icon.clone())
                        } else {
                            ProviderIcon::Custom(config.icon.clone())
                        },
                        is_builtin,
                        balance: None,
                        used: None,
                        available: None,
                        currency: Currency::default(),
                        is_available: None,
                        extra_fields: Default::default(),
                        display_label: None,
                        has_token,
                        has_progress: config.display.progress.is_some(),
                        status: if has_token {
                            ProviderStatus::Fetching
                        } else {
                            ProviderStatus::Unconfigured
                        },
                        last_updated: None,
                        error_message: None,
                    });
                }
            }

            tray::setup_tray(app)?;

            // Show settings window on startup so it's always accessible
            if let Some(settings_window) = app.get_webview_window("settings") {
                let _ = app.handle().set_activation_policy(ActivationPolicy::Regular);
                let _ = settings_window.show();
            }

            let app_handle_for_settings = app.handle().clone();
            if let Some(settings_window) = app.get_webview_window("settings") {
                settings_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(w) = app_handle_for_settings.get_webview_window("settings") {
                            let _ = app_handle_for_settings.set_activation_policy(ActivationPolicy::Accessory);
                            let _ = w.hide();
                        }
                    }
                });
            }

            let app_handle_for_popover = app.handle().clone();
            if let Some(popover_window) = app.get_webview_window("popover") {
                popover_window.on_window_event(move |event| {
                    match event {
                        tauri::WindowEvent::CloseRequested { api, .. } => {
                            api.prevent_close();
                            if let Some(w) = app_handle_for_popover.get_webview_window("popover") {
                                let _ = w.hide();
                            }
                        }
                        tauri::WindowEvent::Focused(false) => {
                            if let Some(w) = app_handle_for_popover.get_webview_window("popover") {
                                let _ = w.hide();
                            }
                        }
                        _ => {}
                    }
                });
            }

            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                let state = app_handle.state::<AppState>();
                let configs = state.configs.lock().unwrap().clone();
                let tokens = match crate::storage::load_tokens() {
                    Ok(t) => t,
                    Err(_) => return,
                };
                for (id, config) in &configs {
                    if let Some(token_entry) = tokens.providers.get(id) {
                        if !token_entry.enabled {
                            continue;
                        }
                        let is_builtin = !id.starts_with("plugin-");
                        let mut result = crate::provider::fetcher::fetch_provider(
                            config, id, &token_entry.token, is_builtin,
                        )
                        .await;
                        result.has_token = true;
                        let mut providers = state.providers.lock().unwrap();
                        if let Some(pos) = providers.iter().position(|p| p.id == *id) {
                            providers[pos] = result;
                        }
                    }
                }
                let _ = app_handle.emit("providers-updated", ());
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_providers,
            commands::save_token,
            commands::delete_token,
            commands::test_connection,
            commands::refresh_provider,
            commands::refresh_all,
            commands::import_plugin,
            commands::remove_plugin,
            commands::get_settings,
            commands::save_settings,
            commands::show_settings_window,
            commands::get_provider_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running CostWatch");
}
