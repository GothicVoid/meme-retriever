<template>
  <div
    class="image-card"
    @click="handleClick"
    @dblclick="emit('open', image.id)"
    @contextmenu.prevent="showMenu"
  >
    <div class="image-media">
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
      :class="{ 'debug-overlay--compact': !reasonSummary }"
    >
        <div class="debug-score">
          最终得分 {{ (image.score * 100).toFixed(1) }}%
        </div>
        <div class="debug-row">
          <span>主路 {{ debugRouteLabel }}</span>
          <span class="dim">{{ (image.debugInfo.mainScore * 100).toFixed(0) }}%</span>
        </div>
        <div class="debug-row">
          <span>辅路补充</span>
          <span class="dim">{{ (image.debugInfo.auxScore * 100).toFixed(0) }}%</span>
        </div>
        <div class="debug-row">
          <span>标签贡献</span>
          <span class="dim">{{ (image.debugInfo.tagScore * 100).toFixed(0) }}%</span>
        </div>
        <div class="debug-row">
          <span>热度加成</span>
          <span class="dim">{{ (image.debugInfo.popularityBoost * 100).toFixed(0) }}%</span>
        </div>
      </div>
    </div>
    <div
      v-if="reasonSummary"
      class="reason-panel"
    >
      <div class="reason-header">
        <span class="relevance-badge" :class="relevanceBadgeClass">{{ relevanceLabel }}</span>
        <span class="reason-title">{{ primaryReasonLabel }}</span>
      </div>
      <div class="reason-evidence">
        <span
          v-for="item in evidenceList"
          :key="item"
          class="reason-pill"
        >
          {{ item }}
        </span>
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
import { getRelevanceBadgeClass, getRelevanceLabel } from "@/utils/relevance";

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
const debugInfo = computed(() => props.image.debugInfo);

const relevanceLabel = computed(() => getRelevanceLabel(props.image.score));

const relevanceBadgeClass = computed(() => getRelevanceBadgeClass(props.image.score));

const primaryReasonLabel = computed(() => {
  const route = debugInfo.value?.mainRoute;
  if (route === "ocr") return "文字匹配优先";
  if (route === "privateRole") return "角色匹配优先";
  return "语义最接近";
});

const debugRouteLabel = computed(() => {
  const route = debugInfo.value?.mainRoute;
  if (route === "ocr") return "文字";
  if (route === "privateRole") return "角色";
  return "语义";
});

const evidenceList = computed(() => {
  if (!debugInfo.value) return [];

  const items: string[] = [];
  for (const term of props.image.matchedOcrTerms ?? []) {
    items.push(`命中文字：${term}`);
  }
  for (const tag of props.image.matchedTags ?? []) {
    items.push(`标签命中：${tag}`);
  }
  if (props.image.matchedRoleName) {
    items.push(`角色命中：${props.image.matchedRoleName}`);
  }
  if (items.length === 0 && debugInfo.value.mainRoute === "ocr" && debugInfo.value.kwScore > 0) {
    items.push(debugInfo.value.kwScore >= 0.6 ? "图片文字高度相关" : "图片文字相关");
  }
  if (items.length === 0 && debugInfo.value.mainRoute === "semantic" && debugInfo.value.semScore > 0) {
    items.push(debugInfo.value.semScore >= 0.65 ? "图片内容很接近" : "图片内容接近");
  }
  if (items.length === 0 && debugInfo.value.mainRoute === "privateRole") {
    items.push("私有角色线索命中");
  }
  if ((props.image.matchedTags?.length ?? 0) === 0 && debugInfo.value.tagScore > 0) {
    items.push(debugInfo.value.tagScore >= 0.6 ? "标签命中较强" : "标签相关");
  }
  if (debugInfo.value.mainRoute !== "semantic" && debugInfo.value.semScore > 0.6) {
    items.push("图片内容也较接近");
  }
  if (debugInfo.value.popularityBoost >= 0.06) {
    items.push("最近常用");
  }

  return items.slice(0, 3);
});

const reasonSummary = computed(() => debugInfo.value ? `${relevanceLabel.value} ${primaryReasonLabel.value}` : "");

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
  border-radius: 10px;
  overflow: hidden;
  background: #fff;
  position: relative;
  display: flex;
  flex-direction: column;
  box-shadow: 0 1px 4px rgba(15, 23, 42, 0.08);
  border: 1px solid #e5e7eb;
}

.image-media {
  position: relative;
  aspect-ratio: 1;
  overflow: hidden;
  background: #eee;
}

.image-card img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}
.image-card:hover {
  border-color: #d0d7de;
  box-shadow: 0 6px 20px rgba(15, 23, 42, 0.12);
}

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

.reason-panel {
  padding: 0.55rem 0.6rem 0.65rem;
  background: #fff;
  color: #111827;
  min-height: 74px;
}

.reason-header {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  margin-bottom: 0.35rem;
}

.relevance-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 0.6rem;
  line-height: 1;
  padding: 0.22rem 0.35rem;
  border-radius: 999px;
  font-weight: 700;
  letter-spacing: 0.02em;
  flex: 0 0 auto;
}

.relevance-badge--strong {
  background: #f3c64d;
  color: #1d1600;
}

.relevance-badge--medium {
  background: #89d0a0;
  color: #08331b;
}

.relevance-badge--weak {
  background: #d0d7de;
  color: #1f2933;
}

.reason-title {
  font-size: 0.76rem;
  font-weight: 700;
  line-height: 1.2;
  color: #111827;
}

.reason-evidence {
  display: flex;
  flex-wrap: wrap;
  gap: 0.28rem;
}

.reason-pill {
  display: inline-flex;
  align-items: center;
  max-width: 100%;
  padding: 0.18rem 0.38rem;
  border-radius: 999px;
  background: #f3f4f6;
  color: #374151;
  font-size: 0.64rem;
  line-height: 1.2;
}

.debug-overlay {
  position: absolute;
  top: 6px;
  left: 6px;
  min-width: 108px;
  background: rgba(7, 14, 22, 0.82);
  color: #fff;
  font-size: 0.64rem;
  padding: 0.35rem 0.45rem;
  line-height: 1.45;
  pointer-events: none;
  border-radius: 6px;
  backdrop-filter: blur(4px);
}
.debug-overlay--compact {
  max-width: calc(100% - 12px);
}
.debug-score { font-size: 0.72rem; font-weight: 700; margin-bottom: 0.15rem; }
.debug-row { display: flex; justify-content: space-between; }
.dim { opacity: 0.7; }

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
