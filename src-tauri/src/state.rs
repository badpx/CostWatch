use crate::provider::types::{GeneralSettings, ProviderConfig, ProviderState};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct AppState {
    pub providers: Mutex<Vec<ProviderState>>,
    pub configs: Mutex<HashMap<String, ProviderConfig>>,
    pub settings: Mutex<GeneralSettings>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            providers: Mutex::new(Vec::new()),
            configs: Mutex::new(HashMap::new()),
            settings: Mutex::new(GeneralSettings::default()),
        }
    }
}