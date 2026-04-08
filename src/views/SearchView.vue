<template>
  <div class="search-view">
    <SearchBar
      v-model="store.query"
      @update:model-value="debouncedSearch"
    />
    <div
      v-if="settings.showDebugInfo && store.results.length"
      class="debug-formula"
    >
      得分 = 0.75×Relevance + 0.25×Popularity | Relevance = max(0.3×关键词, 0.4×OCR, 0.3×CLIP) | Popularity = log(1+点击)/log(1+最大点击)，冷启动=0.5 | Relevance &lt; 0.2 时过滤
    </div>
    <ImageGrid
      :images="store.results"
      :loading="store.loading"
      :show-debug-info="settings.showDebugInfo"
      :empty-message="emptyMessage"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted, computed } from "vue";
import SearchBar from "@/components/SearchBar.vue";
import ImageGrid from "@/components/ImageGrid.vue";
import { useSearch } from "@/composables/useSearch";
import { useSettingsStore } from "@/stores/settings";
import { useLibraryStore } from "@/stores/library";

const { store, debouncedSearch } = useSearch();
const settings = useSettingsStore();
const libraryStore = useLibraryStore();

const emptyMessage = computed(() =>
  libraryStore.images.length === 0
    ? "还没有图片哦，点击添加开始使用吧"
    : "没找到相关图片，试试其他描述？"
);

onMounted(() => {
  store.search("");
  libraryStore.fetchImages();
});
</script>

<style scoped>
.search-view { padding: 1rem; }
.debug-formula {
  font-size: 0.78rem;
  color: #888;
  background: #f5f5f5;
  border-left: 3px solid #646cff;
  padding: 0.35rem 0.75rem;
  margin-bottom: 0.5rem;
  border-radius: 0 4px 4px 0;
}
</style>
