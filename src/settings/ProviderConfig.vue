<template>
  <div class="provider-config">
    <h3 class="section-title">{{ $t('providerConfig.builtinTitle') }}</h3>
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
      <div
        v-if="provider.has_token && isStatusOk(provider) && hasHistoryData(provider.id)"
        class="trend-section"
      >
        <div class="range-selector">
          <button
            v-for="r in ranges"
            :key="r.value"
            :class="['range-btn', { active: (historyRange[provider.id] || currentRange) === r.value }]"
            @click="onRangeChange(provider.id, r.value)"
          >
            {{ r.label }}
          </button>
        </div>
        <TrendChart :dataPoints="historyData[provider.id] || []" />
      </div>
      <div class="provider-actions">
        <template v-if="provider.has_token">
          <div class="token-display">
            <span class="balance-value">{{ balanceText(provider) }}</span>
          </div>
          <div class="provider-actions-right">
            <button class="btn-test" @click="testConnection(provider.id)">
              {{ $t('providerConfig.test') }}
            </button>
            <button class="btn-warning" @click="deleteToken(provider.id)">
              {{ $t('providerConfig.deleteToken') }}
            </button>
          </div>
        </template>
        <template v-else>
          <div class="token-input-group">
            <input
              v-model="tokenInputs[provider.id]"
              @input="stripSpaces(provider.id)"
              :type="showToken[provider.id] ? 'text' : 'password'"
              class="token-input"
              :placeholder="$t('providerConfig.tokenPlaceholder')"
            />
            <button class="btn-toggle" @click="toggleTokenVisibility(provider.id)">
              {{ showToken[provider.id] ? '👁' : '🙈' }}
            </button>
            <button
              class="btn-primary"
              :disabled="!tokenInputs[provider.id]"
              @click="saveToken(provider.id)"
            >
              {{ $t('providerConfig.save') }}
            </button>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { ask } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import type { HistoryPoint, ProviderState } from "../types";
import { getIconUrl } from "../composables/useProviderIcon";
import TrendChart from "../components/TrendChart.vue";

const { t } = useI18n();

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
const historyData = ref<Record<string, HistoryPoint[]>>({});
const historyRange = ref<Record<string, string>>({});
const currentRange = ref("1w");

const builtinProviders = computed(() =>
  providers.value.filter((p) => p.is_builtin)
);

function statusClass(provider: ProviderState): string {
  if (typeof provider.status === "string") {
    return provider.status.toLowerCase();
  }
  return "error";
}

function balanceText(provider: ProviderState): string {
  if (provider.display_label) return provider.display_label;
  if (provider.balance != null) {
    const currencySymbol = getCurrencySymbol(provider.currency);
    const fmt = (v: number) => Number.isInteger(v) ? String(v) : v.toFixed(2);
    return `${currencySymbol}${fmt(Number(provider.balance))}`;
  }
  if (provider.available != null) {
    const currencySymbol = getCurrencySymbol(provider.currency);
    const fmt = (v: number) => Number.isInteger(v) ? String(v) : v.toFixed(2);
    return `${currencySymbol}${fmt(Number(provider.available))}`;
  }
  return "••••••••";
}

function getCurrencySymbol(currency: ProviderState["currency"]): string {
  if (typeof currency === "string") {
    return currency === "USD" ? "$" : currency === "CNY" ? "¥" : currency === "EUR" ? "€" : "";
  }
  if (typeof currency === "object" && "Custom" in currency) return currency.Custom;
  return "";
}

const ranges = [
  { value: "24h", label: "24h" },
  { value: "1w", label: "1w" },
  { value: "1m", label: "1m" },
];

function isStatusOk(provider: ProviderState): boolean {
  return typeof provider.status === "string" && provider.status === "Ok";
}

function hasHistoryData(providerId: string): boolean {
  const data = historyData.value[providerId];
  return !!data && data.length > 0;
}

async function loadHistory(providerId: string, range: string) {
  try {
    const points = await invoke<HistoryPoint[]>("get_provider_history", {
      providerId,
      range,
    });
    console.log(`[history] ${providerId} range=${range} points=${points.length}`);
    historyData.value[providerId] = points;
    historyRange.value[providerId] = range;
  } catch (e) {
    console.error("Failed to load history:", e);
  }
}

async function onRangeChange(providerId: string, range: string) {
  historyRange.value[providerId] = range;
  await loadHistory(providerId, range);
}

function statusText(provider: ProviderState): string {
  if (typeof provider.status === "string") {
    const map: Record<string, string> = {
      Ok: t("providerConfig.statusOk"),
      Fetching: t("providerConfig.statusFetching"),
      Unconfigured: t("providerConfig.statusUnconfigured"),
    };
    return map[provider.status] || provider.status;
  }
  if (typeof provider.status === "object" && "Error" in provider.status) {
    return `✗ ${provider.status.Error}`;
  }
  return t("providerConfig.statusUnknown");
}

function toggleTokenVisibility(id: string) {
  showToken.value[id] = !showToken.value[id];
}

function stripSpaces(id: string) {
  const val = tokenInputs.value[id];
  if (val) {
    tokenInputs.value[id] = val.replace(/\s/g, "");
  }
}

async function saveToken(id: string) {
  const token = tokenInputs.value[id];
  if (!token) return;
  await invoke("save_token", { providerId: id, token });
  tokenInputs.value[id] = "";
  await refreshData();
}

async function deleteToken(id: string) {
  const confirmed = await ask(t("providerConfig.deleteTokenConfirm"), {
    title: t("providerConfig.deleteTokenTitle"),
    kind: "warning",
    okLabel: t("providerConfig.deleteTokenOk"),
    cancelLabel: t("providerConfig.cancel"),
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
  for (const p of providers.value) {
    if (p.has_token && typeof p.status === "string" && p.status === "Ok") {
      await loadHistory(p.id, historyRange.value[p.id] || currentRange.value);
    }
  }
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
.provider-actions-right {
  display: flex;
  gap: 4px;
}
.trend-section {
  margin-bottom: 4px;
}
.range-selector {
  display: flex;
  gap: 2px;
  background: var(--bg-input);
  border-radius: 6px;
  padding: 2px;
  width: fit-content;
  margin-bottom: 4px;
}
.range-btn {
  padding: 2px 10px;
  font-size: 11px;
  border-radius: 5px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s ease;
}
.range-btn.active {
  background: var(--bg-surface-hover);
  color: var(--text-primary);
}
.range-btn:hover:not(.active) {
  color: var(--text-secondary);
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
  width: 100%;
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
.balance-value {
  font-size: 17px;
  font-weight: 700;
  color: var(--text-heading);
  letter-spacing: -0.3px;
}
.provider-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 4px;
}
.provider-icon-wrap {
  width: 22px;
  height: 22px;
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
  width: 15px;
  height: 15px;
  object-fit: contain;
}
</style>