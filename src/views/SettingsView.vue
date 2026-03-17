<template>
  <div class="settings-view">
    <h2>设置</h2>
    <label>
      默认展示数量
      <input type="number" v-model.number="settings.defaultLimit" min="9" max="21" step="3" />
    </label>
    <label>
      <input type="checkbox" v-model="settings.deleteOriginalFile" />
      删除时同时删除原文件
    </label>
    <label>
      图库路径
      <input type="text" v-model="settings.libraryPath" placeholder="留空使用默认路径" />
    </label>
  </div>
</template>

<script setup lang="ts">
import { useSettingsStore } from "@/stores/settings";
const settings = useSettingsStore();
</script>

<style scoped>
.settings-view { padding: 1rem; display: flex; flex-direction: column; gap: 1rem; }
label { display: flex; flex-direction: column; gap: 0.25rem; }
</style>
