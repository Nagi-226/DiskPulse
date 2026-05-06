import { invoke } from "@tauri-apps/api/core";
import { useMemo, useState } from "react";
import type { CleanItem, CleanPreview, CleanResult, RiskItem } from "../types";
import { formatSize } from "../utils/format";

function toCleanItems(items: RiskItem[]): CleanItem[] {
  return items.map((item) => ({
    name: item.name,
    path: item.path,
    size_bytes: item.size_bytes,
    risk_level: item.risk_level,
    safe_to_delete: item.safe_to_delete,
  }));
}

export default function CleanupPreview({ items }: { items: RiskItem[] }) {
  const [preview, setPreview] = useState<CleanPreview | null>(null);
  const [result, setResult] = useState<CleanResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const cleanItems = useMemo(() => toCleanItems(items), [items]);

  async function handlePreview() {
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const previewResult = await invoke<CleanPreview>("preview_cleanup", { items: cleanItems });
      setPreview(previewResult);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function handleExecute() {
    if (!preview?.validation.allowed || preview.accepted.length === 0) {
      setError("Cleanup execution is blocked until the safety check passes.");
      return;
    }

    const confirmMessage = `Move ${preview.accepted.length} item(s) to Recycle Bin? You can restore them later.`;
    if (!window.confirm(confirmMessage)) {
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const cleanResult = await invoke<CleanResult>("clean_items", { items: preview.accepted });
      setResult(cleanResult);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function handleUndo() {
    if (!result || result.succeeded === 0) {
      setError("Nothing to restore.");
      return;
    }

    const paths = result.items
      .filter((r: { original_path: string | null }) => r.original_path)
      .map((r: { original_path: string | null }) => r.original_path!);

    if (paths.length === 0) {
      setError("No restorable paths found.");
      return;
    }

    const confirmMessage = `Restore ${paths.length} item(s) from Recycle Bin?`;
    if (!window.confirm(confirmMessage)) {
      return;
    }

    setLoading(true);
    setError(null);
    try {
      const restoreResult = await invoke<{ restored: number; failed: number }>("undo_cleanup", {
        originalPaths: paths,
      });
      setResult(null);
      setPreview(null);
      setError(
        restoreResult.restored > 0
          ? `Restored ${restoreResult.restored} item(s)` + (restoreResult.failed > 0 ? `, ${restoreResult.failed} failed` : "")
          : `Restore failed: ${restoreResult.failed} item(s)`
      );
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="glass-card p-6 rounded-3xl border border-aurora-border/50 space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider">Safe Cleanup Preview</h3>
          <p className="text-xs text-text-muted mt-1">Whitelist validation only, no deletion is performed here.</p>
        </div>
        <div className="flex gap-2 flex-wrap">
          <button className="btn-primary" onClick={handlePreview} disabled={loading || cleanItems.length === 0}>
            {loading ? "Validating..." : "Run Safety Check"}
          </button>
          <button
            className="btn-primary"
            onClick={handleExecute}
            disabled={loading || !preview?.validation.allowed || preview.accepted.length === 0}
          >
            Execute Safe Cleanup
          </button>
        </div>
      </div>

      {error && <div className="text-sm text-danger">{error}</div>}

      {preview ? (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-sm">
          <Stat label="Accepted" value={preview.validation.valid_items.toString()} detail={formatSize(preview.validation.total_bytes)} tone="success" />
          <Stat label="Blocked" value={preview.validation.blocked_items.toString()} detail={preview.validation.blocked_reason ?? "None"} tone="warning" />
          <Stat label="Allowed" value={preview.validation.allowed ? "Yes" : "No"} detail="Cleanup execution remains disabled" tone="danger" />
        </div>
      ) : (
        <div className="text-sm text-text-muted">Run safety check to validate selected cleanup candidates.</div>
      )}

      {preview && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <ItemGroup title="Accepted items" items={preview.accepted} tone="success" />
          <ItemGroup title="Blocked items" items={preview.blocked} tone="danger" />
        </div>
      )}

      {result && (
        <div className="rounded-2xl border border-success/20 bg-risk-low-bg/15 p-4 text-sm space-y-2">
          <div className="font-semibold text-success">Cleanup result</div>
          <div className="text-text-primary">
            <span className="text-success">{result.succeeded} succeeded</span>
            {result.skipped > 0 && <span className="text-warning">, {result.skipped} skipped</span>}
            {result.failed > 0 && <span className="text-danger">, {result.failed} failed</span>}
          </div>
          <div className="text-text-secondary">Freed: {formatSize(result.freed_bytes)}</div>
          <div className="flex items-center gap-2 pt-2 border-t border-success/10">
            <button
              className="px-3 py-1.5 rounded-lg text-xs font-medium bg-warning/10 border border-warning/20 text-warning hover:bg-warning/20 transition-colors"
              onClick={handleUndo}
              disabled={loading}
            >
              {loading ? "Restoring..." : "Undo (Restore from Recycle Bin)"}
            </button>
          </div>
        </div>
      )}
    </section>
  );
}

function Stat({ label, value, detail, tone }: { label: string; value: string; detail: string; tone: "success" | "warning" | "danger" }) {
  const toneClass = tone === "success" ? "text-success" : tone === "warning" ? "text-warning" : "text-danger";
  return (
    <div className="rounded-2xl bg-aurora-elevated/60 p-4 border border-aurora-border/40">
      <div className="text-xs text-text-muted">{label}</div>
      <div className={`text-lg font-semibold mt-2 ${toneClass}`}>{value}</div>
      <div className="text-xs text-text-muted mt-1">{detail}</div>
    </div>
  );
}

function ItemGroup({ title, items, tone }: { title: string; items: CleanItem[]; tone: "success" | "danger" }) {
  const toneClass = tone === "success" ? "text-success" : "text-danger";
  return (
    <div className="rounded-2xl border border-aurora-border/40 bg-aurora-elevated/40 p-4 space-y-3">
      <div className={`text-sm font-semibold ${toneClass}`}>{title}</div>
      {items.length === 0 ? (
        <div className="text-xs text-text-muted">None</div>
      ) : (
        items.map((item) => (
          <div key={item.path} className="rounded-xl bg-aurora-elevated/70 p-3 text-xs space-y-1">
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
