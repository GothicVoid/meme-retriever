import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface ImageMeta {
  id: string;
  filePath: string;
  fileName: string;
  thumbnailPath: string;
  width: number;
  height: number;
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

  async function fetchImages(page = 0) {
    loading.value = true;
    try {
      images.value = await invoke<ImageMeta[]>("get_images", { page });
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

  return { images, loading, indexing, indexTotal, indexCurrent, fetchImages, addImages, deleteImage, addFolder };
});
