use crate::provider::config_parser;
use crate::provider::types::*;
use chrono::Utc;
use rust_decimal::Decimal;
use std::collections::HashMap;

pub async fn fetch_provider(
    config: &ProviderConfig,
    provider_id: &str,
    token: &str,
    is_builtin: bool,
) -> ProviderState {
    let mut state = ProviderState {
        id: provider_id.to_string(),
        name: config.name.clone(),
        icon: ProviderIcon::Builtin(config.icon.clone()),
        is_builtin,
        balance: None,
        used: None,
        available: None,
        currency: Currency::default(),
        is_available: None,
        extra_fields: HashMap::new(),
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

    match request_builder.send().await {
        Ok(response) => {
            let status = response.status();
            if status == reqwest::StatusCode::UNAUTHORIZED {
                state.status = ProviderStatus::Error("认证失败，请检查Token".into());
                return state;
            }
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                state.status = ProviderStatus::Error("请求限流，稍后重试".into());
                return state;
            }
            if !status.is_success() {
                state.status = ProviderStatus::Error(format!("HTTP错误: {}", status));
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

                            state.extra_fields = fields;

                            state.status = ProviderStatus::Ok;
                            state.last_updated = Some(Utc::now());
                        }
                        Err(e) => {
                            state.status =
                                ProviderStatus::Error(format!("数据解析错误: {}", e));
                        }
                    }
                }
                Err(e) => {
                    state.status =
                        ProviderStatus::Error(format!("响应解析失败: {}", e));
                }
            }
        }
        Err(e) => {
            if e.is_timeout() {
                state.status = ProviderStatus::Error("连接超时".into());
            } else if e.is_connect() {
                state.status = ProviderStatus::Error("网络不可用".into());
            } else {
                state.status = ProviderStatus::Error(format!("请求失败: {}", e));
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