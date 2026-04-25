<template>
  <div class="library-view">
    <div class="page-head">
      <div class="page-head__left">
        <div
          v-if="!showColdLibraryEmptyState"
          class="toolbar"
        >
          <div class="toolbar__row">
            <div class="toolbar-actions">
              <button
                class="ui-button ui-button--primary"
                data-action="add-images"
                :disabled="store.indexing"
                @click="handleAdd"
              >
                导入图片
              </button>
              <button
                class="ui-button ui-button--secondary"
                data-action="add-folder"
                :disabled="store.indexing"
                @click="handleAddFolder"
              >
                导入文件夹
              </button>
              <button
                v-if="!selectionMode && visibleImages.length > 0"
                class="ui-button ui-button--secondary"
                data-action="enter-batch-delete"
                :disabled="managementActionsDisabled"
                @click="enterSelectionMode"
              >
                批量删除
              </button>
              <template v-if="selectionMode">
                <span class="selection-count">已选 {{ store.selectedIds.size }} 张</span>
                <button
                  class="ui-button ui-button--danger"
                  data-action="delete-selected"
                  :disabled="managementActionsDisabled || store.selectedIds.size === 0"
                  @click="handleDeleteSelected"
                >
                  删除选中
                </button>
                <button
                  class="ui-button ui-button--text"
                  data-action="cancel-selection"
                  @click="exitSelectionMode"
                >
                  取消
                </button>
              </template>
            </div>
          </div>
          <p
            v-if="managementActionsDisabled"
            class="toolbar-lock-reason"
          >
            导入处理中，完成后再整理图库。
          </p>
        </div>
      </div>
      <h2 class="page-head__title">图库管理</h2>
      <div class="page-head__meta">
        <div class="gallery-total">
          共 {{ store.total }} 张
        </div>
        <button
          v-if="showBackToSearch"
          type="button"
          class="page-head__back ui-button ui-button--secondary"
          data-action="back-to-search"
          @click="handleBackToSearch"
        >
          返回搜索
        </button>
      </div>
    </div>

    <section
      v-if="showMissingFilter"
      class="main-task-card main-task-card--missing"
    >
      <div class="main-task-card__copy">
        <p class="main-task-card__eyebrow">
          当前主任务
        </p>
        <h3>正在查看已发现的失效图片，共 {{ missingImages.length }} 张</h3>
        <p>
          这些图片的原文件可能已被移动或删除，处理完后可以随时回到全部图片。
        </p>
      </div>
      <div class="main-task-card__actions">
        <button
          class="ui-button ui-button--secondary"
          data-action="view-all-images"
          @click="handleViewAllImages"
        >
          查看全部图片
        </button>
        <button
          class="ui-button ui-button--danger"
          :disabled="managementActionsDisabled || clearingMissing"
          data-action="clear-missing"
          @click="handleClearMissing"
        >
          {{ clearingMissing ? "正在清除失效图片" : "清理失效图片" }}
        </button>
      </div>
    </section>
    <div
      v-else-if="showRecoveryBanner"
      class="main-task-card main-task-card--recovery"
    >
      <div class="main-task-card__copy">
        <p class="main-task-card__eyebrow">
          当前主任务
        </p>
        <h3>
          上次导入中断，还有 {{ recoveryStore.pendingCount }} 张图片未处理
        </h3>
        <p>
          继续处理这批图片。
        </p>
      </div>
      <div class="main-task-card__actions">
        <button
          class="ui-button ui-button--primary"
          data-action="resume-pending-tasks"
          :disabled="recoveryStore.resuming || recoveryStore.clearing"
          @click="handleResumePendingTasks"
        >
          {{ recoveryStore.resuming ? "继续导入中..." : "继续导入" }}
        </button>
        <button
          class="ui-button ui-button--secondary"
          data-action="clear-pending-tasks"
          :disabled="recoveryStore.resuming || recoveryStore.clearing"
          @click="handleClearPendingTasks"
        >
          {{ recoveryStore.clearing ? "放弃中..." : "放弃剩余图片" }}
        </button>
      </div>
    </div>
    <div
      v-else-if="inProgressIndicator"
      class="main-task-card main-task-card--progress"
    >
      <div class="main-task-card__copy">
        <p class="main-task-card__eyebrow">
          当前主任务
        </p>
        <h3>{{ inProgressIndicator.label }}</h3>
        <p>
          处理中时会暂时锁定会改变图库基数的治理操作。
        </p>
        <div class="progress-bar">
          <div
            class="progress-fill"
            :style="{ width: progressPercent + '%' }"
          />
        </div>
      </div>
    </div>
    <section
      v-else-if="showFailureSummary"
      :class="[
        'main-task-card',
        'main-task-card--summary',
        { 'main-task-card--summary-expanded': showImportFailures && displayedFailures.length > 0 },
      ]"
      data-section="latest-import-summary"
    >
      <div class="main-task-card__copy">
        <p class="main-task-card__eyebrow">
          {{ summaryEyebrow }}
        </p>
        <h3>{{ summaryTitle }}</h3>
        <p class="main-task-card__stats">
          <span>新增 {{ displayedSummaryStats.importedCount }}</span>
          <span>已存在 {{ displayedSummaryStats.duplicatedCount }}</span>
          <span class="main-task-card__stat main-task-card__stat--failure">失败 {{ displayedSummaryStats.failedCount }}</span>
        </p>
      </div>
      <ul
        v-if="showImportFailures && displayedFailures.length > 0"
        class="main-task-card__failures"
      >
        <li
          v-for="failure in displayedFailures"
          :key="failure.taskId"
          class="main-task-card__failure-item"
        >
          <p class="main-task-card__failure-name">
            {{ failure.fileName }}
          </p>
          <p class="main-task-card__failure-reason">
            {{ failure.userMessage || failure.errorMessage || '处理失败，请重试' }}
          </p>
        </li>
      </ul>
      <div class="main-task-card__actions">
        <button
          v-if="showImportFailures && displayedFailures.length > 0"
          class="ui-button ui-button--text"
          data-action="dismiss-import-summary"
          @click="handleAcknowledgeFailures"
        >
          知道了
        </button>
        <button
          v-else
          class="ui-button ui-button--primary"
          data-action="show-import-failures"
          @click="handleShowFailures"
        >
          查看失败项
        </button>
        <button
          v-if="showImportFailures && showRetryFailuresAction"
          class="ui-button ui-button--primary"
          data-action="retry-import-failures"
          :disabled="retryingFailures || managementActionsDisabled"
          @click="handleRetryImportFailures"
        >
          {{ retryingFailures ? "重试导入中..." : "重试导入" }}
        </button>
        <button
          v-else-if="displayedSummaryStats.importedCount > 0"
          class="ui-button ui-button--secondary"
          data-action="view-latest-imported"
          @click="handleViewLatestImported"
        >
          查看本次新增
        </button>
        <button
          v-if="recoveryStore.completedRecoverySummary"
          class="ui-button ui-button--text"
          data-action="dismiss-recovery-summary"
          @click="dismissRecoverySummary"
        >
          稍后再看
        </button>
      </div>
    </section>
    <section
      v-else-if="hasMissingImages"
      class="main-task-card main-task-card--missing"
    >
      <div class="main-task-card__copy">
        <p class="main-task-card__eyebrow">
          当前主任务
        </p>
        <h3>发现 {{ missingImages.length }} 张失效图片</h3>
        <p>
          这些图片的原文件可能已被移动或删除。
        </p>
      </div>
      <div class="main-task-card__actions">
        <button
          class="ui-button ui-button--primary"
          data-action="view-missing-images"
          :disabled="managementActionsDisabled"
          @click="handleViewMissingImages"
        >
          查看失效图片
        </button>
        <button
          class="ui-button ui-button--danger"
          :disabled="managementActionsDisabled || clearingMissing"
          data-action="clear-missing"
          @click="handleClearMissing"
        >
          {{ clearingMissing ? "正在清除失效图片" : "清理失效图片" }}
        </button>
      </div>
    </section>
    <section
      v-else-if="showSuccessSummary"
      class="main-task-card main-task-card--summary"
      data-section="latest-import-summary"
    >
      <div class="main-task-card__copy">
        <p class="main-task-card__eyebrow">
          {{ summaryEyebrow }}
        </p>
        <h3>{{ summaryTitle }}</h3>
        <p class="main-task-card__description">
          先看本次新增。
        </p>
        <p class="main-task-card__stats">
          <span>新增 {{ displayedSummaryStats.importedCount }}</span>
          <span>已存在 {{ displayedSummaryStats.duplicatedCount }}</span>
          <span>失败 {{ displayedSummaryStats.failedCount }}</span>
        </p>
      </div>
      <div class="main-task-card__actions">
        <button
          v-if="displayedSummaryStats.importedCount > 0"
          class="ui-button ui-button--primary"
          data-action="view-latest-imported"
          @click="handleViewLatestImported"
        >
          查看本次新增
        </button>
        <button
          v-if="recoveryStore.completedRecoverySummary"
          class="ui-button ui-button--text"
          data-action="dismiss-recovery-summary"
          @click="dismissRecoverySummary"
        >
          稍后再看
        </button>
      </div>
    </section>

    <section class="library-workbench">
      <section
        v-if="showLatestImportedTip"
        class="latest-import-position-tip"
        data-section="latest-import-position-tip"
      >
        已定位到本次新增图片，共 {{ latestImportedState?.count }} 张
      </section>
      <div class="gallery-panel__head">
        <div class="gallery-panel__title-group">
          <p class="usage-notice">
            原文件移动、重命名或删除后会失效
          </p>
        </div>
        <div
          v-if="showAdvancedCapabilities"
          class="advanced-capabilities"
          data-section="private-role-library-entry"
        >
          <button
            type="button"
            class="advanced-capabilities__action"
            data-action="open-private-role-library"
            aria-label="打开角色搜索增强"
            @click="openPrivateRoleLibrary"
          >
            角色搜索增强
          </button>
          <span
            class="advanced-capabilities__hint"
            role="img"
            aria-label="说明：按角色名搜不到时，可补几张示例图帮助识别"
            title="按角色名搜不到时，可补几张示例图帮助识别"
            tabindex="0"
          >?</span>
        </div>
      </div>
      <div
        ref="scrollContainer"
        :class="['gallery-scroll', { 'gallery-scroll--empty': showColdLibraryEmptyState }]"
        @scroll="handleScroll"
      >
        <div
          v-if="loadError && store.images.length === 0"
          class="gallery-feedback gallery-error"
        >
          <p>加载失败，请重试</p>
          <button
            class="ui-button ui-button--secondary ui-button--compact"
            data-action="retry-load"
            @click="retryLoad"
          >
            重试
          </button>
        </div>
        <section
          v-else-if="showColdLibraryEmptyState"
          class="library-empty-state"
          data-section="library-empty-state"
        >
          <div class="library-empty-state__intro">
            <h3>图库还没有图片</h3>
            <p>
              这里用于查看全部图片、补充导入、处理导入失败和失效文件。
            </p>
          </div>
          <div class="library-empty-state__preview">
            <p>导入后，全部图片会按入库时间显示在这里</p>
            <div class="library-empty-state__grid" aria-hidden="true">
              <div
                v-for="item in 6"
                :key="item"
                class="library-empty-state__card"
              >
                <div class="library-empty-state__thumb" />
                <span />
                <small />
              </div>
            </div>
            <div class="library-empty-state__actions">
              <button
                class="ui-button ui-button--primary"
                data-action="empty-add-images"
                :disabled="store.indexing"
                @click="handleAdd"
              >
                导入图片
              </button>
              <button
                class="ui-button ui-button--secondary"
                data-action="empty-add-folder"
                :disabled="store.indexing"
                @click="handleAddFolder"
              >
                导入文件夹
              </button>
            </div>
          </div>
        </section>
        <ImageGrid
          v-else
          :images="visibleImages as unknown as SearchResult[]"
          :loading="store.loading && store.images.length === 0"
          :show-debug-info="false"
          layout="library"
          :selectable="selectionMode"
          :selected-ids="store.selectedIds"
          :focused-ids="latestImportedHighlightIds"
          :status-badge-labels="latestImportedBadgeLabels"
          :card-click-action="selectionMode ? 'select' : 'open'"
          :hover-preview="false"
          :empty-message="emptyMessage"
          @delete="handleDelete"
          @select="store.toggleSelection"
          @open="detailId = $event"
        />
        <div
          v-if="visibleImages.length > 0"
          class="gallery-footer"
        >
          <p
            v-if="pagingError"
            class="gallery-feedback gallery-error"
          >
            加载失败，请重试
            <button
              class="ui-button ui-button--text ui-button--compact"
              data-action="retry-pagination"
              @click="retryLoad"
            >
              重试
            </button>
          </p>
          <p
            v-else-if="isPaging"
            class="gallery-feedback"
          >
            加载中...
          </p>
          <p
            v-else-if="!hasMore"
            class="gallery-feedback"
          >
            已显示全部图片
          </p>
        </div>
      </div>
    </section>
    <button
      v-if="showBackToTop"
      class="back-to-top ui-button ui-button--primary"
      data-action="back-to-top"
      @click="scrollToTop"
    >
      回到顶部
    </button>
    <DetailModal
      v-if="detailId"
      :image-id="detailId"
      :images="store.images as unknown as SearchResult[]"
      @close="detailId = null"
      @delete="handleDeleteFromDetail"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted, computed, ref, inject, watch, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, confirm } from "@tauri-apps/plugin-dialog";
