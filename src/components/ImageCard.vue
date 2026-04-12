<template>
  <div
    class="image-card"
    @click="handleClick"
    @dblclick="emit('open', image.id)"
    @contextmenu.prevent="showMenu"
  >
    <input
      v-if="selectable"
      type="checkbox"
      class="select-checkbox"
      :checked="selected"
      @change.stop="emit('select', image.id)"
      @click.stop
    >
    <img
      v-if="!imgError"
      :src="convertFileSrc(image.thumbnailPath || image.filePath)"
      :alt="image.id"
      loading="lazy"
      @error="imgError = true"
    >
    <div
      v-else
      class="img-missing"
    >
      <span>文件丢失</span>
    </div>
    <span
      v-if="formatBadge"
      class="format-badge"
    >{{ formatBadge }}</span>
    <div
      v-if="showDebugInfo && image.debugInfo"
      class="debug-overlay"
    >
      <div class="debug-score">
        {{ (image.score * 100).toFixed(1) }}%
      </div>
      <div class="debug-row">
        <span>相关性 {{ (image.debugInfo.relevance * 100).toFixed(0) }}%</span>
        <span class="dim">×0.75</span>
      </div>
      <div class="debug-row">
        <span>热度 {{ (image.debugInfo.popularity * 100).toFixed(0) }}%</span>
        <span class="dim">×0.25</span>
      </div>
      <div class="debug-divider" />
      <div class="debug-row">
        <span>CLIP {{ (image.debugInfo.semScore * 100).toFixed(0) }}%</span>
        <span class="dim">×0.3</span>
      </div>
      <div class="debug-row">
        <span>OCR {{ (image.debugInfo.kwScore * 100).toFixed(0) }}%</span>
        <span class="dim">×0.4</span>
      </div>
      <div
        v-if="image.debugInfo.tagHit"
        class="debug-tag"
      >
        标签命中 ×0.3
      </div>
    </div>
    <ul
      v-if="menuVisible"
      class="context-menu"
      :style="{ top: menuY + 'px', left: menuX + 'px' }"
    >
      <li>
        <button @click.stop="handleOpen">
          查看详情
        </button>
      </li>
      <li>
        <button @click.stop="handleReveal">
          在文件夹中显示
        </button>
      </li>
      <li>
        <button
          data-action="delete"
          class="danger"
          @click.stop="handleDelete"
        >
          删除
        </button>
      </li>
    </ul>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { useClipboard } from "@/composables/useClipboard";
import { showToast } from "@/composables/useToast";
import type { SearchResult } from "@/stores/search";

const props = defineProps<{
  image: SearchResult;
  showDebugInfo: boolean;
  selectable?: boolean;
  selected?: boolean;
}>();
const emit = defineEmits<{
  delete: [id: string];
  select: [id: string];
  open: [id: string];
}>();
const { copyImage } = useClipboard();

const menuVisible = ref(false);
const menuX = ref(0);
const menuY = ref(0);
const imgError = ref(false);

const formatBadge = computed(() => {
  const fmt = props.image.fileFormat?.toLowerCase();
  if (fmt === "gif") return "GIF";
  if (fmt === "webp") return "WEBP";
  return null;
});

async function handleClick() {
  try {
    await copyImage(props.image.id);
    showToast("已复制", "info", 1500);
  } catch {
    showToast("复制失败", "error", 1500);
  }
}

function handleOpen() {
  menuVisible.value = false;
  emit("open", props.image.id);
}

async function handleReveal() {
  menuVisible.value = false;
  await invoke("reveal_in_finder", { id: props.image.id }).catch(() => {});
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

.img-missing {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f0f0f0;
  color: #aaa;
  font-size: 0.78rem;
}

.format-badge {
  position: absolute;
  top: 5px;
  right: 5px;
  background: rgba(0, 0, 0, 0.55);
  color: #fff;
  font-size: 0.65rem;
  font-weight: 700;
  padding: 1px 5px;
  border-radius: 3px;
  pointer-events: none;
  letter-spacing: 0.03em;
}

.select-checkbox {
  position: absolute;
  top: 6px;
  left: 6px;
  width: 18px;
  height: 18px;
  cursor: pointer;
  z-index: 10;
  accent-color: #646cff;
}

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
.debug-divider { border-top: 1px solid rgba(255,255,255,0.2); margin: 0.15rem 0; }
.debug-tag { color: #ffd700; font-weight: 600; }

.context-menu {
  position: absolute;
  background: #fff;
  border: 1px solid #ddd;
  border-radius: 6px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.15);
  list-style: none;
  padding: 0.25rem 0;
  min-width: 140px;
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
  color: #333;
}
.context-menu li button:hover { background: #f5f5f5; }
.context-menu li button.danger { color: #c0392b; }
</style>
