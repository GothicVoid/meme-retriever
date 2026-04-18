<template>
  <div class="search-view">
    <section class="search-view__hero">
      <div class="search-input-wrap search-view__search-wrap">
        <SearchBar
          v-model="store.query"
          class="search-view__search search-view__search--hero"
          :placeholder="searchPlaceholder"
          @update:model-value="onQueryChange"
          @focus="handleSearchFocus"
          @blur="handleSearchBlur"
        />
        <div
          v-if="showSearchHistoryDropdown"
          class="search-history-dropdown ui-floating-panel"
          data-testid="search-history-dropdown"
        >
          <div
            v-for="item in recentSearches"
            :key="item.query"
            class="search-history-dropdown__item"
          >
            <button
              type="button"
              class="search-history-dropdown__query"
              data-testid="search-history-dropdown-item"
              @mousedown.prevent
              @click="applyExampleQuery(item.query)"
            >
              {{ item.query }}
            </button>
            <button
              type="button"
              class="search-history-dropdown__delete"
              data-testid="search-history-delete"
              aria-label="删除最近搜索"
              @mousedown.prevent
              @click="removeRecentSearch(item.query)"
            >
              删除
            </button>
          </div>
        </div>
      </div>
    </section>
    <div
      v-if="isHomeMode"
      class="home-landing"
    >
      <p class="home-landing__intro">
        按图片里的字、角色名、动作、场景来找表情
      </p>
      <div class="home-landing__examples search-view__examples">
        <button
          v-for="example in exampleQueries"
          :key="example"
          class="home-landing__example ui-chip-button"
          @click="applyExampleQuery(example)"
        >
          {{ example }}
        </button>
      </div>

      <div
        v-if="showColdStart"
        class="home-empty"
      >
        <p class="home-empty__title">
          先把表情包放进来
        </p>
        <p class="home-empty__text">
          导入后就可以按图片里的字、角色名、动作或场景直接找图
        </p>
        <button
          type="button"
          class="home-empty__action"
        >
          导入图片
        </button>
      </div>

      <template v-else>
        <section
          v-if="recentSearches.length > 0"
          class="home-section"
        >
          <div class="home-section__header">
            <h2 class="home-section__title">
              最近搜索
            </h2>
          </div>
          <div class="home-searches">
            <button
              v-for="item in recentSearches"
              :key="item.query"
              type="button"
              class="home-searches__item"
              data-testid="recent-search-item"
              @click="applyExampleQuery(item.query)"
            >
              {{ item.query }}
            </button>
          </div>
        </section>

        <section
          v-if="recentUsedImages.length > 0"
          class="home-section"
        >
          <div class="home-section__header">
            <h2 class="home-section__title">
              最近用过
            </h2>
          </div>
          <ImageGrid
            :images="recentUsedImages"
            :loading="homeLoading"
            :show-debug-info="false"
            @copied="handleHomeImageCopied"
            @open="openDetail"
          />
        </section>

        <section
          v-if="homeImages.length > 0"
          class="home-section"
        >
          <div class="home-section__header">
            <h2 class="home-section__title">
              常用表情
            </h2>
          </div>
          <ImageGrid
            :images="homeImages"
            :loading="homeLoading"
            :show-debug-info="false"
            @copied="handleHomeImageCopied"
            @open="openDetail"
          />
        </section>
      </template>
    </div>
    <div
      v-else-if="settings.devDebugMode && store.results.length"
      class="debug-formula"
    >
      开发调试模式：显示当前排序主路、贡献项与最终得分，用于辅助排查结果排序
    </div>
    <ImageGrid
      v-if="!isHomeMode"
      :images="visibleResults"
      :loading="store.loading"
      :show-debug-info="settings.devDebugMode"
      :empty-message="emptyMessage"
      @copied="handleSearchImageCopied"
      @open="openDetail"
    />
    <div
      v-if="showZeroResultHint || showLowConfidenceHint || showLowRelevanceStopNotice"
      class="result-feedback"
    >
      <p class="result-feedback__title">
        {{ feedbackTitle }}
      </p>
      <p class="result-feedback__text">
        {{ feedbackText }}
      </p>
      <div
        v-if="showGuidanceList"
        class="result-feedback__guidance"
      >
        <span
          v-for="item in guidanceItems"
          :key="item"
          class="result-feedback__guidance-item"
          data-testid="search-guidance-item"
        >
          {{ item }}
        </span>
      </div>
      <button
        v-if="showRecentUsedShortcut"
        class="result-feedback__action"
        data-action="show-recent-used"
        @click="goBackToHome"
      >
        看看最近用过
      </button>
      <button
        v-if="showZeroResultHint || showLowConfidenceHint"
        class="result-feedback__action"
        data-action="go-gallery-management"
        @click="goToGalleryManagement(showLowConfidenceHint ? 'issues' : 'recent')"
      >
        去图库管理
      </button>
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
import { onMounted, onBeforeUnmount, computed, ref, watch, nextTick, inject } from "vue";
import { confirm } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { routerKey, type Router } from "vue-router";
import SearchBar from "@/components/SearchBar.vue";
import ImageGrid from "@/components/ImageGrid.vue";
import DetailModal from "@/components/DetailModal.vue";
import { useSearch } from "@/composables/useSearch";
import { useSettingsStore } from "@/stores/settings";
import { useLibraryStore } from "@/stores/library";
import { getRelevanceLevel } from "@/utils/relevance";
import { getUserFacingRelevanceLabel } from "@/utils/relevance";
import type { SearchResult } from "@/stores/search";

