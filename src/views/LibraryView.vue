<template>
  <div class="library-view">
    <div class="toolbar">
      <button @click="handleAdd">
        添加图片
      </button>
    </div>
    <ImageGrid
      :images="store.images as unknown as SearchResult[]"
      :loading="store.loading"
      :show-debug-info="false"
      @delete="handleDelete"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from "vue";
import { open, confirm } from "@tauri-apps/plugin-dialog";
import ImageGrid from "@/components/ImageGrid.vue";
import { useLibraryStore } from "@/stores/library";
import type { SearchResult } from "@/stores/search";

const store = useLibraryStore();

onMounted(() => store.fetchImages());

async function handleAdd() {
  const selected = await open({ multiple: true, filters: [{ name: "图片", extensions: ["jpg", "jpeg", "png", "gif", "webp"] }] });
  if (!selected) return;
  const paths = Array.isArray(selected) ? selected : [selected];
  await store.addImages(paths);
}

async function handleDelete(id: string) {
  const ok = await confirm("确定要删除这张图片吗？此操作不可撤销。", { title: "删除图片" });
  if (!ok) return;
  await store.deleteImage(id);
}
</script>

<style scoped>
.library-view { padding: 1rem; }
.toolbar { margin-bottom: 1rem; }
</style>
