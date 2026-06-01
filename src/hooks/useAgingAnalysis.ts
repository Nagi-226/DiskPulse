import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AgingReport, AgingScanProgress } from "../types";

export function useAgingAnalysis() {
  const [report, setReport] = useState<AgingReport | null>(null);
  const [progress, setProgress] = useState<AgingScanProgress | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeRef = useRef(false);

  useEffect(() => {
    const unlisten = listen<AgingScanProgress>("aging-scan-progress", (event) => {
      if (activeRef.current) setProgress(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const analyze = useCallback(async (drive: string) => {
    activeRef.current = true;
    setLoading(true);
    setError(null);
    setReport(null);
    try {
      const result = await invoke<AgingReport>("analyze_file_aging", { drive });
      setReport(result);
      return result;
    } catch (e) {
      setError(String(e));
      return null;
    } finally {
      activeRef.current = false;
      setLoading(false);
    }
  }, []);

  const cancel = useCallback(async () => {
    await invoke("cancel_aging_scan");
  }, []);

  return { report, progress, loading, error, analyze, cancel };
}
