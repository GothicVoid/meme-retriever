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
        v-model="settings.showDebugInfo"
        type="checkbox"
      >
      显示调试信息
    </label>
    <div class="reindex-desc">
      开启后，搜索结果每张图片显示得分详情和计算公式
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

    <div class="clear-gallery-section">
      <div class="reindex-desc">
        清空图库将删除所有图片及其索引数据，此操作不可撤销。
      </div>
      <button
        data-action="clear-gallery"
        class="btn-danger"
        :disabled="libraryStore.images.length === 0 || clearingGallery || libraryStore.indexing"
        @click="handleClearGallery"
      >
        {{ clearingGallery
          ? `正在清空图库（${libraryStore.clearCurrent}/${libraryStore.clearTotal}）`
          : "清空图库" }}
      </button>
      <div
        v-if="clearingGallery"
        class="progress-bar"
      >
        <div
          class="progress-fill danger"
          :style="{ width: clearProgressPercent + '%' }"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { confirm } from "@tauri-apps/plugin-dialog";
import { useLibraryStore } from "@/stores/library";
import { useSettingsStore } from "@/stores/settings";

const settings = useSettingsStore();
const libraryStore = useLibraryStore();

const reindexing = ref(false);
const reindexCurrent = ref(0);
const reindexTotal = ref(0);
const reindexDone = ref(false);

const progressPercent = computed(() =>
  reindexTotal.value > 0 ? (reindexCurrent.value / reindexTotal.value) * 100 : 0
);
const clearingGallery = computed(() => libraryStore.clearing);
const clearProgressPercent = computed(() =>
  libraryStore.clearTotal > 0 ? (libraryStore.clearCurrent / libraryStore.clearTotal) * 100 : 0
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

async function handleClearGallery() {
  const ok = await confirm("确认清空整个图库吗？此操作会删除所有图片及索引数据，且不可撤销。", {
    title: "清空图库",
  });
  if (!ok) return;
  await libraryStore.clearGallery();
}

onUnmounted(() => { unlisten?.(); });
</script>

<style scoped>
.settings-view { padding: 1rem; display: flex; flex-direction: column; gap: 1rem; }
label { display: flex; flex-direction: column; gap: 0.25rem; }
.reindex-section { display: flex; flex-direction: column; gap: 0.5rem; }
.clear-gallery-section { display: flex; flex-direction: column; gap: 0.5rem; }
.reindex-desc { font-size: 0.875rem; color: #666; }
.progress-bar { height: 6px; background: #e0e0e0; border-radius: 3px; overflow: hidden; }
.progress-fill { height: 100%; background: #646cff; transition: width 0.3s; }
.progress-fill.danger { background: #ff4d4f; }
.done-msg { font-size: 0.875rem; color: #4caf50; }
.btn-danger {
  background: #ff4d4f;
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 0.45rem 1.1rem;
  cursor: pointer;
  font-size: 0.9rem;
  align-self: flex-start;
}
.btn-danger:hover:not(:disabled) { background: #ff7875; }
.btn-danger:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
