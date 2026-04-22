<template>
  <div class="search-view">
    <section class="search-view__body">
      <section
        v-if="showColdStart || showPostImportPrompt || !isHomeMode"
        class="search-view__top-banner"
        :class="{ 'search-view__top-banner--cold': showColdStart }"
      >
        <div
          v-if="showColdStart"
          class="search-state-banner search-state-banner--cold"
          data-testid="cold-start-panel"
        >
          <div class="search-state-banner__copy">
            <p class="search-state-banner__title">
              先导入表情包
            </p>
            <p class="search-state-banner__text">
              导入后就能按台词、角色、动作、场景搜表情。
            </p>
          </div>
          <div class="search-state-banner__actions">
            <button
              ref="importButtonRef"
              type="button"
              class="search-state-banner__action search-state-banner__action--primary"
              data-action="open-import-menu"
              @click="toggleImportMenu"
            >
              导入图片
            </button>
            <div
              v-if="showImportMenu"
              ref="importMenuRef"
              class="search-state-banner__import-menu ui-floating-panel"
              data-testid="cold-start-import-menu"
            >
              <button
                type="button"
                class="search-state-banner__import-action"
                data-action="import-images"
                @click="handleImportImages"
              >
                选择图片
              </button>
              <button
                type="button"
                class="search-state-banner__import-action"
                data-action="import-folder"
                @click="handleImportFolder"
              >
                选择文件夹
              </button>
            </div>
          </div>
          <div class="search-state-banner__examples">
            <p class="search-state-banner__label">
              试试这些词
            </p>
            <div class="search-state-banner__chips">
              <button
                v-for="example in exampleQueries"
                :key="example"
                type="button"
                class="search-state-banner__chip"
                data-testid="cold-start-example"
                @click="applyExampleQuery(example)"
              >
                {{ example }}
              </button>
            </div>
          </div>
          <p
            v-if="coldStartHint"
            class="search-state-banner__hint"
            data-testid="cold-start-hint"
          >
            {{ coldStartHint }}
          </p>
          <p class="search-state-banner__footnote">
            下面只是演示，不会加入你的图库。
          </p>
        </div>

        <section
          v-else-if="showPostImportPrompt"
          class="search-state-banner search-state-banner--post-import"
          data-testid="post-import-prompt"
        >
          <div class="search-state-banner__copy">
            <p class="search-state-banner__title">
              现在可以按台词、角色、动作开始找图了
            </p>
            <p class="search-state-banner__text">
              不知道怎么搜的话，先试试下面这些词。
            </p>
          </div>
          <button
            type="button"
            class="search-state-banner__close"
            aria-label="关闭导入后提示"
            @click="dismissPostImportPrompt"
          >
            ×
          </button>
          <div class="search-state-banner__chips">
            <button
              v-for="example in exampleQueries"
              :key="`post-${example}`"
              type="button"
              class="search-state-banner__chip"
              data-testid="post-import-example"
              @click="applyExampleQuery(example)"
            >
              {{ example }}
            </button>
          </div>
        </section>

        <div
          v-else
          class="search-state-banner search-state-banner--summary"
        >
          <p class="search-state-banner__title">
            {{ searchSummaryTitle }}
          </p>
          <p
            v-if="searchSummaryMeta"
            class="search-state-banner__text"
          >
            {{ searchSummaryMeta }}
          </p>
          <p
            v-if="showResultShortcutsHint"
            class="search-state-banner__meta"
            data-testid="result-shortcuts-hint"
          >
            Enter 复制 · Space 预览 · Esc 关闭
          </p>
        </div>
      </section>

      <div
        v-if="isHomeMode"
        class="home-landing"
        :class="{ 'home-landing--cold': showColdStart }"
      >
        <div
          v-if="showColdStart"
          class="home-cold-start"
        >
          <div class="home-preview-wrap">
            <div class="home-preview-wrap__banner">
              导入后这里会显示结果
            </div>
            <div
              class="home-preview-cloud"
              aria-hidden="true"
            >
              <div
                v-for="card in coldStartPreviewCards"
                :key="card.id"
                class="home-preview-cloud__card"
                :style="{
                  '--preview-height': `${card.height}px`,
                  '--preview-tag-width': card.tagWidth,
                  '--preview-primary-width': card.primaryWidth,
                  '--preview-secondary-width': card.secondaryWidth,
                }"
              >
                <div class="home-preview-cloud__frame">
                  <div class="home-preview-cloud__media">
                    <img
                      :src="card.src"
                      alt=""
                      class="home-preview-cloud__image"
                    >
                  </div>
                  <div class="home-preview-cloud__info">
                    <div class="home-preview-cloud__header">
                      <span class="home-preview-cloud__tag" />
                    </div>
                    <div class="home-preview-cloud__pills">
                      <span class="home-preview-cloud__pill home-preview-cloud__pill--wide" />
                      <span class="home-preview-cloud__pill" />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <template v-else>
          <section
            v-if="quickPickImages.length > 0"
            class="home-section"
          >
            <h2 class="home-section__title">
              最近常用
            </h2>
            <ImageGrid
              :images="quickPickImages"
              :loading="homeLoading"
              :show-debug-info="false"
              @copied="handleHomeImageCopied"
              @delete="handleDeleteFromGrid"
              @open="openDetail"
              @preview="openQuickPreview"
            />
          </section>
        </template>
      </div>
      <template v-else>
        <ImageGrid
          :images="visibleResults"
          :loading="store.loading"
          loading-message="正在搜索相关图片..."
          :show-debug-info="settings.devDebugMode"
          :empty-message="emptyMessage"
          :focused-id="focusedResultId"
          @copied="handleSearchImageCopied"
          @delete="handleDeleteFromGrid"
          @open="openDetail"
          @preview="openQuickPreview"
        />
        <div
          v-if="showSearchErrorHint || showZeroResultHint || showLowConfidenceHint"
          class="result-feedback"
          :class="{
            'result-feedback--warning': showSearchErrorHint || showZeroResultHint || showLowConfidenceHint,
          }"
        >
          <p class="result-feedback__title">
            {{ feedbackTitle }}
          </p>
          <p
            v-if="feedbackText"
            class="result-feedback__text"
          >
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
          <div class="result-feedback__actions">
            <button
              v-if="showRecentUsedShortcut"
              class="result-feedback__action"
              data-action="show-recent-used"
              @click="goBackToHome"
            >
              看看最近用过
            </button>
            <button
              v-if="primaryRecoveryAction"
              class="result-feedback__action"
              data-action="primary-recovery-action"
              @click="runPrimaryRecoveryAction"
            >
              {{ primaryRecoveryAction.label }}
            </button>
            <button
              v-if="canShowSecondaryResults"
              class="result-feedback__action"
              :data-action="showSecondaryResults ? 'show-less' : 'show-more-secondary'"
              @click="toggleSecondaryResults"
            >
              {{ showSecondaryResults ? "收起补充结果" : `查看其余 ${secondaryResultsCount} 张` }}
            </button>
          </div>
        </div>
        <p
          v-if="showIncompleteResultsHint"
          class="result-feedback result-feedback--secondary"
          data-testid="search-incomplete-hint"
        >
          当前结果可能不完整，处理完成后再搜一次会更稳。
        </p>
        <div
          v-if="showLowRelevanceStopNotice"
          class="result-more-strip"
        >
          <div class="result-more-strip__copy">
            <p class="result-more-strip__title">
              其余结果先收起来了
            </p>
            <p class="result-more-strip__text">
              最像的 {{ mediumConfidenceCount }} 张已经排在前面。
            </p>
          </div>
          <button
            v-if="canShowSecondaryResults"
            class="result-more-strip__action"
            :data-action="showSecondaryResults ? 'show-less' : 'show-more-secondary'"
            @click="toggleSecondaryResults"
          >
            {{ showSecondaryResults ? "收起补充结果" : `查看更多 ${secondaryResultsCount} 张` }}
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
      </template>
    </section>
    <DetailModal
      v-if="detailId"
      :image-id="detailId"
      :images="detailImages"
      @close="detailId = null"
      @delete="handleDeleteFromDetail"
    />
    <QuickPreviewModal
      v-if="previewImage"
      :image="previewImage"
      :can-prev="canPreviewPrev"
      :can-next="canPreviewNext"
      @close="closeQuickPreview"
      @copy="handleQuickPreviewCopy"
      @detail="openDetail"
      @reveal="handleQuickPreviewReveal"
      @prev="moveQuickPreview(-1)"
      @next="moveQuickPreview(1)"
    />
    <section class="search-dock">
      <div class="search-input-wrap search-view__search-wrap">
        <div
          v-if="showSearchAssistPanel"
          class="search-assist-panel ui-floating-panel"
          data-testid="search-history-dropdown"
        >
          <div
            v-if="recentSearches.length > 0"
            class="search-assist-panel__section"
          >
            <p class="search-assist-panel__label">
              最近搜过
            </p>
            <div class="search-assist-panel__row">
              <div
                v-for="item in recentSearches"
                :key="item.query"
                class="search-assist-panel__item"
              >
                <button
                  type="button"
                  class="search-assist-panel__query"
                  data-testid="search-history-dropdown-item"
                  @mousedown.prevent
                  @click="applyExampleQuery(item.query)"
                >
                  {{ item.query }}
                </button>
                <button
                  type="button"
                  class="search-assist-panel__delete"
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
          <div class="search-assist-panel__section">
            <p class="search-assist-panel__label">
              可以直接试
            </p>
            <div class="search-assist-panel__row">
              <button
                v-for="example in exampleQueries"
                :key="example"
                type="button"
                class="search-assist-panel__chip"
                @mousedown.prevent
                @click="applyExampleQuery(example)"
              >
                {{ example }}
              </button>
            </div>
          </div>
        </div>
        <SearchBar
          ref="searchBarRef"
          v-model="store.query"
          class="search-view__search search-view__search--dock"
          :placeholder="searchPlaceholder"
          @update:model-value="onQueryChange"
          @focus="handleSearchFocus"
          @blur="handleSearchBlur"
        />
      </div>
      <div class="search-view__dock-meta">
        <p
          v-if="!showColdStart"
          class="search-view__shortcut-hint"
          data-testid="search-shortcuts-hint"
        >
          / 或 Ctrl+K 聚焦搜索
        </p>
        <div class="search-view__dock-actions">
          <div
            v-if="isDevMode"
            class="search-view__menu-wrap"
          >
            <button
              ref="devToolsButtonRef"
              type="button"
              class="search-view__menu-button"
              data-action="toggle-dev-tools"
              aria-label="打开开发工具"
              @click="toggleDevToolsPopover"
            >
              <span aria-hidden="true">🔧</span>
            </button>
            <div
              v-if="showDevToolsPopover"
              ref="devToolsPopoverRef"
              class="search-view__dev-tools-popover ui-floating-panel"
            >
              <div class="search-view__dev-tools-header">
                <p class="search-view__dev-tools-title">
                  开发工具
                </p>
                <p class="search-view__dev-tools-copy">
                  仅用于开发调试和本地维护。
                </p>
              </div>
              <label class="search-view__dev-tools-toggle">
                <span class="search-view__dev-tools-section-title">开发调试模式</span>
                <span class="search-view__dev-tools-copy">显示结果排序构成与调试叠层。</span>
                <span class="search-view__dev-tools-switch">
                  <input
                    v-model="settings.devDebugMode"
                    data-action="toggle-debug-mode"
                    type="checkbox"
                  >
                  <span>{{ settings.devDebugMode ? "已开启" : "已关闭" }}</span>
                </span>
              </label>
              <div class="search-view__dev-tools-section">
                <div class="search-view__dev-tools-row">
                  <div>
                    <p class="search-view__dev-tools-section-title">
                      重新生成图像索引
                    </p>
                    <p class="search-view__dev-tools-copy">
                      模型或索引结构变更后，重建全部图片的索引信息。
                    </p>
                  </div>
                  <button
                    type="button"
                    class="search-view__dev-tools-action"
                    data-action="reindex-all"
                    :disabled="reindexing"
                    @click="startReindex"
                  >
                    {{ reindexing ? "索引中..." : "开始重建" }}
                  </button>
                </div>
                <div
                  v-if="reindexing"
                  class="search-view__dev-tools-progress"
                >
                  <div class="search-view__dev-tools-progress-bar">
                    <div
                      class="search-view__dev-tools-progress-fill"
                      :style="{ width: reindexProgressPercent + '%' }"
                    />
                  </div>
                  <span class="search-view__dev-tools-progress-text">
                    {{ `处理中 ${reindexCurrent}/${reindexTotal || "?"}` }}
                  </span>
                </div>
                <p
                  v-else-if="reindexDone"
                  class="search-view__dev-tools-progress-text"
                >
                  索引重建完成
                </p>
              </div>
              <div class="search-view__dev-tools-section search-view__dev-tools-section--danger">
                <div class="search-view__dev-tools-row">
                  <div>
                    <p class="search-view__dev-tools-section-title">
                      清空图库
                    </p>
                    <p class="search-view__dev-tools-copy">
                      仅用于开发时重置本地图库、索引和选择状态。
                    </p>
                  </div>
                  <button
                    type="button"
                    class="search-view__dev-tools-action search-view__dev-tools-action--danger"
                    data-action="clear-gallery"
                    :disabled="!canClearGallery"
                    @click="handleClearGallery"
                  >
                    {{ libraryStore.clearing ? "清空中..." : "清空图库" }}
                  </button>
                </div>
                <div
                  v-if="libraryStore.clearing"
                  class="search-view__dev-tools-progress"
                >
                  <div class="search-view__dev-tools-progress-bar">
                    <div
                      class="search-view__dev-tools-progress-fill search-view__dev-tools-progress-fill--danger"
                      :style="{ width: clearGalleryProgressPercent + '%' }"
                    />
                  </div>
                  <span class="search-view__dev-tools-progress-text">
                    {{ `处理中 ${libraryStore.clearCurrent}/${libraryStore.clearTotal || "?"}` }}
                  </span>
                </div>
              </div>
            </div>
          </div>
          <div class="search-view__menu-wrap">
            <button
              type="button"
              class="search-view__menu-button search-view__menu-button--gallery"
              data-action="open-gallery-management"
              aria-label="打开图库管理"
              @click="openGalleryManagement"
            >
              {{ galleryEntryLabel }}
              <span
                v-if="pendingTaskCount > 0"
                class="search-view__gallery-badge"
                data-testid="gallery-pending-badge"
              >
                {{ pendingTaskCount }}
              </span>
            </button>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { onMounted, onBeforeUnmount, computed, ref, watch, nextTick, inject } from "vue";
