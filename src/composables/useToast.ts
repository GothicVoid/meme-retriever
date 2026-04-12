type ToastType = "info" | "error";

type ToastListener = (message: string, type: ToastType, duration?: number) => void;

const listeners = new Set<ToastListener>();

export function showToast(message: string, type: ToastType = "info", duration?: number) {
  listeners.forEach(listener => listener(message, type, duration));
}

export function useToast() {
  function subscribe(listener: ToastListener) {
    listeners.add(listener);
    return () => listeners.delete(listener);
  }

  return { showToast, subscribe };
}
