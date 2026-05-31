import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AutoCleanupStatus as AutoCleanupStatusType, CleanResult } from "../types";
import { formatSize } from "../utils/format";

function formatNextRun(epochMs: number | null) {
  if (epochMs == null) return "Not scheduled";
  return new Date(epochMs).toLocaleString();
}

export default function AutoCleanupStatus() {
  const [status, setStatus] = useState<AutoCleanupStatusType | null>(null);
  const [loading, setLoading] = useState(false);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadStatus = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<AutoCleanupStatusType>("get_auto_cleanup_status");
      setStatus(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadStatus();
    const unlistenComplete = listen<CleanResult>("auto-cleanup-complete", () => {
      loadStatus();
    });
    const unlistenScheduled = listen<AutoCleanupStatusType>("auto-cleanup-scheduled", (event) => {
      setStatus(event.payload);
    });

    return () => {
      unlistenComplete.then((fn) => fn());
      unlistenScheduled.then((fn) => fn());
    };
  }, [loadStatus]);

  async function handleRunNow() {
    setRunning(true);
    setError(null);
    try {
      await invoke<CleanResult>("run_auto_cleanup_now");
      await loadStatus();
    } catch (err) {
      setError(String(err));
    } finally {
      setRunning(false);
    }
  }

  const enabled = status?.enabled ?? false;
  const statusText = loading && !status ? "Loading" : enabled ? "Scheduled" : "Disabled";

  return (
    <section className="glass-card p-5 border border-aurora-border/50 overflow-hidden relative">
      <div className="absolute -right-12 -top-16 h-40 w-40 rounded-full bg-success/10 blur-3xl" />
      <div className="relative flex flex-wrap items-start justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
              Auto-Cleanup
            </h3>
            <span
              className={`rounded-full border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider ${
                enabled
                  ? "border-success/25 bg-risk-low-bg text-success"
                  : "border-aurora-border/50 bg-aurora-elevated/40 text-text-secondary"
              }`}
            >
              {statusText}
            </span>
          </div>
          <p className="mt-2 max-w-2xl text-sm text-text-secondary">
            {status?.message ?? "Schedule LOW-risk cleanup from Settings, or run it manually after reviewing your configuration."}
          </p>
          {error && <p className="mt-2 text-xs text-danger">{error}</p>}
        </div>

        <button className="btn-primary" onClick={handleRunNow} disabled={running || loading}>
          <span>{running ? "Running..." : "Run Now"}</span>
        </button>
      </div>

      <div className="relative mt-5 grid grid-cols-1 gap-3 md:grid-cols-4">
        <Metric label="Drive" value={`${status?.drive_letter ?? "C"}:`} />
        <Metric label="Frequency" value={status?.frequency ?? "weekly"} />
        <Metric label="Next run" value={formatNextRun(status?.next_run_epoch_ms ?? null)} />
        <Metric label="Last freed" value={formatSize(status?.last_freed_bytes ?? 0)} />
      </div>
    </section>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl border border-aurora-border/35 bg-aurora-elevated/45 px-4 py-3">
      <div className="text-[11px] uppercase tracking-wider text-text-muted">{label}</div>
      <div className="mt-1 truncate font-mono text-sm font-semibold text-text-primary" title={value}>
        {value}
      </div>
    </div>
  );
}