import { listen } from "@tauri-apps/api/event";
import { open, confirm } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { routerKey, type Router } from "vue-router";
import SearchBar from "@/components/SearchBar.vue";
import ImageGrid from "@/components/ImageGrid.vue";
import DetailModal from "@/components/DetailModal.vue";
import QuickPreviewModal from "@/components/QuickPreviewModal.vue";
import { useSearch } from "@/composables/useSearch";
import { useClipboard } from "@/composables/useClipboard";
import { showToast } from "@/composables/useToast";
import { useSettingsStore } from "@/stores/settings";
import { useLibraryStore, type ImportEntry } from "@/stores/library";
import { useTaskRecoveryStore } from "@/stores/taskRecovery";
import { getRelevanceLevel } from "@/utils/relevance";
import type { SearchResult } from "@/stores/search";
import { isDevelopmentMode } from "@/utils/runtime";
import coldStartPreview1 from "@/assets/cold-start-previews/preview-1.jpg";
import coldStartPreview2 from "@/assets/cold-start-previews/preview-2.jpg";
import coldStartPreview3 from "@/assets/cold-start-previews/preview-3.jpg";
import coldStartPreview4 from "@/assets/cold-start-previews/preview-4.jpg";

const { store, debouncedSearch } = useSearch();
const { copyImage } = useClipboard();
const settings = useSettingsStore();
const libraryStore = useLibraryStore();
const recoveryStore = useTaskRecoveryStore();
const router = inject<Router | undefined>(routerKey, undefined);
const cancelDebouncedSearch = debouncedSearch as typeof debouncedSearch & { cancel?: () => void };

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
  pendingTaskCount?: number;
  recentSearches: { query: string; updatedAt: number }[];
  recentUsed: HomeImage[];
  frequentUsed: HomeImage[];
}

