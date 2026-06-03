import { useEffect, useRef, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import * as echarts from "echarts";
import { formatSize } from "../../utils/format";
import type { AnomalyEvent, AnomalyReport, AutoCleanupReport, Snapshot, CleanupLog, DirInfo, CleanItemResult, Prediction } from "../../types";

const TIME_RANGES = [
  { label: "7d", value: 7 },
  { label: "30d", value: 30 },
  { label: "90d", value: 90 },
  { label: "1y", value: 365 },
] as const;

function parseSnapshotJson(json: string): DirInfo[] {
  try {
    return JSON.parse(json) as DirInfo[];
  } catch {
    return [];
  }
}

function parseItemsJson(json: string): CleanItemResult[] {
  try {
    return JSON.parse(json) as CleanItemResult[];
  } catch {
    return [];
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString();
}

// 鈹€鈹€鈹€ Trend Chart Component 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

function TrendChart({ snapshots, prediction }: { snapshots: Snapshot[]; prediction: Prediction | null }) {
  const chartRef = useRef<HTMLDivElement>(null);
  const instanceRef = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    if (!chartRef.current || snapshots.length === 0) return;

    if (!instanceRef.current) {
      instanceRef.current = echarts.init(chartRef.current, undefined);
    }

    const chart = instanceRef.current;

    const toGb = (bytes: number) => bytes / (1024 * 1024 * 1024);
    const sorted = [...snapshots].reverse();

    const timeData = sorted.map((s) => s.created_at);
    const totalSeries = sorted.map((s) => toGb(s.total_bytes));
    const usedSeries = sorted.map((s) => toGb(s.used_bytes));
    const freeSeries = sorted.map((s) => toGb(s.free_bytes));
    const forecastData =
      prediction?.forecast
        .filter((point) => point.is_forecast)
        .map((point) => [point.created_at, toGb(point.used_bytes)]) ?? [];

    chart.setOption(
      {
        tooltip: {
          trigger: "axis",
          backgroundColor: "rgba(12, 16, 25, 0.95)",
          borderColor: "rgba(99, 102, 241, 0.3)",
          borderWidth: 1,
          textStyle: { color: "#e2e8f0", fontSize: 12, fontFamily: "'Segoe UI', sans-serif" },
          valueFormatter: (value: unknown) =>
            typeof value === "number" ? `${value.toFixed(2)} GB` : String(value),
        },
        legend: {
          bottom: 0,
          textStyle: { color: "#94a3b8", fontSize: 12 },
          itemGap: 24,
        },
        grid: {
          left: 80,
          right: 30,
          top: 20,
          bottom: 40,
        },
        xAxis: {
          type: "time",
          axisLine: { lineStyle: { color: "#1e293b" } },
          axisTick: { show: false },
          axisLabel: {
            color: "#64748b",
            fontSize: 11,
            formatter: (value: number) => {
              const d = new Date(value);
              return `${d.getMonth() + 1}/${d.getDate()}`;
            },
          },
          splitLine: { show: false },
        },
        yAxis: {
          type: "value",
          name: "GB",
          nameTextStyle: { color: "#64748b", fontSize: 11 },
          axisLabel: {
            color: "#64748b",
            fontSize: 11,
            formatter: (value: number) => value.toFixed(0),
          },
          splitLine: { lineStyle: { color: "rgba(30, 41, 59, 0.6)" } },
        },
        series: [
          {
            name: "Total",
            type: "line",
            lineStyle: { type: "dashed", width: 2 },
            itemStyle: { color: "#94a3b8" },
            symbol: "none",
            smooth: true,
            data: timeData.map((t, i) => [t, totalSeries[i]]),
          },
          {
            name: "Used",
            type: "line",
            lineStyle: { width: 2 },
            itemStyle: { color: "#f59e0b" },
            areaStyle: { color: "rgba(245, 158, 11, 0.08)" },
            symbol: "circle",
            symbolSize: 4,
            smooth: true,
            data: timeData.map((t, i) => [t, usedSeries[i]]),
          },
          {
            name: "Free",
            type: "line",
            lineStyle: { width: 2 },
            itemStyle: { color: "#10b981" },
            areaStyle: { color: "rgba(16, 185, 129, 0.06)" },
            symbol: "circle",
            symbolSize: 4,
            smooth: true,
            data: timeData.map((t, i) => [t, freeSeries[i]]),
          },
          ...(forecastData.length > 0
            ? [
                {
                  name: "Forecast",
                  type: "line",
                  lineStyle: { width: 2, type: "dashed" },
                  itemStyle: { color: "#38bdf8" },
                  symbol: "diamond",
                  symbolSize: 5,
                  smooth: true,
                  data: [
                    [timeData[timeData.length - 1], usedSeries[usedSeries.length - 1]],
                    ...forecastData,
                  ],
                },
              ]
            : []),
        ],
      },
      { notMerge: true }
    );

    const handleResize = () => chart.resize();
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [snapshots, prediction]);

  useEffect(() => {
    return () => {
      instanceRef.current?.dispose();
      instanceRef.current = null;
    };
  }, []);

  if (snapshots.length === 0) return null;

  return (
    <div
      ref={chartRef}
      className="w-full rounded-2xl overflow-hidden"
      style={{ height: 380 }}
    />
  );
}

// 鈹€鈹€鈹€ Snapshot Table 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

function anomalyTypeLabel(type: AnomalyEvent["anomaly_type"]) {
  switch (type) {
    case "burst_anomaly":
      return "Burst growth";
    case "hotspot_anomaly":
      return "Hotspot";
    case "pattern_deviation":
      return "Pattern shift";
    default:
      return "Rate anomaly";
  }
}

function AnomalyChart({ snapshots, events }: { snapshots: Snapshot[]; events: AnomalyEvent[] }) {
  const chartRef = useRef<HTMLDivElement>(null);
  const instanceRef = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    if (!chartRef.current || snapshots.length === 0) return;

    if (!instanceRef.current) {
      instanceRef.current = echarts.init(chartRef.current, undefined);
    }

    const chart = instanceRef.current;
    const toGb = (bytes: number) => bytes / (1024 * 1024 * 1024);
    const sorted = [...snapshots].reverse();
    const usedByTime = new Map(sorted.map((snapshot) => [snapshot.created_at, toGb(snapshot.used_bytes)]));
    const anomalyPoints = events.map((event) => ({
      value: [event.created_at, usedByTime.get(event.created_at) ?? toGb(Math.abs(event.metric_value))],
      event,
      itemStyle: {
        color: event.severity === "critical" ? "#ef4444" : "#f59e0b",
      },
    }));

    chart.setOption(
      {
        tooltip: {
          trigger: "item",
          backgroundColor: "rgba(12, 16, 25, 0.95)",
          borderColor: "rgba(239, 68, 68, 0.3)",
          borderWidth: 1,
          textStyle: { color: "#e2e8f0", fontSize: 12, fontFamily: "'Segoe UI', sans-serif" },
          formatter: (params: unknown) => {
            const item = params as { data?: { event?: AnomalyEvent } };
            const event = item.data?.event;
            if (!event) return "";
            return [
              "<strong>" + anomalyTypeLabel(event.anomaly_type) + "</strong>",
              event.created_at,
              event.description,
              "Z-score: " + event.modified_z_score.toFixed(1),
              event.path ?? "",
            ]
              .filter(Boolean)
              .join("<br/>");
          },
        },
        legend: {
          bottom: 0,
          textStyle: { color: "#94a3b8", fontSize: 12 },
        },
        grid: {
          left: 80,
          right: 30,
          top: 20,
          bottom: 42,
        },
        xAxis: {
          type: "time",
          axisLine: { lineStyle: { color: "#1e293b" } },
          axisTick: { show: false },
          axisLabel: {
            color: "#64748b",
            fontSize: 11,
            formatter: (value: number) => {
              const d = new Date(value);
              return `${d.getMonth() + 1}/${d.getDate()}`;
            },
          },
          splitLine: { show: false },
        },
        yAxis: {
          type: "value",
          name: "GB used",
          nameTextStyle: { color: "#64748b", fontSize: 11 },
          axisLabel: { color: "#64748b", fontSize: 11 },
          splitLine: { lineStyle: { color: "rgba(30, 41, 59, 0.6)" } },
        },
        series: [
          {
            name: "Used",
            type: "line",
            lineStyle: { width: 2 },
            itemStyle: { color: "#38bdf8" },
            areaStyle: { color: "rgba(56, 189, 248, 0.06)" },
            symbol: "none",
            smooth: true,
            data: sorted.map((snapshot) => [snapshot.created_at, toGb(snapshot.used_bytes)]),
          },
          {
            name: "Anomaly",
            type: "scatter",
            symbolSize: 14,
            data: anomalyPoints,
          },
        ],
      },
      { notMerge: true }
    );

    const handleResize = () => chart.resize();
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [snapshots, events]);

  useEffect(() => {
    return () => {
      instanceRef.current?.dispose();
      instanceRef.current = null;
    };
  }, []);

  if (snapshots.length === 0) return null;

  return <div ref={chartRef} className="w-full rounded-2xl overflow-hidden" style={{ height: 380 }} />;
}

