<template>
  <div class="general-settings">
    <h3 class="section-title">通用设置</h3>

    <div class="setting-item">
      <label class="setting-label">刷新间隔</label>
      <select v-model="localSettings.refresh_interval_secs" @change="save" class="setting-select">
        <option :value="60">1 分钟</option>
        <option :value="120">2 分钟</option>
        <option :value="180">3 分钟</option>
        <option :value="300">5 分钟</option>
        <option :value="600">10 分钟</option>
        <option :value="1800">30 分钟</option>
      </select>
    </div>

    <div class="setting-item">
      <label class="setting-label">开机自启动</label>
      <input
        type="checkbox"
        v-model="localSettings.launch_at_login"
        @change="save"
        class="setting-checkbox"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { GeneralSettings } from "../types";

const localSettings = ref<GeneralSettings>({
  refresh_interval_secs: 180,
  launch_at_login: false,
});

async function loadSettings() {
  localSettings.value = await invoke<GeneralSettings>("get_settings");
}

async function save() {
  await invoke("save_settings", { settings: localSettings.value });
}

onMounted(loadSettings);
</script>

<style scoped>
.general-settings {
  max-width: 480px;
  margin: 0 auto;
}
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 16px;
}
.setting-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid var(--border);
}
.setting-label {
  font-size: 14px;
  color: var(--text-primary);
}
.setting-select {
  padding: 6px 12px;
  background: var(--bg-input);
  border: 1px solid var(--border-strong);
  border-radius: 4px;
  color: var(--text-primary);
  font-size: 13px;
}
.setting-checkbox {
  width: 18px;
  height: 18px;
}
</style>