const { store, debouncedSearch } = useSearch();
const settings = useSettingsStore();
const libraryStore = useLibraryStore();
const router = inject<Router | undefined>(routerKey, undefined);

interface HomeImage {
  id: string;
  filePath: string;
  fileName: string;
  thumbnailPath: string;
  fileFormat: string;
  fileStatus: string;
  width: number;
  height: number;
  fileSize: number;
  addedAt: number;
  useCount: number;
  tags: SearchResult["tags"];
}

interface HomeState {
  imageCount: number;
  recentSearches: { query: string; updatedAt: number }[];
  recentUsed: HomeImage[];
  frequentUsed: HomeImage[];
}

const HIGH_CONFIDENCE_BATCH_SIZE = 12;
const RESULT_FETCH_STEP = 30;
const visibleRelevantCount = ref(HIGH_CONFIDENCE_BATCH_SIZE);
const showSecondaryResults = ref(false);
const loadMoreTrigger = ref<HTMLElement | null>(null);
const homeState = ref<HomeState | null>(null);
const homeLoading = ref(false);
const homeLoadFailed = ref(false);
const searchFocused = ref(false);
let searchBlurTimer: number | null = null;
let loadMoreObserver: IntersectionObserver | null = null;
const exampleQueries = ["撤回消息", "阿布 撇嘴", "猫猫 心虚", "领导 冷笑"];

const isHomeMode = computed(() => !store.query.trim());

const searchPlaceholder = computed(() => "搜台词、角色、动作、场景");

const showColdStart = computed(() =>
  isHomeMode.value
  && !homeLoading.value
  && !homeLoadFailed.value
  && (homeState.value?.imageCount ?? 0) === 0
);

const recentSearches = computed(() => homeState.value?.recentSearches ?? []);
const showSearchHistoryDropdown = computed(() =>
  searchFocused.value && !store.query.trim() && recentSearches.value.length > 0
);

function toHomeSearchResults(images: HomeImage[]): SearchResult[] {
  return images.map((image) => ({
    id: image.id,
    filePath: image.filePath,
    thumbnailPath: image.thumbnailPath,
    fileFormat: image.fileFormat,
    fileStatus: image.fileStatus,
    score: 1,
    tags: image.tags,
    matchedOcrTerms: [],
    matchedTags: [],
    matchedRoleName: null,
    debugInfo: null,
  }));
}

const recentUsedImages = computed<SearchResult[]>(() =>
  toHomeSearchResults(homeState.value?.recentUsed ?? [])
);

const homeImages = computed<SearchResult[]>(() =>
  toHomeSearchResults(homeState.value?.frequentUsed ?? [])
);

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

const showZeroResultHint = computed(() =>
  !!store.query.trim() && !store.loading && store.results.length === 0
);