interface RecoveryAction {
  kind: "gallery" | "role-library";
  label: string;
  targetView?: "recent" | "issues";
}

interface SearchBarExpose {
  focusAndSelect: () => void;
}

const HIGH_CONFIDENCE_BATCH_SIZE = 12;
const RESULT_FETCH_STEP = 30;
const POST_IMPORT_PENDING_KEY = "search-view-post-import-pending";
const exampleQueries = ["笑死", "猫猫无语", "华强买瓜"];
const coldStartPreviewCards = [
  { id: "preview-1", height: 188, src: coldStartPreview1, tagWidth: "1.1rem", primaryWidth: "4.6rem", secondaryWidth: "1.7rem" },
  { id: "preview-2", height: 188, src: coldStartPreview2, tagWidth: "0.92rem", primaryWidth: "4.2rem", secondaryWidth: "2rem" },
  { id: "preview-3", height: 188, src: coldStartPreview3, tagWidth: "1.24rem", primaryWidth: "4.9rem", secondaryWidth: "1.4rem" },
  { id: "preview-4", height: 188, src: coldStartPreview4, tagWidth: "1rem", primaryWidth: "4.35rem", secondaryWidth: "1.85rem" },
];
const visibleRelevantCount = ref(HIGH_CONFIDENCE_BATCH_SIZE);
const showSecondaryResults = ref(false);
const loadMoreTrigger = ref<HTMLElement | null>(null);
const searchBarRef = ref<SearchBarExpose | null>(null);
const devToolsPopoverRef = ref<HTMLElement | null>(null);
const devToolsButtonRef = ref<HTMLElement | null>(null);
const importButtonRef = ref<HTMLButtonElement | null>(null);
const importMenuRef = ref<HTMLElement | null>(null);
const focusedResultIndex = ref(-1);
const previewImageId = ref<string | null>(null);
const homeState = ref<HomeState | null>(null);
const homeLoading = ref(false);
const homeLoadFailed = ref(false);
const searchFocused = ref(false);
const showDevToolsPopover = ref(false);
const showImportMenu = ref(false);
const coldStartHint = ref("");
const showPostImportPrompt = ref(false);
const reindexing = ref(false);
const reindexCurrent = ref(0);
const reindexTotal = ref(0);
const reindexDone = ref(false);
const isDevMode = isDevelopmentMode();
let searchBlurTimer: number | null = null;
let loadMoreObserver: IntersectionObserver | null = null;
let unlistenReindexProgress: (() => void) | null = null;

const isHomeMode = computed(() => !store.query.trim());

const searchPlaceholder = computed(() => "搜台词、角色、动作、场景");

const showColdStart = computed(() =>
  isHomeMode.value
  && !homeLoading.value
  && !homeLoadFailed.value
  && (homeState.value?.imageCount ?? 0) === 0
);

const recentSearches = computed(() => homeState.value?.recentSearches ?? []);
const pendingTaskCount = computed(() =>
  libraryStore.indexing || recoveryStore.activeRecovery
    ? 0
    : Math.max(
      recoveryStore.loaded ? recoveryStore.pendingCount : 0,
      homeState.value?.pendingTaskCount ?? 0
    )
);
const inProgressIndicator = computed(() => recoveryStore.inProgressIndicator);
const galleryEntryLabel = computed(() => inProgressIndicator.value?.label ?? "图库");
const showSearchAssistPanel = computed(() =>
  searchFocused.value && isHomeMode.value && (recentSearches.value.length > 0 || exampleQueries.length > 0)
);

function getPendingPostImportFlag() {
  try {
    return sessionStorage.getItem(POST_IMPORT_PENDING_KEY) === "1";
  } catch {
    return false;
  }
}

function setPendingPostImportFlag() {
  try {
    sessionStorage.setItem(POST_IMPORT_PENDING_KEY, "1");
  } catch {
    // ignore session storage failures
  }
}

function clearPendingPostImportFlag() {
  try {
    sessionStorage.removeItem(POST_IMPORT_PENDING_KEY);
  } catch {
    // ignore session storage failures
  }
}

function syncPostImportPrompt(nextImageCount: number) {
  if (nextImageCount > 0 && getPendingPostImportFlag()) {
    showPostImportPrompt.value = true;
    return;
  }

  showPostImportPrompt.value = false;
}

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

const quickPickImages = computed<SearchResult[]>(() => {
  const merged = [...recentUsedImages.value, ...homeImages.value];
  const seen = new Set<string>();
  return merged.filter((image) => {
    if (seen.has(image.id)) return false;
    seen.add(image.id);
    return true;
  });
});

