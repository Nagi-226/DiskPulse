import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { CleanItem, DiskHealth, Recommendation } from "../types";
import { formatSize } from "../utils/format";
import DiskHealthRadar from "./DiskHealthRadar";

function toCleanItem(recommendation: Recommendation): CleanItem {
  return {
    name: recommendation.item.name,
    path: recommendation.item.path,
    size_bytes: recommendation.estimated_size || recommendation.item.size_bytes,
    risk_level: recommendation.item.risk_level === "low" ? "low" : recommendation.item.risk_level === "high" ? "high" : "medium",
    safe_to_delete: recommendation.item.safe_to_delete,
  };
}

export default function RecommendationCard({ drive, onAddToCleanup }: { drive: string; onAddToCleanup: (items: CleanItem[]) => void }) {
  const [recommendations, setRecommendations] = useState<Recommendation[]>([]);
  const [health, setHealth] = useState<DiskHealth | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    setLoading(true);
    setError(null);
    try {
      const [nextRecommendations, nextHealth] = await Promise.all([
        invoke<Recommendation[]>("get_recommendations", { drive }),
        invoke<DiskHealth>("get_disk_health", { drive }),
      ]);
      setRecommendations(nextRecommendations.slice(0, 5));
      setHealth(nextHealth);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void load();
  }, [drive]);

  const topSafeItems = recommendations.filter((item) => item.item.safe_to_delete).map(toCleanItem);
  const gaugeColor = health && health.score < 50 ? "var(--color-danger)" : health && health.score < 75 ? "var(--color-warning)" : "var(--color-success)";

  return (
    <div className="glass-card p-6">
      <div className="mb-5 flex flex-wrap items-center justify-between gap-4">
        <div>
          <h3 className="text-sm font-semibold uppercase tracking-wider text-text-primary">Smart Recommendations</h3>
          <p className="mt-1 text-xs text-text-muted">Weighted cleanup ranking plus a lightweight disk health score.</p>
        </div>
        <button className="rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-3 py-2 text-xs font-semibold text-text-secondary hover:text-accent-light" onClick={() => void load()} disabled={loading}>
          {loading ? "Checking..." : "Run Health Check"}
        </button>
      </div>

      {error && <div className="mb-4 rounded-xl border border-red-500/20 bg-risk-high-bg/20 p-3 text-sm text-danger">{error}</div>}

      <div className="grid grid-cols-1 gap-5 xl:grid-cols-[220px_1fr]">
        <div className="rounded-2xl border border-aurora-border/40 bg-aurora-elevated/50 p-5 text-center">
          <div className="mx-auto flex h-32 w-32 items-center justify-center rounded-full border-[10px]" style={{ borderColor: gaugeColor }}>
            <div>
              <div className="text-3xl font-bold text-text-primary">{health?.score ?? "--"}</div>
              <div className="text-xs uppercase tracking-wider text-text-muted">health</div>
            </div>
          </div>
          <div className="mt-4 text-sm font-semibold text-text-primary">{health?.status ?? "pending"}</div>
          <p className="mt-2 text-xs leading-5 text-text-muted">{health?.message ?? "Run a health check to compute score."}</p>
          <div className="mt-4">
            <DiskHealthRadar health={health} />
          </div>
        </div>

        <div className="space-y-2">
          {recommendations.map((recommendation) => (
            <div key={recommendation.item.path} className="rounded-xl border border-aurora-border/40 bg-aurora-elevated/40 px-4 py-3">
              <div className="flex items-center gap-3">
                <span className="w-8 font-mono text-xs text-accent-light">#{recommendation.rank}</span>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <div className="truncate text-sm font-semibold text-text-primary" title={recommendation.item.path}>{recommendation.item.name}</div>
                    <UrgencyBadge label={recommendation.urgency_label} />
                    {recommendation.pattern_boost > 1.0 && (
                      <span className="rounded-full border border-accent/20 bg-accent/10 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-accent-light">
                        pattern {recommendation.pattern_boost.toFixed(1)}x
                      </span>
                    )}
                  </div>
                  <div className="truncate text-xs text-text-muted" title={recommendation.item.path}>
                    {recommendation.reason}
                    {recommendation.correlation_bonus > 0 && ` · +${recommendation.correlation_bonus.toFixed(0)} correlation`}
                  </div>
                </div>
                <span className="font-mono text-xs text-text-muted">{recommendation.score.toFixed(0)}</span>
                <span className="font-mono text-xs text-text-secondary">{formatSize(recommendation.estimated_size)}</span>
              </div>
            </div>
          ))}
          {!loading && recommendations.length === 0 && <div className="rounded-xl border border-aurora-border/40 bg-aurora-elevated/40 p-8 text-center text-sm text-text-muted">No recommendations yet.</div>}
        </div>
      </div>

      {topSafeItems.length > 0 && (
        <div className="mt-5 flex justify-end">
          <button className="rounded-xl border border-success/25 bg-risk-low-bg px-4 py-2.5 text-sm font-semibold text-success" onClick={() => onAddToCleanup(topSafeItems)}>
            Add safe recommendations to Cleanup Preview
          </button>
        </div>
      )}
    </div>
  );
}

function UrgencyBadge({ label }: { label: Recommendation["urgency_label"] }) {
  const style =
    label === "critical"
      ? "border-danger/30 bg-risk-high-bg text-danger"
      : label === "elevated"
        ? "border-warning/25 bg-risk-medium-bg text-warning"
        : "border-success/25 bg-risk-low-bg text-success";
  const icon = label === "critical" ? "🔴" : label === "elevated" ? "🟡" : "🟢";

  return (
    <span className={`rounded-full border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider ${style}`}>
      {icon} {label}
    </span>
  );
}
