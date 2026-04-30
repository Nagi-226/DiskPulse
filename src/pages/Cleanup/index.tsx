import { useMemo, useState } from "react";
import type { RiskItem, RiskLevel, RiskReport } from "../../types";
import { formatSize } from "../../utils/format";

type SortKey = "risk" | "size" | "name";
type RiskFilter = RiskLevel | "all";

const RISK_ORDER: Record<RiskLevel, number> = {
  high: 0,
  medium: 1,
  low: 2,
};

const RISK_STYLES: Record<RiskLevel, { panel: string; badge: string; title: string; accent: string }> = {
  low: {
    panel: "border-success/15 bg-risk-low-bg/20",
    badge: "bg-risk-low-bg text-success border-success/20",
    title: "Low risk",
    accent: "text-success",
  },
  medium: {
    panel: "border-warning/15 bg-risk-medium-bg/20",
    badge: "bg-risk-medium-bg text-warning border-warning/20",
    title: "Medium risk",
    accent: "text-warning",
  },
  high: {
    panel: "border-danger/15 bg-risk-high-bg/20",
    badge: "bg-risk-high-bg text-danger border-danger/20",
    title: "High risk",
    accent: "text-danger",
  },
};

function riskLabel(level: RiskLevel) {
  return level.charAt(0).toUpperCase() + level.slice(1);
}

function riskBadgeClass(level: RiskLevel) {
  return RISK_STYLES[level].badge;
}

function serializeCsvCell(value: string | number | boolean) {
  const text = String(value).split('"').join('""');
  return `"${text}"`;
}

function buildCsv(report: RiskReport, items: RiskItem[]) {
  const header = [
    "drive_letter",
    "name",
    "path",
    "size_bytes",
    "file_count",
    "dir_count",
    "risk_level",
    "category",
    "safe_to_delete",
    "explanation",
  ];

  const rows = items.map((item) => [
    report.drive_letter,
    item.name,
    item.path,
    item.size_bytes,
    item.file_count,
    item.dir_count,
    item.risk_level,
    item.category,
    item.safe_to_delete,
    item.explanation,
  ]);

  return [header, ...rows]
    .map((row) => row.map(serializeCsvCell).join(","))
    .join("\r\n");
}

