<template>
  <div
    class="image-card"
    @click="handleClick"
    @contextmenu.prevent="showMenu"
  >
    <img
      :src="convertFileSrc(image.thumbnailPath || image.filePath)"
      :alt="image.id"
      loading="lazy"
    >
    <div
      v-if="showDebugInfo && image.debugInfo"
      class="debug-overlay"
    >
      <div class="debug-score">{{ (image.score * 100).toFixed(1) }}%</div>
      <div class="debug-row">
        <span>语义 {{ (image.debugInfo.semScore * 100).toFixed(0) }}%</span>
        <span class="dim">×{{ image.debugInfo.semWeight.toFixed(1) }}</span>
      </div>
      <div class="debug-row">
        <span>关键词 {{ (image.debugInfo.kwScore * 100).toFixed(0) }}%</span>
        <span class="dim">×{{ image.debugInfo.kwWeight.toFixed(1) }}</span>
      </div>
      <div
        v-if="image.debugInfo.tagHit"
        class="debug-tag"
      >
        标签命中
      </div>
    </div>
    <ul
      v-if="menuVisible"
      class="context-menu"
      :style="{ top: menuY + 'px', left: menuX + 'px' }"
    >
      <li>
        <button
          data-action="delete"
          @click.stop="handleDelete"
        >
          删除
        </button>
      </li>
    </ul>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { useClipboard } from "@/composables/useClipboard";
import type { SearchResult } from "@/stores/search";

const props = defineProps<{ image: SearchResult; showDebugInfo: boolean }>();
const emit = defineEmits<{ delete: [id: string] }>();
const { copyImage } = useClipboard();

const menuVisible = ref(false);
const menuX = ref(0);
const menuY = ref(0);

async function handleClick() {
  await copyImage(props.image.id);
}

function showMenu(e: MouseEvent) {
  menuX.value = e.offsetX;
  menuY.value = e.offsetY;
  menuVisible.value = true;
}

function closeMenu() {
  menuVisible.value = false;
}

function handleDelete() {
  menuVisible.value = false;
  emit("delete", props.image.id);
}

onMounted(() => document.addEventListener("click", closeMenu));
onUnmounted(() => document.removeEventListener("click", closeMenu));
</script>

<style scoped>
.image-card {
  cursor: pointer;
  border-radius: 6px;
  overflow: hidden;
  background: #eee;
  aspect-ratio: 1;
  position: relative;
}
.image-card img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.image-card:hover { opacity: 0.85; }

.debug-overlay {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  background: rgba(0, 0, 0, 0.65);
  color: #fff;
  font-size: 0.68rem;
  padding: 0.25rem 0.4rem;
  line-height: 1.5;
  pointer-events: none;
}
.debug-score { font-size: 0.82rem; font-weight: 600; }
.debug-row { display: flex; justify-content: space-between; }
.dim { opacity: 0.7; }
.debug-tag { color: #ffd700; font-weight: 600; }

.context-menu {
  position: absolute;
  background: #fff;
  border: 1px solid #ddd;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.15);
  list-style: none;
  padding: 0.25rem 0;
  min-width: 120px;
  z-index: 100;
}
.context-menu li button {
  width: 100%;
  padding: 0.5rem 1rem;
  text-align: left;
  background: none;
  border: none;
  cursor: pointer;
  font-size: 0.9rem;
  color: #c0392b;
}
.context-menu li button:hover { background: #f5f5f5; }
</style>
