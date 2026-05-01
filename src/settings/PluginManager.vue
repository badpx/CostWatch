<template>
  <div class="plugin-manager">
    <h3 class="section-title">插件厂商</h3>

    <div
      v-for="provider in pluginProviders"
      :key="provider.id"
      class="plugin-item"
    >
      <div class="plugin-info">
        <span class="plugin-name">{{ provider.name }}</span>
        <span class="plugin-id">{{ provider.id }}</span>
      </div>
      <div class="plugin-actions">
        <button class="btn-danger" @click="removePlugin(provider.id)">
          移除插件
        </button>
      </div>
    </div>

    <div v-if="pluginProviders.length === 0" class="empty-state">
      <p>暂无插件厂商</p>
    </div>

    <div class="import-section">
      <button class="btn-primary" @click="importPlugin">
        + 导入插件配置文件
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProviderState, ProviderConfig } from "../types";

const providers = ref<ProviderState[]>([]);

const pluginProviders = computed(() =>
  providers.value.filter((p) => !p.is_builtin)
);

async function importPlugin() {
  const selected = await open({
    multiple: false,
    filters: [{ name: "YAML", extensions: ["yaml", "yml"] }],
  });

  if (!selected) return;

  try {
    const config = await invoke<ProviderConfig>("import_plugin", {
      yamlPath: selected,
    });
    alert(`成功导入插件: ${config.name}`);
    await refreshData();
  } catch (e) {
    alert(`导入失败: ${e}`);
  }
}

async function removePlugin(id: string) {
  if (!confirm(`确定要移除插件 ${id} 吗？`)) return;

  try {
    await invoke("remove_plugin", { providerId: id });
    await refreshData();
  } catch (e) {
    alert(`移除失败: ${e}`);
  }
}

async function refreshData() {
  providers.value = await invoke<ProviderState[]>("get_providers");
}

onMounted(refreshData);
</script>

<style scoped>
.plugin-manager {
  max-width: 560px;
}
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.6);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-bottom: 12px;
}
.plugin-item {
  background: rgba(255, 255, 255, 0.05);
  border-radius: 8px;
  padding: 12px;
  margin-bottom: 8px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.plugin-info {
  display: flex;
  flex-direction: column;
}
.plugin-name {
  font-weight: 600;
  font-size: 14px;
}
.plugin-id {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.4);
  font-family: monospace;
}
.btn-primary {
  padding: 6px 14px;
  background: #007aff;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
}
.btn-danger {
  padding: 4px 10px;
  background: none;
  border: 1px solid rgba(255, 59, 48, 0.3);
  color: #ff3b30;
  border-radius: 4px;
  cursor: pointer;
  font-size: 12px;
}
.empty-state {
  color: rgba(255, 255, 255, 0.4);
  font-size: 13px;
  text-align: center;
  padding: 16px;
}
.import-section {
  margin-top: 16px;
}
</style>