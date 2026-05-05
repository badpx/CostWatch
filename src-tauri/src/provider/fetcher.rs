use crate::provider::config_parser;
use crate::provider::types::*;
use chrono::Utc;
use rust_decimal::Decimal;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn fetch_provider(
    config: &ProviderConfig,
    provider_id: &str,
    token: &str,
    is_builtin: bool,
) -> ProviderState {
    let icon = if is_builtin {
        ProviderIcon::Builtin(config.icon.clone())
    } else {
        ProviderIcon::Custom(config.icon.clone())
    };
    let mut state = ProviderState {
        id: provider_id.to_string(),
        name: config.name.clone(),
        icon,
        is_builtin,
        balance: None,
        used: None,
        available: None,
        currency: Currency::default(),
        is_available: None,
        extra_fields: HashMap::new(),
        display_label: None,
        has_token: false,
        has_progress: config.display.progress.is_some(),
        status: ProviderStatus::Fetching,
        last_updated: None,
        error_message: None,
    };

    let client = reqwest::Client::new();
    let url = &config.api.url;

    let mut headers = config.api.headers.clone();
    for (_, value) in headers.iter_mut() {
        *value = value.replace("{{token}}", token);
    }

    let request_builder = match config.api.method {
        HttpMethod::Get => client.get(url),
        HttpMethod::Post => client.post(url),
    };

    let request_builder = headers.into_iter().fold(request_builder, |rb, (k, v)| {
        rb.header(k, v)
    });

    let request_builder = if let Some(ref body) = config.api.body {
        let escaped_token = token.replace('\\', "\\\\").replace('"', "\\\"");
        let body_str = body
            .replace("{{token}}", &escaped_token)
            .replace("{{uuid}}", &Uuid::new_v4().to_string());
        let rb = if config.api.headers.contains_key("Content-Type") {
            request_builder
        } else {
            request_builder.header("Content-Type", "application/json")
        };
        rb.body(body_str)
    } else {
        request_builder
    };

    match request_builder.send().await {
        Ok(response) => {
            let status = response.status();
            if status == reqwest::StatusCode::UNAUTHORIZED {
                state.status = ProviderStatus::Error("Authentication failed, please check your token".into());
                return state;
            }
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                state.status = ProviderStatus::Error("Rate limited, please try again later".into());
                return state;
            }
            if !status.is_success() {
                state.status = ProviderStatus::Error(format!("HTTP error: {}", status));
                return state;
            }

            match response.json::<serde_json::Value>().await {
                Ok(json_response) => {
                    match config_parser::extract_fields(&json_response, config) {
                        Ok(fields) => {
                            state.balance = get_decimal_field(&fields, "balance");
                            state.used = get_decimal_field(&fields, "used");
                            state.available = get_decimal_field(&fields, "available");
                            state.is_available = fields
                                .get("is_available")
                                .and_then(|v| v.as_bool());

                            state.currency = determine_currency(config, &fields);
                            state.display_label = Some(resolve_display_label(config, &fields));

                            state.extra_fields = fields;

                            state.status = ProviderStatus::Ok;
                            state.last_updated = Some(Utc::now());
                        }
                        Err(e) => {
                            state.status =
                                ProviderStatus::Error(format!("Data extraction error: {}", e));
                        }
                    }
                }
                Err(e) => {
                    state.status =
                        ProviderStatus::Error(format!("Response parse error: {}", e));
                }
            }
        }
        Err(e) => {
            if e.is_timeout() {
                state.status = ProviderStatus::Error("Connection timed out".into());
            } else if e.is_connect() {
                state.status = ProviderStatus::Error("Network unavailable".into());
            } else {
                state.status = ProviderStatus::Error(format!("Request failed: {}", e));
            }
        }
    }

    state
}

