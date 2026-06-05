import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CleanItem, CleanupGain, DiskFullPrediction } from "../types";
import { formatSize } from "../utils/format";

export default function PredictiveCleanupCard({ drive, onAddToCleanup }: { drive: string; onAddToCleanup: (items: CleanItem[]) => void }) {
  const [prediction, setPrediction] = useState<DiskFullPrediction | null>(null);
  const [items, setItems] = useState<CleanItem[]>([]);
  const [gain, setGain] = useState<CleanupGain | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const [nextPrediction, nextItems] = await Promise.all([
          invoke<DiskFullPrediction>("predict_disk_full", { drive }),
          invoke<CleanItem[]>("get_pre_cleanup_candidates", { drive }),
        ]);
        if (cancelled) return;
        setPrediction(nextPrediction);
        setItems(nextItems);
        setGain(await invoke<CleanupGain>("simulate_cleanup_gain", { drive, items: nextItems }));
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [drive]);

  if (error) return null;
  if (!prediction) {
    return <div className="glass-card rounded-2xl p-5 text-sm text-text-muted">Preparing predictive cleanup...</div>;
  }

  const urgencyTone =
    prediction.urgency === "critical"
      ? "text-danger"
      : prediction.urgency === "high"
        ? "text-warning"
        : "text-success";

  return (
    <div className="glass-card rounded-2xl border border-accent/15 p-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="text-xs uppercase tracking-wider text-text-muted">Predictive Cleanup</p>
          <h3 className="mt-2 text-lg font-bold text-text-primary">
            {prediction.days_to_full == null
              ? "No capacity deadline detected"
              : `${prediction.days_to_full.toFixed(0)} days until pressure`}
          </h3>
          <p className={`mt-1 text-sm font-semibold ${urgencyTone}`}>{prediction.urgency.toUpperCase()}</p>
        </div>
        <button
          className="btn-primary px-4 py-2"
          disabled={items.length === 0}
          onClick={() => onAddToCleanup(items)}
        >
          Review {items.length}
        </button>
      </div>

      {gain && (
        <div className="mt-4 rounded-xl border border-aurora-border/40 bg-aurora-elevated/40 p-4 text-sm text-text-secondary">
          Cleaning {formatSize(gain.freed_bytes)} may add{" "}
          <span className="font-semibold text-text-primary">
            {gain.estimated_extra_days == null ? "unknown" : `${gain.estimated_extra_days.toFixed(1)} days`}
          </span>{" "}
          and reduce urgency to <span className="font-semibold text-accent-light">{gain.new_urgency}</span>.
        </div>
      )}
    </div>
  );
}
