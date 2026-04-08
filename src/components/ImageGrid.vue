<template>
  <div class="image-grid">
    <p v-if="loading" class="hint">加载中...</p>
    <p v-else-if="!images.length" class="hint">{{ emptyMessage ?? '没找到相关图片，试试其他描述？' }}</p>
    <ImageCard
      v-for="img in images"
      :key="img.id"
      :image="img"
      :show-debug-info="showDebugInfo"
      :selectable="selectable"
      :selected="selectedIds?.has(img.id) ?? false"
      @delete="$emit('delete', $event)"
      @select="$emit('select', $event)"
    />
  </div>
</template>

<script setup lang="ts">
import ImageCard from "./ImageCard.vue";
import type { SearchResult } from "@/stores/search";
defineProps<{
  images: SearchResult[];
  loading: boolean;
  showDebugInfo: boolean;
  emptyMessage?: string;
  selectable?: boolean;
  selectedIds?: Set<string>;
}>();
defineEmits<{ delete: [id: string]; select: [id: string] }>();
</script>

<style scoped>
.image-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
  gap: 0.75rem;
}
.hint { color: #888; padding: 1rem 0; }
</style>