const showLowRelevanceStopNotice = computed(() =>
  !!store.query.trim()
  && mediumConfidenceCount.value > 0
  && secondaryResultsCount.value > 0
);

const guidanceItems = [
  "试试图片里的原文",
  "试试角色名 + 动作",
  "试试更短的关键词",
  "试试更长一点的描述",
];

const showGuidanceList = computed(() => showZeroResultHint.value || showLowConfidenceHint.value);

const showRecentUsedShortcut = computed(() =>
  (showZeroResultHint.value || showLowConfidenceHint.value)
  && recentUsedImages.value.length > 0
);

const canShowSecondaryResults = computed(() =>
  secondaryResultsCount.value > 0
  || (showLowConfidenceHint.value && store.results.length > 0)
);

const feedbackTitle = computed(() => {
  if (showZeroResultHint.value) {
    return "没找到这类图片";
  }
  if (showLowConfidenceHint.value) {
    return "没找到足够相关的结果";
  }
  if (mediumConfidenceCount.value > highConfidenceCount.value) {
    return `已展示${getUserFacingRelevanceLabel(1)}和${getUserFacingRelevanceLabel(0.6)}的结果，共 ${mediumConfidenceCount.value} 张`;
  }
  return `已展示全部${getUserFacingRelevanceLabel(1)}结果，共 ${highConfidenceCount.value} 张`;
});

