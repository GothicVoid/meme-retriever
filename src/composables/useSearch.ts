import { useDebounceFn } from "@vueuse/core";
import { useSearchStore } from "@/stores/search";

export function useSearch() {
  const store = useSearchStore();

  const debouncedSearch = useDebounceFn((q: string) => {
    store.search(q);
  }, 300);

  return { store, debouncedSearch };
}
