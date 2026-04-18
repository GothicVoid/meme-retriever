<template>
  <div
    class="quick-preview-backdrop"
    @click.self="$emit('close')"
  >
    <div class="quick-preview">
      <button
        class="quick-preview__close"
        @click="$emit('close')"
      >
        ×
      </button>
      <div class="quick-preview__media">
        <img
          :src="convertFileSrc(image.filePath)"
          :alt="image.id"
          class="quick-preview__image"
        >
      </div>
      <div class="quick-preview__actions">
        <button
          class="quick-preview__action quick-preview__action--primary"
          @click="$emit('copy', image.id)"
        >
          复制
        </button>
        <button
          class="quick-preview__action"
          @click="$emit('detail', image.id)"
        >
          查看详情
        </button>
        <button
          class="quick-preview__action"
          @click="$emit('close')"
        >
          关闭
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import type { SearchResult } from "@/stores/search";

defineProps<{
  image: SearchResult;
}>();

defineEmits<{
  close: [];
  copy: [id: string];
  detail: [id: string];
}>();
</script>

<style scoped>
.quick-preview-backdrop {
  position: fixed;
  inset: 0;
  z-index: 2100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: rgba(31, 35, 41, 0.52);
}

.quick-preview {
  position: relative;
  width: min(680px, calc(100vw - 48px));
  max-height: calc(100vh - 48px);
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 20px;
  border: 1px solid var(--ui-border-subtle);
  border-radius: var(--ui-radius-lg);
  background: color-mix(in srgb, var(--ui-bg-surface-strong) 96%, white);
  box-shadow: var(--ui-shadow-floating);
}

.quick-preview__close {
  position: absolute;
  top: 10px;
  right: 10px;
  width: 32px;
  height: 32px;
  border: none;
  border-radius: 999px;
  background: transparent;
  cursor: pointer;
}

.quick-preview__media {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 280px;
  overflow: hidden;
  border-radius: var(--ui-radius-md);
  background: linear-gradient(180deg, #ece7dd, #e5ded0);
}

.quick-preview__image {
  max-width: 100%;
  max-height: 60vh;
  object-fit: contain;
}

.quick-preview__actions {
  display: flex;
  gap: 12px;
  justify-content: flex-end;
}

.quick-preview__action {
  min-height: 40px;
  padding: 0 16px;
  border: 1px solid var(--ui-border-subtle);
  border-radius: 999px;
  background: color-mix(in srgb, var(--ui-bg-surface-strong) 92%, white);
  cursor: pointer;
}

.quick-preview__action--primary {
  border-color: transparent;
  background: var(--ui-accent);
  color: #fff;
}
</style>
