use crate::provider::config_parser;
use crate::provider::types::*;
use std::fs;
use std::path::PathBuf;

pub fn load_plugin_configs() -> Vec<(String, ProviderConfig)> {
    let providers_dir = crate::storage::providers_dir();

    if !providers_dir.exists() {
        return Vec::new();
    }

    let mut plugins = Vec::new();

    if let Ok(entries) = fs::read_dir(&providers_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                || path.extension().and_then(|e| e.to_str()) == Some("yml")
            {
                let file_stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");

                match fs::read_to_string(&path) {
                    Ok(content) => match config_parser::parse_provider_config(&content) {
                        Ok(config) => {
                            let id = format!("plugin-{}", file_stem);
                            plugins.push((id, config));
                        }
                        Err(e) => {
                            eprintln!("Failed to parse plugin {}: {}", path.display(), e);
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to read plugin {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    plugins
}

pub fn import_plugin(yaml_path: &str) -> Result<(String, ProviderConfig), String> {
    let source_path = PathBuf::from(yaml_path);
    if !source_path.exists() {
        return Err(format!("File not found: {}", yaml_path));
    }

    let content =
        fs::read_to_string(&source_path).map_err(|e| format!("Cannot read file: {}", e))?;

    let config = config_parser::parse_provider_config(&content)?;

    let id = format!(
        "plugin-{}",
        config.name.to_lowercase().replace(' ', "-")
    );

    crate::storage::ensure_dirs()?;
    let dest_path =
        crate::storage::providers_dir().join(format!("{}.yaml", &id[7..]));
    fs::write(&dest_path, &content)
        .map_err(|e| format!("Cannot write plugin file: {}", e))?;

    Ok((id, config))
}

pub fn remove_plugin(provider_id: &str) -> Result<(), String> {
    if !provider_id.starts_with("plugin-") {
        return Err("Cannot remove builtin providers".into());
    }

    let name = &provider_id[7..];
    let providers_dir = crate::storage::providers_dir();

    for ext in &["yaml", "yml"] {
        let path = providers_dir.join(format!("{}.{}", name, ext));
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| format!("Cannot delete plugin file: {}", e))?;
            return Ok(());
        }
    }

    Err(format!(
        "Plugin file not found for provider '{}'",
        provider_id
    ))
}