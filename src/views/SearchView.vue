<template>
  <div class="search-view">
    <SearchBar
      v-model="store.query"
      @update:model-value="onQueryChange"
    />
    <div
      v-if="settings.devDebugMode && store.results.length"
      class="debug-formula"
    >
      开发调试模式：显示当前排序主路、贡献项与最终得分，用于辅助排查结果排序
    </div>
    <ImageGrid
      :images="visibleResults"
      :loading="store.loading"
      :show-debug-info="settings.devDebugMode"
      :empty-message="emptyMessage"
      @open="openDetail"
    />
    <div
      v-if="showLowConfidenceHint || showLowRelevanceStopNotice"
      class="result-feedback"
    >
      <p class="result-feedback__title">
        {{ feedbackTitle }}
      </p>
      <p class="result-feedback__text">
        {{ feedbackText }}
      </p>
      <button
        v-if="canShowSecondaryResults"
        class="result-feedback__action"
        :data-action="showSecondaryResults ? 'show-less' : 'show-more-secondary'"
        @click="toggleSecondaryResults"
      >
        {{ showSecondaryResults ? "收起低相关结果" : `仍然查看其余 ${secondaryResultsCount} 张结果` }}
      </button>
    </div>
    <div
      v-if="shouldAutoLoadMore"
      ref="loadMoreTrigger"
      class="load-more-trigger"
      data-testid="load-more-trigger"
    >
      <p class="load-more-trigger__text">
        {{ loadMoreHint }}
      </p>
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
import { onMounted, onBeforeUnmount, computed, ref, watch, nextTick } from "vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import SearchBar from "@/components/SearchBar.vue";
import ImageGrid from "@/components/ImageGrid.vue";
import DetailModal from "@/components/DetailModal.vue";
import { useSearch } from "@/composables/useSearch";
import { useSettingsStore } from "@/stores/settings";
import { useLibraryStore } from "@/stores/library";
import { getRelevanceLevel } from "@/utils/relevance";

const { store, debouncedSearch } = useSearch();
const settings = useSettingsStore();
const libraryStore = useLibraryStore();

const HIGH_CONFIDENCE_BATCH_SIZE = 12;
const RESULT_FETCH_STEP = 30;
const visibleRelevantCount = ref(HIGH_CONFIDENCE_BATCH_SIZE);
const showSecondaryResults = ref(false);
const loadMoreTrigger = ref<HTMLElement | null>(null);
let loadMoreObserver: IntersectionObserver | null = null;

const highConfidenceCount = computed(() => {
  const results = store.results;
  if (!results.length) return 0;
  if (!store.query.trim()) return results.length;

  let count = 0;
  for (const result of results) {
    if (getRelevanceLevel(result.score) !== "high") {
      break;
    }
    count += 1;
  }

  return count;
});

const mediumConfidenceCount = computed(() => {
  const results = store.results;
  if (!results.length) return 0;
  if (!store.query.trim()) return results.length;

  let count = 0;
  for (const result of results) {
    if (getRelevanceLevel(result.score) === "low") {
      break;
    }
    count += 1;
  }

  return count;
});

const visiblePrimaryCount = computed(() =>
  Math.min(mediumConfidenceCount.value, visibleRelevantCount.value)
);

const secondaryResultsCount = computed(() =>
  Math.max(0, store.results.length - mediumConfidenceCount.value)
);

const visibleResults = computed(() => {
  if (showSecondaryResults.value) {
    return store.results;
  }

  if (mediumConfidenceCount.value > 0) {
    return store.results.slice(0, visiblePrimaryCount.value);
  }

  return [];
});

const hasMoreRelevantLoaded = computed(() =>
  !showSecondaryResults.value && visiblePrimaryCount.value < mediumConfidenceCount.value
);

const canLoadMoreResults = computed(() =>
  !store.loading && store.results.length > 0 && store.results.length >= store.currentLimit
);

const canLoadMoreRelevantResults = computed(() =>
  !showSecondaryResults.value
  && mediumConfidenceCount.value > 0
  && visiblePrimaryCount.value >= mediumConfidenceCount.value
  && mediumConfidenceCount.value === store.results.length
  && canLoadMoreResults.value
);

const shouldAutoLoadMore = computed(() =>
  hasMoreRelevantLoaded.value || canLoadMoreRelevantResults.value
);

const showLowConfidenceHint = computed(() =>
  !!store.query.trim() && store.results.length > 0 && mediumConfidenceCount.value === 0
);

const showLowRelevanceStopNotice = computed(() =>
  !!store.query.trim()
  && mediumConfidenceCount.value > 0
  && secondaryResultsCount.value > 0
);

const canShowSecondaryResults = computed(() =>
  secondaryResultsCount.value > 0
  || (showLowConfidenceHint.value && store.results.length > 0)
);

