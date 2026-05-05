<template>
  <div class="general-settings">
    <h3 class="section-title">{{ $t('generalSettings.title') }}</h3>

    <div class="setting-item">
      <label class="setting-label">{{ $t('generalSettings.language') }}</label>
      <select v-model="localSettings.language" @change="onLanguageChange" class="setting-select">
        <option value="zh-CN">{{ $t('generalSettings.languageZh') }}</option>
        <option value="en">{{ $t('generalSettings.languageEn') }}</option>
      </select>
    </div>

    <div class="setting-item">
      <label class="setting-label">{{ $t('generalSettings.launchAtLogin') }}</label>
      <input
        type="checkbox"
        v-model="localSettings.launch_at_login"
        @change="save"
        class="setting-checkbox"
      />
    </div>

    <div class="setting-item">
      <label class="setting-label">{{ $t('generalSettings.refreshInterval') }}</label>
      <select v-model="localSettings.refresh_interval_secs" @change="save" class="setting-select">
        <option :value="60">{{ $t('generalSettings.minute_1') }}</option>
        <option :value="120">{{ $t('generalSettings.minute_2') }}</option>
        <option :value="180">{{ $t('generalSettings.minute_3') }}</option>
        <option :value="300">{{ $t('generalSettings.minute_5') }}</option>
        <option :value="600">{{ $t('generalSettings.minute_10') }}</option>
        <option :value="1800">{{ $t('generalSettings.minute_30') }}</option>
      </select>
    </div>

    <div class="setting-item">
      <label class="setting-label">{{ $t('generalSettings.trendRange') }}</label>
      <select v-model="localSettings.trend_range" @change="save" class="setting-select">
        <option value="24h">{{ $t('generalSettings.trendRange24h') }}</option>
        <option value="1w">{{ $t('generalSettings.trendRange1w') }}</option>
        <option value="1m">{{ $t('generalSettings.trendRange1m') }}</option>
      </select>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { GeneralSettings } from "../types";
import i18n from "../i18n";

const localSettings = ref<GeneralSettings>({
  refresh_interval_secs: 180,
  launch_at_login: false,
  language: "zh-CN",
  trend_range: "24h",
});

async function loadSettings() {
  localSettings.value = await invoke<GeneralSettings>("get_settings");
}

async function save() {
  await invoke("save_settings", { settings: localSettings.value });
}

async function onLanguageChange() {
  const locale = localSettings.value.language as "zh-CN" | "en";
  (i18n.global.locale as any).value = locale;
  await invoke("save_settings", { settings: localSettings.value });
}

onMounted(loadSettings);
</script>

<style scoped>
.general-settings {
  max-width: 600px;
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
