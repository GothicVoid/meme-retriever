<template>
  <div
    class="image-grid"
    :class="{ 'image-grid--library': layout === 'library' }"
  >
    <p v-if="loading" class="hint">{{ loadingMessage ?? "加载中..." }}</p>
    <p v-else-if="!images.length" class="hint">{{ emptyMessage ?? '没找到相关图片，试试其他描述？' }}</p>
    <ImageCard
      v-for="img in images"
      :key="img.id"
      :image="img"
      :show-debug-info="showDebugInfo"
      :selectable="selectable"
      :selected="selectedIds?.has(img.id) ?? false"
      :focused="focusedIds?.has(img.id) || focusedId === img.id"
      :status-badge-label="statusBadgeLabels?.[img.id]"
      :click-action="cardClickAction"
      :hover-preview="hoverPreview"
      @delete="$emit('delete', $event)"
      @copied="$emit('copied', $event)"
      @select="$emit('select', $event)"
      @open="$emit('open', $event)"
      @preview="$emit('preview', $event)"
    />
  </div>
</template>

<script setup lang="ts">
import ImageCard from "./ImageCard.vue";
import type { SearchResult } from "@/stores/search";
withDefaults(defineProps<{
  images: SearchResult[];
  loading: boolean;
  showDebugInfo: boolean;
  loadingMessage?: string;
  emptyMessage?: string;
  selectable?: boolean;
  selectedIds?: Set<string>;
  focusedId?: string | null;
  focusedIds?: Set<string>;
  statusBadgeLabels?: Record<string, string>;
  layout?: "default" | "library";
  cardClickAction?: "copy" | "open" | "select";
  hoverPreview?: boolean;
}>(), {
  loadingMessage: undefined,
  emptyMessage: undefined,
  selectable: false,
  selectedIds: undefined,
  focusedId: null,
  focusedIds: undefined,
  statusBadgeLabels: undefined,
  layout: "default",
  cardClickAction: "copy",
  hoverPreview: true,
});
defineEmits<{ delete: [id: string]; copied: [id: string]; select: [id: string]; open: [id: string]; preview: [id: string] }>();
</script>

<style scoped>
.image-grid {
  display: grid;
  align-content: start;
  grid-template-columns: repeat(auto-fill, minmax(148px, 1fr));
  gap: 0.625rem;
}
.image-grid--library {
  grid-template-columns: repeat(auto-fill, minmax(172px, 1fr));
  gap: 0.7rem;
}

/* PRD §7.3: <800px 2列，>1400px 4列 */
@media (max-width: 799px) {
  .image-grid {
    grid-template-columns: repeat(2, 1fr);
  }
  .image-grid--library {
    grid-template-columns: repeat(2, 1fr);
    gap: 0.56rem;
  }
}
@media (min-width: 1400px) {
  .image-grid {
    grid-template-columns: repeat(4, 1fr);
  }
  .image-grid--library {
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  }
}
.hint { color: #888; padding: 1rem 0; }
</style>
