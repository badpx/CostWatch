<template>
  <div class="plugin-manager">
    <h3 class="section-title">{{ $t('pluginManager.title') }}</h3>

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
            <span class="balance-value">{{ cachedBalanceText(provider) }}</span>
          </div>
          <div class="provider-actions-right">
            <button class="btn-icon" :title="$t('providerConfig.toggleTrend')" @click="toggleTrend(provider.id)">
              <svg class="trend-chevron" :class="{ expanded: trendExpanded[provider.id] }" width="14" height="14" viewBox="0 0 16 16" fill="none">
                <path d="M4 6L8 10L12 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            </button>
            <button class="btn-test" @click="testConnection(provider.id)">
              {{ $t('pluginManager.test') }}
            </button>
            <button class="btn-warning" @click="deleteToken(provider.id)">
              {{ $t('pluginManager.deleteToken') }}
            </button>
            <button class="btn-danger" @click="removePlugin(provider.id)">
              {{ $t('pluginManager.removePlugin') }}
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
              :placeholder="$t('pluginManager.tokenPlaceholder')"
            />
            <button class="btn-toggle" @click="toggleTokenVisibility(provider.id)">
              {{ showToken[provider.id] ? '👁' : '🙈' }}
            </button>
            <button
              class="btn-primary"
              :disabled="!tokenInputs[provider.id]"
              @click="saveToken(provider.id)"
            >
              {{ $t('pluginManager.save') }}
            </button>
            <button class="btn-danger" @click="removePlugin(provider.id)">
              {{ $t('pluginManager.removePlugin') }}
            </button>
          </div>
        </template>
      </div>
      <div
        v-if="provider.has_token"
        class="trend-collapse"
        :class="{ expanded: trendExpanded[provider.id] }"
      >
        <div class="trend-section">
          <TrendChart :dataPoints="historyData[provider.id] || []" :range="trendRange" :currency="provider.currency" />
        </div>
      </div>
    </div>

    <div v-if="pluginProviders.length === 0" class="empty-state">
      <p>{{ $t('pluginManager.noPlugins') }}</p>
    </div>

    <div class="import-section">
      <button class="btn-link" @click="showGuide = !showGuide">
        {{ showGuide ? $t('pluginManager.hideGuide') : $t('pluginManager.showGuide') }}
      </button>
      <button class="btn-primary" @click="importPlugin">
        {{ $t('pluginManager.importPlugin') }}
      </button>
    </div>

    <div v-if="showGuide" class="guide-panel">
      <h4 class="guide-title">{{ $t('pluginManager.guideTitle') }}</h4>
      <p class="guide-desc">
        {{ $t('pluginManager.guideDesc') }}
      </p>
      <div class="guide-section">
        <h5 class="guide-subtitle">{{ $t('pluginManager.guideKeyFieldsTitle') }}</h5>
        <ul class="guide-list">
          <li><strong>api.url</strong> — {{ $t('pluginManager.guideApiUrl') }}</li>
          <li><strong>api.headers</strong> — {{ $t('pluginManager.guideApiHeaders') }}</li>
          <li><strong>response</strong> — {{ $t('pluginManager.guideResponse') }}</li>
          <li><strong>display.label</strong> — {{ $t('pluginManager.guideDisplayLabel') }}</li>
        </ul>
      </div>
      <div class="guide-section">
        <h5 class="guide-subtitle">{{ $t('pluginManager.guideSampleTitle') }}</h5>
        <div class="code-block">
          <pre>{{ sampleYaml }}</pre>
          <button class="btn-copy" @click="copySample">{{ copied ? $t('pluginManager.copied') : $t('pluginManager.copy') }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useI18n } from "vue-i18n";
import { invoke } from "@tauri-apps/api/core";
import { open, ask } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import type { HistoryPoint, ProviderState, ProviderConfig } from "../types";
import { getIconUrl } from "../composables/useProviderIcon";
import { useSettings } from "../composables/useSettings";
import TrendChart from "../components/TrendChart.vue";

const { t } = useI18n();
const { settings: globalSettings, loadSettings: loadGlobalSettings } = useSettings();

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
const historyData = ref<Record<string, HistoryPoint[]>>({});
const trendExpanded = ref<Record<string, boolean>>({});
const showGuide = ref(false);
const copied = ref(false);

const trendRange = computed(() => globalSettings.value.trend_range || "24h");

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
  return "";
}

function getCurrencySymbol(currency: ProviderState["currency"]): string {
  if (typeof currency === "string") {
    return currency === "USD" ? "$" : currency === "CNY" ? "¥" : currency === "EUR" ? "€" : "";
  }
  if (typeof currency === "object" && "Custom" in currency) return currency.Custom;
  return "";
}

