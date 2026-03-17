import { ref } from "vue";
import { useLibraryStore } from "@/stores/library";

export function useDragDrop() {
  const isDragging = ref(false);
  const library = useLibraryStore();

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    isDragging.value = true;
  }

  function onDragLeave() {
    isDragging.value = false;
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    isDragging.value = false;
    const paths = Array.from(e.dataTransfer?.files ?? [])
      .map((f) => (f as File & { path?: string }).path ?? "")
      .filter(Boolean);
    if (paths.length) library.addImages(paths);
  }

  return { isDragging, onDragOver, onDragLeave, onDrop };
}
