import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { formatSize } from "../utils/format";
import type { AnomalyEvent, AnomalyReport, AnomalySeverity, AnomalyType } from "../types";

const TYPE_LABEL: Record<AnomalyType, string> = {
  rate_anomaly: "Rate anomaly",
  burst_anomaly: "Burst growth",
  hotspot_anomaly: "Hotspot",
  pattern_deviation: "Pattern shift",
};

const SEVERITY_STYLE: Record<AnomalySeverity, { label: string; className: string }> = {
  warning: {
    label: "Watch",
    className: "border-warning/25 bg-risk-medium-bg text-warning",
  },
  critical: {
    label: "Critical",
    className: "border-danger/30 bg-risk-high-bg text-danger",
  },
};

function latestEvent(events: AnomalyEvent[]) {
  return [...events].sort((a, b) => b.created_at.localeCompare(a.created_at))[0] ?? null;
}

function formatMetric(event: AnomalyEvent | null) {
  if (!event) return "No anomaly";
  if (event.anomaly_type === "pattern_deviation") {
    return `${(event.metric_value / (1024 * 1024 * 1024)).toFixed(1)} GB used`;
  }
  return formatSize(Math.abs(event.metric_value));
}

export default function AnomalyCard({ drive }: { drive: string }) {
  const [report, setReport] = useState<AnomalyReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    invoke<AnomalyReport>("detect_anomalies", { drive })
      .then((result) => {
        if (!cancelled) setReport(result);
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

  useEffect(() => {
    const unlisten = listen<AnomalyEvent>("anomaly-detected", (event) => {
      if (event.payload.drive_letter.toUpperCase() !== drive.toUpperCase()) return;
      setReport((current) => {
        const existing = current?.events ?? [];
        const key = `${event.payload.created_at}-${event.payload.anomaly_type}-${event.payload.path ?? ""}`;
        const next = existing.some((item) => `${item.created_at}-${item.anomaly_type}-${item.path ?? ""}` === key)
          ? existing
          : [event.payload, ...existing];
        return {
          drive_letter: drive.toUpperCase(),
          sample_count: current?.sample_count ?? 0,
          events: next,
        };
      });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [drive]);

  const event = useMemo(() => latestEvent(report?.events ?? []), [report]);
  const count = report?.events.length ?? 0;
  const criticalCount = report?.events.filter((item) => item.severity === "critical").length ?? 0;
  const severity = event?.severity ?? "warning";
  const style = SEVERITY_STYLE[severity];

  if (loading && !report) {
    return (
      <div className="glass-card p-5 border border-aurora-border/50">
        <div className="h-4 w-36 rounded bg-aurora-elevated/80 progress-shimmer mb-4" />
        <div className="h-16 rounded-xl bg-aurora-elevated/50 progress-shimmer" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="glass-card p-5 border border-danger/20 bg-risk-high-bg/10">
        <div className="text-sm font-semibold text-danger">Anomaly detection unavailable</div>
        <p className="text-xs text-text-muted mt-1">{error}</p>
      </div>
    );
  }

  return (
    <div className="glass-card p-5 border border-aurora-border/50 overflow-hidden relative">
      <div className="absolute -right-8 -bottom-10 w-36 h-36 rounded-full bg-warning/10 blur-3xl" />
      <div className="relative flex flex-wrap items-start justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
              Anomaly Detection
            </h3>
            {event && (
              <span className={`px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase tracking-wider border ${style.className}`}>
                {style.label}
              </span>
            )}
          </div>
          <p className="text-sm text-text-secondary mt-2 max-w-2xl">
            {event ? event.description : "No abnormal growth, hotspot, or seasonal pattern shift detected yet."}
          </p>
          {event?.path && <p className="text-xs text-text-muted mt-1 truncate">{event.path}</p>}
        </div>
        <div className="text-right">
          <div className="text-2xl font-bold font-mono text-text-primary">{count}</div>
          <div className="text-[11px] uppercase tracking-wider text-text-muted">
            anomalies
          </div>
        </div>
      </div>

      <div className="relative grid grid-cols-1 md:grid-cols-4 gap-3 mt-5">
        <Metric label="Last type" value={event ? TYPE_LABEL[event.anomaly_type] : "Clear"} />
        <Metric label="Magnitude" value={formatMetric(event)} />
        <Metric label="Z-score" value={event ? event.modified_z_score.toFixed(1) : "0.0"} />
        <Metric label="Critical" value={`${criticalCount}`} />
      </div>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl bg-aurora-elevated/45 border border-aurora-border/35 px-4 py-3">
      <div className="text-[11px] uppercase tracking-wider text-text-muted">{label}</div>
      <div className="text-sm font-semibold font-mono text-text-primary mt-1 truncate">{value}</div>
    </div>
  );
}
