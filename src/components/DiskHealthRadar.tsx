import { useEffect, useRef } from "react";
import * as echarts from "echarts";
import type { DiskHealth } from "../types";

export default function DiskHealthRadar({ health }: { health: DiskHealth | null }) {
  const chartRef = useRef<HTMLDivElement>(null);
  const instanceRef = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    if (!chartRef.current || !health) return;
    if (!instanceRef.current) {
      instanceRef.current = echarts.init(chartRef.current, undefined);
    }

    const chart = instanceRef.current;
    chart.setOption(
      {
        tooltip: {
          backgroundColor: "rgba(12, 16, 25, 0.95)",
          borderColor: "rgba(56, 189, 248, 0.3)",
          borderWidth: 1,
          textStyle: { color: "#e2e8f0", fontSize: 12, fontFamily: "'Segoe UI', sans-serif" },
        },
        radar: {
          radius: "68%",
          splitNumber: 4,
          axisName: {
            color: "#94a3b8",
            fontSize: 11,
          },
          splitLine: { lineStyle: { color: "rgba(148, 163, 184, 0.18)" } },
          splitArea: { areaStyle: { color: ["rgba(56, 189, 248, 0.04)", "rgba(56, 189, 248, 0.01)"] } },
          axisLine: { lineStyle: { color: "rgba(148, 163, 184, 0.18)" } },
          indicator: [
            { name: "Space", max: 100 },
            { name: "Waste", max: 100 },
            { name: "Trend", max: 100 },
            { name: "Age", max: 100 },
          ],
        },
        series: [
          {
            name: "Disk health",
            type: "radar",
            symbol: "circle",
            symbolSize: 5,
            lineStyle: { color: "#38bdf8", width: 2 },
            itemStyle: { color: "#38bdf8" },
            areaStyle: { color: "rgba(56, 189, 248, 0.18)" },
            data: [
              {
                value: [health.space_score, health.waste_score, health.trend_score, health.age_score],
                name: health.drive_letter,
              },
            ],
          },
        ],
      },
      { notMerge: true }
    );

    const handleResize = () => chart.resize();
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [health]);

  useEffect(() => {
    return () => {
      instanceRef.current?.dispose();
      instanceRef.current = null;
    };
  }, []);

  if (!health) {
    return (
      <div className="flex h-52 items-center justify-center rounded-2xl border border-aurora-border/40 bg-aurora-elevated/40 text-sm text-text-muted">
        Health radar pending
      </div>
    );
  }

  return <div ref={chartRef} className="h-52 w-full" />;
}
