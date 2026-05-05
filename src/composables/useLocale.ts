import { ref } from "vue";
import i18n from "../i18n";
import { invoke } from "@tauri-apps/api/core";
import type { GeneralSettings } from "../types";

export type AppLocale = "zh-CN" | "en";

const currentLocale = ref<AppLocale>("zh-CN");

/**
 * Detects initial locale from backend settings and applies it.
 * Called once at app startup.
 */
export async function initLocale() {
  try {
    const settings = await invoke<GeneralSettings>("get_settings");
    const lang = settings.language as AppLocale;
    if (lang && (lang === "zh-CN" || lang === "en")) {
      currentLocale.value = lang;
      (i18n.global.locale as any).value = lang;
    }
  } catch {
    // keep default zh-CN
  }
}

export async function setLocale(locale: AppLocale) {
  currentLocale.value = locale;
  (i18n.global.locale as any).value = locale;
  try {
    const settings = await invoke<GeneralSettings>("get_settings");
    await invoke("save_settings", {
      settings: { ...settings, language: locale },
    });
  } catch {
    // locale still applied in-memory
  }
}

export function useLocale() {
  return {
    currentLocale,
    initLocale,
    setLocale,
  };
}
