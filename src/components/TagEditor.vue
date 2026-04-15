<template>
  <div class="tag-editor">
    <section
      v-for="group in groups"
      :key="group.category"
      class="tag-group"
      :class="`tag-group--${group.category}`"
    >
      <header class="tag-group-header">
        <div class="tag-group-title">
          <span class="tag-group-dot" />
          {{ group.label }}
        </div>
      </header>

      <div class="tag-group-body">
        <template v-for="tag in tagsByCategory[group.category]" :key="`${group.category}-${tag.text}`">
          <div
            v-if="editingKey !== tagKey(tag)"
            class="tag-chip"
            :class="{ auto: tag.isAuto }"
          >
            <button class="tag-chip-text" @click="startEditing(tag)">
              {{ tag.text }}
            </button>
            <span v-if="tag.isAuto" class="tag-chip-badge">
              自动
            </span>
            <button class="tag-chip-remove" @click="removeTag(tag)">
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
          v-if="addingCategory === group.category"
          class="tag-chip tag-chip--adding"
        >
          <input
            ref="addInputRef"
            v-model="addInput"
            class="tag-inline-input"
            :placeholder="`添加${group.label}标签`"
            @keydown.enter.prevent="confirmAdd"
            @keydown.esc.prevent="cancelAdd"
            @blur="confirmAdd"
          >
        </div>

        <button
          v-else
          class="tag-add-btn"
          @click="startAdd(group.category)"
        >
          + 添加
        </button>

        <div
          v-if="tagsByCategory[group.category].length === 0 && addingCategory !== group.category"
          class="tag-empty"
        >
          暂无标签
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import type { StructuredTag, TagCategory } from "@/types/tags";
import { createManualTag } from "@/types/tags";

const props = defineProps<{ tags: StructuredTag[] }>();
const emit = defineEmits<{ "update:tags": [tags: StructuredTag[]] }>();

const groups: Array<{ category: TagCategory; label: string }> = [
  { category: "custom", label: "自定义" },
  { category: "meme", label: "梗" },
  { category: "person", label: "人物" },
  { category: "source", label: "出处" },
];

const addInputRef = ref<HTMLInputElement | null>(null);
const editInputRef = ref<HTMLInputElement | null>(null);

const addingCategory = ref<TagCategory | null>(null);
const addInput = ref("");

const editingKey = ref<string | null>(null);
const editingText = ref("");

const tagsByCategory = computed<Record<TagCategory, StructuredTag[]>>(() => ({
  custom: props.tags.filter((tag) => tag.category === "custom"),
  meme: props.tags.filter((tag) => tag.category === "meme"),
  person: props.tags.filter((tag) => tag.category === "person"),
  source: props.tags.filter((tag) => tag.category === "source"),
}));

function tagKey(tag: StructuredTag) {
  return `${tag.category}:${tag.text}`;
}

function replaceTag(target: StructuredTag, next: StructuredTag) {
  emit("update:tags", props.tags.map((tag) => (tag === target ? next : tag)));
}

function removeTag(target: StructuredTag) {
  if (editingKey.value === tagKey(target)) {
    editingKey.value = null;
    editingText.value = "";
  }
  emit("update:tags", props.tags.filter((tag) => tag !== target));
}

function startAdd(category: TagCategory) {
  flushPendingInput();
  addingCategory.value = category;
  void nextTick(() => {
    if (addInputRef.value && typeof addInputRef.value.focus === "function") {
      addInputRef.value.focus();
    }
  });
}

function confirmAdd() {
  const text = addInput.value.trim();
  if (!addingCategory.value) {
    addInput.value = "";
    return;
  }
  if (!text) {
    addingCategory.value = null;
    addInput.value = "";
    return;
  }
  if (props.tags.some((tag) => tag.text === text)) {
    addInput.value = "";
    addingCategory.value = null;
    return;
  }
  const nextTag = {
    ...createManualTag(text),
    category: addingCategory.value,
  };
  emit("update:tags", [...props.tags, nextTag]);
  addInput.value = "";
  addingCategory.value = null;
}

function cancelAdd() {
  addInput.value = "";
  addingCategory.value = null;
}

function startEditing(tag: StructuredTag) {
  flushPendingInput();
  editingKey.value = tagKey(tag);
  editingText.value = tag.text;
  void nextTick(() => {
    if (editInputRef.value && typeof editInputRef.value.focus === "function") {
      editInputRef.value.focus();
    }
  });
}

function confirmEditing() {
  const key = editingKey.value;
  if (!key) {
    return;
  }
  const nextText = editingText.value.trim();
  const target = props.tags.find((tag) => tagKey(tag) === key);
  if (!target) {
    cancelEditing();
    return;
  }
  if (!nextText || (nextText !== target.text && props.tags.some((tag) => tag.text === nextText))) {
    editingText.value = target.text;
    editingKey.value = null;
    return;
  }
  replaceTag(target, { ...target, text: nextText });
  editingKey.value = null;
  editingText.value = "";
}

function cancelEditing() {
  editingKey.value = null;
  editingText.value = "";
}

function flushPendingInput() {
  if (editingKey.value) {
    confirmEditing();
  }
  if (addingCategory.value && addInput.value.trim()) {
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
  gap: 1rem;
}

.tag-group {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
  padding: 0.85rem 0.9rem;
  border-radius: 14px;
  border: 1px solid #e7e7e7;
  background: linear-gradient(180deg, #fff, #fbfbfb);
}

.tag-group--custom {
  border-color: #dbe0ea;
  background: linear-gradient(180deg, #f9fbff, #f3f6fb);
}

.tag-group--meme {
  border-color: #f1d1aa;
  background: linear-gradient(180deg, #fff8ef, #fff3e2);
}

.tag-group--person {
  border-color: #c7e0cf;
  background: linear-gradient(180deg, #f3fbf5, #eaf7ee);
}

.tag-group--source {
  border-color: #e4c6cf;
  background: linear-gradient(180deg, #fff7f8, #fff0f2);
}

.tag-group-header {
  display: flex;
  align-items: center;
}

.tag-group-title {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  font-size: 0.88rem;
  font-weight: 700;
  color: #334155;
}

.tag-group-dot {
  width: 0.6rem;
  height: 0.6rem;
  border-radius: 999px;
  background: currentColor;
  opacity: 0.65;
}

.tag-group--custom .tag-group-title { color: #556277; }
.tag-group--meme .tag-group-title { color: #a55a16; }
.tag-group--person .tag-group-title { color: #2f7a48; }
.tag-group--source .tag-group-title { color: #a04059; }

.tag-group-body {
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
  border: 1px solid rgba(0, 0, 0, 0.08);
  background: rgba(255, 255, 255, 0.86);
}

.tag-chip.auto {
  box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.65);
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

.tag-chip-badge {
  font-size: 0.7rem;
  line-height: 1;
  color: #476355;
  background: rgba(90, 132, 104, 0.12);
  padding: 0.25rem 0.45rem;
  border-radius: 999px;
}

.tag-add-btn {
  padding: 0 0.9rem;
  border: 1px dashed rgba(0, 0, 0, 0.18);
  background: rgba(255, 255, 255, 0.6);
  color: #475569;
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
