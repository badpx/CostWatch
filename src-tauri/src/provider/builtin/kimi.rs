use crate::provider::types::*;

pub fn config() -> ProviderConfig {
    let yaml = r#"
name: Kimi
icon: kimi
api:
  url: https://api.moonshot.cn/v1/users/me/balance
  method: GET
  headers:
    Authorization: "Bearer {{token}}"
response:
  available:
    path: "$.data.available_balance"
    type: number
  cash_balance:
    path: "$.data.cash_balance"
    type: number
  voucher_balance:
    path: "$.data.voucher_balance"
    type: number
  currency:
    value: "CNY"
    type: string
display:
  primary: available
  secondary: cash_balance
  label: "{{currency_unit}}{{available}}"
  unit_prefix: "¥"
"#;
    crate::provider::config_parser::parse_provider_config(yaml)
        .expect("Kimi builtin config should parse correctly")
}
