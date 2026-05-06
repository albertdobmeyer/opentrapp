import { useEffect, useState } from "react";

import { useToast } from "@/hooks/useToast";
import { classifyError } from "@/lib/errors";
import { listComponents } from "@/lib/tauri";

import type { DiscoveredComponent } from "@/lib/types";

export function useManifests() {
  const [components, setComponents] = useState<DiscoveredComponent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const { addToast } = useToast();

  const refresh = async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await listComponents();
      setComponents(result);
    } catch (error_) {
      const msg = error_ instanceof Error ? error_.message : String(error_);
      setError(msg);
      const classified = classifyError(error_);
      addToast({
        type: "error",
        title: "Discovery failed",
        message: classified.message,
        retryFn: () => { void refresh(); },
        duration: 0,
      });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return { components, loading, error, refresh };
}
