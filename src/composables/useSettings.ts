import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { GeneralSettings } from "../types";

export function useSettings() {
  const settings = ref<GeneralSettings>({
    refresh_interval_secs: 180,
    launch_at_login: false,
  });

  async function loadSettings() {
    try {
      settings.value = await invoke<GeneralSettings>("get_settings");
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
  }

  async function saveSettings(newSettings?: GeneralSettings) {
    const toSave = newSettings || settings.value;
    try {
      await invoke("save_settings", { settings: toSave });
      settings.value = toSave;
    } catch (e) {
      console.error("Failed to save settings:", e);
    }
  }

  return {
    settings,
    loadSettings,
    saveSettings,
  };
}
