# CostWatch — AGENTS.md

## Project Overview

macOS menu bar app monitoring LLM API usage/balance. Built with **Tauri v2** (Rust backend + Vue 3 frontend). Uses declarative YAML configs to define provider APIs, with built-in support for OpenRouter and DeepSeek.

## Commands

```bash
# Dev (starts both Vite dev server and Tauri)
pnpm tauri dev

# Build for production
pnpm tauri build

# Rust type check only
cargo check --manifest-path src-tauri/Cargo.toml

# Frontend type check only
pnpm build  # runs vue-tsc --noEmit && vite build

# Rust tests (4 tests in config_parser)
cargo test --manifest-path src-tauri/Cargo.toml
```

## Architecture

### Two-Window App

- **Popover** (308×350, transparent, no decorations) — menu bar popup showing provider balances
- **Settings** (640×480, standard window) — token config, plugin management, general settings
- Popover auto-hides on focus loss (`WindowEvent::Focused(false)`)
- Settings window hides (not closes) on close request
- Settings window is shown on app startup (ensures accessibility even if tray icon has no space)

### Multi-Entry Vite Build

Each window has its own HTML entry point: `popover.html`, `settings.html`. Configured in `vite.config.ts` via `rollupOptions.input`. Frontend entry files: `src/main-popover.ts`, `src/main-settings.ts` (separate from `src/main.ts`).

### Rust Backend Modules

| Module | Purpose |
|--------|---------|
- `lib.rs` | Tauri Builder: plugin registration, state init, provider init, window event handlers, shows settings on startup |
| `tray.rs` | System tray with monochrome template icon (`include_image!` + `icon_as_template(true)`), left-click toggles popover |
| `commands.rs` | 10 Tauri commands exposed to frontend; `save_settings` emits `settings-updated` event |
| `state.rs` | `AppState` with `Mutex<Vec<ProviderState>>`, `Mutex<HashMap<String, ProviderConfig>>`, `Mutex<GeneralSettings>` |
| `provider/types.rs` | Core types: `ProviderState`, `ProviderConfig`, `FieldMapping`, `DisplayConfig`, `ProviderStatus`, etc. |
| `provider/fetcher.rs` | HTTP fetch, JSONPath extraction, `resolve_display_label()`, `format_field_value()`, `format_decimal()` |
| `provider/config_parser.rs` | YAML → `ProviderConfig` parsing, JSONPath extraction, arithmetic expression evaluator (recursive descent parser) |
| `provider/builtin/` | OpenRouter and DeepSeek YAML configs as Rust string constants |
| `provider/plugin.rs` | Load/import/remove YAML plugin files from `~/.costwatch/providers/` |
| `storage.rs` | File-based storage: `~/.costwatch/config.json` (settings), `~/.costwatch/tokens.json` (API tokens, plaintext — Stronghold deferred) |

### Frontend Modules

| Path | Purpose |
|------|---------|
| `src/popover/Popover.vue` | Popover root: provider list, auto-refresh interval from settings, listens to `settings-updated` event |
| `src/popover/ProviderCard.vue` | Single provider card: displays `display_label` from backend, fallback computed label |
| `src/settings/Settings.vue` | Settings root: tab navigation |
| `src/settings/ProviderConfig.vue` | Per-provider token input, test connection, delete |
| `src/settings/PluginManager.vue` | Import/remove YAML plugin files |
| `src/settings/GeneralSettings.vue` | Refresh interval selector, launch-at-login toggle, language switcher |
| `src/composables/useProviders.ts` | Reactive provider list, `invoke('refresh_all')`, frontend `setInterval` for auto-refresh |
| `src/composables/useSettings.ts` | Settings read/write via `invoke` |
| `src/composables/useLocale.ts` | Locale switching: `initLocale()` reads `language` from backend, `setLocale()` persists and applies |
| `src/composables/useProviderIcon.ts` | Provider icon resolution: CDN → local SVG → local PNG fallback |
| `src/i18n/index.ts` | vue-i18n instance (Composition API mode, `legacy: false`) |
| `src/i18n/locales/zh-CN.ts` | Simplified Chinese translations |
| `src/i18n/locales/en.ts` | English translations |
| `src/types.ts` | TypeScript types mirroring Rust structs (includes `display_label: string | null`) |

## Provider Plugin System

Providers are defined via declarative YAML configs. Built-in providers (OpenRouter, DeepSeek) are hardcoded as Rust string constants in `provider/builtin/`. Plugin providers are YAML files in `~/.costwatch/providers/` with `plugin-` prefix.

Key YAML features:
- `response.*.path`: JSONPath extraction from API response (uses `serde_json_path`)
- `response.*.expr`: Arithmetic expressions referencing other fields (e.g., `"available - used"`)
- `response.*.value`: Fixed value (e.g., `"USD"`)
- `display.label`: Template with `{{field_name}}` and `{{currency_unit}}` placeholders
- `display.unit`: Static prefix or dynamic map (e.g., `CNY → ¥`)
- Plugin IDs are prefixed with `plugin-`; builtin IDs are not (e.g., `openrouter`, `deepseek`)

