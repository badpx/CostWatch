import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ProviderState } from "../types";

const providers = ref<ProviderState[]>([]);
let refreshInterval: ReturnType<typeof setInterval> | null = null;
let unlisten: (() => void) | null = null;

export function useProviders() {
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function fetchProviders() {
    try {
      providers.value = await invoke<ProviderState[]>("get_providers");
    } catch (e) {
      error.value = String(e);
    }
  }

  async function refreshAll() {
    loading.value = true;
    error.value = null;
    try {
      await invoke("refresh_all");
      await fetchProviders();
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function refreshProvider(id: string) {
    try {
      const updated = await invoke<ProviderState>("refresh_provider", {
        providerId: id,
      });
      const index = providers.value.findIndex((p) => p.id === id);
      if (index >= 0) {
        providers.value[index] = updated;
      }
    } catch (e) {
      error.value = String(e);
    }
  }

  function startAutoRefresh(intervalMs: number) {
    stopAutoRefresh();
    refreshAll();
    refreshInterval = setInterval(() => refreshAll(), intervalMs);
  }

  function stopAutoRefresh() {
    if (refreshInterval) {
      clearInterval(refreshInterval);
      refreshInterval = null;
    }
    if (unlisten) {
      unlisten();
      unlisten = null;
    }
  }

  async function listenRefreshEvent() {
    unlisten = await listen("refresh-triggered", () => {
      refreshAll();
    });
  }

  return {
    providers,
    loading,
    error,
    fetchProviders,
    refreshAll,
    refreshProvider,
    startAutoRefresh,
    stopAutoRefresh,
    listenRefreshEvent,
  };
}