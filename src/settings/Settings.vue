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
import { ref } from "vue";
import ProviderConfig from "./ProviderConfig.vue";
import PluginManager from "./PluginManager.vue";
import GeneralSettings from "./GeneralSettings.vue";

const activeTab = ref("providers");

const tabs = [
  { id: "providers", label: "提供商配置" },
  { id: "plugins", label: "插件管理" },
  { id: "general", label: "通用" },
];
</script>

<style scoped>
.settings-container {
  width: 100%;
  height: 100%;
  font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Helvetica Neue", sans-serif;
  background: #1e1e1e;
  color: #e0e0e0;
}
.settings-tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  padding: 0 16px;
}
.tab-btn {
  padding: 10px 20px;
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.5);
  font-size: 14px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
}
.tab-btn.active {
  color: #ffffff;
  border-bottom-color: #007aff;
}
.tab-btn:hover {
  color: rgba(255, 255, 255, 0.8);
}
.settings-content {
  padding: 16px;
}
</style>