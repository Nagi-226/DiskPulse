import { useMemo, useState } from "react";
import { useAgingAnalysis } from "../hooks/useAgingAnalysis";
import type { AgeBucket, CleanItem } from "../types";
import { formatSize } from "../utils/format";

interface AgingAnalysisProps {
  drives: string[];
  selectedDrive: string;
  onAddToCleanup: (items: CleanItem[]) => void;
}

function bucketWidth(bucket: AgeBucket, maxBytes: number) {
  if (maxBytes <= 0) return 0;
  return Math.max((bucket.total_bytes / maxBytes) * 100, bucket.total_bytes > 0 ? 2 : 0);
}

export default function AgingAnalysis({ drives, selectedDrive, onAddToCleanup }: AgingAnalysisProps) {
  const [drive, setDrive] = useState(selectedDrive);
  const { report, progress, loading, error, analyze, cancel } = useAgingAnalysis();

  const maxBucketBytes = useMemo(() => Math.max(...(report?.buckets.map((bucket) => bucket.total_bytes) ?? [0])), [report]);
  const zombieItems = useMemo<CleanItem[]>(() => (report?.zombie_files ?? []).map((file) => ({
    name: file.name,
    path: file.path,
    size_bytes: file.size_bytes,
    risk_level: "medium",
    safe_to_delete: false,
  })), [report]);

  return (
    <div className="p-8 space-y-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-wider text-text-primary">File Aging Analysis</h2>
          <p className="mt-1 text-xs text-text-muted">Map modification-age buckets, zombie files, and recent growth hotspots.</p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <select value={drive} onChange={(e) => setDrive(e.target.value)} disabled={loading} className="rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60">
            {drives.map((item) => <option key={item} value={item}>{item}: Drive</option>)}
          </select>
          <button className="btn-primary" onClick={() => void analyze(drive)} disabled={loading}><span>{loading ? "Analyzing..." : "Analyze Aging"}</span></button>
          {loading && <button className="rounded-xl border border-danger/30 bg-risk-high-bg px-4 py-2.5 text-sm font-semibold text-danger" onClick={() => void cancel()}>Cancel</button>}
        </div>
      </div>

      {progress && (
        <div className="glass-card p-4 text-sm text-text-secondary">
          <span className="font-semibold text-accent-light">{progress.files_processed}</span> files processed
          {progress.current_path && <div className="mt-1 truncate text-xs text-text-muted">{progress.current_path}</div>}
        </div>
      )}
      {error && <div className="rounded-xl border border-red-500/20 bg-risk-high-bg/20 p-3 text-sm text-danger">{error}</div>}

      {report ? (
        <>
          <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
            <SummaryTile label="Zombie size" value={formatSize(report.zombies_total_size)} />
            <SummaryTile label="Zombie files" value={String(report.zombie_files.length)} />
            <SummaryTile label="Hotspots" value={String(report.hotspots.length)} />
          </div>

          <div className="glass-card p-6">
            <h3 className="mb-5 text-sm font-semibold uppercase tracking-wider text-text-primary">Age Distribution</h3>
            <div className="space-y-3">
              {report.buckets.map((bucket) => (
                <div key={bucket.id} className="grid grid-cols-[80px_1fr_140px] items-center gap-3 text-sm">
                  <span className="text-text-secondary">{bucket.label}</span>
                  <div className="h-8 overflow-hidden rounded-xl bg-aurora-border/40">
                    <div className="h-full rounded-xl bg-gradient-to-r from-accent to-cyan transition-all" style={{ width: `${bucketWidth(bucket, maxBucketBytes)}%` }} />
                  </div>
                  <span className="text-right font-mono text-xs text-text-muted">{formatSize(bucket.total_bytes)} ? {bucket.file_count}</span>
                </div>
              ))}
            </div>
          </div>

          <div className="grid grid-cols-1 gap-6 xl:grid-cols-2">
            <div className="glass-card p-6">
              <div className="mb-4 flex items-center justify-between gap-3">
                <h3 className="text-sm font-semibold uppercase tracking-wider text-text-primary">Zombie Files</h3>
                {zombieItems.length > 0 && <button className="rounded-lg border border-warning/25 bg-risk-medium-bg px-3 py-2 text-xs font-semibold text-warning" onClick={() => onAddToCleanup(zombieItems)}>Add to Cleanup</button>}
              </div>
              <div className="space-y-2 max-h-96 overflow-y-auto">
                {report.zombie_files.slice(0, 50).map((file) => (
                  <div key={file.path} className="rounded-xl border border-aurora-border/30 bg-aurora-elevated/40 px-3 py-2 text-sm">
                    <div className="truncate text-text-secondary" title={file.path}>{file.path}</div>
                    <div className="mt-1 font-mono text-xs text-text-muted">{formatSize(file.size_bytes)}</div>
                  </div>
                ))}
                {report.zombie_files.length === 0 && <div className="py-12 text-center text-sm text-text-muted">No zombie files detected.</div>}
              </div>
            </div>

            <div className="glass-card p-6">
              <h3 className="mb-4 text-sm font-semibold uppercase tracking-wider text-text-primary">Growth Hotspots</h3>
              <div className="space-y-2 max-h-96 overflow-y-auto">
                {report.hotspots.slice(0, 30).map((hotspot) => (
                  <div key={hotspot.path} className="rounded-xl border border-aurora-border/30 bg-aurora-elevated/40 px-3 py-2 text-sm">
                    <div className="truncate text-text-secondary" title={hotspot.path}>{hotspot.path}</div>
                    <div className="mt-1 font-mono text-xs text-text-muted">{formatSize(hotspot.recent_bytes)} ? {hotspot.file_count} recent files</div>
                  </div>
                ))}
                {report.hotspots.length === 0 && <div className="py-12 text-center text-sm text-text-muted">No recent hotspots detected.</div>}
              </div>
            </div>
          </div>
        </>
      ) : !loading ? (
        <div className="glass-card p-16 text-center text-sm text-text-muted">Run an aging analysis to see bucket distribution and zombie candidates.</div>
      ) : null}
    </div>
  );
}

function SummaryTile({ label, value }: { label: string; value: string }) {
  return <div className="glass-card p-5"><div className="text-xs uppercase tracking-wider text-text-muted">{label}</div><div className="mt-2 text-2xl font-bold text-text-primary">{value}</div></div>;
}
