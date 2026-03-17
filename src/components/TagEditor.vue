<template>
  <div class="tag-editor">
    <span v-for="tag in tags" :key="tag" class="tag">
      {{ tag }}
      <button @click="removeTag(tag)">×</button>
    </span>
    <input
      v-model="input"
      placeholder="添加标签..."
      @keydown.enter.prevent="addTag"
      @keydown.comma.prevent="addTag"
    />
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";

const props = defineProps<{ tags: string[] }>();
const emit = defineEmits<{ "update:tags": [tags: string[]] }>();
const input = ref("");

function addTag() {
  const t = input.value.trim().replace(/,$/, "");
  if (t && !props.tags.includes(t)) emit("update:tags", [...props.tags, t]);
  input.value = "";
}

function removeTag(tag: string) {
  emit("update:tags", props.tags.filter((t) => t !== tag));
}
</script>

<style scoped>
.tag-editor { display: flex; flex-wrap: wrap; gap: 0.4rem; align-items: center; }
.tag { background: #e0e0e0; border-radius: 4px; padding: 0.2rem 0.4rem; font-size: 0.85rem; display: flex; align-items: center; gap: 0.2rem; }
.tag button { border: none; background: none; cursor: pointer; font-size: 0.9rem; line-height: 1; }
input { border: 1px solid #ccc; border-radius: 4px; padding: 0.2rem 0.4rem; font-size: 0.85rem; }
</style>