import { routeLocationKey, routerKey, type RouteLocationNormalizedLoaded, type Router } from "vue-router";
import ImageGrid from "@/components/ImageGrid.vue";
import DetailModal from "@/components/DetailModal.vue";
import { useLibraryStore, type ImportEntry } from "@/stores/library";
import { useTaskRecoveryStore } from "@/stores/taskRecovery";
import type { SearchResult } from "@/stores/search";

const store = useLibraryStore();
const recoveryStore = useTaskRecoveryStore();
const route = inject<RouteLocationNormalizedLoaded | null>(routeLocationKey, null);
const router = inject<Router | undefined>(routerKey, undefined);
const detailId = ref<string | null>(null);
const scrollContainer = ref<HTMLElement | null>(null);
const currentPage = ref(0);
const isPaging = ref(false);
const loadError = ref(false);
const pagingError = ref(false);
const showBackToTop = ref(false);
const clearingMissing = ref(false);
const showMissingFilter = ref(false);
const latestImportSummary = ref<LatestImportSummary | null>(null);
const importFailures = ref<ImportFailure[]>([]);
const showImportFailures = ref(false);
const latestImportedState = ref<LatestImportedState | null>(null);
const retryingFailures = ref(false);
const selectionMode = ref(false);

interface LatestImportSummary {
  batchId: string;
  totalCount: number;
  importedCount: number;
  duplicatedCount: number;
  failedCount: number;
  completedAt: number;
}

