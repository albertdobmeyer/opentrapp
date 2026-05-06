import { createContext, useContext } from "react";

export interface Toast {
  id: string;
  type: "success" | "error" | "warning" | "info";
  title: string;
  message?: string;
  details?: string;
  retryFn?: () => void;
  duration?: number; // ms, 0 = sticky
}

export type AddToastFn = (toast: Omit<Toast, "id">) => string;

export interface ToastContextValue {
  toasts: Toast[];
  addToast: AddToastFn;
  removeToast: (id: string) => void;
}

export const ToastContext = createContext<ToastContextValue>({
  toasts: [],
  addToast: () => "",
  removeToast: () => undefined,
});

export function useToast() {
  return useContext(ToastContext);
}
