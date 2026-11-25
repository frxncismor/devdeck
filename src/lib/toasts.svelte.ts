export type ToastTone = "info" | "success" | "warning" | "error";

export interface Toast {
  id: number;
  tone: ToastTone;
  title: string;
  message?: string;
}

export interface ToastInput {
  tone?: ToastTone;
  title: string;
  message?: string;
}

/** Auto-dismiss delay; errors stay until dismissed. */
export const TOAST_TIMEOUT_MS = 5000;

export const toastState = $state<{ items: Toast[] }>({ items: [] });

let nextId = 1;
const timers: Record<number, ReturnType<typeof setTimeout>> = {};

export function dismissToast(id: number): void {
  clearTimeout(timers[id]);
  delete timers[id];
  toastState.items = toastState.items.filter((x) => x.id !== id);
}

/** Shows a toast and returns its id. */
export function pushToast(input: ToastInput): number {
  const toast: Toast = {
    id: nextId++,
    tone: input.tone ?? "info",
    title: input.title,
    message: input.message,
  };
  toastState.items = [...toastState.items, toast];
  if (toast.tone !== "error") {
    timers[toast.id] = setTimeout(
      () => dismissToast(toast.id),
      TOAST_TIMEOUT_MS,
    );
  }
  return toast.id;
}

export const toasts = {
  show: pushToast,
  info: (title: string, message?: string) =>
    pushToast({ tone: "info", title, message }),
  success: (title: string, message?: string) =>
    pushToast({ tone: "success", title, message }),
  warning: (title: string, message?: string) =>
    pushToast({ tone: "warning", title, message }),
  error: (title: string, message?: string) =>
    pushToast({ tone: "error", title, message }),
  dismiss: dismissToast,
  clear(): void {
    for (const id of Object.keys(timers)) dismissToast(Number(id));
    toastState.items = [];
  },
};