const detailImages = computed<SearchResult[]>(() => {
  if (!isHomeMode.value) {
    return store.results;
  }

  return quickPickImages.value;
});

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

const focusedResultId = computed(() => visibleResults.value[focusedResultIndex.value]?.id ?? null);

const previewSourceImages = computed<SearchResult[]>(() =>
  isHomeMode.value ? detailImages.value : visibleResults.value
);

const previewImage = computed(() =>
  previewSourceImages.value.find((item) => item.id === previewImageId.value) ?? null
);

const previewIndex = computed(() =>
  previewSourceImages.value.findIndex((item) => item.id === previewImageId.value)
);

const canPreviewPrev = computed(() => previewIndex.value > 0);

const canPreviewNext = computed(() =>
  previewIndex.value > -1 && previewIndex.value < previewSourceImages.value.length - 1
);

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
  !!store.query.trim() && !store.error && store.results.length > 0 && mediumConfidenceCount.value === 0
);

const showZeroResultHint = computed(() =>
  !!store.query.trim() && !store.loading && !store.error && store.results.length === 0
);

const showLowRelevanceStopNotice = computed(() =>
  !!store.query.trim()
  && !store.error
  && mediumConfidenceCount.value > 0
  && secondaryResultsCount.value > 0
);

const showSearchErrorHint = computed(() =>
  !!store.query.trim() && !store.loading && !!store.error
);

const showIncompleteResultsHint = computed(() =>
  !!store.query.trim() && !isHomeMode.value && !!inProgressIndicator.value
);

const looksLikeRoleQuery = computed(() => isLikelyRoleQuery(store.query));

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

const showResultShortcutsHint = computed(() =>
  !isHomeMode.value && visibleResults.value.length > 0 && !previewImageId.value
);
const reindexProgressPercent = computed(() =>
  reindexTotal.value > 0 ? (reindexCurrent.value / reindexTotal.value) * 100 : 0
);
const clearGalleryProgressPercent = computed(() =>
  libraryStore.clearTotal > 0 ? (libraryStore.clearCurrent / libraryStore.clearTotal) * 100 : 0
);
const canClearGallery = computed(() =>
  libraryStore.images.length > 0 && !libraryStore.clearing && !libraryStore.indexing
);

const searchSummaryTitle = computed(() => {
  const query = store.query.trim();
  if (!query) return "";

  if (store.loading) {
    return `正在找“${query}”相关的图`;
  }
  if (showSearchErrorHint.value) {
    return "这次没搜出来，换个词再试试";
  }
  if (showZeroResultHint.value) {
    return `没找到“${query}”相关的图`;
  }
  if (showLowConfidenceHint.value) {
    return `没找到和“${query}”更接近的图`;
  }
  if (visibleResults.value.length <= 2) {
    return `找到 ${visibleResults.value.length} 张更接近“${query}”的图`;
  }
  return `找到 ${visibleResults.value.length} 张和“${query}”相关的图`;
});

const searchSummaryMeta = computed(() => {
  if (showSearchErrorHint.value) {
    return pendingTaskCount.value > 0
      ? "可以重试，或先去图库继续导入未完成任务。"
      : "可以重试，或先查看异常图片。";
  }
  if (showZeroResultHint.value) {
    if (pendingTaskCount.value > 0) {
      return "换个说法试试，或先去图库继续导入未完成任务。";
    }
    if (looksLikeRoleQuery.value) {
      return "按角色名搜不到时，可以补几张示例图帮助系统认识这个角色。";
    }
    return "换个说法试试，先从图里的原文、角色名、动作或场景词开始搜。";
  }
  if (showLowConfidenceHint.value) {
    return pendingTaskCount.value > 0
      ? "先去图库继续导入未完成任务，或查看异常图片。"
      : "先试试图里的原文、角色名或更短一点的关键词。";
  }
  if (showLowRelevanceStopNotice.value) {
    return "";
  }
  return "";
});

const feedbackTitle = computed(() => {
  if (showSearchErrorHint.value) {
    return "这次搜索没成功";
  }
  if (showZeroResultHint.value) {
    return "换个说法再试试";
  }
  if (showLowConfidenceHint.value) {
    return "先别急着翻不太像的结果";
  }
  if (mediumConfidenceCount.value > highConfidenceCount.value) {
    return `${mediumConfidenceCount.value} 张更接近的图已排在前面`;
  }
  return `${highConfidenceCount.value} 张更接近的图已全部显示`;
});

const feedbackText = computed(() => {
  if (showSearchErrorHint.value) {
    return pendingTaskCount.value > 0
      ? "可以重试，或先去图库继续导入未完成任务。"
      : "可以重试，或先查看异常图片。";
  }
  if (showZeroResultHint.value) {
    if (pendingTaskCount.value > 0) {
      return "可以先去图库继续导入未完成任务，再回来搜索。";
    }
    if (looksLikeRoleQuery.value) {
      return "如果图片里没字、模型也认不出这个角色，可以补几张角色示例图。";
    }
    return "可以从图片里的原文、角色名、动作或场景词开始搜。";
  }
  if (showLowConfidenceHint.value) {
    return pendingTaskCount.value > 0
      ? "可以先去图库继续导入未完成任务，或先查看异常图片。"
      : "如果你愿意，也可以展开看看其余候选，再决定要不要换词。";
  }
  if (showSecondaryResults.value) {
    return secondaryResultsCount.value > 0
      ? `后面的 ${secondaryResultsCount.value} 张属于补充结果，相关性会弱一些。`
      : "";
  }
  if (mediumConfidenceCount.value > highConfidenceCount.value) {
    return `最像的 ${highConfidenceCount.value} 张已经排在最前面。`;
  }
  return "";
});

const loadMoreHint = computed(() =>
  store.loading ? "正在加载更多相关结果..." : "继续下滑查看更多相关结果"
);

const emptyMessage = computed(() =>
  showSearchErrorHint.value
    ? "搜索暂时失败，请稍后重试"
    : showLowConfidenceHint.value
    ? "没找到足够相关的结果，试试更具体的描述"
    : libraryStore.images.length === 0
      ? "还没有图片哦，点击添加开始使用吧"
      : "没找到相关图片，试试其他描述？"
);

function resetResultView() {
  visibleRelevantCount.value = HIGH_CONFIDENCE_BATCH_SIZE;
  showSecondaryResults.value = false;
  focusedResultIndex.value = -1;
  previewImageId.value = null;
}

async function fetchHomeState() {
  homeLoading.value = true;
  try {
    const nextHomeState = await invoke<HomeState>("get_home_state");
    homeState.value = nextHomeState;
    homeLoadFailed.value = false;
    syncPostImportPrompt(nextHomeState.imageCount);
  } catch {
    homeState.value = null;
    homeLoadFailed.value = true;
    showPostImportPrompt.value = false;
  } finally {
    homeLoading.value = false;
  }
}

function onQueryChange(val: string) {
  resetResultView();
  if (!val.trim()) {
    searchFocused.value = false;
    cancelDebouncedSearch.cancel?.();
    store.results = [];
    void fetchHomeState();
    return;
  }
  coldStartHint.value = "";
  showPostImportPrompt.value = false;
  clearPendingPostImportFlag();
  debouncedSearch(val);
}

