mod commands;
mod provider;
mod state;
mod storage;
mod tray;

use provider::plugin;
use provider::registry::ProviderRegistry;
use state::AppState;
use tauri::Emitter;
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
            app.state::<AppState>().settings.lock().unwrap().clone_from(&settings);
            *app.state::<AppState>().configs.lock().unwrap() = configs;

            tray::setup_tray(app)?;

            let app_handle = app.handle().clone();
            let interval_secs = settings.refresh_interval_secs;
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new().unwrap();
                runtime.block_on(async {
                    let mut interval = tokio::time::interval(
                        std::time::Duration::from_secs(interval_secs)
                    );
                    loop {
                        interval.tick().await;
                        let _ = app_handle.emit("refresh-triggered", ());
                    }
                });
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running TokenWatch");
}