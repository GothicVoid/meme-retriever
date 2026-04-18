<template>
  <div class="search-bar">
    <input
      ref="inputRef"
      :value="modelValue"
      :placeholder="placeholder"
      @input="handleInput"
      @keydown.esc="$emit('update:modelValue', '')"
      @focus="$emit('focus')"
      @blur="$emit('blur')"
      @compositionstart="handleCompositionStart"
      @compositionend="handleCompositionEnd"
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
const emit = defineEmits<{
  "update:modelValue": [value: string];
  focus: [];
  blur: [];
}>();

const inputRef = ref<HTMLInputElement>();
const isComposing = ref(false);

function handleInput(event: Event) {
  if (isComposing.value) return;
  emit("update:modelValue", (event.target as HTMLInputElement).value);
}

function handleCompositionStart() {
  isComposing.value = true;
}

function handleCompositionEnd(event: CompositionEvent) {
  isComposing.value = false;
  emit("update:modelValue", (event.target as HTMLInputElement).value);
}

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
