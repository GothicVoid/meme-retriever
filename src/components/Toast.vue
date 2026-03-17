<template>
  <Teleport to="body">
    <div v-if="visible" class="toast" :class="type">{{ message }}</div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref } from "vue";

const visible = ref(false);
const message = ref("");
const type = ref<"info" | "error">("info");

function show(msg: string, t: "info" | "error" = "info", duration = 2000) {
  message.value = msg;
  type.value = t;
  visible.value = true;
  setTimeout(() => { visible.value = false; }, duration);
}

defineExpose({ show });
</script>

<style scoped>
.toast {
  position: fixed;
  bottom: 1.5rem;
  left: 50%;
  transform: translateX(-50%);
  padding: 0.6rem 1.2rem;
  border-radius: 6px;
  background: #333;
  color: #fff;
  font-size: 0.9rem;
  z-index: 9999;
}
.toast.error { background: #c0392b; }
</style>
