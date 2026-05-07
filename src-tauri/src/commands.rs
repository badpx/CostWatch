use crate::provider::fetcher;
use crate::provider::plugin;
use crate::provider::types::*;
use crate::state::AppState;
use tauri::ActivationPolicy;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_autostart::ManagerExt as AutostartExt;

#[tauri::command]
pub async fn get_providers(state: tauri::State<'_, AppState>) -> Result<Vec<ProviderState>, String> {
    let mut providers = state.providers.lock().unwrap().clone();
    providers.sort_by(|a, b| b.has_token.cmp(&a.has_token).then(a.name.cmp(&b.name)));
    Ok(providers)
}

#[tauri::command]
pub async fn save_token(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    provider_id: String,
    token: String,
) -> Result<(), String> {
    crate::storage::save_token(&provider_id, &token)?;

    let mut providers = state.providers.lock().unwrap();
    if let Some(pos) = providers.iter().position(|p| p.id == provider_id) {
        providers[pos].has_token = true;
        providers[pos].status = ProviderStatus::Fetching;
    } else {
        let config = state.configs.lock().unwrap().get(&provider_id).cloned();
        if let Some(config) = config {
            let is_builtin = !provider_id.starts_with("plugin-");
            providers.push(ProviderState {
                id: provider_id.clone(),
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
                has_token: true,
                has_progress: false,
                status: ProviderStatus::Fetching,
                last_updated: None,
                error_message: None,
            });
        }
    }
    drop(providers);

    let _ = app.emit("providers-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn delete_token(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    provider_id: String,
) -> Result<(), String> {
    crate::storage::delete_token(&provider_id)?;

    let mut providers = state.providers.lock().unwrap();
    if let Some(pos) = providers.iter().position(|p| p.id == provider_id) {
        providers[pos].has_token = false;
        providers[pos].status = ProviderStatus::Unconfigured;
        providers[pos].balance = None;
        providers[pos].used = None;
        providers[pos].available = None;
        providers[pos].display_label = None;
        providers[pos].last_updated = None;
        providers[pos].error_message = None;
    }
    drop(providers);

    let _ = app.emit("providers-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn test_connection(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    provider_id: String,
) -> Result<ProviderState, String> {
    let config = state
        .configs
        .lock()
        .unwrap()
        .get(&provider_id)
        .cloned()
        .ok_or_else(|| format!("Provider '{}' not found", provider_id))?;

    let token = crate::storage::get_token(&provider_id)?
        .ok_or_else(|| String::from("No token configured for this provider"))?;

    let is_builtin = !provider_id.starts_with("plugin-");
    let mut result = fetcher::fetch_provider(&config, &provider_id, &token, is_builtin).await;
    result.has_token = true;

    if result.status == ProviderStatus::Ok {
        if let Some(value) = crate::provider::history::get_primary_value(
            &result.balance,
            &result.available,
        ) {
            let _ = crate::provider::history::record_history(&provider_id, value);
        }
    }

    if matches!(result.status, ProviderStatus::Ok) {
        let mut providers = state.providers.lock().unwrap();
        if let Some(pos) = providers.iter().position(|p| p.id == provider_id) {
            providers[pos] = result.clone();
        }
        drop(providers);
        let _ = app.emit("providers-updated", ());
    }

    match &result.status {
        ProviderStatus::Ok => Ok(result),
        ProviderStatus::Error(e) => Err(e.clone()),
        _ => Err(String::from("Connection test failed")),
    }
}

#[tauri::command]
pub async fn refresh_provider(
    state: tauri::State<'_, AppState>,
    app: tauri::AppHandle,
    provider_id: String,
) -> Result<ProviderState, String> {
    let config = state
        .configs
        .lock()
        .unwrap()
        .get(&provider_id)
        .cloned()
        .ok_or_else(|| format!("Provider '{}' not found", provider_id))?;

    let token = crate::storage::get_token(&provider_id)?
        .ok_or_else(|| String::from("No token configured"))?;

    let is_builtin = !provider_id.starts_with("plugin-");
    let mut result = fetcher::fetch_provider(&config, &provider_id, &token, is_builtin).await;

    // Re-check token still exists (may have been deleted during the fetch)
    let token_still_exists = crate::storage::get_token(&provider_id)
        .unwrap_or(None)
        .is_some();

    if token_still_exists {
        result.has_token = true;

        if result.status == ProviderStatus::Ok {
            if let Some(value) = crate::provider::history::get_primary_value(
                &result.balance,
                &result.available,
            ) {
                let _ = crate::provider::history::record_history(&provider_id, value);
            }
        }

        let mut providers = state.providers.lock().unwrap();
        if let Some(pos) = providers.iter().position(|p| p.id == provider_id) {
            providers[pos] = result.clone();
        }
        drop(providers);

        let _ = app.emit("providers-updated", ());
        Ok(result)
    } else {
        let mut providers = state.providers.lock().unwrap();
        if let Some(pos) = providers.iter().position(|p| p.id == provider_id) {
            providers[pos].has_token = false;
            providers[pos].status = ProviderStatus::Unconfigured;
            providers[pos].balance = None;
            providers[pos].used = None;
            providers[pos].available = None;
            providers[pos].display_label = None;
            providers[pos].last_updated = None;
            providers[pos].error_message = None;
        }
        drop(providers);

        let _ = app.emit("providers-updated", ());
        Err(String::from("Token was deleted during refresh"))
    }
}

pub async fn refresh_all_internal(state: &AppState) -> Result<(), String> {
    let configs = state.configs.lock().unwrap().clone();
    let tokens = crate::storage::load_tokens()?;

    let mut updated_providers = Vec::new();

    for (id, config) in &configs {
        if let Some(token_entry) = tokens.providers.get(id) {
            if !token_entry.enabled {
                continue;
            }
            let is_builtin = !id.starts_with("plugin-");
            let mut result =
                fetcher::fetch_provider(config, id, &token_entry.token, is_builtin).await;

            // Re-check token still exists (may have been deleted during the fetch)
            let token_still_exists = crate::storage::get_token(id)
                .unwrap_or(None)
                .is_some();

            if token_still_exists {
                result.has_token = true;

                if result.status == ProviderStatus::Ok {
                    if let Some(value) = crate::provider::history::get_primary_value(
                        &result.balance,
                        &result.available,
                    ) {
                        let _ = crate::provider::history::record_history(id, value);
                    }
                }

                updated_providers.push(result);
            } else {
                // Token was deleted during fetch — discard result
                let unconfigured = ProviderState {
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
                    has_token: false,
                    has_progress: config.display.progress.is_some(),
                    status: ProviderStatus::Unconfigured,
                    last_updated: None,
                    error_message: None,
                };
                updated_providers.push(unconfigured);
            }
        } else {
            let is_builtin = !id.starts_with("plugin-");
            let unconfigured = ProviderState {
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
                has_token: false,
                has_progress: config.display.progress.is_some(),
                status: ProviderStatus::Unconfigured,
                last_updated: None,
                error_message: None,
            };
            updated_providers.push(unconfigured);
        }
    }

    // Re-check tokens before applying — some may have been deleted during the async fetches
    let current_tokens = crate::storage::load_tokens().unwrap_or_default();
    for provider in &mut updated_providers {
        if provider.has_token && !current_tokens.providers.contains_key(&provider.id) {
            provider.has_token = false;
            provider.status = ProviderStatus::Unconfigured;
            provider.balance = None;
            provider.used = None;
            provider.available = None;
            provider.display_label = None;
            provider.last_updated = None;
            provider.error_message = None;
        }
    }

    *state.providers.lock().unwrap() = updated_providers;
    Ok(())
}

#[tauri::command]
pub async fn refresh_all(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    refresh_all_internal(&state).await?;
    let _ = app.emit("providers-updated", ());
    Ok(())
}

#[tauri::command]
pub async fn import_plugin(
    yaml_path: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<ProviderConfig, String> {
    let (id, config) = plugin::import_plugin(&yaml_path)?;

    let is_builtin = false;
    let tokens = crate::storage::load_tokens().unwrap_or_default();
    let has_token = tokens.providers.contains_key(&id);

    let new_provider = ProviderState {
        id: id.clone(),
        name: config.name.clone(),
        icon: ProviderIcon::Custom(config.icon.clone()),
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
    };

    state.configs.lock().unwrap().insert(id.clone(), config.clone());

    let mut providers = state.providers.lock().unwrap();
    if let Some(existing) = providers.iter_mut().find(|p| p.id == id) {
        existing.name = config.name.clone();
        existing.icon = ProviderIcon::Custom(config.icon.clone());
        existing.has_progress = config.display.progress.is_some();
        existing.balance = None;
        existing.used = None;
        existing.available = None;
        existing.display_label = None;
        existing.status = if has_token { ProviderStatus::Fetching } else { ProviderStatus::Unconfigured };
        existing.last_updated = None;
        existing.error_message = None;
    } else {
        providers.push(new_provider);
    }
    drop(providers);

    let _ = app.emit("providers-updated", ());

    Ok(config)
}

#[tauri::command]
pub async fn remove_plugin(
    provider_id: String,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    plugin::remove_plugin(&provider_id)?;

    let _ = crate::storage::delete_token(&provider_id);

    state.configs.lock().unwrap().remove(&provider_id);
    let mut providers = state.providers.lock().unwrap();
    providers.retain(|p| p.id != provider_id);

    let _ = app.emit("providers-updated", ());

    Ok(())
}

#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, AppState>) -> Result<GeneralSettings, String> {
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
pub async fn save_settings(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    settings: GeneralSettings,
) -> Result<(), String> {
    if settings.launch_at_login {
        let _ = app.autolaunch().enable();
    } else {
        let _ = app.autolaunch().disable();
    }
    crate::storage::save_settings(&settings)?;
    *state.settings.lock().unwrap() = settings;
    let _ = app.emit("settings-updated", ());
    Ok(())
}

#[tauri::command]
pub fn show_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    let _ = app.set_activation_policy(ActivationPolicy::Regular);
    if let Some(w) = app.get_webview_window("settings") {
        let _ = w.show();
        let _ = w.set_focus();
    }
    Ok(())
}

#[tauri::command]
pub fn get_provider_history(
    provider_id: String,
    range: String,
) -> Result<Vec<crate::provider::history::HistoryPoint>, String> {
    crate::provider::history::query_history(&provider_id, &range)
}
