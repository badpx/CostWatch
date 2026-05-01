use crate::provider::fetcher;
use crate::provider::plugin;
use crate::provider::types::*;
use crate::state::AppState;

#[tauri::command]
pub async fn get_providers(state: tauri::State<'_, AppState>) -> Result<Vec<ProviderState>, String> {
    Ok(state.providers.lock().unwrap().clone())
}

#[tauri::command]
pub async fn save_token(
    state: tauri::State<'_, AppState>,
    provider_id: String,
    token: String,
) -> Result<(), String> {
    crate::storage::save_token(&provider_id, &token)?;

    let config = {
        let configs = state.configs.lock().unwrap();
        configs.get(&provider_id).cloned()
    };

    if let Some(config) = config {
        let token_entry = crate::storage::load_tokens()?
            .providers
            .get(&provider_id)
            .cloned();
        let is_builtin = !provider_id.starts_with("plugin-");

        let token_str = token_entry
            .as_ref()
            .map(|t| t.token.as_str())
            .unwrap_or("");

        let fetched =
            fetcher::fetch_provider(&config, &provider_id, token_str, is_builtin).await;

        let mut providers = state.providers.lock().unwrap();
        if let Some(pos) = providers.iter().position(|p| p.id == provider_id) {
            providers[pos] = fetched;
        } else {
            providers.push(fetched);
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_token(provider_id: String) -> Result<(), String> {
    crate::storage::delete_token(&provider_id)
}

#[tauri::command]
pub async fn test_connection(
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
    let result = fetcher::fetch_provider(&config, &provider_id, &token, is_builtin).await;

    match &result.status {
        ProviderStatus::Ok => Ok(result),
        ProviderStatus::Error(e) => Err(e.clone()),
        _ => Err(String::from("Connection test failed")),
    }
}

#[tauri::command]
pub async fn refresh_provider(
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
        .ok_or_else(|| String::from("No token configured"))?;

    let is_builtin = !provider_id.starts_with("plugin-");
    let result = fetcher::fetch_provider(&config, &provider_id, &token, is_builtin).await;

    let mut providers = state.providers.lock().unwrap();
    if let Some(pos) = providers.iter().position(|p| p.id == provider_id) {
        providers[pos] = result.clone();
    }
    drop(providers);

    Ok(result)
}

#[tauri::command]
pub async fn refresh_all(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let configs = state.configs.lock().unwrap().clone();
    let tokens = crate::storage::load_tokens()?;

    let mut updated_providers = Vec::new();

    for (id, config) in &configs {
        if let Some(token_entry) = tokens.providers.get(id) {
            if !token_entry.enabled {
                continue;
            }
            let is_builtin = !id.starts_with("plugin-");
            let result =
                fetcher::fetch_provider(config, id, &token_entry.token, is_builtin).await;
            updated_providers.push(result);
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
                status: ProviderStatus::Unconfigured,
                last_updated: None,
                error_message: None,
            };
            updated_providers.push(unconfigured);
        }
    }

    *state.providers.lock().unwrap() = updated_providers;
    Ok(())
}

#[tauri::command]
pub async fn import_plugin(yaml_path: String) -> Result<ProviderConfig, String> {
    let (_id, config) = plugin::import_plugin(&yaml_path)?;
    Ok(config)
}

#[tauri::command]
pub async fn remove_plugin(provider_id: String) -> Result<(), String> {
    plugin::remove_plugin(&provider_id)
}

#[tauri::command]
pub async fn get_settings(state: tauri::State<'_, AppState>) -> Result<GeneralSettings, String> {
    Ok(state.settings.lock().unwrap().clone())
}

#[tauri::command]
pub async fn save_settings(
    state: tauri::State<'_, AppState>,
    settings: GeneralSettings,
) -> Result<(), String> {
    crate::storage::save_settings(&settings)?;
    *state.settings.lock().unwrap() = settings;
    Ok(())
}