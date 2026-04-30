# TokenWatch — macOS 菜单栏大模型用量监控工具

## 概述

TokenWatch 是一个 macOS 菜单栏工具，用于查询和显示大语言模型（LLM）API 的余额和消耗情况。用户在设置界面配置各厂商的 API Token，菜单栏图标点击后弹出 Popover 窗口准实时展示消耗进度。支持通过声明式配置文件扩展更多厂商。

## 技术栈

| 层 | 技术 |
|---|---|
| 框架 | Tauri v2 |
| 后端 | Rust |
| 前端 | Vue 3 + TypeScript + Vite |
| 加密存储 | Tauri Stronghold 插件 |
| 菜单栏弹窗 | tauri-nspopover（原生 macOS Popover） |
| 窗口定位 | tauri-plugin-positioner（备选方案） |
| HTTP 客户端 | reqwest (Rust) |

## 架构方案：Popover 架构（方案 A）

点击菜单栏图标弹出原生风格的 Popover 窗口（毛玻璃效果），设置面板作为独立标准窗口。

```
菜单栏图标 → 左键点击 → macOS 原生 Popover（毛玻璃效果）
                         ├── 各厂商余额 + 进度条
                         ├── 刷新状态指示
                         └── "打开设置" 按钮 → 主窗口

菜单栏图标 → 右键点击 → macOS 原生菜单（退出、关于...）
```

**选择理由**：
- 最佳 macOS 原生体验，是用户期望的菜单栏交互模式
- `tauri-nspopover` 已有实际项目验证（tokenmeter 等菜单栏应用在用）
- 支持进度条等富 UI 元素

**备选方案**：方案 B（Positioner 窗口架构），使用 `tauri-plugin-positioner` + `Effect::Popover`，全部用官方插件，稳定性更高但非真正原生 Popover。如果 `tauri-nspopover` 出现兼容性问题，可切换到此方案。

## 核心模块

| 模块 | 职责 | 技术层 |
|------|------|--------|
| Tray Manager | 菜单栏图标、点击事件、Popover 生命周期 | Rust (Tauri) |
| Provider Registry | 厂商注册、内置厂商 + 插件厂商管理 | Rust |
| Provider Fetcher | 定时轮询各厂商 API，解析响应 | Rust (reqwest) |
| Storage | Token 加密存储，厂商配置持久化 | Rust (Stronghold) |
| Plugin Loader | 加载声明式插件配置，验证并注册到 Registry | Rust |
| Popover UI | 余额展示、进度条、刷新状态 | Vue 3 |
| Settings UI | Token 管理、厂商配置、插件导入 | Vue 3 |

## 数据流

```
定时器(60s) → Fetcher → 请求各厂商API → 解析响应 → 存入内存状态
                                                        ↓
Popover UI ← 事件驱动 ← 状态变更通知 ← ─────────────────┘
Settings UI ← Tauri Command ← 读写 Stronghold ← Token/配置
```

## API 接口定义

### OpenRouter

**请求**：
```
GET https://openrouter.ai/api/v1/credits
Authorization: Bearer <token>
```

