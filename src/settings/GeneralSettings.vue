<template>
  <div class="general-settings">
    <h3 class="section-title">通用设置</h3>

    <div class="setting-item">
      <label class="setting-label">刷新间隔</label>
      <select v-model="localSettings.refresh_interval_secs" class="setting-select">
        <option :value="30">30 秒</option>
        <option :value="60">60 秒</option>
        <option :value="120">2 分钟</option>
        <option :value="300">5 分钟</option>
      </select>
    </div>

    <div class="setting-item">
      <label class="setting-label">开机自启动</label>
      <input
        type="checkbox"
        v-model="localSettings.launch_at_login"
        class="setting-checkbox"
      />
    </div>

    <button class="btn-primary" @click="save">保存设置</button>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { GeneralSettings } from "../types";

const localSettings = ref<GeneralSettings>({
  refresh_interval_secs: 60,
  launch_at_login: false,
});

async function loadSettings() {
  localSettings.value = await invoke<GeneralSettings>("get_settings");
}

async function save() {
  await invoke("save_settings", { settings: localSettings.value });
  alert("设置已保存");
}

onMounted(loadSettings);
</script>

<style scoped>
.general-settings {
  max-width: 400px;
}
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.6);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 16px;
}
.setting-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}
.setting-label {
  font-size: 14px;
  color: #e0e0e0;
}
.setting-select {
  padding: 6px 12px;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 4px;
  color: #e0e0e0;
  font-size: 13px;
}
.setting-checkbox {
  width: 18px;
  height: 18px;
}
.btn-primary {
  margin-top: 16px;
  padding: 8px 20px;
  background: #007aff;
  color: white;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
}
</style>