<template>
  <div class="library-view">
    <div class="page-head">
      <div class="page-head__copy">
        <h2>图库管理</h2>
        <p>在这里导入、整理和排查图片问题。搜索不到时，也可以先来这里检查新增内容和异常图片。</p>
      </div>
      <div class="gallery-total">
        共 {{ store.total }} 张
      </div>
    </div>

    <div class="view-switches">
      <button
        class="view-switch"
        :class="{ active: currentView === 'all' }"
        data-view="all"
        @click="currentView = 'all'"
      >
        全部图片
      </button>
      <button
        class="view-switch"
        :class="{ active: currentView === 'recent' }"
        data-view="recent"
        @click="currentView = 'recent'"
      >
        最近新增
      </button>
      <button
        class="view-switch"
        :class="{ active: currentView === 'issues' }"
        data-view="issues"
        @click="currentView = 'issues'"
      >
        异常图片
      </button>
    </div>

    <div class="toolbar">
      <div class="toolbar-actions">
        <button
          data-action="add-images"
          :disabled="store.indexing"
          @click="handleAdd"
        >
          添加图片
        </button>
        <button
          data-action="add-folder"
          :disabled="store.indexing"
          @click="handleAddFolder"
        >
          添加文件夹
        </button>
        <button
          :disabled="store.indexing || clearingMissing"
          data-action="clear-missing"
          @click="handleClearMissing"
        >
          {{ clearingMissing ? "正在清除失效图片" : "清除失效图片" }}
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
      <div class="usage-notice">
        图库按原文件路径引用，移动、重命名或删除原图会导致图片失效，并影响复制和定位。
      </div>
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
    <div
      ref="scrollContainer"
      class="gallery-scroll"
      @scroll="handleScroll"
    >
      <div
        v-if="loadError && store.images.length === 0"
        class="gallery-feedback gallery-error"
      >
        <p>加载失败，请重试</p>
        <button
          data-action="retry-load"
          @click="retryLoad"
        >
          重试
        </button>
      </div>
      <ImageGrid
        v-else
        :images="visibleImages as unknown as SearchResult[]"
        :loading="store.loading && store.images.length === 0"
        :show-debug-info="false"
        :selectable="true"
        :selected-ids="store.selectedIds"
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
    <button
      v-if="showBackToTop"
      class="back-to-top"
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
import { onMounted, computed, ref, inject, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, confirm } from "@tauri-apps/plugin-dialog";
import { routeLocationKey, type RouteLocationNormalizedLoaded } from "vue-router";
import ImageGrid from "@/components/ImageGrid.vue";
import DetailModal from "@/components/DetailModal.vue";
import { useLibraryStore } from "@/stores/library";
import type { SearchResult } from "@/stores/search";

const store = useLibraryStore();
const route = inject<RouteLocationNormalizedLoaded | null>(routeLocationKey, null);
const detailId = ref<string | null>(null);
const scrollContainer = ref<HTMLElement | null>(null);
const currentPage = ref(0);
const isPaging = ref(false);
const loadError = ref(false);
const pagingError = ref(false);
const showBackToTop = ref(false);
const clearingMissing = ref(false);
const currentView = ref<"all" | "recent" | "issues">("all");

const progressPercent = computed(() =>
  store.indexTotal > 0 ? (store.indexCurrent / store.indexTotal) * 100 : 0
);

const hasMore = computed(() => store.images.length < store.total);

const visibleImages = computed(() => {
  if (currentView.value === "recent") {
    return [...store.images].sort((a, b) => b.addedAt - a.addedAt);
  }

  if (currentView.value === "issues") {
    return store.images.filter((image) => image.fileStatus && image.fileStatus !== "normal");
  }

  return store.images;
});

const emptyMessage = computed(() => {
  if (currentView.value === "issues") {
    return "当前没有异常图片";
  }

  if (currentView.value === "recent") {
    return "最近暂无新增";
  }

  return "图库为空，请先添加图片";
});

function normalizeView(raw: unknown): "all" | "recent" | "issues" {
  if (raw === "recent" || raw === "issues") {
    return raw;
  }
  return "all";
}

function resolveRouteView() {
  const routeView = route?.query.view;
  if (typeof routeView === "string") {
    return normalizeView(routeView);
  }

  const browserView = new URLSearchParams(window.location.search).get("view");
  return normalizeView(browserView);
}

