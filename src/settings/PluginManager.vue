<template>
  <div class="plugin-manager">
    <h3 class="section-title">插件提供商</h3>

    <div
      v-for="provider in pluginProviders"
      :key="provider.id"
      class="provider-item"
    >
      <div class="provider-info">
        <span class="provider-name">
          <span class="provider-icon-wrap">
            <img
              class="provider-icon"
              :src="iconSrc(provider)"
              @error="onIconError"
              alt=""
            />
          </span>
          {{ provider.name }}
        </span>
        <span :class="['provider-badge', statusClass(provider)]">
          {{ statusText(provider) }}
        </span>
      </div>
      <div class="provider-actions">
        <template v-if="provider.has_token">
          <div class="token-display">
            <span class="token-masked">••••••••</span>
          </div>
          <button class="btn-test" @click="testConnection(provider.id)">
            测试
          </button>
          <button class="btn-warning" @click="deleteToken(provider.id)">
            删除Token
          </button>
          <button class="btn-danger" @click="removePlugin(provider.id)">
            移除插件
          </button>
        </template>
        <template v-else>
          <div class="token-input-group">
            <input
              v-model="tokenInputs[provider.id]"
              :type="showToken[provider.id] ? 'text' : 'password'"
              class="token-input"
              placeholder="输入 API Token"
            />
            <button class="btn-toggle" @click="toggleTokenVisibility(provider.id)">
              {{ showToken[provider.id] ? '👁' : '🙈' }}
            </button>
            <button
              class="btn-primary"
              :disabled="!tokenInputs[provider.id]"
              @click="saveToken(provider.id)"
            >
              保存
            </button>
            <button class="btn-danger" @click="removePlugin(provider.id)">
              移除插件
            </button>
          </div>
        </template>
      </div>
    </div>

    <div v-if="pluginProviders.length === 0" class="empty-state">
      <p>暂无插件提供商</p>
    </div>

    <div class="import-section">
      <button class="btn-primary" @click="importPlugin">
        + 导入插件配置文件
      </button>
      <button class="btn-link" @click="showGuide = !showGuide">
        {{ showGuide ? '收起说明' : '查看示例配置' }}
      </button>
    </div>

    <div v-if="showGuide" class="guide-panel">
      <h4 class="guide-title">插件配置格式说明</h4>
      <p class="guide-desc">
        插件是一个 YAML 文件，定义了如何从 Provider API 获取用量数据并展示。
      </p>
      <div class="guide-section">
        <h5 class="guide-subtitle">关键字段说明</h5>
        <ul class="guide-list">
          <li><strong>api.url</strong> — 查询余额的 API 地址</li>
          <li><strong>api.headers</strong> — 请求头，使用 <code v-pre>{{token}}</code> 占位符表示用户填写的 Token</li>
          <li><strong>response</strong> — 定义如何从 JSON 响应中提取字段</li>
          <li><strong>display.label</strong> — 展示文案，支持 <code v-pre>{{字段名}}</code> 和 <code v-pre>{{currency_unit}}</code> 占位符</li>
        </ul>
      </div>
      <div class="guide-section">
        <h5 class="guide-subtitle">示例配置</h5>
        <div class="code-block">
          <pre>{{ sampleYaml }}</pre>
          <button class="btn-copy" @click="copySample">{{ copied ? '已复制' : '复制' }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, ask } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import type { ProviderState, ProviderConfig } from "../types";
import { getIconUrl } from "../composables/useProviderIcon";

function iconKey(provider: ProviderState): string {
  if (typeof provider.icon === "object" && "Custom" in provider.icon) return provider.icon.Custom;
  return "";
}

function iconSrc(provider: ProviderState): string {
  const key = iconKey(provider);
  return key ? getIconUrl(key) : "/icons/plugin.svg";
}

function onIconError(event: Event) {
  const img = event.target as HTMLImageElement;
  if (img.src.startsWith("https://unpkg.com")) {
    img.src = "/icons/plugin.svg";
  } else if (img.src.endsWith(".svg")) {
    img.src = img.src.replace(/\.svg$/, ".png");
  }
}

const providers = ref<ProviderState[]>([]);
const tokenInputs = ref<Record<string, string>>({});
const showToken = ref<Record<string, boolean>>({});
const showGuide = ref(false);
const copied = ref(false);

const sampleYaml = `name: MyProvider
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
  unit_prefix: "$"`;

async function copySample() {
  await navigator.clipboard.writeText(sampleYaml);
  copied.value = true;
  setTimeout(() => (copied.value = false), 2000);
}

const pluginProviders = computed(() =>
  providers.value.filter((p) => !p.is_builtin)
);

function statusClass(provider: ProviderState): string {
  if (typeof provider.status === "string") {
    return provider.status.toLowerCase();
  }
  return "error";
}

function statusText(provider: ProviderState): string {
  if (typeof provider.status === "string") {
    const map: Record<string, string> = {
      Ok: "✓ 已连接",
      Fetching: "⏳ 连接中",
      Unconfigured: "未配置",
    };
    return map[provider.status] || provider.status;
  }
  if (typeof provider.status === "object" && "Error" in provider.status) {
    return `✗ ${provider.status.Error}`;
  }
  return "未知";
}

function toggleTokenVisibility(id: string) {
  showToken.value[id] = !showToken.value[id];
}

async function saveToken(id: string) {
  const token = tokenInputs.value[id];
  if (!token) return;
  await invoke("save_token", { providerId: id, token });
  tokenInputs.value[id] = "";
  await refreshData();
}

async function deleteToken(id: string) {
  const confirmed = await ask("确定要删除此 Token 吗？删除后需要重新配置才能继续使用。", {
    title: "删除确认",
    kind: "warning",
    okLabel: "删除",
    cancelLabel: "取消",
  });
  if (!confirmed) return;
  await invoke("delete_token", { providerId: id });
  await refreshData();
}

