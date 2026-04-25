<template>
  <div class="search-bar ui-input-shell">
    <span
      class="search-bar__icon"
      aria-hidden="true"
    >
      搜
    </span>
    <input
      ref="inputRef"
      class="search-bar__input ui-input"
      :value="modelValue"
      :placeholder="placeholder"
      aria-label="搜索表情"
      @input="handleInput"
      @keydown="handleKeydown"
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
  keydown: [event: KeyboardEvent];
}>();

const inputRef = ref<HTMLInputElement>();
const isComposing = ref(false);

function focusAndSelect() {
  inputRef.value?.focus();
  inputRef.value?.select();
}

function handleInput(event: Event) {
  if (isComposing.value) return;
  emit("update:modelValue", (event.target as HTMLInputElement).value);
}

function handleKeydown(event: KeyboardEvent) {
  emit("keydown", event);
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

defineExpose({
  focusAndSelect,
});
</script>

<style scoped>
.search-bar {
  margin-bottom: 1rem;
}

.search-bar__input {
  min-height: 48px;
  font-size: 1rem;
}

.search-bar__icon {
  width: 1.75rem;
  height: 1.75rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  background: rgba(183, 121, 31, 0.12);
  color: var(--ui-accent);
  font-size: 0.75rem;
  font-weight: 700;
  flex-shrink: 0;
}

.search-bar__clear {
  font-size: 1rem;
}
</style>