function escapeHtml(text: string): string {
  const map: Record<string, string> = {
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#039;",
  };
  return text.replace(/[&<>"']/g, (ch) => map[ch] ?? ch);
}

function buildHtml(report: RiskReport, items: RiskItem[]) {
  const rows = items
    .map(
      (item) => `
        <tr>
          <td>${escapeHtml(item.name)}</td>
          <td>${escapeHtml(item.path)}</td>
          <td>${formatSize(item.size_bytes)}</td>
          <td>${escapeHtml(item.risk_level)}</td>
          <td>${escapeHtml(item.category)}</td>
          <td>${item.safe_to_delete ? "Yes" : "No"}</td>
          <td>${escapeHtml(item.explanation)}</td>
        </tr>`
    )
    .join("");

  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>DiskPulse Cleanup Report</title>
<style>
  body { font-family: Segoe UI, Arial, sans-serif; margin: 24px; color: #111827; background: #f8fafc; }
  h1, h2 { margin: 0 0 12px; }
  .meta { color: #475569; margin-bottom: 24px; }
  .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; margin: 16px 0 24px; }
  .card { background: white; border: 1px solid #e2e8f0; border-radius: 12px; padding: 16px; }
  .card b { display: block; margin-top: 6px; font-size: 18px; }
  table { width: 100%; border-collapse: collapse; background: white; }
  th, td { border: 1px solid #e2e8f0; padding: 10px; vertical-align: top; text-align: left; }
  th { background: #f1f5f9; }
  .small { font-size: 12px; color: #64748b; }
</style>
</head>
<body>
  <h1>DiskPulse Cleanup Report</h1>
  <div class="meta">Drive ${escapeHtml(report.drive_letter)}: · Generated from ${report.summary.total_items} classified items</div>

  <div class="grid">
    <div class="card"><div class="small">Low risk</div><b>${report.summary.low_risk_count} / ${formatSize(report.summary.low_risk_bytes)}</b></div>
    <div class="card"><div class="small">Medium risk</div><b>${report.summary.medium_risk_count} / ${formatSize(report.summary.medium_risk_bytes)}</b></div>
    <div class="card"><div class="small">High risk</div><b>${report.summary.high_risk_count} / ${formatSize(report.summary.high_risk_bytes)}</b></div>
  </div>

  <p class="small">Safe to delete: ${formatSize(report.summary.safe_deletable_bytes)}</p>
  <table>
    <thead>
      <tr>
        <th>Name</th><th>Path</th><th>Size</th><th>Risk</th><th>Category</th><th>Safe</th><th>Explanation</th>
      </tr>
    </thead>
    <tbody>
      ${rows}
    </tbody>
  </table>
</body>
</html>`;
}

async function downloadText(filename: string, content: string, mime: string) {
  const blob = new Blob([content], { type: mime });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  document.body.appendChild(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

function RiskCard({ item }: { item: RiskItem }) {
  const style = RISK_STYLES[item.risk_level];

  return (
    <article className="glass-card p-4 border border-aurora-border/50 rounded-2xl">
      <div className="flex items-start justify-between gap-4">
        <div className="min-w-0">
          <h4 className="text-sm font-semibold text-text-primary truncate">{item.name}</h4>
          <p className="text-xs text-text-muted mt-1 break-all">{item.path}</p>
        </div>
        <span className={`shrink-0 px-2.5 py-1 rounded-full text-xs font-medium border ${riskBadgeClass(item.risk_level)}`}>
          {riskLabel(item.risk_level)}
        </span>
      </div>

      <div className="grid grid-cols-3 gap-3 mt-4 text-xs">
        <div className="rounded-lg bg-aurora-elevated/60 p-2">
          <div className="text-text-muted">Size</div>
          <div className="text-text-primary font-mono mt-1">{formatSize(item.size_bytes)}</div>
        </div>
        <div className="rounded-lg bg-aurora-elevated/60 p-2">
          <div className="text-text-muted">Files</div>
          <div className="text-text-primary font-mono mt-1">{item.file_count.toLocaleString()}</div>
        </div>
        <div className="rounded-lg bg-aurora-elevated/60 p-2">
          <div className="text-text-muted">Subdirs</div>
          <div className="text-text-primary font-mono mt-1">{item.dir_count.toLocaleString()}</div>
        </div>
      </div>

      <p className="text-sm text-text-secondary mt-4 leading-6">{item.explanation}</p>

      <div className="mt-4 flex items-center justify-between text-xs text-text-muted">
        <span>Category: {item.category}</span>
        <span className={style.accent}>{item.safe_to_delete ? "Safe cleanup candidate" : "Review required"}</span>
      </div>
    </article>
  );
}

function GroupPanel({ level, items }: { level: RiskLevel; items: RiskItem[] }) {
  const style = RISK_STYLES[level];
  const totalBytes = items.reduce((sum, item) => sum + item.size_bytes, 0);

  return (
    <section className={`rounded-3xl border p-5 ${style.panel}`}>
      <div className="flex flex-wrap items-start justify-between gap-3 mb-4">
        <div>
          <div className={`text-sm font-semibold ${style.accent}`}>{style.title}</div>
          <p className="text-xs text-text-muted mt-1">
            {items.length} item{items.length === 1 ? "" : "s"} · {formatSize(totalBytes)}
          </p>
        </div>
        <span className={`px-2.5 py-1 rounded-full text-xs font-medium border ${style.badge}`}>
          {items.length}
        </span>
      </div>

      <div className="space-y-3">
        {items.map((item) => (
          <RiskCard key={item.path} item={item} />
        ))}
      </div>
    </section>
  );
}

export default function CleanupPage({ report }: { report: RiskReport | null }) {
  const [query, setQuery] = useState("");
  const [sortKey, setSortKey] = useState<SortKey>("risk");
  const [filter, setFilter] = useState<RiskFilter>("all");
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  const filteredItems = useMemo(() => {
    const items = report?.items ?? [];
    const q = query.trim().toLowerCase();
    const matched = q
      ? items.filter((item) =>
          [item.name, item.path, item.category, item.explanation].some((value) =>
            value.toLowerCase().includes(q)
          )
        )
      : items;

    const filtered = filter === "all" ? matched : matched.filter((item) => item.risk_level === filter);

    return [...filtered].sort((a, b) => {
      if (sortKey === "size") return b.size_bytes - a.size_bytes;
      if (sortKey === "name") return a.name.localeCompare(b.name);
      return RISK_ORDER[a.risk_level] - RISK_ORDER[b.risk_level] || b.size_bytes - a.size_bytes;
    });
  }, [query, report, sortKey, filter]);

  const groupedItems = useMemo(
    () => ({
      high: filteredItems.filter((item) => item.risk_level === "high"),
      medium: filteredItems.filter((item) => item.risk_level === "medium"),
      low: filteredItems.filter((item) => item.risk_level === "low"),
    }),
    [filteredItems]
  );

  async function handleExportHtml() {
    if (!report) return;
    await downloadText(`diskpulse-cleanup-report-${report.drive_letter}.html`, buildHtml(report, filteredItems), "text/html;charset=utf-8");
    setStatusMessage(`Exported HTML report for ${report.drive_letter}: drive.`);
  }

  async function handleExportCsv() {
    if (!report) return;
    await downloadText(`diskpulse-cleanup-report-${report.drive_letter}.csv`, buildCsv(report, filteredItems), "text/csv;charset=utf-8");
    setStatusMessage(`Exported CSV report for ${report.drive_letter}: drive.`);
  }

  if (!report) {
    return (
      <div className="h-full flex items-center justify-center px-8">
        <div className="glass-card max-w-2xl w-full p-8 text-center border border-aurora-border/50 rounded-3xl">
          <div className="w-16 h-16 mx-auto rounded-2xl bg-aurora-elevated flex items-center justify-center text-text-muted text-2xl">🧭</div>
          <h2 className="mt-5 text-xl font-semibold text-text-primary">Cleanup Report</h2>
          <p className="mt-2 text-sm text-text-secondary leading-6">
            Risk classification output will appear here after a drive scan. This page is prepared for v0.0.5 and will display low, medium, and high risk items in a review-first layout.
          </p>
          <div className="mt-6 grid grid-cols-3 gap-3 text-left">
            <div className="rounded-xl bg-aurora-elevated/60 p-4">
              <div className="text-xs text-text-muted">Low risk</div>
              <div className="text-sm text-text-primary mt-1">Safe cleanup candidates</div>
            </div>
            <div className="rounded-xl bg-aurora-elevated/60 p-4">
              <div className="text-xs text-text-muted">Medium risk</div>
              <div className="text-sm text-text-primary mt-1">Requires confirmation</div>
            </div>
            <div className="rounded-xl bg-aurora-elevated/60 p-4">
              <div className="text-xs text-text-muted">High risk</div>
              <div className="text-sm text-text-primary mt-1">Display only, never auto-delete</div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="p-8 space-y-6">
      <section className="glass-card p-6 rounded-3xl border border-aurora-border/50">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div>
            <h2 className="text-xl font-semibold text-text-primary">Cleanup Report</h2>
            <p className="text-sm text-text-secondary mt-2">
              Drive {report.drive_letter}: — reviewed by risk level, with safe cleanup guidance.
            </p>
          </div>
          <div className="flex flex-wrap gap-3 text-sm">
            <div className="rounded-xl bg-aurora-elevated/60 px-4 py-3 min-w-28">
              <div className="text-text-muted text-xs">Items</div>
              <div className="text-text-primary font-semibold mt-1">{report.summary.total_items}</div>
            </div>
            <div className="rounded-xl bg-aurora-elevated/60 px-4 py-3 min-w-28">
              <div className="text-text-muted text-xs">Safe to delete</div>
              <div className="text-success font-semibold mt-1">{formatSize(report.summary.safe_deletable_bytes)}</div>
            </div>
            <div className="rounded-xl bg-aurora-elevated/60 px-4 py-3 min-w-28">
              <div className="text-text-muted text-xs">Filtered items</div>
              <div className="text-accent-light font-semibold mt-1">{filteredItems.length}</div>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-3 gap-3 mt-6">
          <SummaryTile label="Low risk" value={`${report.summary.low_risk_count} / ${formatSize(report.summary.low_risk_bytes)}`} tone="success" />
          <SummaryTile label="Medium risk" value={`${report.summary.medium_risk_count} / ${formatSize(report.summary.medium_risk_bytes)}`} tone="warning" />
          <SummaryTile label="High risk" value={`${report.summary.high_risk_count} / ${formatSize(report.summary.high_risk_bytes)}`} tone="danger" />
        </div>
      </section>

      <section className="glass-card p-4 rounded-3xl border border-aurora-border/50 flex flex-wrap items-center gap-3">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search name, path, category, or note"
          className="flex-1 min-w-64 rounded-xl bg-aurora-elevated/70 border border-aurora-border/60 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60"
        />
        <select
          value={sortKey}
          onChange={(e) => setSortKey(e.target.value as SortKey)}
          className="rounded-xl bg-aurora-elevated/70 border border-aurora-border/60 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60"
        >
          <option value="risk">Sort by risk</option>
          <option value="size">Sort by size</option>
          <option value="name">Sort by name</option>
        </select>
        <div className="flex flex-wrap gap-2">
          <button className={`px-3 py-2 rounded-xl text-sm border ${filter === "all" ? "bg-accent/15 border-accent/30 text-accent-light" : "bg-aurora-elevated/70 border-aurora-border/60 text-text-secondary"}`} onClick={() => setFilter("all")}>All</button>
          <button className={`px-3 py-2 rounded-xl text-sm border ${filter === "low" ? "bg-success/15 border-success/30 text-success" : "bg-aurora-elevated/70 border-aurora-border/60 text-text-secondary"}`} onClick={() => setFilter("low")}>Low</button>
          <button className={`px-3 py-2 rounded-xl text-sm border ${filter === "medium" ? "bg-warning/15 border-warning/30 text-warning" : "bg-aurora-elevated/70 border-aurora-border/60 text-text-secondary"}`} onClick={() => setFilter("medium")}>Medium</button>
          <button className={`px-3 py-2 rounded-xl text-sm border ${filter === "high" ? "bg-danger/15 border-danger/30 text-danger" : "bg-aurora-elevated/70 border-aurora-border/60 text-text-secondary"}`} onClick={() => setFilter("high")}>High</button>
        </div>
        <div className="ml-auto flex flex-wrap gap-2">
          <button
            onClick={handleExportHtml}
            className="px-3 py-2 rounded-xl text-sm border bg-aurora-elevated/70 border-aurora-border/60 text-text-primary hover:border-accent/40 hover:text-accent-light"
          >
            Export HTML
          </button>
          <button
            onClick={handleExportCsv}
            className="px-3 py-2 rounded-xl text-sm border bg-aurora-elevated/70 border-aurora-border/60 text-text-primary hover:border-accent/40 hover:text-accent-light"
          >
            Export CSV
          </button>
        </div>
      </section>

      <section className="grid grid-cols-1 xl:grid-cols-3 gap-4">
        <GroupPanel level="high" items={groupedItems.high} />
        <GroupPanel level="medium" items={groupedItems.medium} />
        <GroupPanel level="low" items={groupedItems.low} />
      </section>

      {statusMessage && (
        <div className="glass-card p-4 rounded-2xl border border-success/20 bg-risk-low-bg/15 text-sm text-success">
          {statusMessage}
        </div>
      )}

      {filteredItems.length === 0 && (
        <div className="glass-card p-8 rounded-3xl text-center text-text-muted border border-aurora-border/50">
          No matching items in the current report.
        </div>
      )}
    </div>
  );
}

function SummaryTile({ label, value, tone }: { label: string; value: string; tone: "success" | "warning" | "danger" }) {
  const toneClass =
    tone === "success" ? "text-success" : tone === "warning" ? "text-warning" : "text-danger";

  return (
    <div className="rounded-2xl bg-aurora-elevated/60 p-4 border border-aurora-border/40">
      <div className="text-xs text-text-muted">{label}</div>
      <div className={`text-sm font-semibold mt-2 ${toneClass}`}>{value}</div>
    </div>
  );
}
