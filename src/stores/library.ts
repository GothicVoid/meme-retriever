import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

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
    await invoke("add_images", { paths });
    await fetchImages();
  }

  async function deleteImage(id: string) {
    await invoke("delete_image", { id });
    images.value = images.value.filter((img) => img.id !== id);
  }

  return { images, loading, fetchImages, addImages, deleteImage };
});
