# CostWatch

macOS menu bar app for monitoring LLM API usage and balance. Click the tray icon to see provider balances in a popover, or open settings to configure API tokens and manage plugins.

Built-in support for **OpenRouter** and **DeepSeek**. Additional providers can be added via declarative YAML plugins.

## Tech Stack

- **Backend**: Rust (Tauri v2)
- **Frontend**: Vue 3 + TypeScript + Vite
- **Data**: File-based storage (`~/.costwatch/`)
- **HTTP**: reqwest
- **Config Parsing**: serde_yaml + serde_json_path

## Getting Started

### Prerequisites

- macOS 12.0+ (Monterey)
- Rust 1.88+ (stable)
- Node.js + pnpm

### Install & Run

```bash
pnpm install
pnpm tauri dev
```

### Build for Production

```bash
pnpm tauri build
```

### Type Check

```bash
# Rust
cargo check --manifest-path src-tauri/Cargo.toml

# Frontend
pnpm build
```

### Run Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Architecture

CostWatch is a two-window Tauri v2 app:

- **Popover** (308×282, transparent, no decorations) — appears near the tray icon on click, shows provider balances with auto-refresh
- **Settings** (600×500, standard window) — token configuration, plugin management, general settings

Each window has its own HTML entry (`popover.html`, `settings.html`) and Vue entry (`main-popover.ts`, `main-settings.ts`).

### Frontend ↔ Backend

- **Commands**: `invoke('command_name', { args })` from `@tauri-apps/api`
- **Events**: Backend emits `settings-updated` via `app.emit()`; frontend listens with `listen('settings-updated', ...)`
- **State**: Rust `AppState` managed via `tauri::manage()`, accessed with `State<'_, AppState>`

### Key Backend Modules

| Module | Purpose |
|--------|---------|
| `lib.rs` | Tauri Builder: plugin registration, state init, window events, shows settings on startup |
| `tray.rs` | System tray with monochrome template icon, left-click toggles popover |
| `commands.rs` | 10 Tauri commands exposed to frontend |
| `state.rs` | `AppState` with `Mutex<Vec<ProviderState>>`, `Mutex<HashMap<String, ProviderConfig>>`, `Mutex<GeneralSettings>` |
| `provider/types.rs` | Core types: `ProviderState`, `ProviderConfig`, `FieldMapping`, `DisplayConfig`, etc. |
| `provider/fetcher.rs` | HTTP fetch, JSONPath extraction, display label resolution |
| `provider/config_parser.rs` | YAML → `ProviderConfig` parsing, arithmetic expression evaluator |
| `provider/builtin/` | OpenRouter and DeepSeek configs as Rust string constants |
| `provider/plugin.rs` | Load/import/remove YAML plugins from `~/.costwatch/providers/` |
| `storage.rs` | File-based storage: config.json (settings), tokens.json (API tokens) |

### Key Frontend Modules

| Path | Purpose |
|------|---------|
| `src/popover/Popover.vue` | Popover root: provider list, auto-refresh |
| `src/popover/ProviderCard.vue` | Single provider balance card |
| `src/settings/Settings.vue` | Settings root: tab navigation |
| `src/settings/ProviderConfig.vue` | Per-provider token input and test |
| `src/settings/PluginManager.vue` | Import/remove YAML plugin files |
| `src/settings/GeneralSettings.vue` | Refresh interval, launch-at-login |
| `src/composables/useProviders.ts` | Reactive provider list, auto-refresh timer |
| `src/composables/useSettings.ts` | Settings read/write via Tauri commands |
| `src/types.ts` | TypeScript types mirroring Rust structs |

## Adding Providers

### Built-in

Built-in providers are defined as YAML string constants in `src-tauri/src/provider/builtin/`. They are registered at compile time and always available.

### Plugin Providers

Place a YAML file in `~/.costwatch/providers/` or import via the Settings UI. Plugin IDs are automatically prefixed with `plugin-`.

Example YAML:

```yaml
name: MyProvider
icon: myprovider
api:
  url: https://api.example.com/balance
  method: GET
  headers:
    Authorization: "Bearer {{token}}"
response:
  balance:
    path: "$.data.balance"
    type: number
  currency:
    value: "USD"
    type: string
display:
  primary: balance
  label: "{{currency_unit}}{{balance}}"
  unit:
    map:
      field: currency
      CNY: "¥"
      USD: "$"
  progress:
    total: balance
    used: used
    direction: consumption
```

Key YAML features:
- `response.*.path`: JSONPath extraction from API response
- `response.*.expr`: Arithmetic expressions referencing other fields (e.g., `"available - used"`)
- `response.*.value`: Fixed value
- `display.label`: Template with `{{field_name}}` and `{{currency_unit}}` placeholders
- `display.unit`: Static string or dynamic map based on a response field

## Data Storage

| File | Content |
|------|---------|
| `~/.costwatch/config.json` | General settings (refresh interval, launch-at-login) |
| `~/.costwatch/tokens.json` | API tokens (plaintext — Stronghold encryption deferred) |
| `~/.costwatch/providers/*.yaml` | Plugin provider configs |

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Vue - Official](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## License

Private — All rights reserved.