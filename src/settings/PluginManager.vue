<template>
  <div class="plugin-manager">
    <h3 class="section-title">插件提供商</h3>

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
      <p>暂无插件提供商</p>
    </div>

    <div class="import-section">
      <button class="btn-primary" @click="importPlugin">
        + 导入插件配置文件
      </button>
      <button class="btn-link" @click="showGuide = !showGuide">
        {{ showGuide ? '收起说明' : '查看示例配置' }}
      </button>
    </div>

    <div v-if="showGuide" class="guide-panel">
      <h4 class="guide-title">插件配置格式说明</h4>
      <p class="guide-desc">
        插件是一个 YAML 文件，定义了如何从 Provider API 获取用量数据并展示。
      </p>
      <div class="guide-section">
        <h5 class="guide-subtitle">关键字段说明</h5>
        <ul class="guide-list">
          <li><strong>api.url</strong> — 查询余额的 API 地址</li>
          <li><strong>api.headers</strong> — 请求头，使用 <code v-pre>{{token}}</code> 占位符表示用户填写的 Token</li>
          <li><strong>response</strong> — 定义如何从 JSON 响应中提取字段</li>
          <li><strong>display.label</strong> — 展示文案，支持 <code v-pre>{{字段名}}</code> 和 <code v-pre>{{currency_unit}}</code> 占位符</li>
        </ul>
      </div>
      <div class="guide-section">
        <h5 class="guide-subtitle">示例配置</h5>
        <div class="code-block">
          <pre>{{ sampleYaml }}</pre>
          <button class="btn-copy" @click="copySample">{{ copied ? '已复制' : '复制' }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { ProviderState, ProviderConfig } from "../types";

const providers = ref<ProviderState[]>([]);
const showGuide = ref(false);
const copied = ref(false);

const sampleYaml = `name: MyProvider
icon: myprovider
api:
  url: https://api.example.com/balance
  method: GET
  headers:
    Authorization: "Bearer {{token}}"
response:
  balance:
    path: "$.data.balance"
    type: number
  currency:
    value: "USD"
    type: string
display:
  primary: balance
  label: "{{currency_unit}}{{balance}}"
  unit_prefix: "$"`;

async function copySample() {
  await navigator.clipboard.writeText(sampleYaml);
  copied.value = true;
  setTimeout(() => (copied.value = false), 2000);
}

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
  display: flex;
  gap: 12px;
  align-items: center;
}
.btn-link {
  background: none;
  border: none;
  color: #007aff;
  cursor: pointer;
  font-size: 13px;
  padding: 0;
}
.btn-link:hover {
  text-decoration: underline;
}
.guide-panel {
  margin-top: 16px;
  padding: 16px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
}
.guide-title {
  font-size: 14px;
  font-weight: 600;
  color: #e0e0e0;
  margin: 0 0 8px;
}
.guide-desc {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.5);
  margin: 0 0 12px;
  line-height: 1.5;
}
.guide-section {
  margin-bottom: 12px;
}
.guide-subtitle {
  font-size: 12px;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.6);
  margin: 0 0 6px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.guide-list {
  font-size: 12px;
  color: rgba(255, 255, 255, 0.5);
  line-height: 1.6;
  padding-left: 16px;
  margin: 0;
}
.guide-list li {
  margin-bottom: 4px;
}
.guide-list code {
  background: rgba(255, 255, 255, 0.1);
  padding: 1px 4px;
  border-radius: 3px;
  font-size: 11px;
  color: #e0e0e0;
}
.code-block {
  position: relative;
  background: rgba(0, 0, 0, 0.3);
  border-radius: 6px;
  padding: 12px;
  overflow-x: auto;
}
.code-block pre {
  margin: 0;
  font-size: 12px;
  font-family: "SF Mono", "Fira Code", "JetBrains Mono", monospace;
  color: #e0e0e0;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-all;
}
.btn-copy {
  position: absolute;
  top: 8px;
  right: 8px;
  padding: 3px 10px;
  background: rgba(255, 255, 255, 0.1);
  border: 1px solid rgba(255, 255, 255, 0.15);
  color: rgba(255, 255, 255, 0.7);
  border-radius: 4px;
  cursor: pointer;
  font-size: 11px;
}
.btn-copy:hover {
  background: rgba(255, 255, 255, 0.2);
}
</style>