<template>
  <div class="settings-container">
    <div class="settings-tabs">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        :class="['tab-btn', { active: activeTab === tab.id }]"
        @click="activeTab = tab.id"
      >
        {{ tab.label }}
      </button>
    </div>
    <div class="settings-content">
      <ProviderConfig v-if="activeTab === 'providers'" />
      <PluginManager v-else-if="activeTab === 'plugins'" />
      <GeneralSettings v-else-if="activeTab === 'general'" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, provide } from "vue";
import { useI18n } from "vue-i18n";
import ProviderConfig from "./ProviderConfig.vue";
import PluginManager from "./PluginManager.vue";
import GeneralSettings from "./GeneralSettings.vue";
import { listen } from "@tauri-apps/api/event";

const { t } = useI18n();

const activeTab = ref("providers");
const navigateProviderId = ref<string | null>(null);
provide("navigateProviderId", navigateProviderId);

const tabs = [
  { id: "providers", label: computed(() => t("settings.tabs.providers")) },
  { id: "plugins", label: computed(() => t("settings.tabs.plugins")) },
  { id: "general", label: computed(() => t("settings.tabs.general")) },
];

let unlisten: (() => void) | null = null;

onMounted(() => {
  listen<{ tab: string; providerId?: string | null }>("navigate-to-tab", (event) => {
    if (tabs.some((t) => t.id === event.payload.tab)) {
      activeTab.value = event.payload.tab;
    }
    navigateProviderId.value = event.payload.providerId ?? null;
  }).then((fn) => {
    unlisten = fn;
  });
});

onUnmounted(() => {
  if (unlisten) unlisten();
});
</script>

<style scoped>
.settings-container {
  width: 100%;
  height: 100%;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", sans-serif;
  background: var(--bg-app);
  color: var(--text-primary);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.settings-tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--border-strong);
  padding: 0 16px;
  flex-shrink: 0;
}
.tab-btn {
  padding: 10px 20px;
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 14px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
}
.tab-btn.active {
  color: var(--text-heading);
  border-bottom-color: var(--accent);
}
.tab-btn:hover {
  color: var(--text-primary);
}
.settings-content {
  padding: 16px;
  flex: 1;
  overflow-y: auto;
  min-height: 0;
}
</style>