# Usage Trend Chart — Design Spec

## Overview

Record per-provider usage data after each successful API fetch, persist in SQLite, and display a gradient area trend chart in the settings window provider items. Users can toggle between time ranges (24h / 3d / 1w / 1m), defaulting to 1 week.

## Data Model

### Storage

- **Engine**: SQLite via `tauri-plugin-sql`
- **Database file**: `~/.costwatch/history.db`
- **Schema**:

```sql
CREATE TABLE IF NOT EXISTS provider_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id TEXT NOT NULL,
    recorded_at TEXT NOT NULL,
    value REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_provider_time ON provider_history(provider_id, recorded_at);
```

### What Gets Recorded

- Only the **primary display value** (the numeric value backing `display_label` — typically the `balance` field)
- After **every successful fetch** (status = `Ok`)
- Failed fetches are **not recorded** (gaps in chart data are acceptable)
- Raw data points are stored as-is, no aggregation

### Retention

- Keep all data indefinitely. Even with 100 providers × 365 days × 144 records/day, the DB is ~5 MB.

## Backend

### Dependencies

```toml
tauri-plugin-sql = { version = "2", features = ["sqlite"] }
```

### Plugin Registration

Register `tauri_plugin_sql::Builder::default().build()` in `lib.rs` builder chain.

### DB Initialization

On app startup (in `lib.rs` setup), execute the `CREATE TABLE IF NOT EXISTS` migration. Use the plugin's Rust migration API.

### Recording Logic

In `lib.rs` async fetch block (lines 125-152), after a successful fetch:
1. Check `result.status == ProviderStatus::Ok`
2. Extract primary numeric value from `result` (the `balance` field or whichever field backs the display)
3. Execute `INSERT INTO provider_history (provider_id, recorded_at, value) VALUES (?, datetime('now'), ?)`

### Query Command

New Tauri command:

```rust
#[tauri::command]
fn get_provider_history(
    provider_id: String,
    range: String,  // "24h" | "3d" | "1w" | "1m"
) -> Result<Vec<HistoryPoint>, String>
```

Where `HistoryPoint { recorded_at: String, value: f64 }`.

SQL: `SELECT recorded_at, value FROM provider_history WHERE provider_id = ? AND recorded_at >= datetime('now', ?) ORDER BY recorded_at ASC`

The `?` parameter is `-N days` mapped from the range.

## Frontend

### New Component: `TrendChart.vue`

- **Props**: `dataPoints: { recorded_at: string; value: number }[]`
- **Rendering**: HTML5 Canvas (no external chart library)
- **Height**: fixed 52px, width 100% of parent
- **Style**: gradient area chart (option B from mockup)
  - Semi-transparent gradient fill below the line
  - Solid stroke line on top
  - Y-axis labels at min/max (2 labels)
  - Subtle horizontal grid lines
- **Edge cases**:
  - 0 data points → render nothing
  - 1 data point → horizontal line at that value
  - All same value → flat line with area fill

### Time Range Selector

- Segmented control with 4 buttons: `24h` `3d` `1w` `1m`
- Default: `1w`
- Emits `range-changed` event; parent reloads history data

### Integration into Settings Items

#### `ProviderConfig.vue` (built-in providers)

Layout order within each `.provider-item`:
1. `.provider-info` (name + badge)
2. Time range selector + `.trend-chart` (NEW)
3. `.provider-actions` (balance + buttons)

Only rendered when `provider.has_token && provider.status === "Ok"` and history data exists.

#### `PluginManager.vue` (plugin providers)

Same layout as above. Additionally, the chart should be visible for plugin providers too.

### Data Flow

1. `providers-updated` event fires → `refreshData()` updates provider list
2. For each configured provider, call `invoke("get_provider_history", { providerId, range })` 
3. Pass returned data points to `TrendChart` component
4. Time range toggle → update `range` ref → re-call `invoke` → chart re-renders

### No Chart for Unconfigured / No-Data Providers

If a provider has no token (`!has_token`) or no history data, the chart area is not rendered. The provider item layout falls back to the current 2-row layout (name+badge, token input or balance+buttons).

## Files Changed

| File | Change |
|------|--------|
| `src-tauri/Cargo.toml` | Add `tauri-plugin-sql` dependency |
| `src-tauri/src/lib.rs` | Register sql plugin, DB init migration, record history after fetch |
| `src-tauri/src/commands.rs` | Add `get_provider_history` command |
| `src/components/TrendChart.vue` | New chart component (Canvas) |
| `src/settings/ProviderConfig.vue` | Integrate chart + range selector |
| `src/settings/PluginManager.vue` | Integrate chart + range selector |
| `src/types.ts` | Add `HistoryPoint` type |

## Out of Scope

- Exporting chart data
- Adjustable chart height or style options
- Multi-value charts (just primary value)
- Trend for popover window (settings only)
- Data cleanup/pruning (keep all indefinitely)