**响应**：
```json
{
  "data": {
    "total_credits": 100.5,
    "total_usage": 25.75
  }
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `data.total_credits` | `number` (double) | 总充值额度 |
| `data.total_usage` | `number` (double) | 已使用额度 |
| 可用余额 | 计算值 | `total_credits - total_usage` |

**注意**：数值有最多 60 秒缓存延迟。无币种字段，默认 USD。

### DeepSeek

**请求**：
```
GET https://api.deepseek.com/user/balance
Authorization: Bearer <token>
```

**响应**：
```json
{
  "is_available": true,
  "balance_infos": [
    {
      "currency": "CNY",
      "total_balance": "113.09",
      "granted_balance": "0.00",
      "topped_up_balance": "113.09"
    }
  ]
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `is_available` | `boolean` | 余额是否足够调用 |
| `balance_infos[].currency` | `string` | 币种（CNY / USD） |
| `balance_infos[].total_balance` | `string` | 总余额（字符串形式数字） |
| `balance_infos[].granted_balance` | `string` | 赠送余额（未过期） |
| `balance_infos[].topped_up_balance` | `string` | 充值余额 |

**注意**：所有金额为字符串类型，需做 decimal 解析。按币种分组返回，可能有 CNY 和 USD 两条记录。

## 声明式插件系统

### 设计原则

内置厂商和插件厂商使用**同一套声明式接口**。内置厂商的配置在代码中硬编码为 YAML 常量，插件厂商由用户导入 YAML 文件。

### 插件配置文件格式

```yaml
# ~/.tokenwatch/providers/<provider-id>.yaml
name: OpenRouter                           # 显示名称
icon: openrouter                           # 内置图标名或 URL/path
api:
  url: https://openrouter.ai/api/v1/credits
  method: GET
  headers:
    Authorization: "Bearer {{token}}"      # {{token}} 会替换为用户配置的 Token
response:
  available:                                # 字段定义：用 JSONPath 提取
    path: "$.data.total_credits"
    type: number
  used:
    path: "$.data.total_usage"
    type: number
  balance:                                  # 支持表达式计算
    expr: "available - used"
    type: number
  currency:
    value: "USD"                            # 支持固定值
display:
  primary: balance                          # 主显示字段
  secondary: used                           # 次要显示字段
  label: "{{balance}} / {{available}}"      # 显示模板
  unit_prefix: "$"                          # 货币符号前缀
  progress:
    total: available                        # 进度条：总量
    used: used                               # 进度条：已用量
    direction: consumption                   # consumption=已用占比 / remaining=剩余占比
```

```yaml
# DeepSeek 示例
name: DeepSeek
icon: deepseek
api:
  url: https://api.deepseek.com/user/balance
  method: GET
  headers:
    Authorization: "Bearer {{token}}"
response:
  available:
    path: "$.balance_infos[0].total_balance"
    type: string_number                     # 字符串形式的数字
  granted:
    path: "$.balance_infos[0].granted_balance"
    type: string_number
  topped_up:
    path: "$.balance_infos[0].topped_up_balance"
    type: string_number
  currency:
    path: "$.balance_infos[0].currency"
    type: string
  is_available:
    path: "$.is_available"
    type: boolean
display:
  primary: available
  secondary: granted
  label: "{{available}} {{currency_unit}}"   # 显示模板引用映射后的货币符号
  unit:
    map:                                     # 字段值到符号的映射
      field: currency
      CNY: "¥"
      USD: "$"
  progress:
    total: available
    used: granted
    direction: remaining
```

### JSONPath 依赖

使用 `serde_json_path` crate 解析 JSONPath 表达式，提取 API 响应中的字段。

### 插件加载流程

```
用户导入 .yaml → YAML Schema 验证 → 语法检查 → 注册到 Provider Registry
                                              ↳ 验证失败 → Settings UI 显示具体错误信息

运行时：Fetcher 遍历所有已注册且有 Token 的厂商 → 按 api 配置发请求 → 按 response 配置解析 → 存入 ProviderState
```

### 内置厂商 vs 插件厂商

| 特征 | 内置厂商 | 插件厂商 |
|------|---------|---------|
| 定义位置 | Rust 代码中硬编码为 YAML 常量 | `~/.tokenwatch/providers/*.yaml` |
| 加载时机 | 应用启动时 | 用户导入时 |
| 更新方式 | 随应用更新 | 用户手动管理 |
| 配置验证 | 编译时保证 | 运行时验证 YAML schema |
| 底层接口 | 同一套 `ProviderConfig` 结构体 | 同一套 `ProviderConfig` 结构体 |

## 数据模型

### Rust 核心类型

```rust
/// 所有厂商共用的统一数据模型
struct ProviderState {
    id: String,                      // 唯一标识, e.g. "openrouter"
    name: String,                    // 显示名称
    icon: ProviderIcon,              // 图标（内置名或自定义路径）
    is_builtin: bool,                 // 是否内置厂商

    // 余额信息（统一后）
    balance: Option<Decimal>,         // 总余额
    used: Option<Decimal>,            // 已用额度
    available: Option<Decimal>,       // 可用额度
    currency: Currency,               // 货币类型
    is_available: Option<bool>,       // DeepSeek 的可用性标记

    // 状态
    status: ProviderStatus,           // Ok | Error(msg) | Fetching | Unconfigured
    last_updated: Option<DateTime<Utc>>,
    error_message: Option<String>,
}

enum ProviderStatus {
    Ok,
    Fetching,
    Unconfigured,    // 没有配置 Token
    Error(String),
}

enum ProviderIcon {
    Builtin(String),     // "openrouter", "deepseek"
    Custom(PathBuf),    // 自定义图标文件路径
}

enum Currency {
    USD,
    CNY,
    EUR,
    Custom(String),
}
```

### 插件配置类型

```rust
/// 声明式插件配置的 Rust 表示
struct ProviderConfig {
    name: String,
    icon: String,
    api: ApiConfig,
    response: HashMap<String, FieldMapping>,
    display: DisplayConfig,
}

struct ApiConfig {
    url: String,
    method: HttpMethod,
    headers: HashMap<String, String>,  // 支持 {{token}} 模板
}

struct FieldMapping {
    path: Option<String>,     // JSONPath
    value: Option<String>,    // 固定值
    expr: Option<String>,     // 表达式
    r#type: FieldType,        // number, string_number, string, boolean
}

struct DisplayConfig {
    primary: String,           // 主显示字段名
    secondary: Option<String>, // 次要显示字段名
    label: String,             // 显示模板（支持 {{field}} 占位符）
    unit_prefix: Option<String>,           // 固定前缀，如 "$"
    unit: Option<UnitConfig>,             // 动态货币映射
    progress: ProgressConfig,
}

enum UnitConfig {
    Static(String),                  // 固定值，如 "$" 或 "¥"
    Map { field: String, map: HashMap<String, String> },  // 根据 response 字段值映射，如 currency → {"CNY": "¥", "USD": "$"}
}

struct ProgressConfig {
    total: String,        // 总量字段名
    used: String,         // 已用/进度字段名
    direction: ProgressDirection,  // consumption 或 remaining
}
```

### Stronghold 存储 Schema

```
Stronghold Vault:
├── "providers"
│   ├── "openrouter" → ProviderToken { token: "***", enabled: true }
│   ├── "deepseek"   → ProviderToken { token: "***", enabled: true }
│   └── "custom-xxx" → ProviderToken { token: "***", enabled: true }
└── "settings"
    └── "general"    → GeneralSettings { refresh_interval_secs: 60, launch_at_login: false }
```

非敏感配置（如插件 YAML 文件路径、刷新间隔）存储在 `$HOME/.tokenwatch/config.json`，只有 API Token 通过 Stronghold 加密存储。

## UI 设计

### Popover 窗口（约 320×420）

```
┌─ TokenWatch ───────────────────── [刷新中↻] ─┐
│                                               │
│  ┌─ OpenRouter ─────────────────────────────┐ │
│  │  🔗  $74.75 / $100.50                    │ │
│  │  [████████████████░░░░░] 74.4% used     │ │
│  │  上次更新: 30s前                          │ │
│  └─────────────────────────────────────────┘ │
│                                               │
│  ┌─ DeepSeek ──────────────────────────────┐ │
│  │  🐋  ¥113.09  (可用)                     │ │
│  │  [█████████████████████░] ¥13.09 granted │ │
│  │  上次更新: 45s前                          │ │
│  └─────────────────────────────────────────┘ │
│                                               │
│  ┌─ CustomProvider ──────────────────────── ┐ │
│  │  ⚠ 网络错误: 连接超时                    │ │
│  │  [重试]                                   │ │
│  └─────────────────────────────────────────┘ │
│                                               │
│  ────────── 自动刷新 @ 60s ────────── [⚙] ── │
└───────────────────────────────────────────────┘
```

### Settings 窗口（约 600×500）

```
┌─ TokenWatch 设置 ────────────────── ✕ ──────┐
│                                               │
│  [厂商配置]  [插件管理]  [通用]              │
│  ─────────────────────────────────────────    │
│                                               │
│  内置厂商:                                     │
│  ┌─ OpenRouter ─────────────────── ✅ 已配置 ┐│
│  │  Token: sk-••••••••f613            [测试]  ││
│  │  状态: ✓ 连接正常                        ││
│  │  [删除Token] [禁用]                       ││
│  └─────────────────────────────────────────┘ │
│  ┌─ DeepSeek ───────────────────── ✅ 已配置 ┐│
│  │  Token: sk-••••••••eb28            [测试]  ││
│  │  状态: ✓ 连接正常                        ││
│  │  [删除Token] [禁用]                       ││
│  └─────────────────────────────────────────┘ │
│                                               │
│  插件厂商:                                     │
│  ┌─ Anthropic ─────────────────── ⚠ 未配置 ┐│
│  │  [配置Token]  [移除插件]                  ││
│  └─────────────────────────────────────────┘ │
│                                               │
│                          [+ 导入插件配置文件]  │
└───────────────────────────────────────────────┘
```

**通用设置 Tab**：
- 刷新间隔（默认 60s，可调 30s / 60s / 120s / 300s）
- 开机自启动
- 菜单栏图标选择

## 错误处理与边界情况

| 场景 | 行为 |
|------|------|
| API Token 无效/过期 | Popover 显示 "认证失败，请检查Token"；Settings 标记为红色错误状态 |
| 网络断开 | Popover 显示 "网络不可用"，使用上次缓存数据（灰色标注） |
| API 限流 (429) | 保持上次缓存，延长下次刷新间隔到 5 分钟 |
| 插件 YAML 格式错误 | Settings 页显示具体验证错误，拒绝注册 |
| Token 存储损坏 | 提示重新配置，不 crash |
| 多币种混合 | Popover 按币种分组显示，不混排 |
| 应用启动无网络 | 显示缓存数据（如果有），标注 "离线数据" |
| 所有厂商都未配置 | Popover 显示引导信息："请先配置厂商 Token" + 打开设置按钮 |
| 厂商 API 响应格式变更 | 解析失败后显示 "数据解析错误"，不 crash |

## 刷新策略

| 情况 | 策略 |
|------|------|
| 正常 | 每 60 秒刷新 |
| 窗口刚打开 | 立即刷新一次 |
| API 限流 (429) | 退避到 5 分钟 |
| 连续失败 3 次 | 暂停该厂商刷新，显示重试按钮 |
| 用户手动刷新 | 立即请求，重置定时器 |
| 系统休眠恢复 | 下次唤醒时立即刷新 |

## 项目结构

```
tokenwatch/
├── src-tauri/                    # Rust 后端
│   ├── src/
│   │   ├── main.rs              # 入口
│   │   ├── lib.rs               # Tauri Builder，插件注册
│   │   ├── tray.rs              # 菜单栏图标 + 点击事件
│   │   ├── commands.rs          # Tauri Commands（前端调用接口）
│   │   ├── provider/
│   │   │   ├── mod.rs           # Provider trait / struct 定义
│   │   │   ├── registry.rs      # 厂商注册表
│   │   │   ├── fetcher.rs       # 定时轮询调度器
│   │   │   ├── config.rs        # 声明式配置解析
│   │   │   ├── openrouter.rs    # 内置 OpenRouter 配置
│   │   │   └── deepseek.rs      # 内置 DeepSeek 配置
│   │   ├── plugin.rs            # 插件 YAML 加载、验证、注册
│   │   ├── storage.rs           # Stronghold 加密存储
│   │   └── state.rs             # 应用全局状态管理
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                           # Vue 3 前端
│   ├── App.vue                   # 入口
│   ├── popover/
│   │   ├── Popover.vue           # Popover 主界面
│   │   └── ProviderCard.vue      # 单个厂商卡片组件
│   ├── settings/
│   │   ├── Settings.vue           # 设置主界面
│   │   ├── ProviderConfig.vue    # 厂商配置组件
│   │   └── PluginManager.vue     # 插件管理组件
│   ├── composables/
│   │   ├── useProviders.ts       # 厂商状态 composable
│   │   └── useConnection.ts     # 连接状态 composable
│   ├── types.ts                  # TypeScript 类型定义
│   └── main.ts
├── package.json
├── vite.config.ts
└── tsconfig.json
```

## Tauri Commands（前后端接口）

```rust
// commands.rs — 前端可调用的 Tauri 命令

#[command]
async fn get_providers() -> Vec<ProviderState> { ... }

#[command]
async fn save_token(provider_id: String, token: String) -> Result<(), String> { ... }

#[command]
async fn delete_token(provider_id: String) -> Result<(), String> { ... }

#[command]
async fn test_connection(provider_id: String) -> Result<ProviderState, String> { ... }

#[command]
async fn refresh_provider(provider_id: String) -> Result<ProviderState, String> { ... }

#[command]
async fn refresh_all() -> Result<(), String> { ... }

#[command]
async fn import_plugin(yaml_path: String) -> Result<ProviderConfig, String> { ... }

#[command]
async fn remove_plugin(provider_id: String) -> Result<(), String> { ... }

#[command]
async fn get_settings() -> GeneralSettings { ... }

#[command]
async fn save_settings(settings: GeneralSettings) -> Result<(), String> { ... }

// 前端 → 后端事件
// "provider-state-updated" — 厂商状态变更时推送
// "refresh-started" / "refresh-completed" — 刷新状态
```

## 关键依赖

### Cargo (Rust)

| crate | 用途 |
|-------|------|
| `tauri` 2.x | 应用框架 |
| `tauri-plugin-stronghold` | Token 加密存储 |
| `tauri-plugin-nspopover` | macOS 原生 Popover |
| `reqwest` | HTTP 客户端 |
| `serde` + `serde_yaml` | YAML/JSON 序列化 |
| `serde_json_path` | JSONPath 表(expression)解析 |
| `rust_decimal` | 精确小数运算（避免浮点精度问题） |
| `chrono` | 时间处理 |
| `tokio` | 异步运行时 |

### NPM (前端)

| package | 用途 |
|---------|------|
| `vue` 3.x | UI 框架 |
| `@tauri-apps/api` | Tauri 前端 API |
| `@tauri-apps/plugin-stronghold` | Stronghold 前端绑定 |
| `typescript` | 类型安全 |

## 最低系统要求

- macOS 12.0+ (Monterey)
- Apple Silicon (M1+) 和 Intel Mac 均支持