function applyExampleQuery(query: string) {
  if (showColdStart.value) {
    coldStartHint.value = `先导入表情包，导入后就能搜“${query}”这类词`;
    importButtonRef.value?.focus();
    return;
  }

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
  if (showColdStart.value && targetView === "recent") {
    setPendingPostImportFlag();
  }
  showImportMenu.value = false;
  settings.currentWindowMode = "expanded";
  if (router) {
    void router.push({ path: "/library", query: { view: targetView } });
    return;
  }

  const search = new URLSearchParams(window.location.search);
  search.set("view", targetView);
  window.history.pushState({}, "", `/library?${search.toString()}`);
}

function goToPrivateRoleLibrary() {
  showImportMenu.value = false;
  settings.currentWindowMode = "expanded";
  if (router) {
    void router.push("/private-role-maintenance");
    return;
  }

  window.history.pushState({}, "", "/private-role-maintenance");
}

const primaryRecoveryAction = computed<RecoveryAction | null>(() => {
  if (!(showSearchErrorHint.value || showZeroResultHint.value || showLowConfidenceHint.value)) {
    return null;
  }

  if (pendingTaskCount.value > 0) {
    return {
      kind: "gallery",
      label: "去图库继续导入",
      targetView: "recent",
    };
  }

  if (showSearchErrorHint.value || showLowConfidenceHint.value) {
    return {
      kind: "gallery",
      label: "查看异常图片",
      targetView: "issues",
    };
  }

  if (showZeroResultHint.value && looksLikeRoleQuery.value) {
    return {
      kind: "role-library",
      label: "维护角色示例图",
    };
  }

  if (showZeroResultHint.value) {
    return {
      kind: "gallery",
      label: "查看最近新增",
      targetView: "recent",
    };
  }

  return null;
});

function runPrimaryRecoveryAction() {
  const action = primaryRecoveryAction.value;
  if (!action) return;

  if (action.kind === "role-library") {
    goToPrivateRoleLibrary();
    return;
  }

  goToGalleryManagement(action.targetView ?? "recent");
}

function closeDevToolsPopover() {
  showDevToolsPopover.value = false;
}

function closeImportMenu() {
  showImportMenu.value = false;
}

function toggleDevToolsPopover() {
  showDevToolsPopover.value = !showDevToolsPopover.value;
}

function toggleImportMenu() {
  showImportMenu.value = !showImportMenu.value;
}

async function importFromEntries(entries: ImportEntry[]) {
  if (entries.length === 0) return;
  setPendingPostImportFlag();
  coldStartHint.value = "";
  closeImportMenu();
  await libraryStore.importEntries(entries);
  await fetchHomeState();
}

async function handleImportImages() {
  const selected = await open({
    multiple: true,
    filters: [{ name: "图片", extensions: ["jpg", "jpeg", "png", "gif", "webp"] }],
  });
  if (!selected) {
    closeImportMenu();
    return;
  }
  const paths = Array.isArray(selected) ? selected : [selected];
  await importFromEntries(
    paths.map((path) => ({
      kind: "file",
      path,
    }))
  );
}

async function handleImportFolder() {
  const selected = await open({ directory: true });
  if (!selected) {
    closeImportMenu();
    return;
  }
  const path = Array.isArray(selected) ? selected[0] : selected;
  await importFromEntries([{ kind: "directory", path }]);
}

async function openGalleryManagement() {
  settings.currentWindowMode = "expanded";
  closeDevToolsPopover();
  if (inProgressIndicator.value) {
    if (router) {
      await router.push("/library");
      return;
    }

    window.history.pushState({}, "", "/library");
    return;
  }
  if (pendingTaskCount.value > 0) {
    goToGalleryManagement("recent");
    return;
  }
  if (router) {
    await router.push("/library");
    return;
  }

  window.history.pushState({}, "", "/library");
}

function isLikelyRoleQuery(raw: string) {
  const query = raw.trim();
  if (!query) return false;
  if (query.includes(" ") || query.length > 4) return false;
  if (/[0-9]/.test(query)) return false;

  const nonRoleHints = ["表情", "台词", "动作", "场景", "猫猫", "狗狗", "笑", "哭", "生气", "无语", "打工"];
  if (nonRoleHints.some((item) => query.includes(item))) {
    return false;
  }

  return /^[\u4e00-\u9fa5A-Za-z]+$/.test(query);
}

function cleanupReindexProgressListener() {
  unlistenReindexProgress?.();
  unlistenReindexProgress = null;
}

async function startReindex() {
  if (reindexing.value) return;

  cleanupReindexProgressListener();
  reindexing.value = true;
  reindexDone.value = false;
  reindexCurrent.value = 0;
  reindexTotal.value = 0;

  unlistenReindexProgress = await listen<{ current: number; total: number }>("reindex-progress", (event) => {
    reindexCurrent.value = event.payload.current;
    reindexTotal.value = event.payload.total;
    if (event.payload.total > 0 && event.payload.current >= event.payload.total) {
      reindexing.value = false;
      reindexDone.value = true;
      cleanupReindexProgressListener();
    }
  });

  try {
    await invoke("reindex_all");
  } catch (error) {
    console.error("reindex_all failed:", error);
    reindexing.value = false;
    cleanupReindexProgressListener();
  }
}

async function handleClearGallery() {
  const ok = await confirm("确认清空整个图库吗？此操作会删除所有图片及索引数据，且不可撤销。", {
    title: "清空图库",
  });
  if (!ok) return;
  await libraryStore.clearGallery();
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

function handlePointerDown(event: MouseEvent) {
  const target = event.target as Node | null;
  if (
    showDevToolsPopover.value
    && target
    && !devToolsPopoverRef.value?.contains(target)
    && !devToolsButtonRef.value?.contains(target)
  ) {
    closeDevToolsPopover();
  }
  if (
    showImportMenu.value
    && target
    && !importMenuRef.value?.contains(target)
    && !importButtonRef.value?.contains(target)
  ) {
    closeImportMenu();
  }
}

function focusSearchInput() {
  searchBarRef.value?.focusAndSelect();
  searchFocused.value = true;
}

function dismissPostImportPrompt() {
  showPostImportPrompt.value = false;
  clearPendingPostImportFlag();
}

function isEditableTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) return false;
  return target.isContentEditable
    || target.tagName === "INPUT"
    || target.tagName === "TEXTAREA"
    || target.tagName === "SELECT";
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

function moveFocusedResult(step: 1 | -1) {
  if (!visibleResults.value.length) return;

  if (focusedResultIndex.value === -1) {
    focusedResultIndex.value = step > 0 ? 0 : visibleResults.value.length - 1;
    return;
  }

  const nextIndex = focusedResultIndex.value + step;
  focusedResultIndex.value = Math.max(0, Math.min(visibleResults.value.length - 1, nextIndex));
}

