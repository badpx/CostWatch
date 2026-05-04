<template>
  <div class="provider-card" :class="{ error: isError, unconfigured: isUnconfigured }">
    <div class="provider-header">
      <span class="provider-name">
        <img
          v-if="iconKey"
          class="provider-icon"
          :src="iconSrc"
          @error="onIconError"
          alt=""
        />
        {{ provider.name }}
      </span>
      <span class="provider-status">
        <template v-if="statusLabel === 'Ok'">
          <span class="status-dot ok"></span>
        </template>
        <template v-else-if="statusLabel === 'Fetching'">
          <span class="status-dot fetching"></span>
        </template>
        <template v-else>
          <span class="status-dot error"></span>
        </template>
      </span>
    </div>

    <template v-if="isOk">
      <div class="provider-balance">
        <span class="balance-label">{{ displayLabel }}</span>
      </div>
      <div v-if="provider.has_progress" class="progress-bar">
        <div
          class="progress-fill"
          :style="{ width: progressPercent + '%' }"
          :class="progressDirection"
        ></div>
      </div>
      <div class="provider-meta">
        <span v-if="provider.last_updated" class="last-updated">
          {{ timeAgo }}
        </span>
      </div>
    </template>

    <template v-else-if="isUnconfigured">
      <div class="provider-message">未配置 Token</div>
      <button class="btn-small" @click="$emit('openSettings')">配置</button>
    </template>

    <template v-else-if="isError">
      <div class="provider-message error">{{ errorMessage }}</div>
      <button class="btn-small" @click="$emit('retry', provider.id)">重试</button>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, watch } from "vue";
import type { ProviderState } from "../types";

const props = defineProps<{
  provider: ProviderState;
}>();

defineEmits<{
  openSettings: [];
  retry: [id: string];
}>();

function getIconKey(icon: ProviderState["icon"]): string {
  if (typeof icon === "object" && "Builtin" in icon) return icon.Builtin;
  if (typeof icon === "object" && "Custom" in icon) return icon.Custom;
  return "";
}

const iconKey = computed(() => getIconKey(props.provider.icon));

const lobehubMap: Record<string, string> = {
  openrouter: "OpenRouter",
  deepseek: "DeepSeek",
  kimi: "Moonshot",
};

function getLobeHubUrl(key: string): string {
  const name = lobehubMap[key];
  if (!name) return "";
  return `https://unpkg.com/@lobehub/icons-static-svg/icons/${name}.svg`;
}

const iconSrc = ref("");

watch(
  iconKey,
  (key) => {
    const lobehub = getLobeHubUrl(key);
    iconSrc.value = lobehub || `/icons/${key}.svg`;
  },
  { immediate: true }
);

function onIconError(event: Event) {
  const img = event.target as HTMLImageElement;
  const key = iconKey.value;
  if (img.src.startsWith("https://unpkg.com")) {
    img.src = `/icons/${key}.svg`;
  } else if (img.src.endsWith(".svg")) {
    img.src = img.src.replace(/\.svg$/, ".png");
  }
}

const statusLabel = computed(() => {
  if (typeof props.provider.status === "string") return props.provider.status;
  if (typeof props.provider.status === "object" && "Error" in props.provider.status)
    return "Error";
  return "Unknown";
});

const isOk = computed(() => statusLabel.value === "Ok");
const isError = computed(() => statusLabel.value === "Error");
const isUnconfigured = computed(() => statusLabel.value === "Unconfigured");

const errorMessage = computed(() => {
  if (typeof props.provider.status === "object" && "Error" in props.provider.status) {
    return props.provider.status.Error;
  }
  return props.provider.error_message || "未知错误";
});

