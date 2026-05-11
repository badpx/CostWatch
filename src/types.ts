export interface ProviderState {
  id: string;
  name: string;
  icon: { Builtin: string } | { Custom: string };
  is_builtin: boolean;
  balance: number | null;
  used: number | null;
  available: number | null;
  currency: "USD" | "CNY" | "EUR" | { Custom: string };
  is_available: boolean | null;
  display_label: string | null;
  has_token: boolean;
  has_progress: boolean;
  status: "Ok" | "Fetching" | "Unconfigured" | { Error: string };
  last_updated: string | null;
  error_message: string | null;
}

export interface ProviderConfig {
  name: string;
  icon: string;
  api: ApiConfig;
  response: Record<string, FieldMapping>;
  display: DisplayConfig;
}

export interface ApiConfig {
  url: string;
  method: "GET" | "POST";
  headers: Record<string, string>;
  body?: string;
}

export interface FieldMapping {
  path?: string;
  value?: string;
  expr?: string;
  type: "number" | "string_number" | "string" | "boolean";
}

export interface DisplayConfig {
  primary: string;
  secondary?: string;
  label: string;
  unit_prefix?: string;
  unit?: UnitConfig;
  progress?: ProgressConfig;
}

export type UnitConfig =
  | { Static: string }
  | { Map: { field: string; map: Record<string, string> } };

export interface ProgressConfig {
  total: string;
  used: string;
  direction: "consumption" | "remaining";
}

export interface GeneralSettings {
  refresh_interval_secs: number;
  launch_at_login: boolean;
  language: string;
  trend_range: string;
}

export interface HistoryPoint {
  recorded_at: string;
  value: number;
  interpolated?: boolean;
}