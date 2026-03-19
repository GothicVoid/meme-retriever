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

  async function fetchImages(page = 0) {
    loading.value = true;
    try {
      images.value = await invoke<ImageMeta[]>("get_images", { page });
    } finally {
      loading.value = false;
    }
  }

  async function addImages(paths: string[]) {
    let remaining = paths.length;
    const unlistenPromise = listen("index-progress", () => {
      remaining--;
    });
    await invoke("add_images", { paths });
    // 等待所有进度事件到达
    await new Promise<void>((resolve) => {
      const check = setInterval(() => {
        if (remaining <= 0) {
          clearInterval(check);
          resolve();
        }
      }, 50);
    });
    const unlisten = await unlistenPromise;
    unlisten();
    await fetchImages();
  }

  async function deleteImage(id: string) {
    await invoke("delete_image", { id });
    images.value = images.value.filter((img) => img.id !== id);
  }

  return { images, loading, fetchImages, addImages, deleteImage };
});
