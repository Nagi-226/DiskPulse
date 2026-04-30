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
  const [execution, setExecution] = useState<CleanResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [executing, setExecuting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const cleanItems = useMemo(() => toCleanItems(items), [items]);

  async function handlePreview() {
    setLoading(true);
    setError(null);
    setExecution(null);
    try {
      const result = await invoke<CleanPreview>("preview_cleanup", { items: cleanItems });
      setPreview(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  async function handleExecute() {
    setExecuting(true);
    setError(null);
    try {
      const result = await invoke<CleanResult>("clean_items", { items: cleanItems });
      setExecution(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setExecuting(false);
    }
  }

  const acceptedCount = preview?.validation.valid_items ?? 0;
  const canExecute = Boolean(preview?.validation.allowed && acceptedCount > 0);

  return (
    <section className="glass-card p-6 rounded-3xl border border-aurora-border/50 space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider">Safe Cleanup Preview</h3>
          <p className="text-xs text-text-muted mt-1">Whitelist validation only, with recycle-bin-only execution.</p>
        </div>
        <div className="flex gap-2">
          <button className="btn-primary" onClick={handlePreview} disabled={loading || cleanItems.length === 0}>
            {loading ? "Validating..." : "Run Safety Check"}
          </button>
          <button className="btn-primary" onClick={handleExecute} disabled={!canExecute || executing}>
            {executing ? "Cleaning..." : "Execute Cleanup"}
          </button>
        </div>
      </div>

      {error && <div className="text-sm text-danger">{error}</div>}

      {preview ? (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-sm">
          <Stat label="Accepted" value={preview.validation.valid_items.toString()} detail={formatSize(preview.validation.total_bytes)} tone="success" />
          <Stat label="Blocked" value={preview.validation.blocked_items.toString()} detail={preview.validation.blocked_reason ?? "None"} tone="warning" />
          <Stat label="Allowed" value={preview.validation.allowed ? "Yes" : "No"} detail="Execution remains guarded by validation" tone="danger" />
        </div>
      ) : (
        <div className="text-sm text-text-muted">Run safety check to validate selected cleanup candidates.</div>
      )}

      {execution && (
        <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-sm">
          <Stat label="Deleted" value={execution.deleted_items.length.toString()} detail={formatSize(execution.deleted_bytes)} tone="success" />
          <Stat label="Failed" value={execution.failed_items.length.toString()} detail={execution.failed_reason ?? "None"} tone="warning" />
          <Stat label="Mode" value="Recycle Bin" detail="No permanent delete path is exposed" tone="danger" />
        </div>
      )}

      {preview && (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          <ItemGroup title="Accepted items" items={preview.accepted} tone="success" />
          <ItemGroup title="Blocked items" items={preview.blocked} tone="danger" />
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
