import { createContext, useCallback, useContext, useState } from "react";

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

interface ToastContextValue {
  toasts: Toast[];
  addToast: AddToastFn;
  removeToast: (id: string) => void;
}

export const ToastContext = createContext<ToastContextValue>({
  toasts: [],
  addToast: () => "",
  removeToast: () => {},
});

export function useToast() {
  return useContext(ToastContext);
}

let toastCounter = 0;

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const addToast = useCallback<AddToastFn>(
    (toast) => {
      const id = String(++toastCounter);
      const newToast: Toast = { ...toast, id };
      setToasts((prev) => [...prev, newToast]);

      // Auto-dismiss (default 5s, 0 = sticky)
      const duration = toast.duration ?? 5000;
      if (duration > 0) {
        setTimeout(() => { removeToast(id); }, duration);
      }

      return id;
    },
    [removeToast],
  );

  return (
    <ToastContext.Provider value={{ toasts, addToast, removeToast }}>
      {children}
      <ToastList toasts={toasts} onDismiss={removeToast} />
    </ToastContext.Provider>
  );
}

const BORDER_COLORS: Record<Toast["type"], string> = {
  success: "border-success-500",
  error: "border-danger-500",
  warning: "border-warning-500",
  info: "border-info-500",
};

const ICON_COLORS: Record<Toast["type"], string> = {
  success: "text-success-400",
  error: "text-danger-400",
  warning: "text-warning-400",
  info: "text-info-400",
};

function ToastList({
  toasts,
  onDismiss,
}: {
  toasts: Toast[];
  onDismiss: (id: string) => void;
}) {
  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-md">
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} onDismiss={onDismiss} />
      ))}
    </div>
  );
}

function ToastItem({
  toast,
  onDismiss,
}: {
  toast: Toast;
  onDismiss: (id: string) => void;
}) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div
      className={`bg-gray-900 border-l-4 ${BORDER_COLORS[toast.type]} rounded-md shadow-lg p-3 animate-slide-in`}
      role="alert"
    >
      <div className="flex items-start justify-between gap-2">
        <div className="flex-1 min-w-0">
          <p className={`text-sm font-medium ${ICON_COLORS[toast.type]}`}>
            {toast.title}
          </p>
          {toast.message && (
            <p className="text-xs text-gray-400 mt-0.5">{toast.message}</p>
          )}
        </div>
        <button
          onClick={() => { onDismiss(toast.id); }}
          className="text-gray-600 hover:text-gray-400 text-xs shrink-0"
          aria-label="Dismiss"
        >
          &times;
        </button>
      </div>
      {toast.details && (
        <div className="mt-1">
          <button
            onClick={() => { setExpanded(!expanded); }}
            className="text-xs text-gray-500 hover:text-gray-300"
          >
            {expanded ? "Hide details" : "Show details"}
          </button>
          {expanded && (
            <pre className="text-xs text-gray-500 mt-1 whitespace-pre-wrap break-all max-h-32 overflow-y-auto">
              {toast.details}
            </pre>
          )}
        </div>
      )}
      {toast.retryFn && (
        <button
          onClick={() => {
            onDismiss(toast.id);
            toast.retryFn!();
          }}
          className="mt-2 text-xs px-2 py-1 rounded bg-gray-800 hover:bg-gray-700 text-gray-300"
        >
          Retry
        </button>
      )}
    </div>
  );
}
