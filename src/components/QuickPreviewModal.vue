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
        <button
          class="quick-preview__nav quick-preview__nav--prev"
          :disabled="!canPrev"
          data-testid="quick-preview-prev"
          @click="$emit('prev')"
        >
          ←
        </button>
        <img
          :src="convertFileSrc(image.filePath)"
          :alt="image.id"
          class="quick-preview__image"
        >
        <button
          class="quick-preview__nav quick-preview__nav--next"
          :disabled="!canNext"
          data-testid="quick-preview-next"
          @click="$emit('next')"
        >
          →
        </button>
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
          data-testid="quick-preview-reveal"
          @click="$emit('reveal', image.id)"
        >
          在文件夹中显示
        </button>
        <button
          class="quick-preview__action"
          @click="$emit('close')"
        >
          关闭
        </button>
      </div>
      <p
        class="quick-preview__shortcuts"
        data-testid="quick-preview-shortcuts-hint"
      >
        Enter 复制 · Esc 关闭
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import type { SearchResult } from "@/stores/search";

defineProps<{
  image: SearchResult;
  canPrev: boolean;
  canNext: boolean;
}>();

defineEmits<{
  close: [];
  copy: [id: string];
  detail: [id: string];
  reveal: [id: string];
  prev: [];
  next: [];
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
  position: relative;
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

.quick-preview__nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 999px;
  background: rgba(31, 35, 41, 0.68);
  color: #fff;
  cursor: pointer;
}

.quick-preview__nav:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.quick-preview__nav--prev {
  left: 12px;
}

.quick-preview__nav--next {
  right: 12px;
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

.quick-preview__shortcuts {
  margin: 0;
  font-size: 0.82rem;
  color: var(--ui-text-secondary);
  text-align: right;
}
</style>
