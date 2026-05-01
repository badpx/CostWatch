use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================
// Runtime State (what the frontend receives)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderState {
    pub id: String,
    pub name: String,
    pub icon: ProviderIcon,
    pub is_builtin: bool,

    // Balance info (unified across all providers)
    pub balance: Option<Decimal>,
    pub used: Option<Decimal>,
    pub available: Option<Decimal>,
    pub currency: Currency,
    pub is_available: Option<bool>,

    // Additional display fields (provider-specific raw data)
    pub extra_fields: HashMap<String, serde_json::Value>,

    // Resolved display label from config template
    pub display_label: Option<String>,

    // Status
    pub status: ProviderStatus,
    pub last_updated: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderStatus {
    Ok,
    Fetching,
    Unconfigured,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderIcon {
    Builtin(String),
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Currency {
    USD,
    CNY,
    EUR,
    Custom(String),
}

impl Currency {
    pub fn symbol(&self) -> &str {
        match self {
            Currency::USD => "$",
            Currency::CNY => "¥",
            Currency::EUR => "€",
            Currency::Custom(s) => s.as_str(),
        }
    }
}

impl Default for Currency {
    fn default() -> Self {
        Currency::USD
    }
}

// ============================================================
// Declarative Config (parsed from YAML)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub icon: String,
    pub api: ApiConfig,
    pub response: HashMap<String, FieldMapping>,
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub url: String,
    #[serde(default = "default_method")]
    pub method: HttpMethod,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

fn default_method() -> HttpMethod {
    HttpMethod::Get
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expr: Option<String>,
    #[serde(rename = "type")]
    pub field_type: FieldType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Number,
    StringNumber,
    String,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub primary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary: Option<String>,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<UnitConfig>,
    pub progress: ProgressConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UnitConfig {
    Static(String),
    Map {
        field: String,
        map: HashMap<String, String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressConfig {
    pub total: String,
    pub used: String,
    #[serde(default = "default_consumption")]
    pub direction: ProgressDirection,
}

fn default_consumption() -> ProgressDirection {
    ProgressDirection::Consumption
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProgressDirection {
    Consumption,
    Remaining,
}

// ============================================================
// Stored Token (in Stronghold)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderToken {
    pub token: String,
    pub enabled: bool,
}

// ============================================================
// General Settings (in config.json)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,
    #[serde(default)]
    pub launch_at_login: bool,
}

fn default_refresh_interval() -> u64 {
    60
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            refresh_interval_secs: 60,
            launch_at_login: false,
        }
    }
}