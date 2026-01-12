import { reactive } from "vue";

export type ToastKind = "ok" | "bad" | "warn";

export type Toast = {
  id: string;
  kind: ToastKind;
  title: string;
  message: string;
};

export const toasts = reactive<{ items: Toast[] }>({ items: [] });

export function pushToast(kind: ToastKind, title: string, message: string) {
  const id = crypto.randomUUID();
  toasts.items.push({ id, kind, title, message });
  setTimeout(() => {
    const i = toasts.items.findIndex(x => x.id === id);
    if (i >= 0) toasts.items.splice(i, 1);
  }, 3800);
}
