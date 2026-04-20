<template>
  <div class="settings-view">
    <h2>设置</h2>
    <section class="settings-section">
      <div class="section-head">
        <h3>基础设置</h3>
        <p>保留少量默认行为配置，不要求用户理解底层搜索原理。</p>
      </div>
      <div class="window-preference">
        <div class="window-preference__group">
          <span class="window-preference__label">启动工作态</span>
          <div class="window-preference__options">
            <label class="window-option">
              <input
                v-model="settings.startupWindowMode"
                type="radio"
                value="sidebar"
              >
              侧边栏找图
            </label>
            <label class="window-option">
              <input
                v-model="settings.startupWindowMode"
                type="radio"
                value="expanded"
              >
              展开工作台
            </label>
          </div>
        </div>
      </div>
    </section>

    <section class="settings-section">
      <div class="section-head">
        <h3>高级能力</h3>
        <p>面向少量高级维护场景，默认不影响普通用户日常找图流程。</p>
      </div>

      <div
        v-if="isDev"
        class="advanced-entry"
      >
        <div class="advanced-entry__copy">
          <h4>私有角色库</h4>
          <p>适合维护少量冷门角色或私有对象，帮助稳定召回无字图和特殊对象图片。</p>
        </div>
        <button
          data-action="open-private-role-library"
          class="btn-secondary"
          @click="openPrivateRoleLibrary"
        >
          打开维护工具
        </button>
      </div>
    </section>

    <section class="settings-section">
      <div class="section-head">
        <h3>开发调试</h3>
        <p>仅用于排查问题和维护图库，不作为普通用户日常操作入口。</p>
      </div>

      <label>
        <input
          v-model="settings.devDebugMode"
          type="checkbox"
        >
        开发调试模式
      </label>
      <div class="reindex-desc">
        开启后，搜索结果会额外显示底层排序构成，仅用于排查问题
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
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { confirm } from "@tauri-apps/plugin-dialog";
import { useRouter } from "vue-router";
import { useLibraryStore } from "@/stores/library";
import { useSettingsStore } from "@/stores/settings";
import { isDevelopmentMode } from "@/utils/runtime";

const settings = useSettingsStore();
const libraryStore = useLibraryStore();
const router = useRouter();
const isDev = isDevelopmentMode();

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

function openPrivateRoleLibrary() {
  void router.push("/private-role-maintenance");
}

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
.settings-section {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 1rem;
  background: #fff;
  border: 1px solid #e8e8e8;
  border-radius: 12px;
}
.section-head { display: flex; flex-direction: column; gap: 0.25rem; }
.section-head h3 { font-size: 1rem; }
.section-head p { font-size: 0.875rem; color: #666; line-height: 1.5; }
label { display: flex; flex-direction: column; gap: 0.25rem; }
.reindex-section { display: flex; flex-direction: column; gap: 0.5rem; }
.clear-gallery-section { display: flex; flex-direction: column; gap: 0.5rem; }
.reindex-desc { font-size: 0.875rem; color: #666; }
.progress-bar { height: 6px; background: #e0e0e0; border-radius: 3px; overflow: hidden; }
.progress-fill { height: 100%; background: #646cff; transition: width 0.3s; }
.progress-fill.danger { background: #ff4d4f; }
.done-msg { font-size: 0.875rem; color: #4caf50; }
.advanced-entry {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.875rem 1rem;
  border: 1px solid #e8e8e8;
  border-radius: 10px;
  background: #fafafa;
}
.advanced-entry__copy {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}
.advanced-entry__copy h4 { font-size: 0.95rem; }
.advanced-entry__copy p { font-size: 0.875rem; color: #666; line-height: 1.5; }
.window-preference {
  display: grid;
  gap: 0.85rem;
  padding: 0.9rem 1rem;
  border: 1px solid #e8e8e8;
  border-radius: 10px;
  background: #faf8f2;
}
.window-preference__group {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}
.window-preference__label {
  font-size: 0.9rem;
  font-weight: 600;
}
.window-preference__options {
  display: flex;
  gap: 0.75rem;
  flex-wrap: wrap;
}
.window-option {
  flex-direction: row;
  align-items: center;
  gap: 0.4rem;
  padding: 0.55rem 0.75rem;
  border: 1px solid #ddd5c8;
  border-radius: 999px;
  background: #fff;
}
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
.btn-secondary {
  border: 1px solid #d9d9d9;
  background: #fff;
  color: #333;
  border-radius: 6px;
  padding: 0.45rem 1.1rem;
  cursor: pointer;
  font-size: 0.9rem;
  align-self: flex-start;
  white-space: nowrap;
}
.btn-secondary:hover { background: #f5f5f5; }
</style>
