<template>
  <div class="library-view">
    <div class="toolbar">
      <button
        :disabled="store.indexing"
        @click="handleAdd"
      >
        添加图片
      </button>
      <button
        :disabled="store.indexing"
        @click="handleAddFolder"
      >
        添加文件夹
      </button>
      <template v-if="store.selectedIds.size > 0">
        <span class="selection-count">已选 {{ store.selectedIds.size }} 张</span>
        <button
          data-action="delete-selected"
          @click="handleDeleteSelected"
        >
          删除选中
        </button>
      </template>
    </div>
    <div
      v-if="store.indexing"
      class="index-status"
    >
      <span>正在入库… {{ store.indexCurrent }}/{{ store.indexTotal }}</span>
      <div class="progress-bar">
        <div
          class="progress-fill"
          :style="{ width: progressPercent + '%' }"
        />
      </div>
    </div>
    <ImageGrid
      :images="store.images as unknown as SearchResult[]"
      :loading="store.loading"
      :show-debug-info="false"
      :selectable="true"
      :selected-ids="store.selectedIds"
      @delete="handleDelete"
      @select="store.toggleSelection"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted, computed } from "vue";
import { open, confirm } from "@tauri-apps/plugin-dialog";
import ImageGrid from "@/components/ImageGrid.vue";
import { useLibraryStore } from "@/stores/library";
import type { SearchResult } from "@/stores/search";

const store = useLibraryStore();

const progressPercent = computed(() =>
  store.indexTotal > 0 ? (store.indexCurrent / store.indexTotal) * 100 : 0
);

onMounted(() => store.fetchImages());

async function handleAdd() {
  const selected = await open({ multiple: true, filters: [{ name: "图片", extensions: ["jpg", "jpeg", "png", "gif", "webp"] }] });
  if (!selected) return;
  const paths = Array.isArray(selected) ? selected : [selected];
  await store.addImages(paths);
}

async function handleAddFolder() {
  const selected = await open({ directory: true });
  if (!selected) return;
  const path = Array.isArray(selected) ? selected[0] : selected;
  await store.addFolder(path);
}

async function handleDelete(id: string) {
  const ok = await confirm("确定要删除这张图片吗？此操作不可撤销。", { title: "删除图片" });
  if (!ok) return;
  await store.deleteImage(id);
}

async function handleDeleteSelected() {
  const count = store.selectedIds.size;
  const ok = await confirm(`确认删除 ${count} 张图片？此操作不可撤销。`, { title: "批量删除" });
  if (!ok) return;
  await store.deleteSelected();
}
</script>

<style scoped>
.library-view { padding: 1rem; }
.toolbar { margin-bottom: 1rem; display: flex; align-items: center; gap: 0.5rem; }
.selection-count { font-size: 0.875rem; color: #666; }
.index-status { margin-bottom: 0.75rem; font-size: 0.875rem; color: #666; display: flex; flex-direction: column; gap: 0.25rem; }
.progress-bar { height: 6px; background: #e0e0e0; border-radius: 3px; overflow: hidden; }
.progress-fill { height: 100%; background: #646cff; transition: width 0.3s; }
</style>