function AnomalyEventList({ events }: { events: AnomalyEvent[] }) {
  if (events.length === 0) {
    return (
      <div className="rounded-2xl bg-aurora-elevated/40 border border-aurora-border/35 p-4 text-sm text-text-muted">
        No anomalies detected in this window.
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {events.map((event, index) => (
        <div key={`${event.created_at}-${event.anomaly_type}-${event.path ?? index}`} className="rounded-2xl bg-aurora-elevated/45 border border-aurora-border/35 p-4">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div className="flex items-center gap-3 min-w-0">
              <span className={`w-2.5 h-2.5 rounded-full ${event.severity === "critical" ? "bg-danger" : "bg-warning"}`} />
              <div className="min-w-0">
                <div className="text-sm font-semibold text-text-primary">
                  {anomalyTypeLabel(event.anomaly_type)}
                  <span className="ml-2 text-xs font-normal text-text-muted">{formatDate(event.created_at)}</span>
                </div>
                <p className="text-xs text-text-secondary mt-1">{event.description}</p>
                {event.path && <p className="text-xs text-text-muted mt-1 truncate">{event.path}</p>}
              </div>
            </div>
            <span className="text-xs font-mono text-text-muted">
              z={event.modified_z_score.toFixed(1)}
            </span>
          </div>
        </div>
      ))}
    </div>
  );
}
function SnapshotTable({ snapshots }: { snapshots: Snapshot[] }) {
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const selected = selectedId != null ? snapshots.find((s) => s.id === selectedId) : null;

  return (
    <div className="glass-card p-4">
      <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider mb-4 px-2">
        Snapshot History
      </h3>

      {snapshots.length === 0 ? (
        <p className="text-xs text-text-muted px-2">No snapshots recorded yet.</p>
      ) : (
        <>
          <div className="max-h-72 overflow-y-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-xs text-text-muted uppercase tracking-wider">
                  <th className="text-left p-2 font-medium">Date</th>
                  <th className="text-right p-2 font-medium">Drive</th>
                  <th className="text-right p-2 font-medium">Total</th>
                  <th className="text-right p-2 font-medium">Used</th>
                  <th className="text-right p-2 font-medium">Free</th>
                  <th className="text-right p-2 font-medium">Usage</th>
                </tr>
              </thead>
              <tbody>
                {snapshots.map((s) => {
                  const pct = s.total_bytes > 0
                    ? ((s.used_bytes / s.total_bytes) * 100).toFixed(1)
                    : "0";
                  const isSelected = selectedId === s.id;
                  return (
                    <tr
                      key={s.id}
                      className={`cursor-pointer transition-colors hover:bg-aurora-elevated/60 ${
                        isSelected ? "bg-aurora-elevated/80" : ""
                      }`}
                      onClick={() => setSelectedId(isSelected ? null : s.id)}
                    >
                      <td className="p-2 text-text-secondary font-mono text-xs whitespace-nowrap">
                        {formatDate(s.created_at)}
                      </td>
                      <td className="p-2 text-right text-text-primary font-mono">
                        {s.drive_letter}:
                      </td>
                      <td className="p-2 text-right text-text-secondary font-mono">
                        {formatSize(s.total_bytes)}
                      </td>
                      <td className="p-2 text-right text-text-primary font-mono">
                        {formatSize(s.used_bytes)}
                      </td>
                      <td className="p-2 text-right text-success font-mono">
                        {formatSize(s.free_bytes)}
                      </td>
                      <td className="p-2 text-right font-mono">
                        <span
                          className={
                            Number(pct) > 90
                              ? "text-danger"
                              : Number(pct) > 70
                                ? "text-warning"
                                : "text-text-secondary"
                          }
                        >
                          {pct}%
                        </span>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          {/* Expanded snapshot detail */}
          {selected && (
            <div className="mt-4 p-4 rounded-2xl bg-aurora-elevated/50 border border-aurora-border/40">
              <div className="flex items-center justify-between mb-3">
                <span className="text-sm font-semibold text-text-primary">
                  Top directories at {formatDate(selected.created_at)}
                </span>
                <span className="text-xs text-text-muted">
                  {parseSnapshotJson(selected.snapshot_json).length} items
                </span>
              </div>
              <div className="space-y-1.5 max-h-48 overflow-y-auto">
                {parseSnapshotJson(selected.snapshot_json).slice(0, 15).map((dir) => (
                  <div
                    key={dir.path}
                    className="flex items-center justify-between rounded-lg bg-aurora-elevated/60 px-3 py-2 text-xs"
                  >
                    <div className="min-w-0">
                      <span className="text-text-primary truncate block">{dir.name}</span>
                      <span className="text-text-muted truncate block">{dir.path}</span>
                    </div>
                    <div className="flex items-center gap-3 flex-shrink-0 ml-3">
                      <span className="text-text-secondary font-mono">{formatSize(dir.size_bytes)}</span>
                      <span className="text-text-muted">{dir.file_count.toLocaleString()} files</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}

// 鈹€鈹€鈹€ Cleanup Timeline 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

function ForecastStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-24 rounded-xl bg-aurora-elevated/50 border border-aurora-border/35 px-3 py-2">
      <div className="text-[10px] uppercase tracking-wider text-text-muted">{label}</div>
      <div className="text-sm font-semibold font-mono text-text-primary mt-0.5">{value}</div>
    </div>
  );
}

function CleanupTimeline({ logs }: { logs: CleanupLog[] }) {
  const [expandedId, setExpandedId] = useState<number | null>(null);

  if (logs.length === 0) {
    return (
      <div className="glass-card p-6 text-center">
        <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider mb-3">
          Cleanup History
        </h3>
        <p className="text-xs text-text-muted">No cleanup operations recorded yet.</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
        Cleanup History
      </h3>

      {logs.map((log) => {
        const isExpanded = expandedId === log.id;
        const items = parseItemsJson(log.items_json);

        return (
          <div
            key={log.id}
            className="glass-card p-5 transition-colors"
          >
            {/* Header row */}
            <div
              className="flex flex-wrap items-center justify-between gap-3 cursor-pointer"
              onClick={() => setExpandedId(isExpanded ? null : log.id)}
            >
              <div className="flex items-center gap-3">
                <div className="w-2 h-2 rounded-full bg-success" />
                <div>
                  <div className="text-sm text-text-primary font-medium">
                    {formatDate(log.created_at)}
                  </div>
                  <div className="text-xs text-text-muted mt-0.5">
                    {log.item_count} item{log.item_count !== 1 ? "s" : ""} processed
                  </div>
                </div>
              </div>

              <div className="flex items-center gap-4 text-xs">
                <div className="flex items-center gap-1.5">
                  <span className="text-success font-semibold">{formatSize(log.freed_bytes)}</span>
                  <span className="text-text-muted">freed</span>
                </div>
                <div className="flex items-center gap-3 ml-2">
                  <span className="text-success">{log.succeeded} ok</span>
                  {log.skipped > 0 && <span className="text-warning">{log.skipped} skip</span>}
                  {log.failed > 0 && <span className="text-danger">{log.failed} fail</span>}
                </div>
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  className={`text-text-muted transition-transform ${isExpanded ? "rotate-180" : ""}`}
                >
                  <polyline points="6 9 12 15 18 9" />
                </svg>
              </div>
            </div>

            {/* Expanded detail */}
            {isExpanded && items.length > 0 && (
              <div className="mt-4 pt-4 border-t border-aurora-border/40 space-y-2 max-h-64 overflow-y-auto">
                {items.map((item, i) => (
                  <div
                    key={`${item.path}-${i}`}
                    className="flex items-center gap-3 rounded-lg bg-aurora-elevated/60 px-3 py-2 text-xs"
                  >
                    <span
                      className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${
                        item.status === "Success"
                          ? "bg-success"
                          : item.status === "Skipped"
                            ? "bg-warning"
                            : "bg-danger"
                      }`}
                    />
                    <span className="text-text-primary font-medium truncate">{item.name}</span>
                    <span className="text-text-muted font-mono flex-shrink-0">
                      {formatSize(item.size_bytes)}
                    </span>
                    <span
                      className={`flex-shrink-0 ml-auto text-xs ${
                        item.status === "Success"
                          ? "text-success"
                          : item.status === "Skipped"
                            ? "text-warning"
                            : "text-danger"
                      }`}
                    >
                      {item.status}
                    </span>
                    {item.reason && (
                      <span className="text-text-muted truncate max-w-48">{item.reason}</span>
                    )}
                  </div>
                ))}
              </div>
            )}

            {isExpanded && items.length === 0 && (
              <div className="mt-4 pt-4 border-t border-aurora-border/40">
                <p className="text-xs text-text-muted text-center">No item details available.</p>
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}

function AutoCleanupTimeline({ reports }: { reports: AutoCleanupReport[] }) {
  const [expandedId, setExpandedId] = useState<number | null>(null);

  if (reports.length === 0) {
    return (
      <div className="glass-card p-6 text-center">
        <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider mb-3">
          Auto-Cleanup History
        </h3>
        <p className="text-xs text-text-muted">No auto-cleanup runs recorded yet.</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
        Auto-Cleanup History
      </h3>

      {reports.map((report) => {
        const isExpanded = expandedId === report.id;
        const items = parseItemsJson(report.items_json);
        const success = report.status === "completed";

        return (
          <div key={report.id} className="glass-card p-5 transition-colors">
            <div
              className="flex flex-wrap items-center justify-between gap-3 cursor-pointer"
              onClick={() => setExpandedId(isExpanded ? null : report.id)}
            >
              <div className="flex items-center gap-3">
                <div className={`w-2 h-2 rounded-full ${success ? "bg-success" : "bg-danger"}`} />
                <div>
                  <div className="text-sm text-text-primary font-medium">
                    {formatDate(report.created_at)} · {report.drive_letter}:
                  </div>
                  <div className="text-xs text-text-muted mt-0.5">{report.message}</div>
                </div>
              </div>

              <div className="flex items-center gap-4 text-xs">
                <span className={success ? "text-success" : "text-danger"}>{report.status}</span>
                <span className="text-success font-semibold">{formatSize(report.freed_bytes)} freed</span>
                <span className="text-text-muted">{report.succeeded} ok / {report.skipped} skip / {report.failed} fail</span>
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                  className={`text-text-muted transition-transform ${isExpanded ? "rotate-180" : ""}`}
                >
                  <polyline points="6 9 12 15 18 9" />
                </svg>
              </div>
            </div>

            {isExpanded && (
              <div className="mt-4 pt-4 border-t border-aurora-border/40 space-y-2 max-h-64 overflow-y-auto">
                {items.length === 0 ? (
                  <p className="text-xs text-text-muted text-center">No item details available.</p>
                ) : (
                  items.map((item, i) => (
                    <div
                      key={`${item.path}-${i}`}
                      className="flex items-center gap-3 rounded-lg bg-aurora-elevated/60 px-3 py-2 text-xs"
                    >
                      <span
                        className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${
                          item.status === "Success" ? "bg-success" : item.status === "Skipped" ? "bg-warning" : "bg-danger"
                        }`}
                      />
                      <span className="text-text-primary font-medium truncate">{item.name}</span>
                      <span className="text-text-muted font-mono flex-shrink-0">{formatSize(item.size_bytes)}</span>
                      <span className="ml-auto text-text-secondary">{item.status}</span>
                    </div>
                  ))
                )}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
// 鈹€鈹€鈹€ Main History Page 鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€

export default function HistoryPage() {
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const [cleanupLogs, setCleanupLogs] = useState<CleanupLog[]>([]);
  const [autoCleanupReports, setAutoCleanupReports] = useState<AutoCleanupReport[]>([]);
  const [prediction, setPrediction] = useState<Prediction | null>(null);
  const [anomalyReport, setAnomalyReport] = useState<AnomalyReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [selectedDrive, setSelectedDrive] = useState("C");
  const [timeRange, setTimeRange] = useState<number>(30);
  const [drives, setDrives] = useState<string[]>(["C"]);
  const [historyTab, setHistoryTab] = useState<"trend" | "anomaly">("trend");

  useEffect(() => {
    invoke<string[]>("list_drives")
      .then((list) => {
        setDrives(list);
        if (list.length > 0 && !list.includes("C")) {
          setSelectedDrive(list[0]);
        }
      })
      .catch(() => setDrives(["C"]));
  }, []);

  const loadHistory = useCallback(async (drive: string, days: number) => {
    setLoading(true);
    setError(null);
    setPrediction(null);
    setAnomalyReport(null);
    try {
      const [snaps, logs, autoReports, predicted, anomalies] = await Promise.all([
        invoke<Snapshot[]>("get_snapshot_history", { drive, days }),
        invoke<CleanupLog[]>("get_cleanup_history"),
        invoke<AutoCleanupReport[]>("get_auto_cleanup_history"),
        invoke<Prediction>("predict_disk_usage", { drive, days }).catch(() => null),
        invoke<AnomalyReport>("detect_anomalies", { drive }).catch(() => null),
      ]);
      setSnapshots(snaps);
      setCleanupLogs(logs);
      setAutoCleanupReports(autoReports);
      setPrediction(predicted);
      setAnomalyReport(anomalies);
    } catch (e) {
      setError(String(e));
      setPrediction(null);
      setAnomalyReport(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadHistory(selectedDrive, timeRange);
  }, [selectedDrive, timeRange, loadHistory]);

  const hasData = snapshots.length > 0 || cleanupLogs.length > 0 || autoCleanupReports.length > 0;
  const snapshotTimes = new Set(snapshots.map((snapshot) => snapshot.created_at));
  const visibleAnomalies = (anomalyReport?.events ?? []).filter((event) =>
    snapshotTimes.has(event.created_at),
  );

  return (
    <div className="p-8 space-y-8">
      {/* Header + Controls */}
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
            History & Trends
          </h2>
          <p className="text-xs text-text-muted mt-1">
            Disk usage trends over time and cleanup operation history
          </p>
        </div>

        <div className="flex items-center gap-3">
          {/* Drive selector */}
          <select
            value={selectedDrive}
            onChange={(e) => setSelectedDrive(e.target.value)}
            disabled={loading}
            className="px-3 py-2 rounded-lg bg-aurora-elevated border border-aurora-border/50 text-sm text-text-primary
                       focus:outline-none focus:border-accent/50 focus:ring-1 focus:ring-accent/30
                       disabled:opacity-50 cursor-pointer appearance-none"
            style={{
              backgroundImage: `url("data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 20 20'%3e%3cpath stroke='%2394a3b8' stroke-linecap='round' stroke-linejoin='round' stroke-width='1.5' d='M6 8l4 4 4-4'/%3e%3c/svg%3e")`,
              backgroundPosition: "right 8px center",
              backgroundRepeat: "no-repeat",
              backgroundSize: "16px",
              paddingRight: "32px",
            }}
          >
            {drives.map((d) => (
              <option key={d} value={d}>{d}: Drive</option>
            ))}
          </select>

          {/* Time range selector */}
          <div className="flex rounded-lg bg-aurora-elevated border border-aurora-border/50 overflow-hidden">
            {TIME_RANGES.map((r) => (
              <button
                key={r.value}
                className={`px-3 py-2 text-xs font-medium transition-colors ${
                  timeRange === r.value
                    ? "bg-accent/20 text-accent-light border-x border-accent/20"
                    : "text-text-secondary hover:text-text-primary hover:bg-aurora-elevated/80"
                }`}
                onClick={() => setTimeRange(r.value)}
              >
                {r.label}
              </button>
            ))}
          </div>

          {/* Refresh button */}
          <button
            className="px-3 py-2 rounded-lg bg-aurora-elevated border border-aurora-border/50 text-xs text-text-secondary
                       hover:text-accent-light hover:border-accent/30 transition-colors"
            onClick={() => loadHistory(selectedDrive, timeRange)}
            disabled={loading}
          >
            {loading ? "Loading..." : "Refresh"}
          </button>
        </div>
      </div>

      {/* Error banner */}
      {error && (
        <div className="px-4 py-3 rounded-xl bg-risk-high-bg border border-red-500/20 text-sm text-danger flex items-center gap-2">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
            <circle cx="12" cy="12" r="10" />
            <path d="M12 8v4M12 16h.01" />
          </svg>
          {error}
          <button onClick={() => setError(null)} className="ml-auto text-text-muted hover:text-text-primary">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <path d="M18 6L6 18M6 6l12 12" />
            </svg>
          </button>
        </div>
      )}

      {/* Loading state */}
      {loading && !hasData && (
        <div className="flex flex-col items-center justify-center py-16 text-text-muted gap-3">
          <svg className="animate-spin" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="10" strokeDasharray="32" strokeDashoffset="32" />
          </svg>
          <p className="text-sm">Loading history...</p>
        </div>
      )}

      {/* Empty state */}
      {!loading && !hasData && !error && (
        <div className="flex flex-col items-center justify-center py-16 text-text-muted">
          <div className="w-16 h-16 rounded-2xl bg-aurora-elevated border border-aurora-border/40 flex items-center justify-center mb-4">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round">
              <polyline points="1 4 1 10 7 10" />
              <path d="M3.5 17.5A9 9 0 102 12" />
            </svg>
          </div>
          <p className="text-sm">No history data yet</p>
          <p className="text-xs text-text-muted mt-1">
            Run a drive scan or cleanup to start recording history
          </p>
        </div>
      )}

      {hasData && (
        <>
          {/* Trend / anomaly chart */}
          <div className="glass-card p-6">
            <div className="flex flex-wrap items-center justify-between gap-4 mb-6 px-2">
              <div>
                <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
                  {historyTab === "trend" ? "Disk Usage Trend" : "Anomaly Detection"}
                </h3>
                <p className="text-xs text-text-muted mt-1">
                  {selectedDrive}: Drive 鈥?last {timeRange} days 路 {snapshots.length} snapshot
                  {snapshots.length !== 1 ? "s" : ""}
                </p>
              </div>
              <div className="flex rounded-lg bg-aurora-elevated border border-aurora-border/50 overflow-hidden">
                <button
                  className={`px-3 py-2 text-xs font-medium transition-colors ${
                    historyTab === "trend"
                      ? "bg-accent/20 text-accent-light border-r border-accent/20"
                      : "text-text-secondary hover:text-text-primary hover:bg-aurora-elevated/80"
                  }`}
                  onClick={() => setHistoryTab("trend")}
                >
                  Trend
                </button>
                <button
                  className={`px-3 py-2 text-xs font-medium transition-colors ${
                    historyTab === "anomaly"
                      ? "bg-danger/15 text-danger border-l border-danger/20"
                      : "text-text-secondary hover:text-text-primary hover:bg-aurora-elevated/80"
                  }`}
                  onClick={() => setHistoryTab("anomaly")}
                >
                  Anomaly Detection
                </button>
              </div>
            </div>
            {snapshots.length > 0 ? (
              historyTab === "trend" ? (
                <TrendChart snapshots={snapshots} prediction={prediction} />
              ) : (
                <div className="space-y-5">
                  <AnomalyChart snapshots={snapshots} events={visibleAnomalies} />
                  <AnomalyEventList events={visibleAnomalies} />
                </div>
              )
            ) : (
              <div className="flex items-center justify-center h-[380px] text-text-muted text-sm">
                No snapshots for the selected time range.
              </div>
            )}
          </div>

          {prediction && (
            <div className="glass-card p-5">
              <div className="flex flex-wrap items-center justify-between gap-4">
                <div>
                  <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
                    Forecast Summary
                  </h3>
                  <p className="text-sm text-text-secondary mt-2">{prediction.message}</p>
                </div>
                <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-right">
                  <ForecastStat label="Growth" value={`${formatSize(Math.abs(prediction.growth_bytes_per_day))}/day`} />
                  <ForecastStat label="Confidence" value={`${Math.round(prediction.confidence_score * 100)}%`} />
                  <ForecastStat label="Seasonal" value={formatSize(Math.abs(prediction.seasonal_component))} />
                  <ForecastStat
                    label="To 95%"
                    value={
                      prediction.days_to_95_percent == null
                        ? "N/A"
                        : `${Math.round(prediction.days_to_95_percent)}d`
                    }
                  />
                </div>
              </div>
            </div>
          )}

          {/* Snapshot Table */}
          <SnapshotTable snapshots={snapshots} />

          {/* Cleanup Timeline */}
          <CleanupTimeline logs={cleanupLogs} />

          {/* Auto-Cleanup Timeline */}
          <AutoCleanupTimeline reports={autoCleanupReports} />
        </>
      )}
    </div>
  );
}

