import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { DuplicateGroup, DuplicateScanProgress } from "../types";

export function useDuplicateScan() {
  const [groups, setGroups] = useState<DuplicateGroup[]>([]);
  const [progress, setProgress] = useState<DuplicateScanProgress | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeRef = useRef(false);

  useEffect(() => {
    const unlisten = listen<DuplicateScanProgress>("duplicate-scan-progress", (event) => {
      if (activeRef.current) setProgress(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const scan = useCallback(async (drive: string, minSize: number) => {
    activeRef.current = true;
    setLoading(true);
    setError(null);
    setGroups([]);
    try {
      const result = await invoke<DuplicateGroup[]>("scan_duplicates", { drive, minSize });
      setGroups(result);
      return result;
    } catch (e) {
      setError(String(e));
      return [];
    } finally {
      activeRef.current = false;
      setLoading(false);
    }
  }, []);

  const cancel = useCallback(async () => {
    await invoke("cancel_duplicate_scan");
  }, []);

  return { groups, progress, loading, error, scan, cancel };
}
