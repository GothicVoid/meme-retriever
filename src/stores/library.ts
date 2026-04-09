import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface ImageMeta {
  id: string;
  filePath: string;
  fileName: string;
  thumbnailPath: string;
  fileFormat: string;
  width: number;
  height: number;
  fileSize: number;
  addedAt: number;
  useCount: number;
  tags: string[];
}

export const useLibraryStore = defineStore("library", () => {
  const images = ref<ImageMeta[]>([]);
  const loading = ref(false);
  const indexing = ref(false);
  const indexTotal = ref(0);
  const indexCurrent = ref(0);
  const selectedIds = ref<Set<string>>(new Set());

  async function fetchImages(page = 0) {
    loading.value = true;
    try {
      images.value = (await invoke<ImageMeta[]>("get_images", { page })) ?? [];
    } finally {
      loading.value = false;
    }
  }

  async function addImages(paths: string[]) {
    indexing.value = true;
    indexTotal.value = paths.length;
    indexCurrent.value = 0;
    const unlistenPromise = listen("index-progress", () => {
      indexCurrent.value++;
    });
    await invoke("add_images", { paths });
    await new Promise<void>((resolve) => {
      const check = setInterval(() => {
        if (indexCurrent.value >= indexTotal.value) {
          clearInterval(check);
          resolve();
        }
      }, 50);
    });
    const unlisten = await unlistenPromise;
    unlisten();
    indexing.value = false;
    await fetchImages();
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
    const total = await invoke<number>("add_folder", { path: dirPath });
    if (total === 0) return;
    indexing.value = true;
    indexTotal.value = total;
    indexCurrent.value = 0;
    const unlistenPromise = listen("index-progress", () => {
      indexCurrent.value++;
    });
    await new Promise<void>((resolve) => {
      const check = setInterval(() => {
        if (indexCurrent.value >= indexTotal.value) {
          clearInterval(check);
          resolve();
        }
      }, 50);
    });
    const unlisten = await unlistenPromise;
    unlisten();
    indexing.value = false;
    await fetchImages();
  }

  return { images, loading, indexing, indexTotal, indexCurrent, selectedIds, fetchImages, addImages, deleteImage, addFolder, toggleSelection, clearSelection, deleteSelected };
});
