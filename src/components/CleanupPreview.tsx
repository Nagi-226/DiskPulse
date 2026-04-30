import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useMemo, useState, useEffect } from "react";
import type { CleanItem, CleanPreview, CleanProgress, CleanResult, RiskItem } from "../types";
import { formatSize } from "../utils/format";

type Phase = "idle" | "preview" | "confirm" | "running" | "done";

function toCleanItems(items: RiskItem[], onlySafe: boolean): CleanItem[] {
  const filtered = onlySafe ? items.filter((i) => i.safe_to_delete) : items;
  return filtered.map((item) => ({
    name: item.name,
    path: item.path,
    size_bytes: item.size_bytes,
    risk_level: item.risk_level,
    safe_to_delete: item.safe_to_delete,
  }));
}

export default function CleanupPreview({ items }: { items: RiskItem[] }) {
  const [phase, setPhase] = useState<Phase>("idle");
  const [preview, setPreview] = useState<CleanPreview | null>(null);
  const [execution, setExecution] = useState<CleanResult | null>(null);
  const [progress, setProgress] = useState<CleanProgress | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const safeCleanItems = useMemo(() => toCleanItems(items, true), [items]);

  // Listen for cleanup progress events during execution
  useEffect(() => {
    const unlisten = listen<CleanProgress>("clean-progress", (event) => {
      setProgress(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function handlePreview() {
    setLoading(true);
    setError(null);
    setExecution(null);
    try {
      const result = await invoke<CleanPreview>("preview_cleanup", {
        items: safeCleanItems,
      });
      setPreview(result);
      setPhase("preview");
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  function handleConfirm() {
    setPhase("confirm");
  }

  function handleCancelConfirm() {
    setPhase("preview");
  }

  async function handleExecute() {
    setPhase("running");
    setError(null);
    setProgress(null);
    try {
      const result = await invoke<CleanResult>("clean_items", {
        items: safeCleanItems,
      });
      setExecution(result);
      setPhase("done");
    } catch (err) {
      setError(String(err));
      setPhase("preview");
    }
  }

  function handleReset() {
    setPhase("idle");
    setPreview(null);
    setExecution(null);
    setProgress(null);
    setError(null);
  }

  const canPreview = safeCleanItems.length > 0;
  const canExecute = Boolean(preview?.validation.allowed && preview.accepted.length > 0);
  const acceptedItems = preview?.accepted ?? [];
  const progressPct = progress ? (progress.total > 0 ? (progress.current / progress.total) * 100 : 0) : 0;

  return (
    <section className="glass-card p-6 rounded-3xl border border-aurora-border/50 space-y-4">
      {/* Header */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
            Safe Cleanup
          </h3>
          <p className="text-xs text-text-muted mt-1">
            Recycle Bin only — no permanent deletion
          </p>
        </div>
        <div className="flex gap-2">
          {phase === "idle" && (
            <button
              className="btn-primary"
              onClick={handlePreview}
              disabled={loading || !canPreview}
            >
              {loading ? "Validating..." : "Run Safety Check"}
            </button>
          )}
          {phase === "preview" && (
            <>
              <button className="btn-primary" onClick={handleConfirm} disabled={!canExecute}>
                Review & Clean
              </button>
              <button
                className="px-4 py-2 rounded-xl text-sm border bg-aurora-elevated/70 border-aurora-border/60 text-text-secondary hover:text-text-primary"
                onClick={handleReset}
              >
                Reset
              </button>
            </>
          )}
        </div>
      </div>

      {error && (
        <div className="p-3 rounded-xl bg-risk-high-bg/20 border border-red-500/20 text-sm text-danger">
          {error}
        </div>
      )}

      {/* Idle hint */}
      {phase === "idle" && (
        <p className="text-sm text-text-muted">
          Run a safety check to validate {safeCleanItems.length} safe-to-delete item
          {safeCleanItems.length !== 1 ? "s" : ""} before cleanup.
        </p>
      )}

      {/* Preview summary */}
      {preview && (phase === "preview" || phase === "confirm") && (
        <>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-sm">
            <Stat
              label="Accepted"
              value={preview.validation.valid_items.toString()}
              detail={formatSize(preview.validation.total_bytes)}
              tone="success"
            />
            <Stat
              label="Blocked"
              value={preview.validation.blocked_items.toString()}
              detail={preview.validation.blocked_reason ?? "None"}
              tone="warning"
            />
            <Stat
              label="Status"
              value={preview.validation.allowed ? "Ready" : "Blocked"}
              detail="Recycle Bin only"
              tone={preview.validation.allowed ? "success" : "danger"}
            />
          </div>

          {/* Accepted & Blocked lists */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            <ItemGroup
              title={`Accepted (${acceptedItems.length})`}
              items={acceptedItems}
              tone="success"
            />
            <ItemGroup
              title={`Blocked (${preview.blocked.length})`}
              items={preview.blocked}
              tone="danger"
            />
          </div>

          {preview.unsafe_items.length > 0 && (
            <div className="rounded-2xl border border-warning/20 bg-risk-medium-bg/10 p-4">
              <div className="text-sm font-medium text-warning mb-2">
                Pre-delete safety issues ({preview.unsafe_items.length})
              </div>
              <div className="space-y-2">
                {preview.unsafe_items.map((r) => (
                  <div key={r.path} className="text-xs text-text-secondary flex items-start gap-2">
                    <span className="text-warning mt-0.5">!</span>
                    <span>
                      <span className="text-text-primary">{r.name}</span> — {r.reason ?? "Unknown issue"}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </>
      )}

      {/* ── Confirmation Modal ── */}
      {phase === "confirm" && preview && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
          <div className="glass-card max-w-2xl w-full mx-4 p-6 rounded-3xl border border-aurora-border/60 max-h-[80vh] flex flex-col">
            <div className="flex items-start justify-between mb-4">
              <div>
                <h2 className="text-lg font-semibold text-text-primary">Confirm Cleanup</h2>
                <p className="text-sm text-text-secondary mt-1">
                  The following {acceptedItems.length} item
                  {acceptedItems.length !== 1 ? "s" : ""} will be moved to Recycle Bin
                </p>
              </div>
              <span className="px-2.5 py-1 rounded-full text-xs font-medium bg-success/15 text-success border border-success/20">
                {formatSize(preview.validation.total_bytes)}
              </span>
            </div>

            <div className="flex-1 overflow-y-auto space-y-2 mb-4">
              {acceptedItems.map((item) => (
                <div
                  key={item.path}
                  className="flex items-center gap-3 rounded-xl bg-aurora-elevated/70 p-3 text-xs"
                >
                  <span className="w-1.5 h-1.5 rounded-full bg-success flex-shrink-0" />
                  <span className="text-text-primary font-medium truncate">{item.name}</span>
                  <span className="text-text-muted font-mono ml-auto flex-shrink-0">
                    {formatSize(item.size_bytes)}
                  </span>
                </div>
              ))}
            </div>

            <div className="flex items-center gap-3 pt-4 border-t border-aurora-border/40">
              <button className="btn-primary flex-1" onClick={handleExecute}>
                Move to Recycle Bin
              </button>
              <button
                className="px-4 py-2.5 rounded-xl text-sm border bg-aurora-elevated/70 border-aurora-border/60 text-text-secondary hover:text-text-primary"
                onClick={handleCancelConfirm}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      )}

      {/* ── Progress Modal ── */}
      {phase === "running" && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
          <div className="glass-card max-w-md w-full mx-4 p-6 rounded-3xl border border-aurora-border/60 text-center">
            <div className="w-12 h-12 mx-auto mb-4 rounded-full border-2 border-accent/30 flex items-center justify-center">
              <svg className="animate-spin" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" style={{ color: "var(--color-accent)" }}>
                <circle cx="12" cy="12" r="10" strokeDasharray="32" strokeDashoffset="32" />
              </svg>
            </div>
            <h3 className="text-lg font-semibold text-text-primary mb-2">Cleaning...</h3>
            <p className="text-sm text-text-secondary mb-4">
              Moving files to Recycle Bin — this can be undone manually
            </p>

            {/* Progress bar */}
            <div className="h-2 rounded-full bg-aurora-border/60 overflow-hidden mb-3">
              <div
                className="h-full rounded-full transition-all duration-300 ease-out"
                style={{
                  width: `${Math.max(progressPct, 2)}%`,
                  background: "linear-gradient(90deg, var(--color-accent), var(--color-cyan))",
                }}
              />
            </div>
            <div className="flex items-center justify-between text-xs text-text-muted">
              <span>
                {progress ? `${progress.current} / ${progress.total}` : "Starting..."}
              </span>
              <span>{Math.round(progressPct)}%</span>
            </div>
            {progress?.current_item && (
              <p className="mt-2 text-xs text-text-muted truncate">
                Current: {progress.current_item}
              </p>
            )}
          </div>
        </div>
      )}

      {/* ── Completion Modal ── */}
      {phase === "done" && execution && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
          <div className="glass-card max-w-lg w-full mx-4 p-6 rounded-3xl border border-aurora-border/60 max-h-[80vh] flex flex-col">
            <div className="text-center mb-4">
              <div className="w-12 h-12 mx-auto mb-3 rounded-full bg-success/15 flex items-center justify-center">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" className="text-success">
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              </div>
              <h2 className="text-lg font-semibold text-text-primary">Cleanup Complete</h2>
              <p className="text-sm text-text-secondary mt-1">
                {formatSize(execution.freed_bytes)} freed — all items in Recycle Bin
              </p>
            </div>

            {/* Stats grid */}
            <div className="grid grid-cols-3 gap-3 mb-4">
              <StatSmall label="Cleaned" value={execution.succeeded.toString()} tone="success" />
              <StatSmall label="Skipped" value={execution.skipped.toString()} tone="warning" />
              <StatSmall label="Failed" value={execution.failed.toString()} tone="danger" />
            </div>

            {/* Failed items detail */}
            {execution.items.filter((r) => r.status === "Failed" || r.status === "Skipped").length >
              0 && (
              <div className="flex-1 overflow-y-auto mb-4 space-y-2">
                <div className="text-xs font-medium text-text-muted uppercase tracking-wider">
                  Skipped / Failed Items
                </div>
                {execution.items
                  .filter((r) => r.status === "Failed" || r.status === "Skipped")
                  .map((r) => (
                    <div
                      key={r.path}
                      className="flex items-start gap-2 rounded-xl bg-aurora-elevated/70 p-2.5 text-xs"
                    >
                      <span className="text-warning mt-0.5 flex-shrink-0">!</span>
                      <div className="min-w-0">
                        <div className="text-text-primary font-medium truncate">{r.name}</div>
                        <div className="text-text-muted mt-0.5">{r.reason ?? "Unknown"}</div>
                      </div>
                    </div>
                  ))}
              </div>
            )}

            <button className="btn-primary w-full" onClick={handleReset}>
              Done
            </button>
          </div>
        </div>
      )}

      {/* Idle hint when no preview yet */}
      {!preview && phase === "idle" && !error && (
        <div className="text-sm text-text-muted">
          {canPreview
            ? `${safeCleanItems.length} safe-to-delete item${safeCleanItems.length !== 1 ? "s" : ""} ready for validation.`
            : "No safe-to-delete items available in the current report."}
        </div>
      )}
    </section>
  );
}

function Stat({
  label,
  value,
  detail,
  tone,
}: {
  label: string;
  value: string;
  detail: string;
  tone: "success" | "warning" | "danger";
}) {
  const toneClass =
    tone === "success" ? "text-success" : tone === "warning" ? "text-warning" : "text-danger";
  return (
    <div className="rounded-2xl bg-aurora-elevated/60 p-4 border border-aurora-border/40">
      <div className="text-xs text-text-muted">{label}</div>
      <div className={`text-lg font-semibold mt-2 ${toneClass}`}>{value}</div>
      <div className="text-xs text-text-muted mt-1">{detail}</div>
    </div>
  );
}

function StatSmall({
  label,
  value,
  tone,
}: {
  label: string;
  value: string;
  tone: "success" | "warning" | "danger";
}) {
  const toneClass =
    tone === "success" ? "text-success" : tone === "warning" ? "text-warning" : "text-danger";
  return (
    <div className="rounded-2xl bg-aurora-elevated/60 p-3 border border-aurora-border/40 text-center">
      <div className={`text-xl font-bold ${toneClass}`}>{value}</div>
      <div className="text-xs text-text-muted mt-1">{label}</div>
    </div>
  );
}

function ItemGroup({
  title,
  items,
  tone,
}: {
  title: string;
  items: CleanItem[];
  tone: "success" | "danger";
}) {
  const toneClass = tone === "success" ? "text-success" : "text-danger";
  return (
    <div className="rounded-2xl border border-aurora-border/40 bg-aurora-elevated/40 p-4 space-y-3">
      <div className={`text-sm font-semibold ${toneClass}`}>{title}</div>
      {items.length === 0 ? (
        <div className="text-xs text-text-muted">None</div>
      ) : (
        items.map((item) => (
          <div
            key={item.path}
            className="rounded-xl bg-aurora-elevated/70 p-3 text-xs space-y-1"
          >
            <div className="flex items-center justify-between gap-3">
              <span className="text-text-primary font-medium truncate">{item.name}</span>
              <span className="text-text-muted font-mono">{formatSize(item.size_bytes)}</span>
            </div>
            <div className="text-text-muted break-all">{item.path}</div>
          </div>
        ))
      )}
    </div>
  );
}
