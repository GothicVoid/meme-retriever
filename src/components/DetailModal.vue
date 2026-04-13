<template>
  <div
    class="modal-backdrop"
    @click.self="emit('close')"
    @keydown.esc="emit('close')"
  >
    <div class="modal">
      <button class="close-btn" @click="emit('close')">×</button>

      <button
        v-if="hasPrev"
        class="nav-btn prev"
        @click="navigate(-1)"
      >
        ‹
      </button>
      <button
        v-if="hasNext"
        class="nav-btn next"
        @click="navigate(1)"
      >
        ›
      </button>

      <div class="modal-body">
        <!-- 图片区 -->
        <div class="img-area">
          <template v-if="!isMissing">
            <img
              :src="imgSrc"
              :alt="currentImage.id"
              class="main-img"
            >
          </template>
          <div
            v-else
            class="missing-state"
          >
            <p class="missing-title">
              原文件已丢失
            </p>
            <p class="missing-desc">
              你可以重新定位图片文件以恢复详情和复制能力。
            </p>
            <div class="missing-actions">
              <button
                class="relocate-btn"
                :disabled="relocating"
                @click="handleRelocate"
              >
                {{ relocating ? "重新定位中..." : "重新定位" }}
              </button>
              <button
                class="delete-btn"
                @click="emit('delete', currentImage.id)"
              >
                删除图片
              </button>
            </div>
          </div>
          <button
            v-if="isLargeGif && !isMissing"
            class="gif-toggle"
            @click="gifPlaying = !gifPlaying"
          >
            {{ gifPlaying ? '⏸ 显示缩略图' : '▶ 播放 GIF（大文件）' }}
          </button>
        </div>

        <!-- 元数据区 -->
        <div class="meta-area">
          <div class="meta-row">
            <span class="meta-label">格式</span>
            <span>{{ meta?.fileFormat?.toUpperCase() ?? currentImage.fileFormat?.toUpperCase() ?? '—' }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">状态</span>
            <span>{{ isMissing ? "文件已丢失" : "正常" }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">尺寸</span>
            <span>{{ meta ? `${meta.width} × ${meta.height}` : '—' }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">大小</span>
            <span>{{ meta ? formatSize(meta.fileSize ?? 0) : '—' }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">添加时间</span>
            <span>{{ meta ? formatDate(meta.addedAt) : '—' }}</span>
          </div>
          <div class="meta-row">
            <span class="meta-label">使用次数</span>
            <span>{{ meta?.useCount ?? '—' }}</span>
          </div>

          <div class="tags-section">
            <div class="meta-label">标签</div>
            <TagEditor
              :tags="editTags"
              @update:tags="editTags = $event"
            />
            <button
              class="save-btn"
              :disabled="saving || isMissing"
              @click="saveTags"
            >
              {{ saving ? '保存中...' : '保存标签' }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import TagEditor from "@/components/TagEditor.vue";
import type { SearchResult } from "@/stores/search";
import type { ImageMeta } from "@/stores/library";
import { showToast } from "@/composables/useToast";

const props = defineProps<{
  imageId: string;
  images: SearchResult[];
}>();
const emit = defineEmits<{ close: []; delete: [id: string] }>();

const currentIndex = ref(props.images.findIndex((i) => i.id === props.imageId));
const currentImage = computed(() => props.images[currentIndex.value]);
const hasPrev = computed(() => currentIndex.value > 0);
const hasNext = computed(() => currentIndex.value < props.images.length - 1);

const meta = ref<ImageMeta | null>(null);
const editTags = ref<string[]>([]);
const saving = ref(false);
const relocating = ref(false);
const isMissing = computed(() => meta.value?.fileStatus === "missing");

const isGif = computed(() => {
  const fmt = (meta.value?.fileFormat ?? currentImage.value?.fileFormat ?? "").toLowerCase();
  return fmt === "gif";
});
// >10MB 大文件 GIF 默认不播放
const isLargeGif = computed(() => isGif.value && (meta.value?.fileSize ?? 0) > 10 * 1024 * 1024);
const gifPlaying = ref(false);

// 大文件 GIF 未播放时显示缩略图，否则显示原图
const imgSrc = computed(() => {
  const previewPath = meta.value?.thumbnailPath || currentImage.value.thumbnailPath || currentImage.value.filePath;
  const originalPath = meta.value?.filePath || currentImage.value.filePath;
  if (isLargeGif.value && !gifPlaying.value) {
    return convertFileSrc(previewPath);
  }
  return convertFileSrc(originalPath);
});

async function loadMeta(id: string) {
  meta.value = null;
  try {
    meta.value = await invoke<ImageMeta | null>("get_image_meta", { id });
    editTags.value = meta.value?.tags ?? currentImage.value?.tags ?? [];
    // GIF 自动播放（小文件）
    gifPlaying.value = isGif.value && !isLargeGif.value;
  } catch {
    editTags.value = currentImage.value?.tags ?? [];
  }
}

watch(currentImage, (img) => {
  if (img) loadMeta(img.id);
}, { immediate: true });

function navigate(dir: -1 | 1) {
  const next = currentIndex.value + dir;
  if (next >= 0 && next < props.images.length) {
    currentIndex.value = next;
  }
}

async function saveTags() {
  if (isMissing.value) return;
  saving.value = true;
  try {
    await invoke("update_tags", { imageId: currentImage.value.id, tags: editTags.value });
    if (meta.value) meta.value.tags = [...editTags.value];
  } finally {
    saving.value = false;
  }
}

function formatSize(bytes: number): string {
  if (bytes === 0) return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function formatDate(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString("zh-CN");
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "ArrowLeft") navigate(-1);
  else if (e.key === "ArrowRight") navigate(1);
  else if (e.key === "Escape") emit("close");
}

async function handleRelocate() {
  const selected = await open({
    multiple: false,
    filters: [{ name: "图片", extensions: ["jpg", "jpeg", "png", "gif", "webp"] }],
  });
  if (!selected || Array.isArray(selected)) return;

  relocating.value = true;
  try {
    meta.value = await invoke<ImageMeta>("relocate_image", {
      id: currentImage.value.id,
      newPath: selected,
    });
    gifPlaying.value = isGif.value && !isLargeGif.value;
    showToast("已重新定位图片", "info", 1500);
  } catch (error) {
    showToast(String(error), "error", 2000);
  } finally {
    relocating.value = false;
  }
}

onMounted(() => document.addEventListener("keydown", onKeydown));
onUnmounted(() => document.removeEventListener("keydown", onKeydown));
</script>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal {
  background: #fff;
  border-radius: 10px;
  max-width: 860px;
  width: 90vw;
  max-height: 90vh;
  overflow: hidden;
  position: relative;
  display: flex;
  flex-direction: column;
}

.close-btn {
  position: absolute;
  top: 10px;
  right: 14px;
  background: none;
  border: none;
  font-size: 1.6rem;
  cursor: pointer;
  color: #666;
  z-index: 10;
  line-height: 1;
}
.close-btn:hover { color: #333; }

.nav-btn {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  background: rgba(0, 0, 0, 0.35);
  color: #fff;
  border: none;
  font-size: 2rem;
  width: 36px;
  height: 60px;
  cursor: pointer;
  z-index: 10;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.nav-btn:hover { background: rgba(0, 0, 0, 0.55); }
.prev { left: 8px; }
.next { right: 8px; }

.modal-body {
  display: flex;
  overflow: hidden;
  flex: 1;
}

.img-area {
  flex: 1;
  background: #111;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  min-height: 300px;
}

.missing-state {
  color: #fff;
  text-align: center;
  padding: 2rem;
  max-width: 26rem;
}

.missing-title {
  font-size: 1.2rem;
  font-weight: 600;
  margin: 0 0 0.5rem;
}

.missing-desc {
  color: rgba(255, 255, 255, 0.75);
  margin: 0 0 1rem;
  line-height: 1.5;
}

.missing-actions {
  display: flex;
  gap: 0.75rem;
  justify-content: center;
  flex-wrap: wrap;
}

.main-img {
  max-width: 100%;
  max-height: 70vh;
  object-fit: contain;
}

.gif-toggle {
  position: absolute;
  bottom: 10px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(0, 0, 0, 0.6);
  color: #fff;
  border: none;
  border-radius: 4px;
  padding: 0.3rem 0.8rem;
  cursor: pointer;
  font-size: 0.85rem;
}

.relocate-btn {
  border: 1px solid #fff;
  background: transparent;
  color: #fff;
  padding: 0.6rem 1rem;
  border-radius: 6px;
  cursor: pointer;
}

.relocate-btn:disabled {
  opacity: 0.6;
  cursor: default;
}

.delete-btn {
  border: 1px solid #c0392b;
  background: #c0392b;
  color: #fff;
  padding: 0.6rem 1rem;
  border-radius: 6px;
  cursor: pointer;
}

.meta-area {
  width: 220px;
  padding: 2.5rem 1rem 1rem;
  overflow-y: auto;
  border-left: 1px solid #eee;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.meta-row {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
}

.meta-label {
  font-size: 0.72rem;
  color: #999;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.tags-section {
  margin-top: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.save-btn {
  padding: 0.35rem 0.75rem;
  background: #646cff;
  color: #fff;
  border: none;
  border-radius: 5px;
  cursor: pointer;
  font-size: 0.85rem;
  align-self: flex-start;
}
.save-btn:disabled { opacity: 0.6; cursor: not-allowed; }
.save-btn:hover:not(:disabled) { background: #535bf2; }
</style>
