<template>
  <div
    class="image-card"
    @click="handleClick"
  >
    <img
      :src="convertFileSrc(image.thumbnailPath || image.filePath)"
      :alt="image.id"
      loading="lazy"
    >
  </div>
</template>

<script setup lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import { useClipboard } from "@/composables/useClipboard";
import type { SearchResult } from "@/stores/search";

const props = defineProps<{ image: SearchResult }>();
const { copyImage } = useClipboard();

async function handleClick() {
  await copyImage(props.image.id);
}
</script>

<style scoped>
.image-card {
  cursor: pointer;
  border-radius: 6px;
  overflow: hidden;
  background: #eee;
  aspect-ratio: 1;
}
.image-card img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.image-card:hover { opacity: 0.85; }
</style>