const feedbackText = computed(() => {
  if (showZeroResultHint.value) {
    return "换个说法试试。可以从图片里的原文、角色名、动作或场景词开始搜。";
  }
  if (showLowConfidenceHint.value) {
    return "试试补充图片里的文字、角色名、动作或场景词，例如“阿布 撇嘴”“撤回消息 猫猫”“领导 冷笑”。";
  }
  if (showSecondaryResults.value) {
    return `后续 ${secondaryResultsCount.value} 张结果相关性较低，当前仅因你主动展开才显示。`;
  }
  if (mediumConfidenceCount.value > highConfidenceCount.value) {
    return `其中${getUserFacingRelevanceLabel(1)} ${highConfidenceCount.value} 张、${getUserFacingRelevanceLabel(0.6)} ${mediumConfidenceCount.value - highConfidenceCount.value} 张；后续 ${secondaryResultsCount.value} 张结果相关性明显下降，已默认隐藏。`;
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

async function fetchHomeState() {
  homeLoading.value = true;
  try {
    homeState.value = await invoke<HomeState>("get_home_state");
    homeLoadFailed.value = false;
  } catch {
    homeState.value = null;
    homeLoadFailed.value = true;
  } finally {
    homeLoading.value = false;
  }
}

function onQueryChange(val: string) {
  resetResultView();
  if (!val.trim()) {
    searchFocused.value = false;
    debouncedSearch.cancel?.();
    store.results = [];
    void fetchHomeState();
    return;
  }
  debouncedSearch(val);
}

function applyExampleQuery(query: string) {
  searchFocused.value = false;
  store.query = query;
  onQueryChange(query);
}

function goBackToHome() {
  searchFocused.value = false;
  store.query = "";
  onQueryChange("");
}

function goToGalleryManagement(targetView: "recent" | "issues") {
  if (router) {
    void router.push({ path: "/library", query: { view: targetView } });
    return;
  }

  const search = new URLSearchParams(window.location.search);
  search.set("view", targetView);
  window.history.pushState({}, "", `/library?${search.toString()}`);
}

function handleSearchFocus() {
  if (searchBlurTimer !== null) {
    window.clearTimeout(searchBlurTimer);
    searchBlurTimer = null;
  }
  searchFocused.value = true;
}

function handleSearchBlur() {
  searchBlurTimer = window.setTimeout(() => {
    searchFocused.value = false;
    searchBlurTimer = null;
  }, 120);
}

async function removeRecentSearch(query: string) {
  await invoke("delete_search_history", { query });
  await fetchHomeState();
  searchFocused.value = true;
}

function handleHomeImageCopied() {
  void fetchHomeState();
}

function handleSearchImageCopied() {
  if (isHomeMode.value) {
    void fetchHomeState();
  }
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
  if (isHomeMode.value) {
    await fetchHomeState();
  }
}

onMounted(async () => {
  await fetchHomeState();
  void libraryStore.fetchImages();
  await nextTick();
  attachLoadMoreObserver();
});

onBeforeUnmount(() => {
  if (searchBlurTimer !== null) {
    window.clearTimeout(searchBlurTimer);
  }
  loadMoreObserver?.disconnect();
});
</script>

<style scoped>
.search-view {
  padding: 1rem;
}

.search-view__hero {
  margin-bottom: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.search-input-wrap {
  position: relative;
}

.search-view__search-wrap {
  z-index: 20;
}

.search-view__search {
  margin-bottom: 0;
}

.search-view__search--hero {
  min-height: 64px;
  padding-inline: 1rem;
}

.search-view__examples {
  gap: 0.625rem;
}

.search-history-dropdown {
  position: absolute;
  top: calc(100% + 0.5rem);
  left: 0;
  right: 0;
  z-index: 30;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  padding: 0.625rem;
}
.search-history-dropdown__item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}
.search-history-dropdown__query,
.search-history-dropdown__delete {
  border: none;
  background: transparent;
  cursor: pointer;
}
.search-history-dropdown__query {
  flex: 1;
  padding: 0.5rem 0.625rem;
  text-align: left;
  border-radius: 8px;
  color: var(--ui-text-primary);
}
.search-history-dropdown__query:hover {
  background: var(--ui-bg-hover);
}
.search-history-dropdown__delete {
  padding: 0.35rem 0.5rem;
  border-radius: 6px;
  color: var(--ui-text-secondary);
}
.search-history-dropdown__delete:hover {
  background: var(--ui-bg-hover);
  color: var(--ui-text-primary);
}
.home-landing {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
  padding: 1.5rem;
  border: 1px solid var(--ui-border-subtle);
  border-radius: var(--ui-radius-lg);
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.92), rgba(251, 248, 242, 0.9));
  box-shadow: var(--ui-shadow-soft);
}
.home-landing__intro {
  margin: 0;
  max-width: 42rem;
  font-size: 1rem;
  line-height: 1.65;
  color: var(--ui-text-secondary);
}
.home-landing__examples {
  display: flex;
  flex-wrap: wrap;
}
.home-landing__example {
  font-size: 0.85rem;
}
.home-empty {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 1.25rem 1.125rem;
  border: 1px solid var(--ui-border-subtle);
  border-radius: var(--ui-radius-md);
  background: color-mix(in srgb, var(--ui-bg-surface-strong) 95%, white);
}
.home-empty__title {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 700;
  color: var(--ui-text-primary);
}
.home-empty__text {
  margin: 0;
  font-size: 0.9rem;
  line-height: 1.5;
  color: var(--ui-text-secondary);
}
.home-empty__action {
  margin-top: 0.25rem;
  padding: 0.5rem 1rem;
  border: 1px solid transparent;
  border-radius: 999px;
  background: var(--ui-accent);
  color: #fff;
  cursor: pointer;
  font-size: 0.88rem;
}
.home-empty__action:hover {
  background: var(--ui-accent-hover);
}
.home-section {
  display: flex;
  flex-direction: column;
  gap: 0.875rem;
}
.home-searches {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}
.home-searches__item {
  border: 1px solid var(--ui-border-subtle);
  border-radius: 999px;
  background: color-mix(in srgb, var(--ui-bg-surface-strong) 92%, white);
  color: var(--ui-text-primary);
  padding: 0.38rem 0.8rem;
  font-size: 0.85rem;
  cursor: pointer;
}
.home-searches__item:hover {
  background: var(--ui-bg-hover);
}
.home-section__header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.home-section__title {
  margin: 0;
  font-size: 1rem;
  font-weight: 700;
  color: var(--ui-text-primary);
}
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
.result-feedback__guidance {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  margin-top: 0.75rem;
}
.result-feedback__guidance-item {
  padding: 0.35rem 0.625rem;
  border-radius: 999px;
  background: rgba(254, 243, 199, 0.9);
  color: #92400e;
  font-size: 0.85rem;
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
