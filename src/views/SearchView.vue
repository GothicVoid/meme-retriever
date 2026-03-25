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
      得分 = 语义权重 × 语义得分 + 关键词权重 × 关键词得分（默认 0.7/0.3，标签命中时 0.4/0.6）
    </div>
    <ImageGrid
      :images="store.results"
      :loading="store.loading"
      :show-debug-info="settings.showDebugInfo"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from "vue";
import SearchBar from "@/components/SearchBar.vue";
import ImageGrid from "@/components/ImageGrid.vue";
import { useSearch } from "@/composables/useSearch";
import { useSettingsStore } from "@/stores/settings";

const { store, debouncedSearch } = useSearch();
const settings = useSettingsStore();

onMounted(() => store.search(""));
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
