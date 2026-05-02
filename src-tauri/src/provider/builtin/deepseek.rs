use crate::provider::types::*;

pub fn config() -> ProviderConfig {
    let yaml = r#"
name: DeepSeek
icon: deepseek
api:
  url: https://api.deepseek.com/user/balance
  method: GET
  headers:
    Authorization: "Bearer {{token}}"
response:
  total_balance:
    path: "$.balance_infos[0].total_balance"
    type: string_number
  granted:
    path: "$.balance_infos[0].granted_balance"
    type: string_number
  topped_up:
    path: "$.balance_infos[0].topped_up_balance"
    type: string_number
  currency:
    path: "$.balance_infos[0].currency"
    type: string
  is_available:
    path: "$.is_available"
    type: boolean
  available:
    path: "$.balance_infos[0].total_balance"
    type: string_number
display:
  primary: available
  secondary: granted
  label: "{{available}} {{currency_unit}}"
  unit:
    field: currency
    map:
      CNY: "¥"
      USD: "$"
"#;
    crate::provider::config_parser::parse_provider_config(yaml)
        .expect("DeepSeek builtin config should parse correctly")
}