<template>
  <div class="search-bar">
    <input
      ref="inputRef"
      :value="modelValue"
      :placeholder="placeholder"
      @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
      @keydown.esc="$emit('update:modelValue', '')"
    >
    <button
      v-if="modelValue"
      @click="$emit('update:modelValue', '')"
    >
      ✕
    </button>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";

withDefaults(defineProps<{
  modelValue: string;
  placeholder?: string;
}>(), {
  placeholder: "搜索表情包...",
});
defineEmits<{ "update:modelValue": [value: string] }>();

const inputRef = ref<HTMLInputElement>();

function handleGlobalKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === "f") {
    e.preventDefault();
    inputRef.value?.focus();
  }
}

onMounted(() => window.addEventListener("keydown", handleGlobalKeydown));
onUnmounted(() => window.removeEventListener("keydown", handleGlobalKeydown));
</script>

<style scoped>
.search-bar {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 1rem;
}
.search-bar input {
  flex: 1;
  padding: 0.5rem 0.75rem;
  font-size: 1rem;
  border: 1px solid #ccc;
  border-radius: 6px;
}
.search-bar button {
  padding: 0.4rem 0.6rem;
  cursor: pointer;
  border: none;
  background: transparent;
  font-size: 1rem;
  color: #888;
}
</style>