function syncFocusedResultFromPreview(id: string) {
  if (isHomeMode.value) return;
  const nextIndex = visibleResults.value.findIndex((item) => item.id === id);
  if (nextIndex !== -1) {
    focusedResultIndex.value = nextIndex;
  }
}

function openQuickPreview(id: string) {
  previewImageId.value = id;
  syncFocusedResultFromPreview(id);
}

function closeQuickPreview() {
  previewImageId.value = null;
}

function moveQuickPreview(step: 1 | -1) {
  if (previewIndex.value === -1) return;
  const nextIndex = previewIndex.value + step;
  if (nextIndex < 0 || nextIndex >= previewSourceImages.value.length) return;
  const nextImage = previewSourceImages.value[nextIndex];
  previewImageId.value = nextImage.id;
  syncFocusedResultFromPreview(nextImage.id);
}

async function handleQuickPreviewCopy(id: string) {
  await copyImage(id);
  handleSearchImageCopied();
}

async function handleQuickPreviewReveal(id: string) {
  await invoke("reveal_in_finder", { id }).catch((error) => {
    if (String(error).includes("原文件已丢失")) {
      showToast("原文件已丢失，无法定位", "error", 1500);
    }
  });
}

function handleGlobalKeydown(event: KeyboardEvent) {
  const shouldFocusSearch = event.key === "/" || ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k");
  if (shouldFocusSearch) {
    if (event.key === "/" && isEditableTarget(event.target)) return;
    event.preventDefault();
    focusSearchInput();
    return;
  }

  if (previewImageId.value) {
    if (event.key === "Escape") {
      event.preventDefault();
      closeQuickPreview();
      return;
    }

    if (event.key === "Enter" && previewImage.value) {
      event.preventDefault();
      void handleQuickPreviewCopy(previewImage.value.id);
      return;
    }

    if (event.key === "ArrowRight" || event.key === "ArrowDown") {
      event.preventDefault();
      moveQuickPreview(1);
      return;
    }

    if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
      event.preventDefault();
      moveQuickPreview(-1);
      return;
    }
  }

  if (event.key === "Escape" && showSearchAssistPanel.value) {
    event.preventDefault();
    searchFocused.value = false;
    return;
  }

  if (isHomeMode.value) return;

  if (event.key === "Escape") {
    return;
  }

  if (detailId.value || !visibleResults.value.length) return;

  if (event.key === "ArrowRight" || event.key === "ArrowDown") {
    event.preventDefault();
    moveFocusedResult(1);
    return;
  }

  if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
    event.preventDefault();
    moveFocusedResult(-1);
    return;
  }

  const focusedResult = visibleResults.value[focusedResultIndex.value];
  if (!focusedResult) return;

  if (event.key === "Enter") {
    event.preventDefault();
    void handleSearchImageCopied();
    const target = document.querySelector<HTMLElement>(".image-card--focused");
    target?.click();
    return;
  }

  if (event.key === " " || event.key === "Spacebar") {
    event.preventDefault();
    openQuickPreview(focusedResult.id);
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

watch(showColdStart, (nextColdStart) => {
  if (!nextColdStart) {
    coldStartHint.value = "";
    closeImportMenu();
  }
});

watch(() => store.results.length, (nextLen, prevLen) => {
  if (nextLen < prevLen) {
    visibleRelevantCount.value = HIGH_CONFIDENCE_BATCH_SIZE;
  }
});

const detailId = ref<string | null>(null);

function openDetail(id: string) {
  detailId.value = id;
  previewImageId.value = null;
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

async function handleDeleteFromGrid(id: string) {
  const ok = await confirm("确定要删除这张图片吗？此操作不可撤销。", { title: "删除图片" });
  if (!ok) return;

  await invoke("delete_image", { id });
  store.results = store.results.filter((img) => img.id !== id);
  libraryStore.images = libraryStore.images.filter((img) => img.id !== id);

  if (detailId.value === id) {
    detailId.value = null;
  }

  if (isHomeMode.value) {
    await fetchHomeState();
  }
}

onMounted(async () => {
  await Promise.all([
    fetchHomeState(),
    recoveryStore.fetchPendingTasks(true),
  ]);
  void libraryStore.fetchImages();
  await nextTick();
  attachLoadMoreObserver();
  document.addEventListener("keydown", handleGlobalKeydown);
  document.addEventListener("mousedown", handlePointerDown);
});

onBeforeUnmount(() => {
  if (searchBlurTimer !== null) {
    window.clearTimeout(searchBlurTimer);
  }
  cleanupReindexProgressListener();
  loadMoreObserver?.disconnect();
  document.removeEventListener("keydown", handleGlobalKeydown);
  document.removeEventListener("mousedown", handlePointerDown);
});
</script>

<style scoped>
.search-view {
  display: flex;
  flex-direction: column;
  min-height: 0;
  height: 100%;
  gap: 0.6rem;
  padding-top: 0.55rem;
}

.search-view__body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  padding-right: 0.15rem;
  padding-bottom: 0.75rem;
  scrollbar-gutter: stable;
}

.search-view__top-banner {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.search-view__top-banner--cold {
  gap: 0.55rem;
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

.search-view__search--dock {
  margin: 0;
}

.search-view__shortcut-hint,
.search-view__result-shortcuts {
  margin: 0;
  font-size: 0.78rem;
  color: var(--ui-text-secondary);
}

.search-view__dock-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.search-view__result-shortcuts {
  margin-top: -0.1rem;
}

.search-view__dock-actions {
  display: inline-flex;
  align-items: center;
  gap: 0.5rem;
  margin-left: auto;
}
.search-view__menu-wrap {
  position: relative;
  flex-shrink: 0;
}

.search-view__menu-wrap--solo {
  margin-left: auto;
}

.search-view__menu-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 2rem;
  height: 2rem;
  padding: 0 0.55rem;
  border: 1px solid var(--ui-border-subtle);
  border-radius: 999px;
  background: color-mix(in srgb, var(--ui-bg-surface-strong) 92%, white);
  color: var(--ui-text-secondary);
  cursor: pointer;
  transition:
    background-color 120ms ease,
    border-color 120ms ease,
    color 120ms ease;
}

.search-view__menu-button:hover {
  background: var(--ui-bg-hover);
  border-color: var(--ui-border-strong);
  color: var(--ui-text-primary);
}

.search-view__menu-button--gallery {
  position: relative;
  min-width: auto;
  padding-inline: 0.95rem;
  font-size: 0.82rem;
  font-weight: 600;
}

.search-view__gallery-badge {
  position: absolute;
  top: -0.3rem;
  right: -0.3rem;
  min-width: 1.05rem;
  height: 1.05rem;
  padding: 0 0.28rem;
  border-radius: 999px;
  background: #b42318;
  color: #fff;
  font-size: 0.7rem;
  font-weight: 700;
  line-height: 1.05rem;
  text-align: center;
}

.search-view__dev-tools-popover {
  position: absolute;
  right: 0;
  bottom: calc(100% + 0.45rem);
  width: min(22rem, calc(100vw - 2rem));
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  z-index: 35;
}
.search-view__dev-tools-header,
.search-view__dev-tools-section,
.search-view__dev-tools-toggle {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}
.search-view__dev-tools-title,
.search-view__dev-tools-copy,
.search-view__dev-tools-section-title,
.search-view__dev-tools-progress-text {
  margin: 0;
}
.search-view__dev-tools-title,
.search-view__dev-tools-section-title {
  font-size: 0.84rem;
  font-weight: 700;
  color: var(--ui-text-primary);
}
.search-view__dev-tools-copy,
.search-view__dev-tools-progress-text {
  font-size: 0.74rem;
  line-height: 1.45;
  color: var(--ui-text-secondary);
}
.search-view__dev-tools-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
}
.search-view__dev-tools-toggle {
  padding: 0.75rem;
  border: 1px solid var(--ui-border-subtle);
  border-radius: 0.95rem;
  background: rgba(255, 255, 255, 0.64);
}
.search-view__dev-tools-switch {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.78rem;
  color: var(--ui-text-primary);
}
.search-view__dev-tools-section {
  padding: 0.75rem;
  border: 1px solid var(--ui-border-subtle);
  border-radius: 0.95rem;
  background: rgba(255, 255, 255, 0.64);
}
.search-view__dev-tools-section--danger {
  border-color: rgba(180, 63, 46, 0.22);
  background: rgba(255, 245, 242, 0.78);
}
.search-view__dev-tools-action {
  flex-shrink: 0;
  min-width: 5.2rem;
  padding: 0.52rem 0.8rem;
  border: 1px solid var(--ui-border-subtle);
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.9);
  color: var(--ui-text-primary);
  cursor: pointer;
}
.search-view__dev-tools-action:hover {
  background: var(--ui-bg-hover);
}
.search-view__dev-tools-action:disabled {
  cursor: default;
  opacity: 0.5;
}
.search-view__dev-tools-action--danger {
  border-color: rgba(180, 63, 46, 0.22);
  color: #a33423;
}
.search-view__dev-tools-progress {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}
.search-view__dev-tools-progress-bar {
  height: 0.36rem;
  border-radius: 999px;
  overflow: hidden;
  background: rgba(149, 157, 179, 0.22);
}
.search-view__dev-tools-progress-fill {
  height: 100%;
  background: var(--ui-accent);
  transition: width 120ms ease;
}
.search-view__dev-tools-progress-fill--danger {
  background: #d24f3d;
}