interface ImportFailure {
  taskId: string;
  filePath?: string;
  errorMessage?: string;
  fileName: string;
  failureKind?: string;
  retryable?: boolean;
  userMessage?: string;
}

interface LatestImportedState {
  sourceKey: string;
  count: number;
  activated: boolean;
}

const IMPORT_SUMMARY_ACK_KEY = "library.latestImportSummaryAck";
const IMPORT_SUMMARY_TTL_SECONDS = 30 * 60;

function getAcknowledgedImportBatchId() {
  try {
    return localStorage.getItem(IMPORT_SUMMARY_ACK_KEY);
  } catch {
    return null;
  }
}

function setAcknowledgedImportBatchId(batchId: string | null) {
  try {
    if (!batchId) {
      localStorage.removeItem(IMPORT_SUMMARY_ACK_KEY);
      return;
    }
    localStorage.setItem(IMPORT_SUMMARY_ACK_KEY, batchId);
  } catch {
    // ignore storage errors
  }
}

function shouldDisplayImportSummary(summary: LatestImportSummary) {
  const nowSeconds = Math.floor(Date.now() / 1000);
  if (summary.completedAt <= 0 || nowSeconds - summary.completedAt > IMPORT_SUMMARY_TTL_SECONDS) {
    return false;
  }

  return getAcknowledgedImportBatchId() !== summary.batchId;
}

