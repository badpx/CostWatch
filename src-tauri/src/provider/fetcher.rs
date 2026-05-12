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

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
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

/// Derive currency from config alone (no API response needed).
/// Used at startup to show the correct symbol before the first fetch.
pub fn currency_from_config(config: &ProviderConfig) -> Currency {
    if let Some(ref unit_config) = config.display.unit {
        match unit_config {
            UnitConfig::Static(s) => Currency::Custom(s.clone()),
            UnitConfig::Map { default: Some(d), .. } => symbol_to_currency(d),
            UnitConfig::Map { .. } => Currency::default(),
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

fn symbol_to_currency(s: &str) -> Currency {
    match s {
        "$" => Currency::USD,
        "¥" => Currency::CNY,
        "€" => Currency::EUR,
        _ => Currency::Custom(s.to_string()),
    }
}

fn determine_currency(
    config: &ProviderConfig,
    fields: &HashMap<String, serde_json::Value>,
) -> Currency {
    if let Some(ref unit_config) = config.display.unit {
        match unit_config {
            UnitConfig::Static(s) => Currency::Custom(s.clone()),
            UnitConfig::Map { field, map, default } => {
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
                if let Some(ref d) = default {
                    symbol_to_currency(d)
                } else {
                    Currency::default()
                }
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
            UnitConfig::Map { field, map, default } => {
                if let Some(value) = fields.get(field) {
                    if let Some(key) = value.as_str() {
                        if let Some(symbol) = map.get(key) {
                            return symbol.clone();
                        }
                    }
                }
                default.clone().unwrap_or_default()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_config(display: DisplayConfig) -> ProviderConfig {
        ProviderConfig {
            name: "Test".to_string(),
            icon: "test".to_string(),
            api: ApiConfig {
                url: "http://example.com".to_string(),
                method: HttpMethod::Get,
                headers: HashMap::new(),
                body: None,
            },
            response: HashMap::new(),
            display,
        }
    }

    #[test]
    fn test_currency_from_config_unit_prefix_usd() {
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: Some("$".to_string()),
            unit: None,
            progress: None,
        });
        assert_eq!(currency_from_config(&config), Currency::USD);
    }

    #[test]
    fn test_currency_from_config_unit_prefix_cny() {
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: Some("¥".to_string()),
            unit: None,
            progress: None,
        });
        assert_eq!(currency_from_config(&config), Currency::CNY);
    }

    #[test]
    fn test_currency_from_config_unit_prefix_eur() {
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: Some("€".to_string()),
            unit: None,
            progress: None,
        });
        assert_eq!(currency_from_config(&config), Currency::EUR);
    }

    #[test]
    fn test_currency_from_config_unit_prefix_custom() {
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: Some("£".to_string()),
            unit: None,
            progress: None,
        });
        assert_eq!(currency_from_config(&config), Currency::Custom("£".to_string()));
    }

    #[test]
    fn test_currency_from_config_static_unit() {
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: None,
            unit: Some(UnitConfig::Static("₩".to_string())),
            progress: None,
        });
        assert_eq!(currency_from_config(&config), Currency::Custom("₩".to_string()));
    }

    #[test]
    fn test_currency_from_config_map_with_default() {
        let mut map = HashMap::new();
        map.insert("CNY".to_string(), "¥".to_string());
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: None,
            unit: Some(UnitConfig::Map {
                field: "currency".to_string(),
                map,
                default: Some("¥".to_string()),
            }),
            progress: None,
        });
        assert_eq!(currency_from_config(&config), Currency::CNY);
    }

    #[test]
    fn test_currency_from_config_map_without_default() {
        let mut map = HashMap::new();
        map.insert("USD".to_string(), "$".to_string());
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: None,
            unit: Some(UnitConfig::Map {
                field: "currency".to_string(),
                map,
                default: None,
            }),
            progress: None,
        });
        assert_eq!(currency_from_config(&config), Currency::default());
    }

    #[test]
    fn test_currency_from_config_none() {
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: None,
            unit: None,
            progress: None,
        });
        assert_eq!(currency_from_config(&config), Currency::default());
    }

    #[test]
    fn test_symbol_to_currency_known() {
        assert_eq!(symbol_to_currency("$"), Currency::USD);
        assert_eq!(symbol_to_currency("¥"), Currency::CNY);
        assert_eq!(symbol_to_currency("€"), Currency::EUR);
    }

    #[test]
    fn test_symbol_to_currency_unknown() {
        assert_eq!(symbol_to_currency("₩"), Currency::Custom("₩".to_string()));
    }

    #[test]
    fn test_determine_currency_with_api_response() {
        let mut map = HashMap::new();
        map.insert("USD".to_string(), "$".to_string());
        map.insert("CNY".to_string(), "¥".to_string());
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: None,
            unit: Some(UnitConfig::Map {
                field: "currency".to_string(),
                map,
                default: Some("$".to_string()),
            }),
            progress: None,
        });
        let mut fields = HashMap::new();
        fields.insert("currency".to_string(), serde_json::json!("CNY"));
        assert_eq!(determine_currency(&config, &fields), Currency::CNY);
    }

    #[test]
    fn test_determine_currency_fallback_to_default() {
        let mut map = HashMap::new();
        map.insert("USD".to_string(), "$".to_string());
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: None,
            unit: Some(UnitConfig::Map {
                field: "currency".to_string(),
                map,
                default: Some("¥".to_string()),
            }),
            progress: None,
        });
        let fields = HashMap::new(); // missing currency field
        assert_eq!(determine_currency(&config, &fields), Currency::CNY);
    }

    #[test]
    fn test_determine_currency_fallback_to_usd_when_no_default() {
        let mut map = HashMap::new();
        map.insert("USD".to_string(), "$".to_string());
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: None,
            unit: Some(UnitConfig::Map {
                field: "currency".to_string(),
                map,
                default: None,
            }),
            progress: None,
        });
        let fields = HashMap::new();
        assert_eq!(determine_currency(&config, &fields), Currency::default());
    }

    #[test]
    fn test_resolve_currency_unit_from_api() {
        let mut map = HashMap::new();
        map.insert("CNY".to_string(), "¥".to_string());
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: None,
            unit: Some(UnitConfig::Map {
                field: "currency".to_string(),
                map,
                default: Some("$".to_string()),
            }),
            progress: None,
        });
        let mut fields = HashMap::new();
        fields.insert("currency".to_string(), serde_json::json!("CNY"));
        assert_eq!(resolve_currency_unit(&config, &fields), "¥");
    }

    #[test]
    fn test_resolve_currency_unit_fallback_default() {
        let mut map = HashMap::new();
        map.insert("USD".to_string(), "$".to_string());
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: None,
            unit: Some(UnitConfig::Map {
                field: "currency".to_string(),
                map,
                default: Some("¥".to_string()),
            }),
            progress: None,
        });
        let fields = HashMap::new();
        assert_eq!(resolve_currency_unit(&config, &fields), "¥");
    }

    #[test]
    fn test_resolve_currency_unit_prefix() {
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: Some("€".to_string()),
            unit: None,
            progress: None,
        });
        let fields = HashMap::new();
        assert_eq!(resolve_currency_unit(&config, &fields), "€");
    }

    // Builtin provider startup currency tests
    #[test]
    fn test_openrouter_startup_currency_is_usd() {
        let config = crate::provider::builtin::openrouter::config();
        assert_eq!(currency_from_config(&config), Currency::USD);
    }

    #[test]
    fn test_deepseek_startup_currency_is_cny() {
        let config = crate::provider::builtin::deepseek::config();
        assert_eq!(currency_from_config(&config), Currency::CNY);
    }

    #[test]
    fn test_deepinfra_startup_currency_is_usd() {
        let config = crate::provider::builtin::deepinfra::config();
        assert_eq!(currency_from_config(&config), Currency::USD);
    }

    #[test]
    fn test_runware_startup_currency_is_usd() {
        let config = crate::provider::builtin::runware::config();
        assert_eq!(currency_from_config(&config), Currency::USD);
    }

    #[test]
    fn test_kimi_startup_currency_is_cny() {
        let config = crate::provider::builtin::kimi::config();
        assert_eq!(currency_from_config(&config), Currency::CNY);
    }

    // get_decimal_field tests
    #[test]
    fn test_get_decimal_field_from_number() {
        let mut fields = HashMap::new();
        fields.insert("balance".to_string(), serde_json::json!(123.45));
        let result = get_decimal_field(&fields, "balance").unwrap();
        assert_eq!(result, Decimal::try_from(123.45f64).unwrap());
    }

    #[test]
    fn test_get_decimal_field_from_string_number() {
        let mut fields = HashMap::new();
        fields.insert("balance".to_string(), serde_json::json!("99.99"));
        let result = get_decimal_field(&fields, "balance").unwrap();
        assert_eq!(result, Decimal::from_str_radix("99.99", 10).unwrap());
    }

    #[test]
    fn test_get_decimal_field_from_null() {
        let mut fields = HashMap::new();
        fields.insert("balance".to_string(), serde_json::Value::Null);
        assert!(get_decimal_field(&fields, "balance").is_none());
    }

    #[test]
    fn test_get_decimal_field_from_non_numeric_string() {
        let mut fields = HashMap::new();
        fields.insert("balance".to_string(), serde_json::json!("not_a_number"));
        assert!(get_decimal_field(&fields, "balance").is_none());
    }

    #[test]
    fn test_get_decimal_field_missing_key() {
        let fields = HashMap::new();
        assert!(get_decimal_field(&fields, "balance").is_none());
    }

    // format_decimal tests
    #[test]
    fn test_format_decimal_two_places() {
        assert_eq!(format_decimal(123.456), "123.46");
    }

    #[test]
    fn test_format_decimal_whole_number() {
        assert_eq!(format_decimal(100.0), "100.00");
    }

    #[test]
    fn test_format_decimal_small_value() {
        assert_eq!(format_decimal(0.005), "0.01");
    }

    // format_field_value tests
    #[test]
    fn test_format_field_value_string_type() {
        let mapping = FieldMapping {
            path: None,
            value: None,
            expr: None,
            field_type: FieldType::String,
        };
        assert_eq!(format_field_value(&serde_json::json!("hello"), Some(&mapping)), "hello");
    }

    #[test]
    fn test_format_field_value_number_type() {
        let mapping = FieldMapping {
            path: None,
            value: None,
            expr: None,
            field_type: FieldType::Number,
        };
        assert_eq!(format_field_value(&serde_json::json!(42.5), Some(&mapping)), "42.50");
    }

    #[test]
    fn test_format_field_value_string_number_type_from_string() {
        let mapping = FieldMapping {
            path: None,
            value: None,
            expr: None,
            field_type: FieldType::StringNumber,
        };
        assert_eq!(format_field_value(&serde_json::json!("3.14"), Some(&mapping)), "3.14");
    }

    #[test]
    fn test_format_field_value_boolean_type() {
        let mapping = FieldMapping {
            path: None,
            value: None,
            expr: None,
            field_type: FieldType::Boolean,
        };
        assert_eq!(format_field_value(&serde_json::json!(true), Some(&mapping)), "true");
    }

    #[test]
    fn test_format_field_value_auto_number() {
        assert_eq!(format_field_value(&serde_json::json!(7.5), None), "7.50");
    }

    #[test]
    fn test_format_field_value_auto_string() {
        assert_eq!(format_field_value(&serde_json::json!("text"), None), "text");
    }

    // resolve_display_label tests
    #[test]
    fn test_resolve_display_label_simple_template() {
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{currency_unit}}{{balance}}".to_string(),
            unit_prefix: Some("$".to_string()),
            unit: None,
            progress: None,
        });
        let mut fields = HashMap::new();
        fields.insert("balance".to_string(), serde_json::json!(42.50));
        let label = resolve_display_label(&config, &fields);
        assert_eq!(label, "$42.50");
    }

    #[test]
    fn test_resolve_display_label_multiple_placeholders() {
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{currency_unit}}{{balance}} / {{currency_unit}}{{available}}".to_string(),
            unit_prefix: Some("¥".to_string()),
            unit: None,
            progress: None,
        });
        let mut fields = HashMap::new();
        fields.insert("balance".to_string(), serde_json::json!(10.0));
        fields.insert("available".to_string(), serde_json::json!(100.0));
        let label = resolve_display_label(&config, &fields);
        assert_eq!(label, "¥10.00 / ¥100.00");
    }

    #[test]
    fn test_resolve_display_label_no_currency_unit() {
        let config = minimal_config(DisplayConfig {
            primary: "balance".to_string(),
            secondary: None,
            label: "{{balance}}".to_string(),
            unit_prefix: None,
            unit: None,
            progress: None,
        });
        let mut fields = HashMap::new();
        fields.insert("balance".to_string(), serde_json::json!(5.0));
        let label = resolve_display_label(&config, &fields);
        assert_eq!(label, "5.00");
    }
}