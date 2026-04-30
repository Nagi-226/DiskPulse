import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface FsEventItem {
  kind: "Added" | "Removed" | "Modified";
  path: string;
  is_directory: boolean;
  size_bytes: number;
}

export interface FsChangeBatch {
  watched_dir: string;
  events: FsEventItem[];
  event_count: number;
  timestamp_ms: number;
}

export function useFsEvents() {
  const [isWatching, setIsWatching] = useState(false);
  const [lastBatch, setLastBatch] = useState<FsChangeBatch | null>(null);
  const [eventCount, setEventCount] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    return () => {
      if (isWatching) {
        invoke("stop_fs_watcher").catch(() => {});
      }
    };
  }, []);

  const startWatching = useCallback(async () => {
    setError(null);
    try {
      // Subscribe to events first
      const unlisten = await listen<FsChangeBatch>("fs-event-batch", (event) => {
        setLastBatch(event.payload);
        setEventCount((prev) => prev + (event.payload.event_count || 1));
      });
      unlistenRef.current = unlisten;

      // Then start the watcher
      const msg = await invoke<string>("start_fs_watcher");
      setIsWatching(true);
      return msg;
    } catch (e) {
      setError(String(e));
      // Clean up listener on failure
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
      throw e;
    }
  }, []);

  const stopWatching = useCallback(async () => {
    try {
      await invoke("stop_fs_watcher");
    } catch (e) {
      setError(String(e));
    }
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
    setIsWatching(false);
    setLastBatch(null);
  }, []);

  return {
    isWatching,
    lastBatch,
    eventCount,
    error,
    startWatching,
    stopWatching,
  };
}
