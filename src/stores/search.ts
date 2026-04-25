import { defineStore } from "pinia";
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { StructuredTag } from "@/types/tags";

export interface ScoreDebugInfo {
  mainRoute: string;
  mainScore: number;
  auxScore: number;
  semScore: number;
  kwScore: number;
  tagScore: number;
  popularityBoost: number;
}

export interface SearchResult {
  id: string;
  filePath: string;
  thumbnailPath: string;
  fileFormat: string;
  fileStatus?: string;
  score: number;
  tags: StructuredTag[];
  matchedOcrTerms?: string[];
  matchedTags?: string[];
  matchedRoleName?: string | null;
  debugInfo: ScoreDebugInfo | null;
}

export const useSearchStore = defineStore("search", () => {
  const DEFAULT_SEARCH_LIMIT = 30;
  const query = ref("");
  const results = ref<SearchResult[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const currentLimit = ref(DEFAULT_SEARCH_LIMIT);

  async function search(q: string, limit = DEFAULT_SEARCH_LIMIT) {
    loading.value = true;
    error.value = null;
    currentLimit.value = limit;
    try {
      results.value = await invoke<SearchResult[]>("search", { query: q, limit });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function recordSearchHistory(q: string) {
    const normalized = q.trim();
    if (!normalized) {
      return;
    }
    await invoke("record_search_history", { query: normalized });
  }

  return { query, results, loading, error, currentLimit, search, recordSearchHistory };
});
