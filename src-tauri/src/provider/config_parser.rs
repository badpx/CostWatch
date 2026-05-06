use crate::provider::types::*;
use serde_yaml;
use std::collections::HashMap;

/// Parse a YAML string into a ProviderConfig, validating required fields.
pub fn parse_provider_config(yaml_str: &str) -> Result<ProviderConfig, String> {
    let config: ProviderConfig =
        serde_yaml::from_str(yaml_str).map_err(|e| format!("YAML parse error: {}", e))?;

    validate_provider_config(&config)?;

    Ok(config)
}

/// Validate a parsed ProviderConfig has required fields and consistent references.
fn validate_provider_config(config: &ProviderConfig) -> Result<(), String> {
    if config.api.url.is_empty() {
        return Err("api.url is required and cannot be empty".into());
    }

    let response_fields: Vec<&str> = config.response.keys().map(|s| s.as_str()).collect();

    if !response_fields.contains(&config.display.primary.as_str()) {
        return Err(format!(
            "display.primary '{}' not found in response fields: {:?}",
            config.display.primary, response_fields
        ));
    }

    if let Some(ref secondary) = config.display.secondary {
        if !response_fields.contains(&secondary.as_str()) {
            return Err(format!(
                "display.secondary '{}' not found in response fields: {:?}",
                secondary, response_fields
            ));
        }
    }

    if let Some(ref progress) = config.display.progress {
        if !response_fields.contains(&progress.total.as_str()) {
            return Err(format!(
                "progress.total '{}' not found in response fields",
                progress.total
            ));
        }
        if !response_fields.contains(&progress.used.as_str()) {
            return Err(format!(
                "progress.used '{}' not found in response fields",
                progress.used
            ));
        }
    }

    for (name, mapping) in &config.response {
        if mapping.path.is_none() && mapping.value.is_none() && mapping.expr.is_none() {
            return Err(format!(
                "response.{} must have at least one of: path, value, or expr",
                name
            ));
        }
        if let Some(ref expr) = mapping.expr {
            if expr.trim().is_empty() {
                return Err(format!("response.{}.expr cannot be empty", name));
            }
        }
    }

    Ok(())
}

/// Extract values from a JSON response using the field mappings in a ProviderConfig.
pub fn extract_fields(
    response: &serde_json::Value,
    config: &ProviderConfig,
) -> Result<HashMap<String, serde_json::Value>, String> {
    let mut extracted = HashMap::new();

    for (name, mapping) in &config.response {
        let value = if let Some(ref path) = mapping.path {
            let json_path = serde_json_path::JsonPath::parse(path)
                .map_err(|e| format!("Invalid JSONPath '{}' for field '{}': {}", path, name, e))?;
            let queried = json_path.query(response);
            let queried_vec: Vec<&serde_json::Value> = queried.all().into_iter().collect();

            if queried_vec.is_empty() {
                serde_json::Value::Null
            } else if queried_vec.len() > 1 {
                serde_json::Value::Null
            } else {
                queried_vec[0].clone()
            }
        } else if let Some(ref val) = mapping.value {
            serde_json::Value::String(val.clone())
        } else if let Some(ref _expr) = mapping.expr {
            serde_json::Value::Null // placeholder, resolved in second pass
        } else {
            return Err(format!(
                "Field '{}' has no path, value, or expr defined",
                name
            ));
        };

        extracted.insert(name.clone(), value);
    }

    // Second pass: resolve expressions (they reference other extracted fields)
    for (name, mapping) in &config.response {
        if let Some(ref expr) = mapping.expr {
            let field_names: Vec<&str> = expr
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .filter(|s| !s.is_empty() && extracted.contains_key(*s))
                .collect();
            let has_null = field_names.iter().any(|n| extracted[*n].is_null());

            if has_null {
                extracted.insert(name.clone(), serde_json::Value::Null);
            } else if let Ok(resolved) = evaluate_expression(expr, &extracted) {
                extracted.insert(
                    name.clone(),
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(resolved)
                            .unwrap_or(serde_json::Number::from(0)),
                    ),
                );
            } else {
                extracted.insert(name.clone(), serde_json::Value::Null);
            }
        }
    }

    Ok(extracted)
}

