<template>
  <div class="popover-container">
    <div class="popover-header">
      <h1 class="popover-title">TokenWatch</h1>
      <button
        v-if="!loading"
        class="refresh-btn"
        @click="refreshAll"
        title="刷新"
      >
        ↻
      </button>
      <span v-else class="refresh-btn spinning">↻</span>
    </div>

    <div class="providers-list">
      <ProviderCard
        v-for="provider in providers"
        :key="provider.id"
        :provider="provider"
        @retry="refreshProvider"
        @open-settings="openSettings"
      />

      <div v-if="providers.length === 0" class="empty-state">
        <p>暂无配置厂商</p>
        <button class="btn-primary" @click="openSettings">
          打开设置
        </button>
      </div>
    </div>

    <div class="popover-footer">
      <span class="auto-refresh">
        自动刷新 @ {{ settings.refresh_interval_secs }}s
      </span>
      <button class="settings-btn" @click="openSettings">⚙</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from "vue";
import ProviderCard from "./ProviderCard.vue";
import { useProviders } from "../composables/useProviders";
import { useSettings } from "../composables/useSettings";
import { getAllWebviewWindows } from "@tauri-apps/api/webviewWindow";
import { listen } from "@tauri-apps/api/event";

const {
  providers,
  loading,
  refreshAll,
  refreshProvider,
  startAutoRefresh,
  stopAutoRefresh,
} = useProviders();

const { settings, loadSettings } = useSettings();

async function openSettings() {
  const windows = await getAllWebviewWindows();
  const settingsWindow = windows.find((w: { label: string }) => w.label === "settings");
  if (settingsWindow) {
    await settingsWindow.show();
    await settingsWindow.setFocus();
  }
}

let unlistenSettings: (() => void) | null = null;

onMounted(async () => {
  await loadSettings();
  startAutoRefresh(settings.value.refresh_interval_secs * 1000);

  unlistenSettings = await listen("settings-updated", async () => {
    await loadSettings();
    startAutoRefresh(settings.value.refresh_interval_secs * 1000);
  });
});

onUnmounted(() => {
  stopAutoRefresh();
  if (unlistenSettings) {
    unlistenSettings();
    unlistenSettings = null;
  }
});
</script>

<style scoped>
.popover-container {
  width: 100%;
  background: rgba(40, 40, 40, 0.96);
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 10px;
  color: #e0e0e0;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", sans-serif;
  -webkit-font-smoothing: antialiased;
  padding: 14px;
  box-sizing: border-box;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.4);
}
.popover-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  user-select: none;
}
.popover-title {
  font-size: 15px;
  font-weight: 700;
  color: #ffffff;
  margin: 0;
  letter-spacing: -0.2px;
}
.refresh-btn {
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.5);
  font-size: 16px;
  cursor: pointer;
  padding: 2px 4px;
  line-height: 1;
  transition: color 0.15s ease;
}
.refresh-btn:hover {
  color: #ffffff;
}
.refresh-btn.spinning {
  animation: spin 0.8s linear infinite;
}
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
.providers-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.empty-state {
  text-align: center;
  padding: 20px 0;
  color: rgba(255, 255, 255, 0.35);
  font-size: 13px;
}
.empty-state .btn-primary {
  margin-top: 10px;
}
.btn-primary {
  padding: 5px 14px;
  background: #007aff;
  color: white;
  border: none;
  border-radius: 5px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  transition: background 0.15s ease;
}
.btn-primary:hover {
  background: #3395ff;
}
.popover-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 6px;
  padding-top: 6px;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
  user-select: none;
}
.auto-refresh {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.3);
}
.settings-btn {
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.35);
  font-size: 18px;
  cursor: pointer;
  padding: 4px 6px;
  line-height: 1;
  transition: color 0.15s ease;
}
.settings-btn:hover {
  color: rgba(255, 255, 255, 0.8);
}
</style>