const progressPercent = computed(() =>
  inProgressIndicator.value && inProgressIndicator.value.total > 0
    ? (inProgressIndicator.value.current / inProgressIndicator.value.total) * 100
    : 0
);

const showRecoveryBanner = computed(() =>
  recoveryStore.pendingCount > 0 && !recoveryStore.activeRecovery && !store.indexing
);

const inProgressIndicator = computed(() => recoveryStore.inProgressIndicator);

const managementActionsDisabled = computed(() => !!inProgressIndicator.value);

const displayedSummary = computed(() =>
  recoveryStore.completedRecoverySummary ?? latestImportSummary.value
);

const displayedSummaryStats = computed(() => ({
  importedCount: displayedSummary.value?.importedCount ?? 0,
  duplicatedCount: displayedSummary.value?.duplicatedCount ?? 0,
  failedCount: displayedSummary.value?.failedCount ?? 0,
}));

const displayedFailures = computed(() =>
  recoveryStore.completedRecoverySummary?.failures ?? importFailures.value
);

const showFailureSummary = computed(() =>
  !!displayedSummary.value && displayedSummary.value.failedCount > 0
);

const showSuccessSummary = computed(() =>
  !!displayedSummary.value && displayedSummary.value.failedCount === 0 && displayedSummary.value.importedCount > 0
);

const showBackToSearch = computed(() => !!router);

const displayedSummarySourceKey = computed(() => {
  if (recoveryStore.completedRecoverySummary) {
    const summary = recoveryStore.completedRecoverySummary;
    return `recovery:${summary.totalCount}:${summary.importedCount}:${summary.failedCount}:${summary.duplicatedCount}`;
  }

  if (latestImportSummary.value) {
    return `import:${latestImportSummary.value.batchId}`;
  }

  return null;
});

const summaryEyebrow = computed(() =>
  recoveryStore.completedRecoverySummary ? "刚刚继续导入" : "最近一次导入"
);

const summaryTitle = computed(() => {
  const summary = displayedSummary.value;
  if (!summary) return "";
  if (recoveryStore.completedRecoverySummary) {
    return `刚导完剩余 ${summary.totalCount} 张`;
  }
  return `共处理 ${summary.totalCount} 张`;
});

const hasMore = computed(() => store.images.length < store.total);

const missingImages = computed(() =>
  store.images.filter((image) => image.fileStatus === "missing")
);

const hasMissingImages = computed(() => missingImages.value.length > 0);

const visibleImages = computed(() =>
  showMissingFilter.value ? missingImages.value : store.images
);

const showColdLibraryEmptyState = computed(() =>
  !showMissingFilter.value && !store.loading && store.total === 0 && store.images.length === 0
);

const emptyMessage = computed(() =>
  showMissingFilter.value ? "当前没有失效图片" : "图库为空，先导入图片开始使用"
);

const showAdvancedCapabilities = computed(() => store.total > 0);

const latestImportedHighlightIds = computed(() => {
  const count = latestImportedState.value?.count ?? 0;
  if (count <= 0) {
    return new Set<string>();
  }

  return new Set(
    [...store.images]
      .sort((a, b) => b.addedAt - a.addedAt)
      .slice(0, count)
      .map((image) => image.id)
  );
});

const latestImportedBadgeLabels = computed(() => {
  const labels: Record<string, string> = {};
  latestImportedHighlightIds.value.forEach((id) => {
    labels[id] = "新";
  });
  return labels;
});

const showLatestImportedTip = computed(() =>
  !!latestImportedState.value && latestImportedState.value.activated && latestImportedHighlightIds.value.size > 0
);

const retryableImportFailures = computed(() =>
  recoveryStore.completedRecoverySummary
    ? []
    : importFailures.value.filter((failure) => failure.retryable && failure.filePath)
);

const showRetryFailuresAction = computed(() =>
  showImportFailures.value && retryableImportFailures.value.length > 0
);

watch(displayedSummarySourceKey, (sourceKey) => {
  if (!sourceKey || !displayedSummary.value) {
    return;
  }

  if (displayedSummary.value.importedCount <= 0) {
    latestImportedState.value = null;
    return;
  }

  if (!latestImportedState.value || latestImportedState.value.sourceKey !== sourceKey) {
    latestImportedState.value = {
      sourceKey,
      count: displayedSummary.value.importedCount,
      activated: false,
    };
    return;
  }

  if (latestImportedState.value.count !== displayedSummary.value.importedCount) {
    latestImportedState.value = {
      ...latestImportedState.value,
      count: displayedSummary.value.importedCount,
    };
  }
});

watch(
  () => store.importState,
  (state) => {
    if (state === "preparing" || state === "importing") {
      latestImportedState.value = null;
    }
  }
);

watch(hasMissingImages, (nextHasMissing) => {
  if (!nextHasMissing) {
    showMissingFilter.value = false;
  }
});

watch(visibleImages, (images) => {
  if (images.length === 0) {
    exitSelectionMode();
  }
});