## Tauri v2 Specifics

### macOS Tray Icon

Tray icon must use `tauri::include_image!("icons/tray-icon.png")` (compile-time PNG embedding) with `.icon_as_template(true)` for automatic light/dark mode adaptation. `Image::from_bytes()` failed to properly decode palette PNGs — use `include_image!` macro which handles this at compile time.

The tray icon source is `src-tauri/icons/icon-mono.png` (512×512 monochrome), scaled to 22×22 as `tray-icon.png` via `sips`.

### macOS Private API

Required for transparent popover window. Both are needed:
- `Cargo.toml`: `tauri = { version = "2", features = ["tray-icon", "macos-private-api"] }`
- `tauri.conf.json`: `"macOSPrivateApi": true` under `app`

**Warning**: Using macOS private APIs prevents App Store acceptance.

### Window Events

Popover and settings windows use `api.prevent_close()` on `CloseRequested` + `window.hide()` to keep the app running. Popover additionally hides on `Focused(false)`.

### Frontend ↔ Backend Communication

- Commands: `invoke('command_name', { args })` from `@tauri-apps/api`
- Events: Backend emits `settings-updated` via `app.emit()`; frontend listens with `listen('settings-updated', ...)`
- State: Rust `AppState` managed via `tauri::manage()`, accessed with `State<'_, AppState>` in commands

## Data Storage

- Settings: `~/.costwatch/config.json` (refresh_interval_secs, launch_at_login, language)
- Tokens: `~/.costwatch/tokens.json` (plaintext JSON — Stronghold encryption deferred)
- Plugin YAML: `~/.costwatch/providers/*.yaml`

## Internationalization (i18n)

The app uses **vue-i18n** (Composition API mode, `legacy: false`) to support multiple languages. Currently supports **Simplified Chinese** (`zh-CN`, default) and **English** (`en`).

### Architecture

- `src/i18n/index.ts`: Creates the vue-i18n instance, registered as a Vue plugin in both `main-popover.ts` and `main-settings.ts`
- `src/i18n/locales/zh-CN.ts`: Chinese translations (the canonical source of all i18n keys)
- `src/i18n/locales/en.ts`: English translations (must mirror every key in `zh-CN.ts`)
- `src/composables/useLocale.ts`: `initLocale()` loads the persisted language from backend on startup; `setLocale()` switches language and persists to `GeneralSettings`
- Language preference is stored in `GeneralSettings.language` field (both Rust `provider/types.rs` and TypeScript `src/types.ts`)
- `serde(default = "default_language")` on the Rust side ensures backward compatibility — existing `config.json` files without `language` default to `"zh-CN"`

### Adding New UI Strings (MANDATORY)

**NEVER hardcode user-visible text in Vue templates or scripts.** Every UI string must go through the i18n system:

1. **Add the key to `src/i18n/locales/zh-CN.ts`** first — this is the canonical key registry
2. **Add the same key to `src/i18n/locales/en.ts`** with the English translation
3. **Use `$t('key.path')` in templates** or `t('key.path')` in `<script setup>` (after `const { t } = useI18n()`)
4. **For interpolated strings**, use named parameters: `$t('key', { var: value })` and in locale: `"text {var} more text"`

### Key Naming Convention

- Top-level namespace per component/feature: `popover`, `provider`, `settings`, `providerConfig`, `pluginManager`, `generalSettings`
- Nested keys for sub-groups: `settings.tabs.providers`
- Descriptive key names: `deleteTokenConfirm` not `msg1`

### Backend Error Messages

Rust backend error messages (in `provider/fetcher.rs`) are **always in English** — they are technical/diagnostic strings, not user-facing labels. The frontend i18n system handles all user-visible text.

## Known Gotchas

- `ProviderConfig.UnitConfig` uses `#[serde(untagged)]` enum — YAML must use either a plain string (static) or a map with `field` and `map` keys
- Expression evaluator in `config_parser.rs` replaces field names length-first (longest first) to avoid partial matches
- `display_label` is resolved server-side from YAML template; frontend falls back to computed label only if `display_label` is null
- Numbers are formatted to 2 decimal places both in Rust (`format_decimal`) and Vue (`fmt()`)
- The `dirs` crate version is 6 (not 5) — uses `dirs::home_dir()`
- Frontend refresh timer is entirely client-side (`setInterval`), restarted on `settings-updated` event — no backend timer
- `Image::from_bytes()` fails to properly decode palette PNGs for tray icons — always use `tauri::include_image!` macro instead
- Window labels must match `^[a-zA-Z][a-zA-Z0-9/_:.-]*$` — no spaces or special chars
- `vue-i18n` Composition API mode: `i18n.global.locale` is a `WritableComputedRef`, access via `(i18n.global.locale as any).value` in non-component code