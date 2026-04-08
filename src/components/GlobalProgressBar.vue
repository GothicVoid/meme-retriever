<template>
  <div
    v-if="store.indexing"
    class="global-progress"
    @click="router.push('/library')"
  >
    <div
      class="global-progress-fill"
      :style="{ width: percent + '%' }"
    />
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useRouter } from "vue-router";
import { useLibraryStore } from "@/stores/library";

const store = useLibraryStore();
const router = useRouter();

const percent = computed(() =>
  store.indexTotal > 0 ? (store.indexCurrent / store.indexTotal) * 100 : 0
);
</script>

<style scoped>
.global-progress {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  height: 3px;
  background: #e0e0e0;
  cursor: pointer;
  z-index: 1000;
}
.global-progress-fill {
  height: 100%;
  background: #646cff;
  transition: width 0.3s;
}
</style>