.search-assist-panel {
  position: absolute;
  bottom: calc(100% + 0.55rem);
  left: 0;
  right: 0;
  z-index: 30;
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  padding: 0.5rem;
  max-height: min(230px, 38vh);
  overflow-y: auto;
}
.search-assist-panel__section {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
}

.search-assist-panel__label {
  margin: 0;
  font-size: 0.72rem;
  color: var(--ui-text-secondary);
}

.search-assist-panel__row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}

.search-assist-panel__item {
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  padding: 0.15rem;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.75);
  border: 1px solid var(--ui-border-subtle);
}

.search-assist-panel__query,
.search-assist-panel__chip,
.search-assist-panel__delete {
  border: none;
  background: transparent;
  cursor: pointer;
}

.search-assist-panel__query,
.search-assist-panel__chip {
  padding: 0.35rem 0.6rem;
  border-radius: 999px;
  color: var(--ui-text-primary);
}

.search-assist-panel__chip {
  background: color-mix(in srgb, var(--ui-bg-surface-strong) 92%, white);
  border: 1px solid var(--ui-border-subtle);
}

.search-assist-panel__query:hover,
.search-assist-panel__chip:hover {
  background: var(--ui-bg-hover);
}

.search-assist-panel__delete {
  padding: 0.3rem 0.5rem;
  border-radius: 999px;
  color: var(--ui-text-secondary);
}

