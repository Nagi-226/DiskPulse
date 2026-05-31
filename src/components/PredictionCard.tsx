import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatSize } from "../utils/format";
import type { Prediction } from "../types";

const STATUS_STYLE: Record<Prediction["status"], { label: string; className: string }> = {
  insufficient_data: {
    label: "Learning",
    className: "border-aurora-border/50 bg-aurora-elevated/40 text-text-secondary",
  },
  stable: {
    label: "Stable",
    className: "border-success/25 bg-risk-low-bg text-success",
  },
  growing: {
    label: "Growing",
    className: "border-accent/25 bg-accent/10 text-accent-light",
  },
  shrinking: {
    label: "Improving",
    className: "border-success/25 bg-risk-low-bg text-success",
  },
  warning: {
    label: "Watch",
    className: "border-warning/25 bg-risk-medium-bg text-warning",
  },
  critical: {
    label: "Critical",
    className: "border-danger/30 bg-risk-high-bg text-danger",
  },
};

function formatDays(days: number | null) {
  if (days == null) return "Not projected";
  if (days < 1) return "Now";
  if (days < 60) return `${Math.round(days)} days`;
  return `${Math.round(days / 30)} months`;
}

function formatGrowth(bytesPerDay: number) {
  const prefix = bytesPerDay > 0 ? "+" : bytesPerDay < 0 ? "-" : "";
  return `${prefix}${formatSize(Math.abs(bytesPerDay))}/day`;
}

export default function PredictionCard({ drive }: { drive: string }) {
  const [prediction, setPrediction] = useState<Prediction | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    invoke<Prediction>("predict_disk_usage", { drive, days: 30 })
      .then((result) => {
        if (!cancelled) setPrediction(result);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [drive]);

  if (loading && !prediction) {
    return (
      <div className="glass-card p-5 border border-aurora-border/40">
        <div className="h-4 w-40 rounded bg-aurora-elevated/80 progress-shimmer mb-4" />
        <div className="grid grid-cols-3 gap-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <div key={i} className="h-16 rounded-xl bg-aurora-elevated/50 progress-shimmer" />
          ))}
        </div>
      </div>
    );
  }

  if (error || !prediction) {
    return (
      <div className="glass-card p-5 border border-danger/20 bg-risk-high-bg/10">
        <div className="text-sm font-semibold text-danger">Prediction unavailable</div>
        <p className="text-xs text-text-muted mt-1">{error ?? "No prediction data returned."}</p>
      </div>
    );
  }

  const style = STATUS_STYLE[prediction.status];

  return (
    <div className="glass-card p-5 border border-aurora-border/50 overflow-hidden relative">
      <div className="absolute -right-10 -top-10 w-36 h-36 rounded-full bg-accent/10 blur-3xl" />
      <div className="relative flex flex-wrap items-start justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
              Capacity Forecast
            </h3>
            <span className={`px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase tracking-wider border ${style.className}`}>
              {style.label}
            </span>
          </div>
          <p className="text-sm text-text-secondary mt-2 max-w-2xl">{prediction.message}</p>
        </div>
        <div className="text-right">
          <div className="text-2xl font-bold font-mono text-text-primary">
            {formatDays(prediction.days_to_95_percent)}
          </div>
          <div className="text-[11px] uppercase tracking-wider text-text-muted">
            to 95% usage
          </div>
        </div>
      </div>

      <div className="relative grid grid-cols-1 md:grid-cols-4 gap-3 mt-5">
        <Metric label="Current usage" value={`${prediction.current_usage_percent.toFixed(1)}%`} />
        <Metric label="Growth" value={formatGrowth(prediction.growth_bytes_per_day)} />
        <Metric label="Confidence" value={`${Math.round(prediction.confidence_score * 100)}%`} />
        <Metric
          label="Samples"
          value={`${prediction.sample_count}`}
          detail={`${prediction.window_days}d window`}
        />
      </div>
    </div>
  );
}

function Metric({ label, value, detail }: { label: string; value: string; detail?: string }) {
  return (
    <div className="rounded-xl bg-aurora-elevated/45 border border-aurora-border/35 px-4 py-3">
      <div className="text-[11px] uppercase tracking-wider text-text-muted">{label}</div>
      <div className="text-sm font-semibold font-mono text-text-primary mt-1">{value}</div>
      {detail && <div className="text-[11px] text-text-muted mt-0.5">{detail}</div>}
    </div>
  );
}
