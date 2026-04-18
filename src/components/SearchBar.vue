<template>
  <div class="search-bar ui-input-shell">
    <input
      ref="inputRef"
      class="search-bar__input ui-input"
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
      class="search-bar__clear ui-input-clear"
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
  margin-bottom: 1rem;
}

.search-bar__input {
  min-height: 48px;
  font-size: 1rem;
}

.search-bar__clear {
  font-size: 1rem;
}
</style>