watch(
  () => route?.query.fileStatus,
  (fileStatus) => {
    if (fileStatus === "missing") {
      showMissingFilter.value = true;
      exitSelectionMode();
      return;
    }

    const browserFileStatus = new URLSearchParams(window.location.search).get("fileStatus");
    if (browserFileStatus === "missing") {
      showMissingFilter.value = true;
      exitSelectionMode();
      return;
    }

    showMissingFilter.value = false;
  },
  { immediate: true }
);

onMounted(() => {
  void reloadGallery();
  void recoveryStore.fetchPendingTasks(true);
});

async function handleResumePendingTasks() {
  await recoveryStore.resumePendingTasks();
}

async function handleClearPendingTasks() {
  await recoveryStore.clearPendingTasks();
}

async function loadPage(page: number, append = false) {
  if (append) {
    isPaging.value = true;
    pagingError.value = false;
  } else {
    loadError.value = false;
  }

  try {
    await store.fetchImages(page, append);
    currentPage.value = page;
  } catch {
    if (append) {
      pagingError.value = true;
    } else {
      loadError.value = true;
    }
  } finally {
    if (append) {
      isPaging.value = false;
    }
  }
}

async function fetchLatestImportSummary() {
  showImportFailures.value = false;
  importFailures.value = [];
  try {
    const summary = await invoke<LatestImportSummary | null>("get_latest_import_summary");
    if (!summary || typeof summary !== "object" || !("batchId" in summary)) {
      latestImportSummary.value = null;
      return;
    }

    if (!shouldDisplayImportSummary(summary)) {
      latestImportSummary.value = null;
      return;
    }

    if (getAcknowledgedImportBatchId() && getAcknowledgedImportBatchId() !== summary.batchId) {
      setAcknowledgedImportBatchId(null);
    }

    latestImportSummary.value = summary;
    if (summary.failedCount <= 0) {
      return;
    }

    const failures =
      (await invoke<Array<{
        taskId: string;
        filePath: string;
        errorMessage?: string;
        failureKind?: string;
        retryable?: boolean;
        userMessage?: string;
      }>>("get_import_batch_failures", {
        batchId: summary.batchId,
      })) ?? [];
    importFailures.value = failures.map((item) => ({
      ...item,
      fileName: item.filePath.split(/[\\/]/).pop() || item.filePath,
    }));
  } catch {
    latestImportSummary.value = null;
  }
}

async function reloadGallery() {
  pagingError.value = false;
  showBackToTop.value = false;
  exitSelectionMode();
  try {
    await store.fetchImageCount();
    await fetchLatestImportSummary();
    await loadPage(0);
  } catch {
    loadError.value = true;
  }

  const container = scrollContainer.value;
  if (typeof container?.scrollTo === "function") {
    container.scrollTo({ top: 0 });
  }

  await ensureScrollableContent();
}

async function loadNextPage() {
  if (!hasMore.value || isPaging.value || store.loading) return;
  await loadPage(currentPage.value + 1, true);
  if (!pagingError.value) {
    await ensureScrollableContent();
  }
}

async function ensureScrollableContent() {
  await nextTick();

  const container = scrollContainer.value;
  if (!container) {
    return;
  }

  let attempts = 0;
  while (
    hasMore.value
    && !store.loading
    && !isPaging.value
    && !pagingError.value
    && container.scrollHeight <= container.clientHeight + 24
    && attempts < 10
  ) {
    attempts += 1;
    await loadPage(currentPage.value + 1, true);
    await nextTick();
  }
}

function handleScroll() {
  const el = scrollContainer.value;
  if (!el) return;

  showBackToTop.value = el.scrollTop > el.clientHeight;
  const nearBottom = el.scrollTop + el.clientHeight >= el.scrollHeight - 120;
  if (nearBottom) {
    void loadNextPage();
  }
}

function scrollToTop() {
  const container = scrollContainer.value;
  if (typeof container?.scrollTo === "function") {
    container.scrollTo({ top: 0, behavior: "smooth" });
  }
}

function handleViewMissingImages() {
  showMissingFilter.value = true;
  exitSelectionMode();
  scrollToTop();
}

function handleViewAllImages() {
  showMissingFilter.value = false;
  exitSelectionMode();
}

function enterSelectionMode() {
  if (managementActionsDisabled.value || visibleImages.value.length === 0) {
    return;
  }
  selectionMode.value = true;
}

function exitSelectionMode() {
  selectionMode.value = false;
  store.clearSelection();
}

async function retryLoad() {
  if (pagingError.value && store.images.length > 0) {
    await loadNextPage();
    return;
  }
  await reloadGallery();
}

function dismissRecoverySummary() {
  recoveryStore.dismissCompletedRecoverySummary();
  showImportFailures.value = false;
  latestImportedState.value = null;
}

function dismissImportSummary() {
  const batchId = latestImportSummary.value?.batchId;
  if (batchId) {
    setAcknowledgedImportBatchId(batchId);
  }
  latestImportSummary.value = null;
  importFailures.value = [];
  showImportFailures.value = false;
}

function handleAcknowledgeFailures() {
  if (recoveryStore.completedRecoverySummary) {
    recoveryStore.markRecoveryResultSeen();
    showImportFailures.value = false;
    latestImportedState.value = null;
    return;
  }

  dismissImportSummary();
}

function handleShowFailures() {
  showImportFailures.value = true;
}

