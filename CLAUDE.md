# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Dev (starts Vite dev server + Tauri)
pnpm tauri dev

# Build for production
pnpm tauri build

# Frontend type check + build
pnpm build

# Frontend unit tests
pnpm test

# Rust type check
cargo check --manifest-path src-tauri/Cargo.toml

# Rust unit tests
cargo test --manifest-path src-tauri/Cargo.toml

# Run a single Rust test by name
cargo test --manifest-path src-tauri/Cargo.toml <test_name>
```

## Architecture

macOS menu bar app monitoring LLM API usage/balance. **Tauri v2** (Rust backend + Vue 3 frontend). Providers defined via declarative YAML configs.

### Two-Window System

- **Popover** (308×350, transparent, no decorations) — menu bar popup, auto-hides on focus loss, entry: `popover.html` → `src/main-popover.ts`
- **Settings** (640×480, standard window) — hides (not closes) on close request, shown on startup for accessibility, entry: `settings.html` → `src/main-settings.ts`

### Backend (src-tauri/src/)

- `lib.rs` — Tauri Builder, plugin registration, state init, window event handlers, async provider refresh after startup
- `tray.rs` — System tray with `include_image!` + `icon_as_template(true)` for light/dark mode
- `commands.rs` — 10 Tauri commands exposed to frontend; `save_settings` emits `settings-updated` event
- `state.rs` — `AppState`: `Mutex<Vec<ProviderState>>`, `Mutex<HashMap<String, ProviderConfig>>`, `Mutex<GeneralSettings>`
- `provider/fetcher.rs` — HTTP fetch, JSONPath extraction, display label resolution, `currency_from_config()` for startup currency
- `provider/config_parser.rs` — YAML→ProviderConfig parsing, JSONPath extraction, arithmetic expression evaluator (recursive descent)
- `provider/types.rs` — Core types: `ProviderState`, `ProviderConfig`, `UnitConfig`, `Currency`, `DisplayConfig`, etc.
- `provider/builtin/` — Built-in provider YAML configs as Rust string constants (openrouter, deepseek, kimi, deepinfra, runware)
- `provider/plugin.rs` — Load/import/remove YAML plugin files from `~/.costwatch/providers/`
- `provider/history.rs` — Balance history tracking via SQLite at `~/.costwatch/history.db`, 30-second dedup on writes
- `storage.rs` — File-based storage: `~/.costwatch/config.json`, `~/.costwatch/tokens.json`

### Frontend (src/)

- `popover/` — Popover UI: `Popover.vue` (root), `ProviderCard.vue` (single provider card)
- `settings/` — Settings UI: `Settings.vue` (tab nav), `ProviderConfig.vue`, `PluginManager.vue`, `GeneralSettings.vue`
- `composables/` — `useProviders.ts` (reactive provider list + auto-refresh), `useSettings.ts`, `useLocale.ts`, `useProviderIcon.ts`
- `components/TrendChart.vue` — Canvas-based trend chart (uses `src/utils/chart.ts` for pure functions)
- `utils/chart.ts` — Extracted pure functions: `rangeToMs`, `formatVal`, `getCurrencySymbol`
- `i18n/` — vue-i18n (Composition API mode), `zh-CN.ts` is the canonical key source, `en.ts` mirrors it
- `types.ts` — TypeScript types mirroring Rust structs

### Frontend ↔ Backend Communication

- **Commands**: `invoke('command_name', { args })` — synchronous request/response
- **Events**: Backend emits `providers-updated`, `settings-updated`; frontend listens with `listen()`
- **State**: Rust `AppState` via `tauri::manage()`, accessed with `State<'_, AppState>` in commands

## Provider Plugin System

Two provider types:
1. **Built-in** — YAML configs hardcoded as Rust string constants in `provider/builtin/`, IDs without prefix (e.g., `openrouter`)
2. **Plugin** — YAML files in `~/.costwatch/providers/`, auto-prefixed with `plugin-`

Key YAML features:
- `response.*.path`: JSONPath extraction (`serde_json_path`)
- `response.*.expr`: Arithmetic expressions referencing other fields (e.g., `"available - used"`)
- `response.*.value`: Fixed value
- `display.label`: Template with `{{field_name}}` and `{{currency_unit}}` placeholders
- `display.unit`: `Static(string)` or `Map { field, map, default? }` — the `default` field provides startup currency before first API fetch

## Key Conventions

- **i18n**: All user-visible text must go through `$t('key')` / `t('key')`. Add keys to `zh-CN.ts` first (canonical), then `en.ts`.
- **Currency**: `Currency` enum (USD/CNY/EUR/Custom) with `PartialEq`. At startup, derived from config via `currency_from_config()`, not persisted. Frontend `getCurrencySymbol()` in `src/utils/chart.ts` maps to display symbols.
- **Trend chart**: Canvas-based, reads CSS custom properties (`--border`, `--text-tertiary`) for theme adaptation. Single data point renders as a dot, not a line.
- **History dedup**: 30-second window in `record_history()` prevents duplicate entries from concurrent fetch paths (e.g., `refresh_all` + `test_connection`).
- **UnitConfig**: `#[serde(untagged)]` enum — YAML must use either a plain string (static) or a map with `field` and `map` keys.
- **Expression evaluator**: Replaces field names length-first (longest first) to avoid partial matches.
- **macOS private API**: Required for transparent popover (`macos-private-api` feature + `"macOSPrivateApi": true`). Prevents App Store acceptance.
- **Tray icon**: Must use `tauri::include_image!` macro (not `Image::from_bytes()`) — palette PNG decoding fails otherwise.

## Data Storage

```
~/.costwatch/
├── config.json          # GeneralSettings (refresh_interval_secs, launch_at_login, language, trend_range)
├── tokens.json          # API tokens (plaintext)
├── history.db           # SQLite: provider_history table (provider_id, recorded_at, value)
└── providers/           # Plugin YAML files
```
