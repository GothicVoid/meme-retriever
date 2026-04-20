import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { StructuredTag } from "@/types/tags";

export interface ImageMeta {
  id: string;
  filePath: string;
  fileName: string;
  thumbnailPath: string;
  fileFormat?: string;
  fileStatus?: string;
  width: number;
  height: number;
  fileSize?: number;
  addedAt: number;
  useCount: number;
  tags: StructuredTag[];
}

export interface ImportEntry {
  kind: "file" | "directory";
  path: string;
}

export type ImportState = "idle" | "preparing" | "importing" | "completed" | "partial_failed" | "failed";

export const useLibraryStore = defineStore("library", () => {
  const images = ref<ImageMeta[]>([]);
  const loading = ref(false);
  const total = ref(0);
  const importState = ref<ImportState>("idle");
  const indexing = ref(false);
  const indexTotal = ref(0);
  const indexCurrent = ref(0);
  const clearing = ref(false);
  const clearTotal = ref(0);
  const clearCurrent = ref(0);
  const selectedIds = ref<Set<string>>(new Set());
  let resumeIndexingUnlisten: null | (() => void) = null;

  async function fetchImages(page = 0, append = false) {
    loading.value = true;
    try {
      const nextImages = (await invoke<ImageMeta[]>("get_images", { page })) ?? [];
      images.value = append ? [...images.value, ...nextImages] : nextImages;
    } finally {
      loading.value = false;
    }
  }

  async function fetchImageCount() {
    total.value = await invoke<number>("get_image_count");
  }

  async function waitForImportProgress(expectedTotal: number) {
    indexing.value = true;
    indexTotal.value = expectedTotal;
    indexCurrent.value = 0;
    importState.value = "importing";
    const unlistenPromise = listen("index-progress", () => {
      indexCurrent.value++;
    });
    try {
      await new Promise<void>((resolve) => {
        const check = setInterval(() => {
          if (indexCurrent.value >= indexTotal.value) {
            clearInterval(check);
            resolve();
          }
        }, 50);
      });
      importState.value = "completed";
      await Promise.all([fetchImages(), fetchImageCount()]);
    } finally {
      const unlisten = await unlistenPromise;
      unlisten();
      indexing.value = false;
    }
  }

  async function importEntries(entries: ImportEntry[]) {
    if (entries.length === 0) {
      importState.value = "idle";
      return;
    }

    importState.value = "preparing";
    try {
      const nextTotal = await invoke<number>("import_entries", { entries });
      if (nextTotal === 0) {
        importState.value = "idle";
        return;
      }
      await waitForImportProgress(nextTotal);
    } catch (error) {
      importState.value = "failed";
      throw error;
    }
  }

  async function addImages(paths: string[]) {
    await importEntries(paths.map((path) => ({ kind: "file", path })));
  }

  async function deleteImage(id: string) {
    await invoke("delete_image", { id });
    images.value = images.value.filter((img) => img.id !== id);
  }

  function toggleSelection(id: string) {
    if (selectedIds.value.has(id)) {
      selectedIds.value.delete(id);
    } else {
      selectedIds.value.add(id);
    }
  }

  function clearSelection() {
    selectedIds.value = new Set();
  }

  async function deleteSelected() {
    const ids = [...selectedIds.value];
    for (const id of ids) {
      await invoke("delete_image", { id });
    }
    const idSet = new Set(ids);
    images.value = images.value.filter((img) => !idSet.has(img.id));
    clearSelection();
  }

  async function addFolder(dirPath: string) {
    await importEntries([{ kind: "directory", path: dirPath }]);
  }

  function stopResumeIndexing() {
    if (resumeIndexingUnlisten) {
      resumeIndexingUnlisten();
      resumeIndexingUnlisten = null;
    }
    indexing.value = false;
  }

  async function resumeIndexing(totalCount: number) {
    stopResumeIndexing();
    if (totalCount <= 0) return;

    indexing.value = true;
    indexTotal.value = totalCount;
    indexCurrent.value = 0;
    resumeIndexingUnlisten = await listen("index-progress", () => {
      indexCurrent.value += 1;
      if (indexCurrent.value >= indexTotal.value) {
        stopResumeIndexing();
        void fetchImages();
        void fetchImageCount();
      }
    });
  }

  async function clearGallery() {
    if (images.value.length === 0) {
      return;
    }

    clearing.value = true;
    clearTotal.value = images.value.length;
    clearCurrent.value = 0;

    const unlistenPromise = listen<{ current: number; total: number }>("clear-gallery-progress", (event) => {
      clearCurrent.value = event.payload.current;
      clearTotal.value = event.payload.total;
    });

    let completed = false;
    try {
      await invoke("clear_gallery");
      await new Promise<void>((resolve) => {
        const check = setInterval(() => {
          if (clearCurrent.value >= clearTotal.value && clearTotal.value > 0) {
            clearInterval(check);
            completed = true;
            resolve();
          }
        }, 50);
      });
    } finally {
      const unlisten = await unlistenPromise;
      unlisten();
      clearing.value = false;
      if (completed) {
        images.value = [];
        total.value = 0;
        clearSelection();
      }
    }
  }

  return {
    images,
    loading,
    total,
    importState,
    indexing,
    indexTotal,
    indexCurrent,
    clearing,
    clearTotal,
    clearCurrent,
    selectedIds,
    fetchImages,
    fetchImageCount,
    importEntries,
    addImages,
    deleteImage,
    addFolder,
    resumeIndexing,
    stopResumeIndexing,
    clearGallery,
    toggleSelection,
    clearSelection,
    deleteSelected,
  };
});