async function testConnection(id: string) {
  try {
    const result = await invoke<ProviderState>("test_connection", {
      providerId: id,
    });
    const index = providers.value.findIndex((p) => p.id === id);
    if (index >= 0) providers.value[index] = result;
  } catch (e) {
    console.error("Connection test failed:", e);
  }
}

async function importPlugin() {
  const selected = await open({
    multiple: false,
    filters: [{ name: "YAML", extensions: ["yaml", "yml"] }],
  });

  if (!selected) return;

  try {
    const config = await invoke<ProviderConfig>("import_plugin", {
      yamlPath: selected,
    });
    const existed = providers.value.some(
      (p) => !p.is_builtin && p.name === config.name
    );
    alert(
      existed
        ? `成功覆盖插件：${config.name}`
        : `成功导入插件：${config.name}`
    );
    await refreshData();
  } catch (e) {
    alert(`导入失败: ${e}`);
  }
}

async function removePlugin(id: string) {
  const confirmed = await ask(`确定要移除插件 ${id} 吗？`, {
    title: "移除确认",
    kind: "warning",
    okLabel: "移除",
    cancelLabel: "取消",
  });
  if (!confirmed) return;

  try {
    await invoke("remove_plugin", { providerId: id });
    await refreshData();
  } catch (e) {
    alert(`移除失败: ${e}`);
  }
}

async function refreshData() {
  providers.value = await invoke<ProviderState[]>("get_providers");
}

let unlisten: (() => void) | null = null;

onMounted(async () => {
  await refreshData();
  unlisten = await listen("providers-updated", () => refreshData());
});

onUnmounted(() => {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
});
</script>

<style scoped>
.plugin-manager {
  max-width: 600px;
  margin: 0 auto;
}
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 12px;
}
.provider-item {
  background: var(--bg-surface);
  border-radius: 8px;
  padding: 12px;
  margin-bottom: 8px;
}
.provider-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.provider-name {
  font-weight: 600;
  font-size: 15px;
  display: flex;
  align-items: center;
}
.provider-badge {
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 4px;
}
.provider-badge.ok {
  background: var(--success-bg);
  color: var(--success-text);
}
.provider-badge.unconfigured {
  background: var(--bg-surface-hover);
  color: var(--text-muted);
}
.provider-badge.error {
  background: var(--danger-bg);
  color: var(--danger-text);
}
.token-input-group {
  display: flex;
  gap: 6px;
  align-items: center;
}
.token-input {
  flex: 1;
  padding: 6px 10px;
  background: var(--bg-input);
  border: 1px solid var(--border-strong);
  border-radius: 4px;
  color: var(--text-primary);
  font-size: 13px;
}
.token-input:focus {
  outline: none;
  border-color: var(--accent);
}
.btn-primary {
  padding: 6px 14px;
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
}
.btn-primary:disabled {
  opacity: 0.5;
}
.btn-test {
  padding: 4px 10px;
  background: var(--bg-surface-hover);
  border: 1px solid var(--border-hover);
  color: var(--text-primary);
}
.btn-danger {
  padding: 4px 10px;
  background: none;
  border: 1px solid var(--danger-border);
  color: var(--danger-text);
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}
.btn-warning {
  padding: 4px 10px;
  background: none;
  border: 1px solid var(--warning-border);
  color: var(--warning-text);
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}
.btn-toggle {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 16px;
  padding: 4px;
}
.token-masked {
  font-family: monospace;
  font-size: 13px;
  color: var(--text-secondary);
}
.provider-actions {
  margin-top: 4px;
}
.provider-icon-wrap {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--icon-bg);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  overflow: hidden;
  vertical-align: middle;
  margin-right: 4px;
}
.provider-icon {
  width: 12px;
  height: 12px;
  object-fit: contain;
}
.empty-state {
  color: var(--text-tertiary);
  font-size: 13px;
  text-align: center;
  padding: 16px;
}
.import-section {
  margin-top: 16px;
  display: flex;
  gap: 12px;
  align-items: center;
}
.btn-link {
  background: none;
  border: none;
  color: var(--accent);
  cursor: pointer;
  font-size: 13px;
  padding: 0;
}
.btn-link:hover {
  text-decoration: underline;
}
.guide-panel {
  margin-top: 16px;
  padding: 16px;
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: 8px;
}
.guide-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0 0 8px;
}
.guide-desc {
  font-size: 13px;
  color: var(--text-muted);
  margin: 0 0 12px;
  line-height: 1.5;
}
.guide-section {
  margin-bottom: 12px;
}
.guide-subtitle {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  margin: 0 0 6px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.guide-list {
  font-size: 12px;
  color: var(--text-muted);
  line-height: 1.6;
  padding-left: 16px;
  margin: 0;
}
.guide-list li {
  margin-bottom: 4px;
}
.guide-list code {
  background: var(--bg-surface-hover);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 11px;
  color: var(--text-primary);
}
.code-block {
  position: relative;
  background: var(--bg-code);
  border-radius: 6px;
  padding: 12px;
  overflow-x: auto;
}
.code-block pre {
  margin: 0;
  font-size: 12px;
  font-family: "SF Mono", "Fira Code", "JetBrains Mono", monospace;
  color: var(--text-primary);
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-all;
}
.btn-copy {
  position: absolute;
  top: 8px;
  right: 8px;
  padding: 3px 10px;
  background: var(--bg-surface-hover);
  border: 1px solid var(--border-strong);
  color: var(--text-primary);
  border-radius: 4px;
  cursor: pointer;
  font-size: 11px;
}
.btn-copy:hover {
  background: var(--border-hover);
}
</style>