watch(
  () => route?.query.view,
  () => {
    currentView.value = resolveRouteView();
  },
  { immediate: true }
);

onMounted(() => {
  void reloadGallery();
});

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

async function reloadGallery() {
  pagingError.value = false;
  showBackToTop.value = false;
  try {
    await store.fetchImageCount();
    await loadPage(0);
  } catch {
    loadError.value = true;
  }

  const container = scrollContainer.value;
  if (typeof container?.scrollTo === "function") {
    container.scrollTo({ top: 0 });
  }
}

async function loadNextPage() {
  if (!hasMore.value || isPaging.value || store.loading) return;
  await loadPage(currentPage.value + 1, true);
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

async function retryLoad() {
  if (pagingError.value && store.images.length > 0) {
    await loadNextPage();
    return;
  }
  await reloadGallery();
}

async function handleAdd() {
  const selected = await open({ multiple: true, filters: [{ name: "图片", extensions: ["jpg", "jpeg", "png", "gif", "webp"] }] });
  if (!selected) return;
  const paths = Array.isArray(selected) ? selected : [selected];
  await store.addImages(paths);
  await reloadGallery();
}

async function handleAddFolder() {
  const selected = await open({ directory: true });
  if (!selected) return;
  const path = Array.isArray(selected) ? selected[0] : selected;
  await store.addFolder(path);
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
  const ok = await confirm(`确认删除 ${count} 张图片？此操作不可撤销。`, { title: "批量删除" });
  if (!ok) return;
  await store.deleteSelected();
  store.total = Math.max(0, store.total - count);
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
      store.clearSelection();
    }
    await reloadGallery();
  } finally {
    clearingMissing.value = false;
  }
}
</script>

<style scoped>
.library-view { padding: 1rem; display: flex; flex-direction: column; gap: 1rem; }
.page-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
  flex-wrap: wrap;
}
.page-head__copy {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}
.page-head__copy h2 { font-size: 1.25rem; }
.page-head__copy p {
  max-width: 720px;
  color: #666;
  font-size: 0.9rem;
  line-height: 1.5;
}
.view-switches {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}
.view-switch {
  border: 1px solid #d9d9d9;
  background: #fff;
  color: #444;
  border-radius: 999px;
  padding: 0.45rem 0.9rem;
  cursor: pointer;
  font-size: 0.9rem;
}
.view-switch.active {
  border-color: #111827;
  background: #111827;
  color: #fff;
}
.usage-notice {
  flex: 1 1 360px;
  min-width: 240px;
  color: #8a6a2f;
  font-size: 0.75rem;
  line-height: 1.35;
  text-align: center;
}
.toolbar { display: flex; align-items: center; justify-content: space-between; gap: 1rem; flex-wrap: wrap; }
.toolbar-actions { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
.selection-count { font-size: 0.875rem; color: #666; }
.gallery-total { font-size: 0.95rem; color: #444; font-weight: 600; }
.index-status { margin-bottom: 0.75rem; font-size: 0.875rem; color: #666; display: flex; flex-direction: column; gap: 0.25rem; }
.progress-bar { height: 6px; background: #e0e0e0; border-radius: 3px; overflow: hidden; }
.progress-fill { height: 100%; background: #646cff; transition: width 0.3s; }
.gallery-scroll {
  height: calc(100vh - 170px);
  min-height: 320px;
  overflow-y: auto;
  padding-right: 0.25rem;
}
.gallery-footer {
  display: flex;
  justify-content: center;
  padding: 1rem 0 0.25rem;
}
.gallery-feedback {
  color: #666;
  font-size: 0.9rem;
  display: flex;
  align-items: center;
  gap: 0.5rem;
}
.gallery-error { color: #b42318; }
.back-to-top {
  position: fixed;
  right: 1.5rem;
  bottom: 1.5rem;
  border: none;
  border-radius: 999px;
  background: #111827;
  color: #fff;
  padding: 0.7rem 1rem;
  cursor: pointer;
  box-shadow: 0 10px 30px rgba(17, 24, 39, 0.18);
}
@media (max-width: 799px) {
  .gallery-scroll { height: calc(100vh - 210px); }
  .back-to-top { right: 1rem; bottom: 1rem; }
}
</style>
