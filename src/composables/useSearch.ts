import { useDebounceFn } from "@vueuse/core";
import { useSearchStore } from "@/stores/search";
import { useSettingsStore } from "@/stores/settings";

export function useSearch() {
  const store = useSearchStore();
  const settings = useSettingsStore();

  const debouncedSearch = useDebounceFn((q: string) => {
    const { w1, w2, w3 } = settings.normalizedWeights;
    store.search(q, undefined, w1, w2, w3);
  }, 300);

  return { store, debouncedSearch };
}
