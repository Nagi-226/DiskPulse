import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { FileEntry, LargeFileProgress } from "../types";

interface LargeFileScanOptions {
  drive: string;
  minSize: number;
  limit: number;
}

export function useLargeFileFinder() {
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [progress, setProgress] = useState<LargeFileProgress | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const activeDriveRef = useRef<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const stopListening = useCallback(() => {
    unlistenRef.current?.();
    unlistenRef.current = null;
  }, []);

  useEffect(() => {
    return () => {
      stopListening();
      void invoke("cancel_large_file_scan");
    };
  }, [stopListening]);

  const scan = useCallback(
    async ({ drive, minSize, limit }: LargeFileScanOptions) => {
      stopListening();
      setLoading(true);
      setError(null);
      setFiles([]);
      setProgress(null);
      activeDriveRef.current = drive;

      unlistenRef.current = await listen<LargeFileProgress>("large-file-progress", (event) => {
        if (!activeDriveRef.current || event.payload.drive_letter === activeDriveRef.current) {
          setProgress(event.payload);
        }
      });

      try {
        const result = await invoke<FileEntry[]>("find_large_files", {
          drive,
          minSize,
          limit,
        });
        setFiles(result);
        return result;
      } catch (err) {
        const message = String(err);
        if (!message.toLowerCase().includes("cancelled")) {
          setError(message);
        }
        return [];
      } finally {
        setLoading(false);
        stopListening();
      }
    },
    [stopListening],
  );

  const cancel = useCallback(async () => {
    await invoke("cancel_large_file_scan");
    setLoading(false);
  }, []);

  return {
    files,
    progress,
    loading,
    error,
    setError,
    scan,
    cancel,
  };
}