.search-assist-panel__delete:hover {
  background: var(--ui-bg-hover);
  color: var(--ui-text-primary);
}
.home-landing {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  padding: 0.1rem 0.05rem 0.4rem;
}
.home-landing--cold {
  flex: 1;
  min-height: 0;
  padding: 0;
  border: none;
  border-radius: 0;
  background: transparent;
  box-shadow: none;
}
.home-cold-start {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  min-height: 0;
}
.home-preview-wrap {
  position: relative;
  padding-top: 0.7rem;
}
.home-preview-wrap__banner {
  position: absolute;
  top: 0;
  left: 50%;
  z-index: 2;
  transform: translateX(-50%);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 11.5rem;
  padding: 0.34rem 0.78rem;
  border: 1px solid rgba(219, 205, 180, 0.84);
  border-radius: 999px;
  background: rgba(255, 251, 245, 0.88);
  box-shadow: 0 8px 20px rgba(95, 77, 48, 0.08);
  font-size: 0.74rem;
  color: var(--ui-text-secondary);
  text-align: center;
}
.home-preview-cloud {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.8rem;
  padding: 0.3rem 0 0;
  opacity: 0.74;
}
.home-preview-cloud__frame {
  display: flex;
  flex-direction: column;
  min-height: var(--preview-height);
  border-radius: 1rem;
  overflow: hidden;
  border: 1px solid rgba(215, 203, 182, 0.88);
  background: rgba(255, 255, 255, 0.9);
  box-shadow: 0 10px 24px rgba(95, 77, 48, 0.08);
}
.home-preview-cloud__media {
  position: relative;
  aspect-ratio: 1 / 1;
  min-height: 0;
  padding: 0.18rem;
  background: rgba(248, 244, 236, 0.78);
}
.home-preview-cloud__image {
  width: 100%;
  height: 100%;
  min-height: 0;
  border-radius: 0.72rem;
  object-fit: cover;
  display: block;
  filter: saturate(0.92);
}
.home-preview-cloud__info {
  display: flex;
  flex-direction: column;
  gap: 0.34rem;
  padding: 0.48rem 0.52rem 0.56rem;
  background: rgba(255, 255, 255, 0.92);
}
.home-preview-cloud__header {
  display: flex;
  align-items: center;
  gap: 0.22rem;
}
.home-preview-cloud__tag {
  width: var(--preview-tag-width);
  height: 0.46rem;
  border-radius: 999px;
  background: rgba(173, 218, 180, 0.72);
}
.home-preview-cloud__pills {
  display: flex;
  flex-wrap: wrap;
  gap: 0.24rem;
}
.home-preview-cloud__pill {
  display: block;
  height: 0.44rem;
  width: var(--preview-secondary-width);
  border-radius: 0.28rem;
  background: rgba(123, 112, 96, 0.1);
}
.home-preview-cloud__pill--wide {
  width: var(--preview-primary-width);
}
.search-state-banner {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  gap: 0.45rem 0.7rem;
  padding: 0.68rem 0.78rem;
  border: 1px solid color-mix(in srgb, var(--ui-accent) 18%, var(--ui-border-subtle));
  border-radius: 0.95rem;
  background: rgba(255, 251, 245, 0.76);
}
.search-state-banner--summary {
  gap: 0.22rem;
  padding: 0.05rem 0 0.1rem;
  border: none;
  border-radius: 0;
  background: transparent;
}
.search-state-banner__copy {
  flex: 1 1 200px;
}
.search-state-banner__title,
.search-state-banner__text,
.search-state-banner__meta,
.search-state-banner__label,
.search-state-banner__footnote {
  margin: 0;
}
.search-state-banner__title {
  font-size: 0.84rem;
  font-weight: 700;
  color: var(--ui-text-primary);
}
.search-state-banner__text {
  margin-top: 0.18rem;
  font-size: 0.75rem;
  line-height: 1.45;
  color: var(--ui-text-secondary);
}
.search-state-banner__meta {
  font-size: 0.72rem;
  color: var(--ui-text-secondary);
}
.search-state-banner__actions,
.search-state-banner__examples {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.45rem;
}
.search-state-banner__actions {
  position: relative;
}
.search-state-banner__examples {
  flex: 1 1 100%;
}
.search-state-banner__label {
  font-size: 0.74rem;
  color: var(--ui-text-secondary);
}
.search-state-banner__chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}
.search-state-banner__action {
  padding: 0.44rem 0.92rem;
  border-radius: 999px;
  border: 1px solid transparent;
  cursor: pointer;
  font-size: 0.82rem;
}
.search-state-banner__action--primary {
  background: var(--ui-accent);
  color: #fff;
}
.search-state-banner__action--primary:hover {
  background: var(--ui-accent-hover);
}
.search-state-banner__import-menu {
  position: absolute;
  top: calc(100% + 0.5rem);
  right: 0;
  z-index: 6;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  min-width: 10rem;
  padding: 0.4rem;
}
.search-state-banner__import-action {
  border: 0;
  background: transparent;
  color: var(--ui-text-primary);
  text-align: left;
  border-radius: 0.8rem;
  padding: 0.65rem 0.8rem;
  cursor: pointer;
}
.search-state-banner__import-action:hover {
  background: var(--ui-bg-hover);
}
.search-state-banner__chip {
  border: 1px solid var(--ui-border-subtle);
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.88);
  color: var(--ui-text-primary);
  padding: 0.38rem 0.72rem;
  font-size: 0.82rem;
  cursor: pointer;
}
.search-state-banner__chip:hover {
  background: var(--ui-bg-hover);
}
.search-state-banner__hint {
  margin-top: -0.05rem;
  font-size: 0.76rem;
  line-height: 1.45;
  color: color-mix(in srgb, var(--ui-accent) 78%, #533500);
}
.search-state-banner__footnote {
  flex: 1 1 100%;
  font-size: 0.72rem;
  color: var(--ui-text-secondary);
}
.search-state-banner__close {
  border: none;
  background: transparent;
  color: var(--ui-text-secondary);
  font-size: 1rem;
  line-height: 1;
  cursor: pointer;
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
.home-section__title {
  margin: 0;
  font-size: 0.86rem;
  font-weight: 700;
  color: var(--ui-text-primary);
}
.result-feedback {
  display: flex;
  flex-direction: column;
  gap: 0.42rem;
  padding: 0.72rem 0.82rem;
  border-radius: 1rem;
  border: 1px solid rgba(219, 205, 180, 0.9);
  background: rgba(255, 250, 242, 0.85);
  color: var(--ui-text-primary);
  text-align: left;
}
.result-feedback--secondary {
  background: rgba(250, 247, 240, 0.82);
}
.result-feedback--warning {
  border-color: color-mix(in srgb, var(--ui-accent) 22%, var(--ui-border-subtle));
}
.result-feedback__title {
  margin: 0;
  font-size: 0.84rem;
  font-weight: 600;
  color: var(--ui-text-primary);
}
.result-feedback__text {
  margin: 0;
  font-size: 0.78rem;
  line-height: 1.5;
  color: var(--ui-text-secondary);
}
.result-feedback__guidance {
  display: flex;
  flex-wrap: wrap;
  gap: 0.45rem;
}
.result-feedback__guidance-item {
  padding: 0.34rem 0.62rem;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.88);
  border: 1px solid rgba(219, 205, 180, 0.86);
  color: var(--ui-text-secondary);
  font-size: 0.8rem;
}
.result-feedback__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}
.result-feedback__action {
  padding: 0.38rem 0.8rem;
  border: 1px solid color-mix(in srgb, var(--ui-accent) 28%, var(--ui-border-subtle));
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.78);
  color: var(--ui-text-primary);
  cursor: pointer;
  font-size: 0.8rem;
}
.result-feedback__action:hover {
  background: var(--ui-bg-hover);
}
.result-more-strip {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.7rem;
  padding: 0.72rem 0.82rem;
  border: 1px solid rgba(219, 205, 180, 0.82);
  border-radius: 1rem;
  background: rgba(255, 251, 245, 0.7);
}
.result-more-strip__copy {
  display: flex;
  flex-direction: column;
  gap: 0.16rem;
  min-width: 0;
}
.result-more-strip__title,
.result-more-strip__text {
  margin: 0;
}
.result-more-strip__title {
  font-size: 0.82rem;
  font-weight: 600;
  color: var(--ui-text-primary);
}
.result-more-strip__text {
  font-size: 0.76rem;
  color: var(--ui-text-secondary);
}
.result-more-strip__action {
  flex-shrink: 0;
  padding: 0.38rem 0.8rem;
  border: 1px solid color-mix(in srgb, var(--ui-accent) 28%, var(--ui-border-subtle));
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.82);
  color: var(--ui-text-primary);
  cursor: pointer;
  font-size: 0.8rem;
}
.result-more-strip__action:hover {
  background: var(--ui-bg-hover);
}
.load-more-trigger {
  padding: 1rem 0 0.5rem;
  text-align: center;
}
.load-more-trigger__text {
  margin: 0;
  font-size: 0.82rem;
  color: #888;
}

@media (max-width: 799px) {
  .search-view__dev-tools-row {
    flex-direction: column;
  }
  .search-view__dev-tools-action {
    width: 100%;
  }
}

.search-dock {
  flex-shrink: 0;
  position: sticky;
  bottom: 0;
  z-index: 25;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding-top: 0.35rem;
  padding-bottom: 0.2rem;
  background:
    linear-gradient(180deg, rgba(243, 241, 236, 0), rgba(243, 241, 236, 0.82) 18%, rgba(243, 241, 236, 0.98) 50%);
}

:deep(.search-view__search--dock .search-bar) {
  margin-bottom: 0;
}

:deep(.search-view__search--dock .ui-input-shell) {
  min-height: 60px;
  border-radius: 20px;
  border-color: color-mix(in srgb, var(--ui-accent) 28%, var(--ui-border-subtle));
  box-shadow:
    0 20px 40px rgba(48, 40, 25, 0.14),
    0 0 0 1px rgba(255, 255, 255, 0.55);
}

:deep(.search-view__search--dock .search-bar__input) {
  min-height: 52px;
  font-size: 0.98rem;
}

:deep(.search-view__search--dock .search-bar__icon) {
  background: rgba(183, 121, 31, 0.18);
}
</style>
