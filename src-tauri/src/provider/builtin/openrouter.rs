use crate::provider::types::*;

pub fn config() -> ProviderConfig {
    let yaml = r#"
name: OpenRouter
icon: openrouter
api:
  url: https://openrouter.ai/api/v1/credits
  method: GET
  headers:
    Authorization: "Bearer {{token}}"
response:
  total_credits:
    path: "$.data.total_credits"
    type: number
  total_usage:
    path: "$.data.total_usage"
    type: number
  used:
    path: "$.data.total_usage"
    type: number
  available:
    path: "$.data.total_credits"
    type: number
  balance:
    expr: "total_credits - total_usage"
    type: number
  currency:
    value: "USD"
    type: string
display:
  primary: balance
  secondary: used
  label: "{{currency_unit}}{{balance}} / {{currency_unit}}{{available}}"
  unit_prefix: "$"
  progress:
    total: available
    used: used
    direction: consumption
"#;
    crate::provider::config_parser::parse_provider_config(yaml)
        .expect("OpenRouter builtin config should parse correctly")
}