const feedbackTitle = computed(() => {
  if (showLowConfidenceHint.value) {
    return "没找到足够相关的结果";
  }
  if (mediumConfidenceCount.value > highConfidenceCount.value) {
    return `已展示高相关和较相关结果，共 ${mediumConfidenceCount.value} 张`;
  }
  return `已展示全部高相关结果，共 ${highConfidenceCount.value} 张`;
});

const feedbackText = computed(() => {
  if (showLowConfidenceHint.value) {
    return "试试补充图片里的文字、角色名、动作或场景词，例如“阿布 撇嘴”“撤回消息 猫猫”“领导 冷笑”。";
  }
  if (showSecondaryResults.value) {
    return `后续 ${secondaryResultsCount.value} 张结果相关性较低，当前仅因你主动展开才显示。`;
  }
  if (mediumConfidenceCount.value > highConfidenceCount.value) {
    return `其中高相关 ${highConfidenceCount.value} 张、较相关 ${mediumConfidenceCount.value - highConfidenceCount.value} 张；后续 ${secondaryResultsCount.value} 张结果相关性明显下降，已默认隐藏。`;
  }
  return `后续 ${secondaryResultsCount.value} 张结果相关性明显下降，已默认隐藏，避免把不相关图片混进来。`;
});

const loadMoreHint = computed(() =>
  store.loading ? "正在加载更多相关结果..." : "继续下滑查看更多相关结果"
);

const emptyMessage = computed(() =>
  showLowConfidenceHint.value
    ? "没找到足够相关的结果，试试更具体的描述"
    : libraryStore.images.length === 0
      ? "还没有图片哦，点击添加开始使用吧"
      : "没找到相关图片，试试其他描述？"
);

function resetResultView() {
  visibleRelevantCount.value = HIGH_CONFIDENCE_BATCH_SIZE;
  showSecondaryResults.value = false;
}

function onQueryChange(val: string) {
  resetResultView();
  debouncedSearch(val);
}

function toggleSecondaryResults() {
  showSecondaryResults.value = !showSecondaryResults.value;
}

function revealMoreRelevantResults() {
  visibleRelevantCount.value += HIGH_CONFIDENCE_BATCH_SIZE;
}

async function loadMoreResults() {
  await store.search(store.query, store.currentLimit + RESULT_FETCH_STEP);
}

async function handleAutoLoadMore() {
  if (store.loading || showSecondaryResults.value) return;

  if (hasMoreRelevantLoaded.value) {
    revealMoreRelevantResults();
    return;
  }

  if (canLoadMoreRelevantResults.value) {
    await loadMoreResults();
  }
}

function attachLoadMoreObserver() {
  if (typeof IntersectionObserver === "undefined") return;
  if (!loadMoreTrigger.value) return;

  loadMoreObserver?.disconnect();
  loadMoreObserver = new IntersectionObserver((entries) => {
    if (entries.some((entry) => entry.isIntersecting)) {
      void handleAutoLoadMore();
    }
  }, { rootMargin: "160px 0px" });
  loadMoreObserver.observe(loadMoreTrigger.value);
}

watch(loadMoreTrigger, () => {
  nextTick(() => attachLoadMoreObserver());
});

watch(shouldAutoLoadMore, async () => {
  await nextTick();
  attachLoadMoreObserver();
});

watch(() => store.query, () => {
  resetResultView();
});

watch(() => store.results.length, (nextLen, prevLen) => {
  if (nextLen < prevLen) {
    visibleRelevantCount.value = HIGH_CONFIDENCE_BATCH_SIZE;
  }
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

onMounted(async () => {
  await store.search("");
  libraryStore.fetchImages();
  await nextTick();
  attachLoadMoreObserver();
});

onBeforeUnmount(() => {
  loadMoreObserver?.disconnect();
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
.result-feedback {
  margin-top: 1rem;
  padding: 0.75rem 1rem;
  border-radius: 8px;
  background: #fff7e8;
  border: 1px solid #f5d39a;
  color: #8a5a00;
  text-align: left;
}
.result-feedback__title {
  margin: 0 0 0.25rem;
  font-size: 0.95rem;
  font-weight: 600;
}
.result-feedback__text {
  margin: 0;
  font-size: 0.88rem;
  line-height: 1.5;
}
.result-feedback__action {
  margin-top: 0.75rem;
  padding: 0.4rem 0.9rem;
  border: 1px solid #d6a23f;
  border-radius: 6px;
  background: none;
  color: #8a5a00;
  cursor: pointer;
  font-size: 0.85rem;
}
.result-feedback__action:hover { background: rgba(245, 211, 154, 0.25); }
.load-more-trigger {
  padding: 1rem 0 0.5rem;
  text-align: center;
}
.load-more-trigger__text {
  margin: 0;
  font-size: 0.85rem;
  color: #888;
}
</style>
