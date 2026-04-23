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
          v-if="previewVisible"
          :src="previewSrc"
          :alt="image.id"
          class="quick-preview__image"
          :class="{ 'quick-preview__image--missing': isMissing }"
        >
        <div
          v-if="showMissingOverlay"
          class="quick-preview__missing quick-preview__missing--overlay"
        >
          <p class="quick-preview__missing-title">
            原文件已丢失
          </p>
          <p class="quick-preview__missing-desc">
            可查看详情重新定位，或删除这条记录。
          </p>
        </div>
        <div
          v-else-if="showMissingFallback"
          class="quick-preview__missing"
        >
          <p class="quick-preview__missing-title">
            原文件已丢失
          </p>
          <p class="quick-preview__missing-desc">
            可查看详情重新定位，或删除这条记录。
          </p>
        </div>
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
          v-if="!isMissing"
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
          v-if="!isMissing"
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
        {{ isMissing ? "Esc 关闭" : "Enter 复制 · Esc 关闭" }}
      </p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { convertFileSrc } from "@tauri-apps/api/core";
import { computed } from "vue";
import type { SearchResult } from "@/stores/search";

const props = defineProps<{
  image: SearchResult;
  canPrev: boolean;
  canNext: boolean;
}>();

const isMissing = computed(() => props.image.fileStatus === "missing");
const previewSrc = computed(() => {
  const path = isMissing.value ? props.image.thumbnailPath : props.image.filePath;
  return convertFileSrc(path);
});
const previewVisible = computed(() => !isMissing.value || Boolean(props.image.thumbnailPath));
const showMissingOverlay = computed(() => isMissing.value && previewVisible.value);
const showMissingFallback = computed(() => isMissing.value && !previewVisible.value);

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
  padding: 16px;
  background: rgba(31, 35, 41, 0.52);
}

.quick-preview {
  position: relative;
  width: min(520px, calc(100vw - 32px));
  max-height: calc(100vh - 32px);
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 16px;
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
  min-height: 0;
  aspect-ratio: 1 / 1;
  overflow: hidden;
  border-radius: var(--ui-radius-md);
  background: linear-gradient(180deg, #ece7dd, #e5ded0);
}

.quick-preview__image {
  max-width: 100%;
  max-height: min(56vh, 100%);
  object-fit: contain;
}

.quick-preview__image--missing {
  opacity: 0.55;
}

.quick-preview__missing {
  max-width: 28rem;
  padding: 2rem 1.5rem;
  text-align: center;
  color: var(--ui-text-primary);
}

.quick-preview__missing--overlay {
  position: absolute;
  inset: 0;
  max-width: none;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  background: rgba(24, 18, 13, 0.28);
  color: #fff6eb;
  backdrop-filter: blur(1px);
}

.quick-preview__missing--overlay .quick-preview__missing-desc {
  color: rgba(255, 246, 235, 0.9);
}

.quick-preview__missing-title {
  margin: 0 0 0.5rem;
  font-size: 1.125rem;
  font-weight: 600;
}

.quick-preview__missing-desc {
  margin: 0;
  color: var(--ui-text-secondary);
  line-height: 1.5;
}

.quick-preview__nav {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  width: 34px;
  height: 34px;
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
  left: 10px;
}

.quick-preview__nav--next {
  right: 10px;
}

.quick-preview__actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  justify-content: center;
}

.quick-preview__action {
  min-height: 40px;
  padding: 0 14px;
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
  text-align: center;
}

@media (max-width: 480px) {
  .quick-preview-backdrop {
    padding: 10px;
  }

  .quick-preview {
    width: calc(100vw - 20px);
    max-height: calc(100vh - 20px);
    gap: 10px;
    padding: 12px;
    border-radius: 20px;
  }

  .quick-preview__close {
    top: 6px;
    right: 6px;
    width: 28px;
    height: 28px;
  }

  .quick-preview__media {
    border-radius: 16px;
  }

  .quick-preview__image {
    max-height: min(50vh, 100%);
  }

  .quick-preview__nav {
    width: 30px;
    height: 30px;
  }

  .quick-preview__nav--prev {
    left: 8px;
  }

  .quick-preview__nav--next {
    right: 8px;
  }

  .quick-preview__actions {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .quick-preview__action {
    min-height: 38px;
    padding: 0 12px;
  }

  .quick-preview__missing {
    padding: 1.25rem 1rem;
  }

  .quick-preview__missing-title {
    font-size: 1rem;
  }

  .quick-preview__shortcuts {
    font-size: 0.76rem;
  }
}
</style>
