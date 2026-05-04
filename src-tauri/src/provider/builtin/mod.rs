pub mod openrouter;
pub mod deepseek;
pub mod deepinfra;
pub mod runware;
pub mod kimi;

use crate::provider::types::ProviderConfig;

pub fn all_builtin_configs() -> Vec<(&'static str, ProviderConfig)> {
    vec![
        ("openrouter", openrouter::config()),
        ("deepseek", deepseek::config()),
        ("deepinfra", deepinfra::config()),
        ("runware", runware::config()),
        ("kimi", kimi::config()),
    ]
}