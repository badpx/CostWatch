use crate::provider::types::*;

pub fn config() -> ProviderConfig {
    let yaml = r#"
name: Runware
icon: runware
api:
  url: https://api.runware.ai/v1
  method: POST
  body: '[{"taskType":"authentication","apiKey":"{{token}}"},{"taskType":"accountManagement","taskUUID":"{{uuid}}","operation":"getDetails"}]'
response:
  available:
    path: "$.data[0].balance"
    type: number
  usage_total:
    path: "$.data[0].usage.total.credits"
    type: number
  currency:
    value: "USD"
    type: string
display:
  primary: available
  secondary: usage_total
  label: "{{currency_unit}}{{available}}"
  unit_prefix: "$"
"#;
    crate::provider::config_parser::parse_provider_config(yaml)
        .expect("Runware builtin config should parse correctly")
}