function cachedBalanceText(provider: ProviderState): string {
  const text = balanceText(provider);
  if (text) return text;
  const data = historyData.value[provider.id];
  if (data && data.length > 0) {
    const lastValue = data[data.length - 1].value;
    const currencySymbol = getCurrencySymbol(provider.currency);
    const fmt = (v: number) => Number.isInteger(v) ? String(v) : v.toFixed(2);
    return `${currencySymbol}${fmt(lastValue)}`;
  }
  return "";
}

async function loadHistory(providerId: string, range: string) {
  try {
    const points = await invoke<HistoryPoint[]>("get_provider_history", {
      providerId,
      range,
    });
    historyData.value[providerId] = points;
  } catch (e) {
    console.error("Failed to load history:", e);
  }
}

function statusText(provider: ProviderState): string {
  if (typeof provider.status === "string") {
    const map: Record<string, string> = {
      Ok: t("pluginManager.statusOk"),
      Fetching: t("pluginManager.statusFetching"),
      Unconfigured: t("pluginManager.statusUnconfigured"),
    };
    return map[provider.status] || provider.status;
  }
  if (typeof provider.status === "object" && "Error" in provider.status) {
    return `✗ ${provider.status.Error}`;
  }
  return t("pluginManager.statusUnknown");
}

function toggleTokenVisibility(id: string) {
  showToken.value[id] = !showToken.value[id];
}

function toggleTrend(id: string) {
  trendExpanded.value[id] = !trendExpanded.value[id];
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
  const confirmed = await ask(t("pluginManager.deleteTokenConfirm"), {
    title: t("pluginManager.deleteTokenTitle"),
    kind: "warning",
    okLabel: t("pluginManager.deleteTokenOk"),
    cancelLabel: t("pluginManager.cancel"),
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
        ? t("pluginManager.pluginOverwritten", { name: config.name })
        : t("pluginManager.pluginImported", { name: config.name })
    );
    await refreshData();
    const newId = `plugin-${config.name.toLowerCase().replace(/ /g, "-")}`;
    const freshProvider = providers.value.find((p) => p.id === newId);
    if (freshProvider?.has_token) {
      await invoke("refresh_provider", { providerId: newId }).catch(() => {});
      await refreshData();
    }
  } catch (e) {
    alert(t("pluginManager.importFailed", { error: e }));
  }
}

async function removePlugin(id: string) {
  const confirmed = await ask(t("pluginManager.removeConfirm", { id }), {
    title: t("pluginManager.removeTitle"),
    kind: "warning",
    okLabel: t("pluginManager.removeOk"),
    cancelLabel: t("pluginManager.cancel"),
  });
  if (!confirmed) return;

  try {
    await invoke("remove_plugin", { providerId: id });
    await refreshData();
  } catch (e) {
    alert(t("pluginManager.removeFailed", { error: e }));
  }
}

async function refreshData() {
  providers.value = await invoke<ProviderState[]>("get_providers");
  for (const p of providers.value) {
    if (p.has_token) {
      await loadHistory(p.id, trendRange.value);
    }
  }
}

let unlisten: (() => void) | null = null;
let unlistenSettings: (() => void) | null = null;

onMounted(async () => {
  await loadGlobalSettings();
  await refreshData();
  unlisten = await listen("providers-updated", () => refreshData());
  unlistenSettings = await listen("settings-updated", async () => {
    await loadGlobalSettings();
  });
});

// Reload history when global trend range changes
watch(trendRange, (newRange) => {
  for (const p of providers.value) {
    if (p.has_token) {
      loadHistory(p.id, newRange);
    }
  }
});

onUnmounted(() => {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
  if (unlistenSettings) {
    unlistenSettings();
    unlistenSettings = null;
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
.provider-actions-right {
  display: flex;
  gap: 4px;
}
.trend-section {
  margin-top: 8px;
}
.trend-collapse {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows 0.25s ease;
}
.trend-collapse.expanded {
  grid-template-rows: 1fr;
}
.trend-collapse > .trend-section {
  overflow: hidden;
  min-height: 0;
}
.btn-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 4px;
  background: none;
  border: 1px solid var(--border-hover);
  border-radius: 4px;
  cursor: pointer;
  color: var(--text-secondary);
}
.btn-icon:hover {
  background: var(--bg-surface-hover);
  color: var(--text-primary);
}
.trend-chevron {
  transition: transform 0.25s ease;
}
.trend-chevron.expanded {
  transform: rotate(180deg);
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
  padding-left: 26px;
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
  justify-content: flex-end;
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