async function handleViewLatestImported() {
  const sourceKey = displayedSummarySourceKey.value;
  const importedCount = displayedSummary.value?.importedCount ?? 0;
  if (!sourceKey || importedCount <= 0) {
    return;
  }
  await reloadGallery();

  latestImportedState.value = {
    sourceKey,
    count: importedCount,
    activated: true,
  };

  if (recoveryStore.completedRecoverySummary) {
    recoveryStore.markRecoveryResultSeen();
  } else {
    dismissImportSummary();
  }

  scrollToTop();
}

async function handleRetryImportFailures() {
  if (retryableImportFailures.value.length === 0) {
    return;
  }

  retryingFailures.value = true;
  try {
    await store.importEntries(
      retryableImportFailures.value.map((failure) => ({
        kind: "file" as const,
        path: failure.filePath!,
      }))
    );
    dismissImportSummary();
    await reloadGallery();
  } finally {
    retryingFailures.value = false;
  }
}

async function handleAdd() {
  const selected = await open({ multiple: true, filters: [{ name: "图片", extensions: ["jpg", "jpeg", "png", "gif", "webp"] }] });
  if (!selected) return;
  const paths = Array.isArray(selected) ? selected : [selected];
  const entries: ImportEntry[] = paths.map((path) => ({
    kind: "file",
    path,
  }));
  await store.importEntries(entries);
  await reloadGallery();
}

async function handleAddFolder() {
  const selected = await open({ directory: true });
  if (!selected) return;
  const path = Array.isArray(selected) ? selected[0] : selected;
  await store.importEntries([
    {
      kind: "directory",
      path,
    },
  ]);
  await reloadGallery();
}

async function handleDelete(id: string) {
  const ok = await confirm("确定要删除这张图片吗？此操作不可撤销。", { title: "删除图片" });
  if (!ok) return;
  await store.deleteImage(id);
  store.total = Math.max(0, store.total - 1);
}

async function handleDeleteFromDetail(id: string) {
  await handleDelete(id);
  detailId.value = null;
}

async function handleDeleteSelected() {
  const count = store.selectedIds.size;
  if (count === 0) return;
  const ok = await confirm(`确认删除 ${count} 张图片？此操作不可撤销。`, { title: "批量删除" });
  if (!ok) return;
  await store.deleteSelected();
  store.total = Math.max(0, store.total - count);
  exitSelectionMode();
}

async function handleClearMissing() {
  const ok = await confirm("确认清除所有失效图片？此操作会删除原文件已丢失的图片记录，且不可撤销。", {
    title: "清除失效图片",
  });
  if (!ok) return;

  clearingMissing.value = true;
  try {
    const removed = await invoke<number>("clear_missing_images");
    if (removed > 0) {
      exitSelectionMode();
    }
    await reloadGallery();
    if (missingImages.value.length === 0) {
      showMissingFilter.value = false;
    }
  } finally {
    clearingMissing.value = false;
  }
}

async function openPrivateRoleLibrary() {
  if (router) {
    await router.push("/private-role-maintenance");
    return;
  }

  window.history.pushState({}, "", "/private-role-maintenance");
}

async function handleBackToSearch() {
  if (router) {
    await router.push("/");
    return;
  }

  window.history.pushState({}, "", "/");
}
</script>

