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

const props = defineProps<{ image: SearchResult }>();
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