/// Evaluate simple arithmetic expressions referencing extracted fields.
fn evaluate_expression(
    expr: &str,
    fields: &HashMap<String, serde_json::Value>,
) -> Result<f64, String> {
    let mut resolved_expr = expr.to_string();
    let mut sorted_names: Vec<&String> = fields.keys().collect();
    sorted_names.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
    for name in sorted_names {
        if !resolved_expr.contains(name) {
            continue;
        }
        let value = fields.get(name).unwrap();
        if value.is_null() {
            continue;
        }
        let num = if value.is_number() {
            value
                .as_f64()
                .ok_or_else(|| format!("Field '{}' is not a number", name))?
        } else if value.is_string() {
            value
                .as_str()
                .ok_or_else(|| format!("Field '{}' is not a string", name))?
                .parse::<f64>()
                .map_err(|_| format!("Cannot parse field '{}' value as number", name))?
        } else {
            return Err(format!("Field '{}' is not a numeric type", name));
        };
        let num_str = if num < 0.0 {
            format!("({})", num)
        } else {
            num.to_string()
        };
        resolved_expr = resolved_expr.replace(name.as_str(), &num_str);
    }

    let allowed = resolved_expr
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.' || "+-*/() ".contains(c));
    if !allowed {
        return Err(format!(
            "Expression contains disallowed characters: {}",
            resolved_expr
        ));
    }

    parse_expression(&resolved_expr)
}

/// Simple recursive descent parser for arithmetic expressions.
fn parse_expression(input: &str) -> Result<f64, String> {
    let chars: Vec<char> = input.chars().filter(|c| !c.is_whitespace()).collect();
    let (result, _) = parse_additive(&chars, 0)?;
    Ok(result)
}

fn parse_additive(chars: &[char], pos: usize) -> Result<(f64, usize), String> {
    let (mut left, mut pos) = parse_multiplicative(chars, pos)?;
    while pos < chars.len() && (chars[pos] == '+' || chars[pos] == '-') {
        let op = chars[pos];
        pos += 1;
        let (right, new_pos) = parse_multiplicative(chars, pos)?;
        pos = new_pos;
        left = if op == '+' {
            left + right
        } else {
            left - right
        };
    }
    Ok((left, pos))
}

fn parse_multiplicative(chars: &[char], pos: usize) -> Result<(f64, usize), String> {
    let (mut left, mut pos) = parse_primary(chars, pos)?;
    while pos < chars.len() && (chars[pos] == '*' || chars[pos] == '/') {
        let op = chars[pos];
        pos += 1;
        let (right, new_pos) = parse_primary(chars, pos)?;
        pos = new_pos;
        left = if op == '*' {
            left * right
        } else {
            left / right
        };
    }
    Ok((left, pos))
}