const displayLabel = computed(() => {
  const p = props.provider;
  if (p.display_label) return p.display_label;

  const currencySymbol = getCurrencySymbol(p.currency);
  const fmt = (v: number) => Number.isInteger(v) ? String(v) : v.toFixed(2);
  const balance = p.balance != null ? `${currencySymbol}${fmt(Number(p.balance))}` : "";
  const available = p.available != null ? `${currencySymbol}${fmt(Number(p.available))}` : "";

  if (balance && available) return `${balance} / ${available}`;
  if (balance) return balance;
  if (available) return available;
  if (p.is_available != null) return p.is_available ? "可用" : "不可用";
  return "—";
});

const progressPercent = computed(() => {
  const p = props.provider;
  if (p.used == null || p.available == null) return 0;
  if (p.available === 0) return 0;
  const pct = (Number(p.used) / Number(p.available)) * 100;
  return Math.min(Math.max(pct, 0), 100);
});

const progressDirection = computed(() => "consumption");

const now = ref(Date.now());
let timer: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  timer = setInterval(() => { now.value = Date.now(); }, 60000);
});

onUnmounted(() => {
  if (timer) { clearInterval(timer); timer = null; }
});

const timeAgo = computed(() => {
  const dateStr = props.provider.last_updated;
  if (!dateStr) return "";
  const date = new Date(dateStr);
  const seconds = Math.floor((now.value - date.getTime()) / 1000);
  if (seconds < 60) return "";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}分钟前`;
  const hours = Math.floor(minutes / 60);
  return `${hours}小时前`;
});

function getCurrencySymbol(currency: ProviderState["currency"]): string {
  if (typeof currency === "string") {
    return currency === "USD" ? "$" : currency === "CNY" ? "¥" : currency === "EUR" ? "€" : "";
  }
  if (typeof currency === "object" && "Custom" in currency) return currency.Custom;
  return "";
}
</script>

<style scoped>
.provider-card {
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.04);
  border-radius: 8px;
  padding: 8px 12px;
  transition: background 0.15s ease;
}
.provider-card:hover {
  background: rgba(255, 255, 255, 0.06);
}
.provider-card.error {
  border-color: rgba(255, 59, 48, 0.25);
}
.provider-card.unconfigured {
  opacity: 0.55;
}
.provider-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}
.provider-name {
  font-weight: 600;
  font-size: 13px;
  color: #f0f0f0;
  letter-spacing: -0.1px;
  display: flex;
  align-items: center;
  gap: 6px;
}
.provider-icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  object-fit: contain;
  filter: brightness(0.8);
}
.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  display: inline-block;
}
.status-dot.ok {
  background: #30d158;
}
.status-dot.fetching {
  background: #ff9f0a;
  animation: pulse 1.2s ease-in-out infinite;
}
.status-dot.error {
  background: #ff453a;
}
@keyframes pulse {
  0% { opacity: 1; }
  50% { opacity: 0.35; }
  100% { opacity: 1; }
}
.provider-balance {
  font-size: 17px;
  font-weight: 700;
  color: #ffffff;
  margin-bottom: 6px;
  letter-spacing: -0.3px;
}
.progress-bar {
  height: 4px;
  background: rgba(255, 255, 255, 0.08);
  border-radius: 2px;
  overflow: hidden;
}
.progress-fill {
  height: 100%;
  border-radius: 2px;
  transition: width 0.3s ease;
}
.progress-fill.consumption {
  background: #30d158;
}
.provider-meta {
  margin-top: 5px;
  display: flex;
  justify-content: flex-end;
}
.last-updated {
  font-size: 10px;
  color: rgba(255, 255, 255, 0.3);
}
.provider-message {
  color: rgba(255, 255, 255, 0.5);
  font-size: 12px;
  margin: 5px 0 7px;
}
.provider-message.error {
  color: #ff453a;
  font-size: 11px;
}
.btn-small {
  padding: 3px 10px;
  font-size: 11px;
  font-weight: 500;
  background: rgba(255, 255, 255, 0.08);
  color: rgba(255, 255, 255, 0.75);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s ease;
}
.btn-small:hover {
  background: rgba(255, 255, 255, 0.14);
  border-color: rgba(255, 255, 255, 0.2);
  color: #ffffff;
}
</style>