<style scoped>
.library-view {
  box-sizing: border-box;
  height: 100%;
  min-height: 0;
  overflow: hidden;
  padding: 0.12rem 0.18rem 0.22rem;
  display: flex;
  flex-direction: column;
  gap: 0.28rem;
  background:
    radial-gradient(circle at top left, color-mix(in srgb, #fde7cf 42%, transparent), transparent 32%),
    linear-gradient(180deg, color-mix(in srgb, var(--ui-bg-surface-strong) 96%, #fffaf2), var(--ui-bg-app));
}
.page-head {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
  align-items: center;
  gap: 0.6rem;
  padding: 0.14rem 0.06rem 0.04rem;
}
.page-head__left {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  min-width: 0;
}
.page-head__title {
  margin: 0;
  font-size: 1rem;
  line-height: 1.15;
  font-weight: 800;
  color: var(--ui-text-primary);
  text-align: center;
  white-space: nowrap;
}
.page-head__meta {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.5rem;
  flex-wrap: wrap;
  min-width: 0;
}
.page-head__back {
  color: var(--ui-text-secondary);
}

.main-task-card {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.72rem 0.8rem;
  border: 1px solid color-mix(in srgb, var(--ui-accent) 18%, var(--ui-border-subtle));
  border-radius: 0.95rem;
  box-shadow: none;
}
.main-task-card--recovery {
  background: linear-gradient(135deg, color-mix(in srgb, #fff7e6 88%, white), color-mix(in srgb, #fff1cc 72%, white));
}
.main-task-card--missing {
  background: linear-gradient(135deg, color-mix(in srgb, #fff6f5 88%, white), color-mix(in srgb, #ffe2df 68%, white));
}
.main-task-card--summary {
  background: linear-gradient(135deg, color-mix(in srgb, #eef6ff 88%, white), color-mix(in srgb, #dbeafe 72%, white));
}
.main-task-card--summary-expanded {
  align-items: flex-start;
}
.main-task-card--progress {
  background: linear-gradient(135deg, color-mix(in srgb, #f5f8ff 90%, white), color-mix(in srgb, #e0e7ff 72%, white));
}
.main-task-card__copy {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  min-width: 0;
}
.main-task-card__copy h3,
.main-task-card__copy p,
.main-task-card__eyebrow {
  margin: 0;
}
.main-task-card__copy h3 {
  font-size: 0.84rem;
  line-height: 1.3;
}
.main-task-card__eyebrow {
  font-size: 0.7rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  color: #8a5a14;
}
.main-task-card__description,
.main-task-card__copy p:not(.main-task-card__eyebrow) {
  color: var(--ui-text-secondary);
  line-height: 1.4;
  font-size: 0.76rem;
}
.main-task-card__stats {
  display: flex;
  flex-wrap: wrap;
  gap: 0.7rem;
  color: var(--ui-text-secondary);
  font-size: 0.78rem;
}
.main-task-card__stat {
  display: inline-flex;
  align-items: center;
}
.main-task-card__stat--failure {
  color: #9d6320;
  font-weight: 700;
  background: color-mix(in srgb, #f2c98c 20%, transparent);
  border-radius: 999px;
  padding: 0.04rem 0.45rem;
}
.main-task-card__actions {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  flex-wrap: wrap;
  flex-shrink: 0;
}
.ui-button {
  font: inherit;
  font-weight: 600;
  border-radius: 999px;
}
.ui-button--compact {
  min-height: 32px;
  padding: 0 12px;
}
.ui-button--danger {
  background: color-mix(in srgb, #fff1f1 90%, white);
  border-color: color-mix(in srgb, #d26a6a 30%, var(--ui-border-subtle));
  color: #a33838;
}
.ui-button--danger:hover:not(:disabled) {
  background: color-mix(in srgb, #ffe7e7 92%, white);
  border-color: color-mix(in srgb, #b94d4d 46%, var(--ui-border-subtle));
}
.ui-button--text {
  min-height: 36px;
  padding: 0 4px;
  border-color: transparent;
  background: transparent;
  color: var(--ui-text-secondary);
}
.ui-button--text:hover:not(:disabled) {
  background: transparent;
  color: var(--ui-text-primary);
}
.ui-button:disabled {
  opacity: 0.58;
  cursor: not-allowed;
}
.main-task-card__failures {
  margin: 0;
  padding-left: 1.1rem;
  display: flex;
  flex-direction: column;
  gap: 0.625rem;
  flex: 1;
  min-width: 0;
}
.main-task-card__failure-item {
  color: var(--ui-text-primary);
}
.main-task-card__failure-name,
.main-task-card__failure-reason {
  margin: 0;
}
.main-task-card__failure-name {
  font-weight: 600;
  font-size: 0.78rem;
}
.main-task-card__failure-reason {
  color: var(--ui-text-secondary);
  line-height: 1.45;
  font-size: 0.74rem;
}
.main-task-card--summary-expanded .main-task-card__actions {
  flex-direction: column;
  align-items: flex-end;
  gap: 0.5rem;
  min-width: 88px;
}
.usage-notice {
  margin: 0;
  max-width: none;
  color: var(--ui-text-secondary);
  font-size: 0.72rem;
  line-height: 1.4;
  text-align: left;
}
.toolbar {
  display: flex;
  flex-direction: column;
  gap: 0.18rem;
  padding: 0;
  border: none;
  border-radius: 0;
  background: transparent;
}
.library-workbench {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 0.32rem;
  padding: 0.02rem 0.06rem 0;
}
.toolbar__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem 0.7rem;
  flex-wrap: wrap;
}
.toolbar-actions { display: flex; align-items: center; gap: 0.45rem; flex-wrap: wrap; }
.advanced-capabilities {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.26rem;
  flex-wrap: wrap;
}
.toolbar-lock-reason {
  margin: 0;
  font-size: 0.74rem;
  color: #8a5a14;
}
.latest-import-position-tip {
  padding: 0.52rem 0.74rem;
  border-radius: 0.95rem;
  border: 1px solid color-mix(in srgb, #b7791f 28%, var(--ui-border-subtle));
  background: color-mix(in srgb, #fff7eb 88%, white);
  color: #8a5a14;
  font-size: 0.76rem;
}
.advanced-capabilities__action {
  flex-shrink: 0;
  min-height: 28px;
  padding: 0 0.62rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid color-mix(in srgb, var(--ui-border-subtle) 78%, transparent);
  border-radius: 999px;
  background: color-mix(in srgb, var(--ui-bg-surface-strong) 72%, transparent);
  color: var(--ui-text-secondary);
  font: inherit;
  font-size: 0.76rem;
  font-weight: 600;
  cursor: pointer;
}
.advanced-capabilities__action:hover,
.advanced-capabilities__action:focus-visible {
  border-color: color-mix(in srgb, var(--ui-accent) 26%, var(--ui-border-subtle));
  background: color-mix(in srgb, var(--ui-accent) 9%, var(--ui-bg-surface-strong));
  color: var(--ui-text-primary);
}
.advanced-capabilities__hint {
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid color-mix(in srgb, var(--ui-border-subtle) 76%, transparent);
  border-radius: 999px;
  color: var(--ui-text-secondary);
  background: color-mix(in srgb, var(--ui-bg-surface-strong) 60%, transparent);
  font-size: 0.68rem;
  font-weight: 700;
  line-height: 1;
  cursor: help;
}
.advanced-capabilities__hint:hover,
.advanced-capabilities__hint:focus-visible {
  color: var(--ui-text-primary);
  border-color: color-mix(in srgb, var(--ui-accent) 24%, var(--ui-border-subtle));
  outline: none;
}
.gallery-panel__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.32rem;
  flex-wrap: wrap;
  padding: 0.04rem 0 0;
}
.gallery-panel__title-group {
  display: flex;
  align-items: center;
  gap: 0.55rem;
  flex-wrap: wrap;
}
.selection-count { font-size: 0.76rem; color: var(--ui-text-secondary); }
.gallery-total { font-size: 0.8rem; color: var(--ui-text-primary); font-weight: 600; }
.progress-bar { height: 6px; background: #e0e0e0; border-radius: 3px; overflow: hidden; }
.progress-fill { height: 100%; background: #646cff; transition: width 0.3s; }
.gallery-scroll {
  flex: 1;
  min-height: 240px;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 0.18rem 0.18rem 0.24rem 0.02rem;
  border-radius: 1.1rem;
  background:
    linear-gradient(180deg, rgba(255, 255, 255, 0.46), rgba(255, 251, 245, 0.18)),
    repeating-linear-gradient(
      180deg,
      transparent,
      transparent 88px,
      rgba(196, 171, 134, 0.035) 88px,
      rgba(196, 171, 134, 0.035) 89px
    );
  scrollbar-gutter: stable;
}
.gallery-scroll--empty {
  padding: 0;
}
.library-empty-state {
  display: flex;
  flex-direction: column;
  gap: 0.8rem;
  min-height: 100%;
  padding: 0.92rem;
  border: 1px solid color-mix(in srgb, var(--ui-border-subtle) 74%, transparent);
  border-radius: 1.1rem;
  background:
    linear-gradient(135deg, color-mix(in srgb, var(--ui-bg-surface-strong) 94%, white), color-mix(in srgb, #fff7ea 46%, white));
}
.library-empty-state__intro {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.32rem;
}
.library-empty-state__intro h3,
.library-empty-state__intro p,
.library-empty-state__preview p {
  margin: 0;
}
.library-empty-state__intro h3 {
  color: var(--ui-text-primary);
  font-size: 1rem;
  line-height: 1.28;
}
.library-empty-state__intro p,
.library-empty-state__preview p {
  color: var(--ui-text-secondary);
  font-size: 0.78rem;
  line-height: 1.55;
}
.library-empty-state__preview {
  display: flex;
  flex-direction: column;
  flex: 1;
  gap: 0.85rem;
  align-items: center;
  justify-content: center;
  min-height: 330px;
  border-radius: 0.95rem;
  background:
    radial-gradient(circle at 50% 0%, rgba(255, 255, 255, 0.86), transparent 42%),
    color-mix(in srgb, var(--ui-bg-app) 82%, white);
}
.library-empty-state__grid {
  display: grid;
  width: min(100%, 520px);
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.68rem;
}
.library-empty-state__actions {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.55rem;
  flex-wrap: wrap;
  margin-top: 0.1rem;
}
.library-empty-state__card {
  min-height: 116px;
  padding: 0.42rem;
  border: 1px solid color-mix(in srgb, var(--ui-border-subtle) 68%, transparent);
  border-radius: 0.86rem;
  background: color-mix(in srgb, white 72%, transparent);
}
.library-empty-state__thumb {
  height: 72px;
  border-radius: 0.62rem;
  background:
    linear-gradient(135deg, rgba(31, 45, 77, 0.08), rgba(196, 171, 134, 0.13)),
    repeating-linear-gradient(90deg, rgba(31, 45, 77, 0.08), rgba(31, 45, 77, 0.08) 7px, transparent 7px, transparent 14px);
  filter: blur(0.25px);
}
.library-empty-state__card span,
.library-empty-state__card small {
  display: block;
  height: 6px;
  margin-top: 0.46rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--ui-border-subtle) 64%, white);
}
.library-empty-state__card span {
  width: 62%;
}
.library-empty-state__card small {
  width: 36%;
}
.gallery-footer {
  display: flex;
  justify-content: center;
  padding: 0.75rem 0 0.125rem;
}
.gallery-feedback {
  color: var(--ui-text-secondary);
  font-size: 0.76rem;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.gallery-error { color: #b42318; }
.back-to-top {
  position: fixed;
  right: 1.5rem;
  bottom: 1.5rem;
  padding: 0.7rem 1rem;
  box-shadow: 0 10px 30px rgba(17, 24, 39, 0.18);
}
@media (max-width: 799px) {
  .library-view {
    padding: 0.08rem 0.14rem 0.2rem;
  }
  .page-head__meta {
    align-items: flex-start;
  }
  .main-task-card,
  .gallery-panel__head,
  .toolbar__row,
  .library-empty-state {
    align-items: flex-start;
    flex-direction: column;
  }
  .library-empty-state {
    display: flex;
  }
  .library-empty-state__preview {
    width: 100%;
    min-height: 280px;
  }
  .library-empty-state__grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
  .advanced-capabilities {
    width: 100%;
    justify-content: flex-start;
    align-items: flex-start;
  }
  .gallery-panel__title-group {
    align-items: flex-start;
    gap: 0.32rem;
  }
  .main-task-card__actions {
    width: 100%;
  }
  .main-task-card--summary-expanded .main-task-card__actions {
    align-items: flex-start;
  }
  .usage-notice {
    text-align: left;
  }
  .back-to-top { right: 1rem; bottom: 1rem; }
}
</style>
