import { useDebounceFn } from "@vueuse/core";
import { useSearchStore } from "@/stores/search";

export function useSearch() {
  const store = useSearchStore();

  const debouncedSearch = useDebounceFn((q: string) => {
    store.search(q);
  }, 300);

  const debouncedRecordSearchHistory = useDebounceFn(async (
    q: string,
    onRecorded?: (query: string) => void,
    shouldRecord?: (query: string) => boolean,
  ) => {
    const normalized = q.trim();
    if (!normalized) {
      return;
    }
    if (shouldRecord && !shouldRecord(normalized)) {
      return;
    }
    await store.recordSearchHistory(normalized);
    onRecorded?.(normalized);
  }, 2000);

  return { store, debouncedSearch, debouncedRecordSearchHistory };
}
