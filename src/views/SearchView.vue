<template>
  <div class="search-view">
    <SearchBar v-model="store.query" @update:model-value="debouncedSearch" />
    <ImageGrid :images="store.results" :loading="store.loading" />
  </div>
</template>

<script setup lang="ts">
import SearchBar from "@/components/SearchBar.vue";
import ImageGrid from "@/components/ImageGrid.vue";
import { useSearch } from "@/composables/useSearch";

const { store, debouncedSearch } = useSearch();
</script>

<style scoped>
.search-view { padding: 1rem; }
</style>
