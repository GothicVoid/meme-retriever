<template>
  <div class="search-view">
    <SearchBar
      v-model="store.query"
      @update:model-value="onQueryChange"
    />
    <div
      v-if="settings.showDebugInfo && store.results.length"
      class="debug-formula"
    >
      得分 = 0.75×Relevance + 0.25×Popularity | Relevance = 0.3×标签 + 0.4×OCR + 0.3×CLIP | Popularity = log(1+点击)/log(1+最大点击)，冷启动=0.1 | Relevance &lt; 0.2 时过滤
    </div>
    <ImageGrid
      :images="visibleResults"
      :loading="store.loading"
      :show-debug-info="settings.showDebugInfo"
      :empty-message="emptyMessage"
      @open="openDetail"
    />
    <div
      v-if="store.results.length > visibleCount"
      class="show-more"
    >
      <button data-action="show-more" @click="showMore">
        展示更多（{{ store.results.length - visibleCount }} 张）
      </button>
    </div>
    <DetailModal
      v-if="detailId"
      :image-id="detailId"
      :images="store.results"
      @close="detailId = null"
      @delete="handleDeleteFromDetail"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted, computed, ref, watch } from "vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import SearchBar from "@/components/SearchBar.vue";
import ImageGrid from "@/components/ImageGrid.vue";
import DetailModal from "@/components/DetailModal.vue";
import { useSearch } from "@/composables/useSearch";
import { useSettingsStore } from "@/stores/settings";
import { useLibraryStore } from "@/stores/library";

const { store, debouncedSearch } = useSearch();
const settings = useSettingsStore();
const libraryStore = useLibraryStore();

const INITIAL_COUNT = 9;
const EXPANDED_COUNT = 21;
const visibleCount = ref(INITIAL_COUNT);

const visibleResults = computed(() => store.results.slice(0, visibleCount.value));

const emptyMessage = computed(() =>
  libraryStore.images.length === 0
    ? "还没有图片哦，点击添加开始使用吧"
    : "没找到相关图片，试试其他描述？"
);

function onQueryChange(val: string) {
  visibleCount.value = INITIAL_COUNT;
  debouncedSearch(val);
}

function showMore() {
  visibleCount.value = EXPANDED_COUNT;
}

// 查询变化时重置分页
watch(() => store.query, () => {
  visibleCount.value = INITIAL_COUNT;
});

const detailId = ref<string | null>(null);

function openDetail(id: string) {
  detailId.value = id;
}

async function handleDeleteFromDetail(id: string) {
  const ok = await confirm("确定要删除这张图片吗？此操作不可撤销。", { title: "删除图片" });
  if (!ok) return;
  await invoke("delete_image", { id });
  store.results = store.results.filter((img) => img.id !== id);
  libraryStore.images = libraryStore.images.filter((img) => img.id !== id);
  detailId.value = null;
}

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
.show-more {
  text-align: center;
  margin-top: 1rem;
}
.show-more button {
  padding: 0.5rem 1.5rem;
  border: 1px solid #646cff;
  border-radius: 6px;
  background: none;
  color: #646cff;
  cursor: pointer;
  font-size: 0.9rem;
}
.show-more button:hover { background: #f0f0ff; }
</style>
