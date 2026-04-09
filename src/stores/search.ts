import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface ScoreDebugInfo {
  semScore: number;
  kwScore: number;
  tagHit: boolean;
  semWeight: number;
  kwWeight: number;
  relevance: number;
  popularity: number;
}

export interface SearchResult {
  id: string;
  filePath: string;
  thumbnailPath: string;
  fileFormat: string;
  score: number;
  tags: string[];
  debugInfo: ScoreDebugInfo | null;
}

export const useSearchStore = defineStore("search", () => {
  const query = ref("");
  const results = ref<SearchResult[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function search(q: string, limit = 9, w1?: number, w2?: number, w3?: number) {
    loading.value = true;
    error.value = null;
    try {
      results.value = await invoke<SearchResult[]>("search", { query: q, limit, w1, w2, w3 });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  return { query, results, loading, error, search };
});
