use crate::provider::types::*;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

static TOKEN_FILE_LOCK: Mutex<()> = Mutex::new(());

pub fn app_data_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Cannot determine home directory")
        .join(".costwatch")
}

pub fn providers_dir() -> PathBuf {
    app_data_dir().join("providers")
}

pub fn config_file_path() -> PathBuf {
    app_data_dir().join("config.json")
}

pub fn ensure_dirs() -> Result<(), String> {
    fs::create_dir_all(app_data_dir())
        .map_err(|e| format!("Cannot create app dir: {}", e))?;
    fs::create_dir_all(providers_dir())
        .map_err(|e| format!("Cannot create providers dir: {}", e))?;
    Ok(())
}

pub fn load_settings() -> Result<GeneralSettings, String> {
    let path = config_file_path();
    if !path.exists() {
        let default = GeneralSettings::default();
        save_settings(&default)?;
        return Ok(default);
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Cannot read config: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Cannot parse config: {}", e))
}

pub fn save_settings(settings: &GeneralSettings) -> Result<(), String> {
    ensure_dirs()?;
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Cannot serialize config: {}", e))?;
    fs::write(config_file_path(), content)
        .map_err(|e| format!("Cannot write config: {}", e))
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenStore {
    pub providers: HashMap<String, ProviderToken>,
}

fn load_tokens_inner() -> Result<TokenStore, String> {
    let path = app_data_dir().join("tokens.json");
    if !path.exists() {
        return Ok(TokenStore::default());
    }
    let content =
        fs::read_to_string(&path).map_err(|e| format!("Cannot read tokens: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Cannot parse tokens: {}", e))
}

fn save_tokens_inner(store: &TokenStore) -> Result<(), String> {
    ensure_dirs()?;
    let content = serde_json::to_string_pretty(store)
        .map_err(|e| format!("Cannot serialize tokens: {}", e))?;
    let path = app_data_dir().join("tokens.json");
    fs::write(path, content).map_err(|e| format!("Cannot write tokens: {}", e))
}

pub fn load_tokens() -> Result<TokenStore, String> {
    let _guard = TOKEN_FILE_LOCK.lock().unwrap();
    load_tokens_inner()
}

pub fn get_token(provider_id: &str) -> Result<Option<String>, String> {
    let _guard = TOKEN_FILE_LOCK.lock().unwrap();
    let store = load_tokens_inner()?;
    Ok(store.providers.get(provider_id).map(|t| t.token.clone()))
}

pub fn save_token(provider_id: &str, token: &str) -> Result<(), String> {
    let _guard = TOKEN_FILE_LOCK.lock().unwrap();
    let mut store = load_tokens_inner()?;
    store.providers.insert(
        provider_id.to_string(),
        ProviderToken {
            token: token.to_string(),
            enabled: true,
        },
    );
    save_tokens_inner(&store)
}

pub fn delete_token(provider_id: &str) -> Result<(), String> {
    let _guard = TOKEN_FILE_LOCK.lock().unwrap();
    let mut store = load_tokens_inner()?;
    store.providers.remove(provider_id);
    save_tokens_inner(&store)
}