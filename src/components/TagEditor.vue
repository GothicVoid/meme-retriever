<template>
  <div class="tag-editor">
    <div class="tag-list">
      <template
        v-for="tag in tags"
        :key="tag.text"
      >
        <div
          v-if="editingTextFor !== tag.text"
          class="tag-chip"
        >
          <button
            class="tag-chip-text"
            @click="startEditing(tag)"
          >
            {{ tag.text }}
          </button>
          <button
            class="tag-chip-remove"
            aria-label="删除标签"
            @click="removeTag(tag)"
          >
            ×
          </button>
        </div>

        <div
          v-else
          class="tag-chip tag-chip--editing"
        >
          <input
            ref="editInputRef"
            v-model="editingText"
            class="tag-inline-input"
            @keydown.enter.prevent="confirmEditing"
            @keydown.esc.prevent="cancelEditing"
            @blur="confirmEditing"
          >
        </div>
      </template>

      <div
        v-if="adding"
        class="tag-chip tag-chip--adding"
      >
        <input
          ref="addInputRef"
          v-model="addInput"
          class="tag-inline-input"
          placeholder="例如：猫猫疑惑、老板阴阳怪气"
          @keydown.enter.prevent="confirmAdd"
          @keydown.esc.prevent="cancelAdd"
          @blur="confirmAdd"
        >
      </div>

      <button
        v-else
        class="tag-add-btn"
        @click="startAdd"
      >
        + 添加标签
      </button>

      <div
        v-if="tags.length === 0 && !adding"
        class="tag-empty"
      >
        还没有标签
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import type { StructuredTag } from "@/types/tags";
import { createManualTag } from "@/types/tags";

const props = defineProps<{ tags: StructuredTag[] }>();
const emit = defineEmits<{ "update:tags": [tags: StructuredTag[]] }>();

const addInputRef = ref<HTMLInputElement | null>(null);
const editInputRef = ref<HTMLInputElement | null>(null);

const adding = ref(false);
const addInput = ref("");

const editingTextFor = ref<string | null>(null);
const editingText = ref("");

const normalizedTagTexts = computed(() => new Set(props.tags.map((tag) => tag.text.trim()).filter(Boolean)));

function replaceTag(target: StructuredTag, next: StructuredTag) {
  emit("update:tags", props.tags.map((tag) => (tag === target ? next : tag)));
}

function removeTag(target: StructuredTag) {
  if (editingTextFor.value === target.text) {
    editingTextFor.value = null;
    editingText.value = "";
  }
  emit("update:tags", props.tags.filter((tag) => tag !== target));
}

function startAdd() {
  flushPendingInput();
  adding.value = true;
  void nextTick(() => {
    if (addInputRef.value && typeof addInputRef.value.focus === "function") {
      addInputRef.value.focus();
    }
  });
}

function confirmAdd() {
  const text = addInput.value.trim();
  if (!adding.value) {
    addInput.value = "";
    return;
  }
  if (!text) {
    adding.value = false;
    addInput.value = "";
    return;
  }
  if (normalizedTagTexts.value.has(text)) {
    addInput.value = "";
    adding.value = false;
    return;
  }
  emit("update:tags", [...props.tags, createManualTag(text)]);
  addInput.value = "";
  adding.value = false;
}

function cancelAdd() {
  addInput.value = "";
  adding.value = false;
}

function startEditing(tag: StructuredTag) {
  flushPendingInput();
  editingTextFor.value = tag.text;
  editingText.value = tag.text;
  void nextTick(() => {
    if (editInputRef.value && typeof editInputRef.value.focus === "function") {
      editInputRef.value.focus();
    }
  });
}

function confirmEditing() {
  const originalText = editingTextFor.value;
  if (!originalText) {
    return;
  }
  const nextText = editingText.value.trim();
  const target = props.tags.find((tag) => tag.text === originalText);
  if (!target) {
    cancelEditing();
    return;
  }
  if (!nextText || (nextText !== target.text && normalizedTagTexts.value.has(nextText))) {
    editingText.value = target.text;
    editingTextFor.value = null;
    return;
  }
  replaceTag(target, { ...createManualTag(nextText), category: target.category });
  editingTextFor.value = null;
  editingText.value = "";
}

function cancelEditing() {
  editingTextFor.value = null;
  editingText.value = "";
}

function flushPendingInput() {
  if (editingTextFor.value) {
    confirmEditing();
  }
  if (adding.value && addInput.value.trim()) {
    confirmAdd();
  }
}

defineExpose({
  flushPendingInput,
});
</script>

<style scoped>
.tag-editor {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
}

.tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
  align-items: center;
}

.tag-chip,
.tag-add-btn {
  min-height: 2.2rem;
  border-radius: 999px;
  display: inline-flex;
  align-items: center;
}

.tag-chip {
  gap: 0.4rem;
  padding: 0.2rem 0.2rem 0.2rem 0.7rem;
  border: 1px solid #d6c8b4;
  background: #fff9ef;
}

.tag-chip-text,
.tag-chip-remove {
  border: none;
  background: none;
  cursor: pointer;
  padding: 0;
}

.tag-chip-text {
  font-size: 0.9rem;
  color: #1f2937;
}

.tag-chip-remove {
  width: 1.7rem;
  height: 1.7rem;
  border-radius: 999px;
  font-size: 1rem;
  color: #6b7280;
}

.tag-chip-remove:hover {
  background: rgba(0, 0, 0, 0.06);
  color: #111827;
}

.tag-add-btn {
  padding: 0 0.9rem;
  border: 1px dashed #c7b291;
  background: rgba(255, 255, 255, 0.6);
  color: #6c4a1b;
  cursor: pointer;
  font-size: 0.88rem;
}

.tag-add-btn:hover {
  background: rgba(255, 255, 255, 0.92);
}

.tag-chip--adding,
.tag-chip--editing {
  padding: 0.2rem 0.55rem;
}

.tag-inline-input {
  min-width: 9rem;
  border: none;
  background: transparent;
  font-size: 0.9rem;
  outline: none;
  color: #111827;
}

.tag-empty {
  font-size: 0.84rem;
  color: #7b8794;
}
</style>
