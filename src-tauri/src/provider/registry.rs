use crate::provider::types::*;
use std::collections::HashMap;

pub struct ProviderRegistry {
    configs: HashMap<String, ProviderConfig>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    pub fn register(&mut self, id: String, config: ProviderConfig) -> Result<(), String> {
        if self.configs.contains_key(&id) {
            return Err(format!("Provider '{}' already registered", id));
        }
        self.configs.insert(id, config);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get(&self, id: &str) -> Option<&ProviderConfig> {
        self.configs.get(id)
    }

    pub fn list(&self) -> &HashMap<String, ProviderConfig> {
        &self.configs
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, id: &str) -> Result<ProviderConfig, String> {
        self.configs
            .remove(id)
            .ok_or_else(|| format!("Provider '{}' not found", id))
    }
}