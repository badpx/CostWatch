<template>
  <div class="provider-config">
    <h3 class="section-title">内置提供商</h3>
    <div
      v-for="provider in builtinProviders"
      :key="provider.id"
      class="provider-item"
    >
      <div class="provider-info">
        <span class="provider-name">
          <span class="provider-icon-wrap">
            <img
              class="provider-icon"
              :src="iconSrc(provider)"
              :data-key="iconKey(provider)"
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
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ask } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import type { ProviderState } from "../types";
import { getIconUrl } from "../composables/useProviderIcon";

function iconKey(provider: ProviderState): string {
  if (typeof provider.icon === "object" && "Builtin" in provider.icon) return provider.icon.Builtin;
  if (typeof provider.icon === "object" && "Custom" in provider.icon) return provider.icon.Custom;
  return "";
}

function iconSrc(provider: ProviderState): string {
  return getIconUrl(iconKey(provider));
}

function onIconError(event: Event) {
  const img = event.target as HTMLImageElement;
  const key = (img.dataset.key as string) || "";
  if (img.src.startsWith("https://unpkg.com")) {
    img.src = `/icons/${key}.svg`;
  } else if (img.src.endsWith(".svg")) {
    img.src = img.src.replace(/\.svg$/, ".png");
  }
}

const providers = ref<ProviderState[]>([]);
const tokenInputs = ref<Record<string, string>>({});
const showToken = ref<Record<string, boolean>>({});

const builtinProviders = computed(() =>
  providers.value.filter((p) => p.is_builtin)
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
.provider-config {
  max-width: 600px;
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
</style>