fn parse_primary(chars: &[char], pos: usize) -> Result<(f64, usize), String> {
    if pos >= chars.len() {
        return Err("Unexpected end of expression".into());
    }
    if chars[pos] == '-' {
        let (result, new_pos) = parse_primary(chars, pos + 1)?;
        return Ok((-result, new_pos));
    }
    if chars[pos] == '(' {
        let (result, pos) = parse_additive(chars, pos + 1)?;
        if pos >= chars.len() || chars[pos] != ')' {
            return Err("Missing closing parenthesis".into());
        }
        Ok((result, pos + 1))
    } else {
        let mut num_str = String::new();
        let mut pos = pos;
        let mut has_dot = false;
        while pos < chars.len()
            && (chars[pos].is_ascii_digit() || (chars[pos] == '.' && !has_dot))
        {
            if chars[pos] == '.' {
                has_dot = true;
            }
            num_str.push(chars[pos]);
            pos += 1;
        }
        if num_str.is_empty() {
            return Err(format!("Expected number at position {}", pos));
        }
        num_str
            .parse::<f64>()
            .map(|n| (n, pos))
            .map_err(|e| format!("Invalid number '{}': {}", num_str, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_config() {
        let yaml = r#"
name: TestProvider
icon: test
api:
  url: https://example.com/api
  method: GET
  headers:
    Authorization: "Bearer {{token}}"
response:
  value:
    path: "$.data.value"
    type: number
display:
  primary: value
  label: "{{value}}"
  progress:
    total: value
    used: value
    direction: consumption
"#;
        let config = parse_provider_config(yaml).unwrap();
        assert_eq!(config.name, "TestProvider");
        assert_eq!(config.api.url, "https://example.com/api");
    }

    #[test]
    fn test_validate_missing_primary() {
        let yaml = r#"
name: TestProvider
icon: test
api:
  url: https://example.com/api
response:
  value:
    path: "$.data.value"
    type: number
display:
  primary: nonexistent
  label: "{{value}}"
  progress:
    total: nonexistent
    used: nonexistent
"#;
        let result = parse_provider_config(yaml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("not found in response fields"));
    }

    #[test]
    fn test_evaluate_expression() {
        let mut fields = HashMap::new();
        fields.insert("available".into(), serde_json::json!(100.5));
        fields.insert("used".into(), serde_json::json!(25.75));
        let result = evaluate_expression("available - used", &fields).unwrap();
        assert!((result - 74.75).abs() < 0.001);
    }

    #[test]
    fn test_expression_with_parens() {
        let mut fields = HashMap::new();
        fields.insert("a".into(), serde_json::json!(10.0));
        fields.insert("b".into(), serde_json::json!(20.0));
        fields.insert("c".into(), serde_json::json!(5.0));
        let result = evaluate_expression("(a + b) * c", &fields).unwrap();
        assert!((result - 150.0).abs() < 0.001);
    }

    #[test]
    fn test_expression_with_negative_field() {
        let mut fields = HashMap::new();
        fields.insert("stripe_balance".into(), serde_json::json!(-5.25));
        let result = evaluate_expression("0 - stripe_balance", &fields).unwrap();
        assert!((result - 5.25).abs() < 0.001);
    }

    // extract_fields tests
    #[test]
    fn test_extract_fields_jsonpath() {
        let yaml = r#"
name: Test
icon: test
api:
  url: https://example.com
response:
  balance:
    path: "$.data.balance"
    type: number
display:
  primary: balance
  label: "{{balance}}"
"#;
        let config = parse_provider_config(yaml).unwrap();
        let json = serde_json::json!({"data": {"balance": 42.5}});
        let fields = extract_fields(&json, &config).unwrap();
        assert_eq!(fields["balance"], serde_json::json!(42.5));
    }

    #[test]
    fn test_extract_fields_fixed_value() {
        let yaml = r#"
name: Test
icon: test
api:
  url: https://example.com
response:
  currency:
    value: "USD"
    type: string
display:
  primary: currency
  label: "{{currency}}"
"#;
        let config = parse_provider_config(yaml).unwrap();
        let json = serde_json::json!({});
        let fields = extract_fields(&json, &config).unwrap();
        assert_eq!(fields["currency"], serde_json::json!("USD"));
    }

    #[test]
    fn test_extract_fields_expression() {
        let yaml = r#"
name: Test
icon: test
api:
  url: https://example.com
response:
  total_credits:
    path: "$.data.total"
    type: number
  total_usage:
    path: "$.data.used"
    type: number
  balance:
    expr: "total_credits - total_usage"
    type: number
display:
  primary: balance
  label: "{{balance}}"
"#;
        let config = parse_provider_config(yaml).unwrap();
        let json = serde_json::json!({"data": {"total": 100.0, "used": 30.0}});
        let fields = extract_fields(&json, &config).unwrap();
        assert_eq!(fields["balance"].as_f64().unwrap(), 70.0);
    }

    #[test]
    fn test_extract_fields_jsonpath_missing() {
        let yaml = r#"
name: Test
icon: test
api:
  url: https://example.com
response:
  balance:
    path: "$.data.balance"
    type: number
display:
  primary: balance
  label: "{{balance}}"
"#;
        let config = parse_provider_config(yaml).unwrap();
        let json = serde_json::json!({"data": {}});
        let fields = extract_fields(&json, &config).unwrap();
        assert!(fields["balance"].is_null());
    }

    #[test]
    fn test_extract_fields_expression_with_null_field() {
        let yaml = r#"
name: Test
icon: test
api:
  url: https://example.com
response:
  total_credits:
    path: "$.data.total"
    type: number
  total_usage:
    path: "$.data.used"
    type: number
  balance:
    expr: "total_credits - total_usage"
    type: number
display:
  primary: balance
  label: "{{balance}}"
"#;
        let config = parse_provider_config(yaml).unwrap();
        let json = serde_json::json!({"data": {"total": 100.0}}); // used missing
        let fields = extract_fields(&json, &config).unwrap();
        assert!(fields["balance"].is_null());
    }

    #[test]
    fn test_validate_empty_url() {
        let yaml = r#"
name: Test
icon: test
api:
  url: ""
response:
  balance:
    path: "$.data.balance"
    type: number
display:
  primary: balance
  label: "{{balance}}"
"#;
        let result = parse_provider_config(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("api.url is required"));
    }

    #[test]
    fn test_validate_field_without_path_value_or_expr() {
        let yaml = r#"
name: Test
icon: test
api:
  url: https://example.com
response:
  balance:
    type: number
display:
  primary: balance
  label: "{{balance}}"
"#;
        let result = parse_provider_config(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must have at least one of"));
    }
}