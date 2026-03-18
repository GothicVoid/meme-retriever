<template>
  <div class="settings-view">
    <h2>设置</h2>
    <label>
      默认展示数量
      <input
        v-model.number="settings.defaultLimit"
        type="number"
        min="9"
        max="21"
        step="3"
      >
    </label>
    <label>
      <input
        v-model="settings.deleteOriginalFile"
        type="checkbox"
      >
      删除时同时删除原文件
    </label>
    <label>
      图库路径
      <input
        v-model="settings.libraryPath"
        type="text"
        placeholder="留空使用默认路径"
      >
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
