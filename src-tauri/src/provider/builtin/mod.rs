pub mod openrouter;
pub mod deepseek;

use crate::provider::types::ProviderConfig;

pub fn all_builtin_configs() -> Vec<(&'static str, ProviderConfig)> {
    vec![
        ("openrouter", openrouter::config()),
        ("deepseek", deepseek::config()),
    ]
}