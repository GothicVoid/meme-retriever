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
      v-if="placeholderState === 'normal'"
      :src="convertFileSrc(image.thumbnailPath || image.filePath)"
      :alt="image.id"
      loading="lazy"
      @error="handleImageError"
    >
    <div
      v-else
      class="img-missing"
      :title="placeholderTitle"
    >
      <span>{{ placeholderText }}</span>
    </div>
    <span
      v-if="formatBadge"
      class="format-badge"
    >{{ formatBadge }}</span>
    <span
      v-if="image.fileStatus === 'missing'"
      class="status-badge"
    >文件已丢失</span>
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
    <Teleport to="body">
      <ul
        v-if="menuVisible"
        ref="menuRef"
        class="context-menu"
        :style="{ top: `${menuY}px`, left: `${menuX}px` }"
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
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { useClipboard } from "@/composables/useClipboard";
import { showToast } from "@/composables/useToast";
import type { SearchResult } from "@/stores/search";

const CLOSE_CONTEXT_MENU_EVENT = "image-card:close-context-menu";

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
const menuRef = ref<HTMLElement | null>(null);
const imgError = ref<"normal" | "missing" | "load-failed" | "gif-damaged">(
  props.image.fileStatus === "missing" ? "missing" : "normal",
);

const formatBadge = computed(() => {
  const fmt = props.image.fileFormat?.toLowerCase();
  if (fmt === "gif") return "GIF";
  if (fmt === "webp") return "WEBP";
  return null;
});

const placeholderState = computed(() => imgError.value);
const placeholderText = computed(() => {
  if (placeholderState.value === "missing") return "图片不存在";
  return "加载失败";
});
const placeholderTitle = computed(() => {
  if (placeholderState.value === "missing") return "原文件已丢失";
  if (placeholderState.value === "gif-damaged") return "GIF文件损坏";
  return "";
});

async function handleClick() {
  try {
    await copyImage(props.image.id);
    showToast("已复制", "info", 1500);
  } catch (error) {
    const message = String(error);
    showToast(
      message.includes("原文件已丢失") ? "原文件已丢失，无法复制" : "复制失败",
      "error",
      1500,
    );
  }
}

function handleOpen() {
  menuVisible.value = false;
  emit("open", props.image.id);
}

async function handleReveal() {
  menuVisible.value = false;
  await invoke("reveal_in_finder", { id: props.image.id }).catch((error) => {
    if (String(error).includes("原文件已丢失")) {
      showToast("原文件已丢失，无法定位", "error", 1500);
    }
  });
}

function showMenu(e: MouseEvent) {
  e.stopPropagation();
  document.dispatchEvent(new CustomEvent(CLOSE_CONTEXT_MENU_EVENT));
  menuX.value = e.clientX;
  menuY.value = e.clientY;
  menuVisible.value = true;
  void nextTick(() => {
    const menu = menuRef.value;
    if (!menu) return;

    const padding = 8;
    const { innerWidth, innerHeight } = window;
    const { width, height } = menu.getBoundingClientRect();

    menuX.value = Math.min(menuX.value, innerWidth - width - padding);
    menuY.value = Math.min(menuY.value, innerHeight - height - padding);
    menuX.value = Math.max(padding, menuX.value);
    menuY.value = Math.max(padding, menuY.value);
  });
}

function closeMenu() {
  menuVisible.value = false;
}

function handleDelete() {
  menuVisible.value = false;
  emit("delete", props.image.id);
}

function handleImageError() {
  if (props.image.fileStatus === "missing") {
    imgError.value = "missing";
    return;
  }
  imgError.value = props.image.fileFormat?.toLowerCase() === "gif" ? "gif-damaged" : "load-failed";
}

onMounted(() => document.addEventListener("click", closeMenu));
onMounted(() => document.addEventListener(CLOSE_CONTEXT_MENU_EVENT, closeMenu));
onUnmounted(() => document.removeEventListener("click", closeMenu));
onUnmounted(() => document.removeEventListener(CLOSE_CONTEXT_MENU_EVENT, closeMenu));
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

.status-badge {
  position: absolute;
  left: 5px;
  bottom: 5px;
  background: rgba(192, 57, 43, 0.86);
  color: #fff;
  font-size: 0.64rem;
  padding: 2px 6px;
  border-radius: 4px;
  pointer-events: none;
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
  position: fixed;
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
