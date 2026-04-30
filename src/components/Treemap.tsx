import { useEffect, useRef } from "react";
import * as echarts from "echarts";
import { formatSize } from "../utils/format";
import type { DirInfo } from "../types";

interface TreemapProps {
  data: DirInfo[];
  totalBytes: number;
  onDrillDown: (path: string, name: string) => void;
}

// Classify directory into a category for color-coding
function classifyCategory(name: string): string {
  const lower = name.toLowerCase();
  if (["windows", "program files", "program files (x86)", "programdata", "system volume information"].includes(lower))
    return "system";
  if (["users", "user", "desktop", "documents", "downloads", "music", "pictures", "videos", "onedrive"].includes(lower))
    return "user";
  if (["temp", "tmp", "cache", ".npm", ".cargo", ".cache", "node_modules", "__pycache__"].some((p) => lower.includes(p)))
    return "cache";
  if ([".git", "src", "node_modules", "target", "dist", "build", ".vscode", ".idea", "vendor"].some((p) => lower.includes(p)))
    return "dev";
  return "other";
}

const CATEGORY_COLORS: Record<string, string[]> = {
  system: ["#6366f1", "#818cf8", "#4f46e5"],
  user: ["#06b6d4", "#22d3ee", "#0891b2"],
  cache: ["#10b981", "#34d399", "#059669"],
  dev: ["#f59e0b", "#fbbf24", "#d97706"],
  other: ["#64748b", "#94a3b8", "#475569"],
};

export default function Treemap({ data, totalBytes, onDrillDown }: TreemapProps) {
  const chartRef = useRef<HTMLDivElement>(null);
  const instanceRef = useRef<echarts.ECharts | null>(null);

  useEffect(() => {
    if (!chartRef.current || data.length === 0) return;

    if (!instanceRef.current) {
      instanceRef.current = echarts.init(chartRef.current, undefined);
    }

    const chart = instanceRef.current;

    const treemapData = data.map((dir) => ({
      name: dir.name,
      value: dir.size_bytes,
      path: dir.path,
      fileCount: dir.file_count,
      dirCount: dir.dir_count,
      category: classifyCategory(dir.name),
      itemStyle: {
        borderColor: "rgba(6, 8, 13, 0.8)",
        borderWidth: 2,
        borderRadius: 4,
      },
    }));

    chart.setOption(
      {
        tooltip: {
          backgroundColor: "rgba(12, 16, 25, 0.95)",
          borderColor: "rgba(99, 102, 241, 0.3)",
          borderWidth: 1,
          textStyle: { color: "#e2e8f0", fontSize: 13, fontFamily: "'Segoe UI', sans-serif" },
          formatter: (params: any) => {
            const d = params.data;
            if (!d || d.value === undefined) return "";
            const pct = totalBytes > 0 ? ((d.value / totalBytes) * 100).toFixed(2) : "0";
            return `
              <div style="font-weight:600;margin-bottom:6px;color:#e2e8f0">${d.name}</div>
              <div style="color:#94a3b8;font-size:12px">
                <div>Size: <span style="color:#e2e8f0;font-family:monospace">${formatSize(d.value)}</span></div>
                <div>Share: <span style="color:#e2e8f0;font-family:monospace">${pct}%</span></div>
                ${d.fileCount ? `<div>Files: <span style="color:#e2e8f0;font-family:monospace">${d.fileCount.toLocaleString()}</span></div>` : ""}
                ${d.dirCount ? `<div>Subdirs: <span style="color:#e2e8f0;font-family:monospace">${d.dirCount.toLocaleString()}</span></div>` : ""}
              </div>
              <div style="margin-top:6px;font-size:11px;color:#64748b">Click to drill down</div>
            `;
          },
        },
        series: [
          {
            type: "treemap",
            roam: false,
            nodeClick: false,
            breadcrumb: { show: false },
            label: {
              show: true,
              formatter: (params: any) => {
                const name = params.name ?? "";
                const pct = totalBytes > 0 ? ((params.value / totalBytes) * 100).toFixed(1) : "0";
                return `${name}\n${formatSize(params.value)} (${pct}%)`;
              },
              fontSize: 12,
              fontFamily: "'Segoe UI', system-ui, sans-serif",
              color: "#e2e8f0",
              textShadowColor: "rgba(0,0,0,0.5)",
              textShadowBlur: 4,
              overflow: "truncate",
            },
            upperLabel: {
              show: false,
            },
            itemStyle: {
              borderColor: "rgba(6, 8, 13, 0.8)",
              borderWidth: 2,
              gapWidth: 2,
            },
            levels: [
              {
                colorMappingBy: "value",
                color: [
                  CATEGORY_COLORS.dev[0],
                  CATEGORY_COLORS.system[0],
                  CATEGORY_COLORS.user[0],
                  CATEGORY_COLORS.cache[0],
                  CATEGORY_COLORS.other[0],
                ],
              },
            ],
            data: treemapData,
          },
        ],
      },
      { notMerge: true }
    );

    // Click handler for drill-down
    chart.off("click");
    chart.on("click", (params: any) => {
      if (params.data && params.data.path) {
        onDrillDown(params.data.path, params.data.name);
      }
    });

    const handleResize = () => chart.resize();
    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
    };
  }, [data, totalBytes, onDrillDown]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      instanceRef.current?.dispose();
      instanceRef.current = null;
    };
  }, []);

  return (
    <div
      ref={chartRef}
      className="w-full rounded-2xl overflow-hidden"
      style={{ height: 500 }}
    />
  );
}
