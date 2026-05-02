use crate::provider::types::*;

pub fn config() -> ProviderConfig {
    let yaml = r#"
name: DeepInfra
icon: deepinfra
api:
  url: https://api.deepinfra.com/v1/me?checklist=true
  method: GET
  headers:
    Authorization: "Bearer {{token}}"
response:
  stripe_balance:
    path: "$.checklist.stripe_balance"
    type: number
  recent:
    path: "$.checklist.recent"
    type: number
  available:
    expr: "0 - stripe_balance"
    type: number
  used:
    path: "$.checklist.recent"
    type: number
  currency:
    value: "USD"
    type: string
display:
  primary: available
  secondary: used
  label: "{{currency_unit}}{{available}}"
  unit_prefix: "$"
"#;
    crate::provider::config_parser::parse_provider_config(yaml)
        .expect("DeepInfra builtin config should parse correctly")
}
