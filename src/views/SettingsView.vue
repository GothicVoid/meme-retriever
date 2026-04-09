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
    <label>
      <input
        v-model="settings.showDebugInfo"
        type="checkbox"
      >
      显示调试信息
    </label>
    <div class="reindex-desc">
      开启后，搜索结果每张图片显示得分详情和计算公式
    </div>

    <div class="weights-section">
      <div class="section-title">搜索权重调节</div>
      <div class="weight-row">
        <span class="weight-label">关键词 (w1)</span>
        <input
          v-model.number="settings.w1"
          type="range"
          min="0"
          max="1"
          step="0.05"
          class="weight-slider"
        >
        <span class="weight-val">{{ settings.normalizedWeights.w1.toFixed(2) }}</span>
      </div>
      <div class="weight-row">
        <span class="weight-label">OCR (w2)</span>
        <input
          v-model.number="settings.w2"
          type="range"
          min="0"
          max="1"
          step="0.05"
          class="weight-slider"
        >
        <span class="weight-val">{{ settings.normalizedWeights.w2.toFixed(2) }}</span>
      </div>
      <div class="weight-row">
        <span class="weight-label">CLIP (w3)</span>
        <input
          v-model.number="settings.w3"
          type="range"
          min="0"
          max="1"
          step="0.05"
          class="weight-slider"
        >
        <span class="weight-val">{{ settings.normalizedWeights.w3.toFixed(2) }}</span>
      </div>
      <div class="reindex-desc">
        显示值为归一化后的权重（三者之和=1）
      </div>
    </div>

    <div class="reindex-section">
      <div class="reindex-desc">
        重新生成图像索引（更新模型或首次使用时需要）
      </div>
      <button
        :disabled="reindexing"
        @click="startReindex"
      >
        {{ reindexing ? `重新索引中… (${reindexCurrent}/${reindexTotal})` : '重新生成图像索引' }}
      </button>
      <div
        v-if="reindexing"
        class="progress-bar"
      >
        <div
          class="progress-fill"
          :style="{ width: progressPercent + '%' }"
        />
      </div>
      <div
        v-if="reindexDone"
        class="done-msg"
      >
        索引已更新完毕 ✓
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSettingsStore } from "@/stores/settings";

const settings = useSettingsStore();

const reindexing = ref(false);
const reindexCurrent = ref(0);
const reindexTotal = ref(0);
const reindexDone = ref(false);

const progressPercent = computed(() =>
  reindexTotal.value > 0 ? (reindexCurrent.value / reindexTotal.value) * 100 : 0
);

let unlisten: (() => void) | null = null;

async function startReindex() {
  reindexing.value = true;
  reindexDone.value = false;
  reindexCurrent.value = 0;
  reindexTotal.value = 0;

  unlisten = await listen<{ current: number; total: number }>("reindex-progress", (event) => {
    reindexCurrent.value = event.payload.current;
    reindexTotal.value = event.payload.total;
    if (event.payload.current >= event.payload.total && event.payload.total > 0) {
      reindexing.value = false;
      reindexDone.value = true;
      unlisten?.();
      unlisten = null;
    }
  });

  try {
    await invoke("reindex_all");
  } catch (e) {
    console.error("reindex_all failed:", e);
    reindexing.value = false;
    unlisten?.();
    unlisten = null;
  }
}

onUnmounted(() => { unlisten?.(); });
</script>

<style scoped>
.settings-view { padding: 1rem; display: flex; flex-direction: column; gap: 1rem; }
label { display: flex; flex-direction: column; gap: 0.25rem; }
.reindex-section { display: flex; flex-direction: column; gap: 0.5rem; }
.reindex-desc { font-size: 0.875rem; color: #666; }
.progress-bar { height: 6px; background: #e0e0e0; border-radius: 3px; overflow: hidden; }
.progress-fill { height: 100%; background: #646cff; transition: width 0.3s; }
.done-msg { font-size: 0.875rem; color: #4caf50; }

.weights-section { display: flex; flex-direction: column; gap: 0.5rem; }
.section-title { font-weight: 600; font-size: 0.95rem; }
.weight-row { display: flex; align-items: center; gap: 0.75rem; }
.weight-label { width: 90px; font-size: 0.875rem; }
.weight-slider { flex: 1; accent-color: #646cff; }
.weight-val { width: 36px; text-align: right; font-size: 0.875rem; color: #555; font-variant-numeric: tabular-nums; }
</style>
