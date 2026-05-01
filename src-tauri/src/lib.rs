mod commands;
mod provider;
mod state;
mod storage;
mod tray;

use provider::plugin;
use provider::registry::ProviderRegistry;
use provider::types::{ProviderIcon, ProviderState, ProviderStatus};
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_positioner::init())
        .manage(AppState::default())
        .setup(|app| {
            storage::ensure_dirs()?;

            let settings = storage::load_settings().unwrap_or_default();

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

            // Initialize providers as Unconfigured so the UI always shows them
            {
                let mut providers = state.providers.lock().unwrap();
                for (id, config) in &configs {
                    let is_builtin = !id.starts_with("plugin-");
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
                        currency: provider::types::Currency::default(),
                        is_available: None,
                        extra_fields: Default::default(),
                        display_label: None,
                        status: ProviderStatus::Unconfigured,
                        last_updated: None,
                        error_message: None,
                    });
                }
            }

            tray::setup_tray(app)?;

            let app_handle_for_settings = app.handle().clone();
            if let Some(settings_window) = app.get_webview_window("settings") {
                settings_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(w) = app_handle_for_settings.get_webview_window("settings") {
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running TokenWatch");
}