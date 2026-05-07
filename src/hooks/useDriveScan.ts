import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { DirInfo, DriveInfo, DriveMeta, RiskReport, ScanProgress } from "../types";

type DriveDataSource = "empty" | "meta" | "cached" | "fresh";

function mergeDirs(existing: DirInfo[], incoming: DirInfo[]) {
  const byPath = new Map(existing.map((dir) => [dir.path, dir]));
  for (const dir of incoming) {
    byPath.set(dir.path, dir);
  }
  return Array.from(byPath.values()).sort((a, b) => b.size_bytes - a.size_bytes);
}

function toDriveInfo(meta: DriveMeta): DriveInfo {
  return {
    drive_letter: meta.drive_letter,
    total_bytes: meta.total_bytes,
    used_bytes: meta.used_bytes,
    free_bytes: meta.free_bytes,
    top_dirs: meta.cached_top_dirs ?? [],
  };
}

export function useDriveScan(initialDrive = "C") {
  const [driveInfo, setDriveInfo] = useState<DriveInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selectedDrive, setSelectedDrive] = useState(initialDrive);
  const [riskReport, setRiskReport] = useState<RiskReport | null>(null);
  const [dataSource, setDataSource] = useState<DriveDataSource>("empty");
  const [cacheAgeMs, setCacheAgeMs] = useState<number | null>(null);
  const requestIdRef = useRef(0);
  const selectedDriveRef = useRef(initialDrive);

  useEffect(() => {
    selectedDriveRef.current = selectedDrive;
  }, [selectedDrive]);

  const classifyRisks = useCallback(async (info: DriveInfo) => {
    try {
      const report = await invoke<RiskReport>("classify_risks", { scan: info });
      setRiskReport(report);
    } catch {
      setRiskReport(null);
    }
  }, []);

  useEffect(() => {
    const unlistenScan = listen<ScanProgress>("scan-progress", (event) => {
      const payload = event.payload;
      if (payload.drive_letter !== selectedDriveRef.current.toUpperCase()) {
        return;
      }

      setProgress(payload);
      if (payload.partial_results?.length) {
        setDriveInfo((current) => {
          if (!current || current.drive_letter !== payload.drive_letter) {
            return current;
          }
          return {
            ...current,
            top_dirs: mergeDirs(current.top_dirs, payload.partial_results ?? []),
          };
        });
      }
    });
    const unlistenCacheRefresh = listen<DriveInfo>("drive-cache-refreshed", (event) => {
      const payload = event.payload;
      if (payload.drive_letter !== selectedDriveRef.current.toUpperCase()) {
        return;
      }

      setDriveInfo(payload);
      setDataSource("fresh");
      setCacheAgeMs(0);
      void classifyRisks(payload);
    });

    return () => {
      unlistenScan.then((fn) => fn());
      unlistenCacheRefresh.then((fn) => fn());
    };
  }, [classifyRisks]);

  const scanDrive = useCallback(
    async (drive: string) => {
      const requestId = requestIdRef.current + 1;
      requestIdRef.current = requestId;
      const normalizedDrive = drive.toUpperCase();

      setSelectedDrive(normalizedDrive);
      setLoading(true);
      setError(null);
      setProgress(null);
      setRiskReport(null);
      setDataSource("empty");
      setCacheAgeMs(null);

      try {
        const meta = await invoke<DriveMeta>("scan_drive_meta", { drive: normalizedDrive });
        if (requestIdRef.current !== requestId) {
          return null;
        }

        const cachedInfo = toDriveInfo(meta);
        setDriveInfo(cachedInfo);
        setCacheAgeMs(meta.cache_age_ms);
        setDataSource(cachedInfo.top_dirs.length > 0 ? "cached" : "meta");

        if (cachedInfo.top_dirs.length > 0) {
          void classifyRisks(cachedInfo);
        }

        // Let the ring chart and cached treemap paint before the expensive walk starts.
        await new Promise((resolve) => window.setTimeout(resolve, 300));
        if (requestIdRef.current !== requestId) {
          return null;
        }

        const dirs = await invoke<DirInfo[]>("scan_drive_dirs", { drive: normalizedDrive });
        if (requestIdRef.current !== requestId) {
          return null;
        }

        const freshInfo: DriveInfo = {
          ...cachedInfo,
          top_dirs: dirs,
        };
        setDriveInfo(freshInfo);
        setDataSource("fresh");
        setCacheAgeMs(0);
        await classifyRisks(freshInfo);
        return freshInfo;
      } catch (e) {
        if (requestIdRef.current === requestId) {
          setError(String(e));
        }
        return null;
      } finally {
        if (requestIdRef.current === requestId) {
          setLoading(false);
          setProgress(null);
        }
      }
    },
    [classifyRisks],
  );

  const cancelScan = useCallback(async () => {
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    try {
      await invoke("cancel_scan");
    } catch (e) {
      setError(String(e));
    } finally {
      if (requestIdRef.current === requestId) {
        setLoading(false);
        setProgress(null);
      }
    }
  }, []);

  return {
    driveInfo,
    loading,
    progress,
    error,
    setError,
    selectedDrive,
    setSelectedDrive,
    riskReport,
    dataSource,
    cacheAgeMs,
    scanDrive,
    cancelScan,
  };
}