fn get_decimal_field(
    fields: &HashMap<String, serde_json::Value>,
    name: &str,
) -> Option<Decimal> {
    fields.get(name).and_then(|v| {
        if v.is_number() {
            v.as_f64().and_then(|f| Decimal::try_from(f).ok())
        } else if v.is_string() {
            v.as_str()
                .and_then(|s| Decimal::from_str_radix(s, 10).ok())
        } else {
            None
        }
    })
}

fn determine_currency(
    config: &ProviderConfig,
    fields: &HashMap<String, serde_json::Value>,
) -> Currency {
    if let Some(ref unit_config) = config.display.unit {
        match unit_config {
            UnitConfig::Static(s) => Currency::Custom(s.clone()),
            UnitConfig::Map { field, map } => {
                if let Some(value) = fields.get(field) {
                    if let Some(currency_str) = value.as_str() {
                        if let Some(_symbol) = map.get(currency_str) {
                            return match currency_str {
                                "USD" => Currency::USD,
                                "CNY" => Currency::CNY,
                                "EUR" => Currency::EUR,
                                _ => Currency::Custom(currency_str.to_string()),
                            };
                        }
                    }
                }
                Currency::default()
            }
        }
    } else if let Some(ref prefix) = config.display.unit_prefix {
        match prefix.as_str() {
            "$" => Currency::USD,
            "¥" => Currency::CNY,
            "€" => Currency::EUR,
            _ => Currency::Custom(prefix.clone()),
        }
    } else {
        Currency::default()
    }
}

fn resolve_display_label(
    config: &ProviderConfig,
    fields: &HashMap<String, serde_json::Value>,
) -> String {
    let mut label = config.display.label.clone();

    if label.contains("{{currency_unit}}") {
        let unit = resolve_currency_unit(config, fields);
        label = label.replace("{{currency_unit}}", &unit);
    }

    let mut field_names: Vec<&String> = fields.keys().collect();
    field_names.sort_by(|a, b| b.len().cmp(&a.len()));

    for name in field_names {
        let placeholder = format!("{{{{{}}}}}", name);
        if !label.contains(&placeholder) {
            continue;
        }
        let value = fields.get(name).unwrap();
        let mapping = config.response.get(name);
        let formatted = format_field_value(value, mapping);
        label = label.replace(&placeholder, &formatted);
    }

    label
}

fn resolve_currency_unit(
    config: &ProviderConfig,
    fields: &HashMap<String, serde_json::Value>,
) -> String {
    if let Some(ref unit_config) = config.display.unit {
        match unit_config {
            UnitConfig::Static(s) => s.clone(),
            UnitConfig::Map { field, map } => {
                if let Some(value) = fields.get(field) {
                    if let Some(key) = value.as_str() {
                        if let Some(symbol) = map.get(key) {
                            return symbol.clone();
                        }
                    }
                }
                String::new()
            }
        }
    } else if let Some(ref prefix) = config.display.unit_prefix {
        prefix.clone()
    } else {
        String::new()
    }
}

fn format_field_value(
    value: &serde_json::Value,
    mapping: Option<&FieldMapping>,
) -> String {
    let field_type = mapping.map(|m| &m.field_type);
    match field_type {
        Some(FieldType::String) => value.as_str().unwrap_or("").to_string(),
        Some(FieldType::Number) | Some(FieldType::StringNumber) => {
            if let Some(f) = value.as_f64() {
                format_decimal(f)
            } else if let Some(s) = value.as_str() {
                s.parse::<f64>()
                    .map(|f| format_decimal(f))
                    .unwrap_or_else(|_| s.to_string())
            } else {
                value.to_string()
            }
        }
        Some(FieldType::Boolean) => value
            .as_bool()
            .map(|b| b.to_string())
            .unwrap_or_else(|| value.to_string()),
        None => format_auto(value),
    }
}

fn format_decimal(f: f64) -> String {
    format!("{:.2}", f)
}

fn format_auto(value: &serde_json::Value) -> String {
    if value.is_number() {
        if let Some(f) = value.as_f64() {
            return format_decimal(f);
        }
    }
    value.as_str().unwrap_or("").to_string()
}