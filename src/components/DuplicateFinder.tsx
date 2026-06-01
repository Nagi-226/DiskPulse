import { useMemo, useState } from "react";
import { useDuplicateScan } from "../hooks/useDuplicateScan";
import type { CleanItem, DuplicateGroup } from "../types";
import { formatSize } from "../utils/format";

interface DuplicateFinderProps {
  drives: string[];
  selectedDrive: string;
  onAddToCleanup: (items: CleanItem[]) => void;
}

const MIN_SIZE_OPTIONS = [1, 10, 100, 1024];

function cleanupItemsFromGroups(groups: DuplicateGroup[]): CleanItem[] {
  return groups.flatMap((group) => {
    const sorted = [...group.files].sort((a, b) => b.modified_epoch_ms - a.modified_epoch_ms);
    return sorted.slice(1).map((file) => ({
      name: file.name,
      path: file.path,
      size_bytes: file.size_bytes,
      risk_level: "medium" as const,
      safe_to_delete: false,
    }));
  });
}

export default function DuplicateFinder({ drives, selectedDrive, onAddToCleanup }: DuplicateFinderProps) {
  const [drive, setDrive] = useState(selectedDrive);
  const [minSizeMb, setMinSizeMb] = useState(10);
  const { groups, progress, loading, error, scan, cancel } = useDuplicateScan();

  const wastedBytes = useMemo(() => groups.reduce((sum, group) => sum + group.total_size_wasted, 0), [groups]);
  const cleanupItems = useMemo(() => cleanupItemsFromGroups(groups), [groups]);

  return (
    <div className="p-8 space-y-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <h2 className="text-sm font-semibold uppercase tracking-wider text-text-primary">Duplicate Files</h2>
          <p className="mt-1 text-xs text-text-muted">Three-phase detection: size, first 4KB hash, then full SHA-256 confirmation.</p>
        </div>
        <div className="flex flex-wrap items-center gap-3">
          <select value={drive} onChange={(e) => setDrive(e.target.value)} disabled={loading} className="rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60">
            {drives.map((item) => <option key={item} value={item}>{item}: Drive</option>)}
          </select>
          <select value={minSizeMb} onChange={(e) => setMinSizeMb(Number(e.target.value))} disabled={loading} className="rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-4 py-2.5 text-sm text-text-primary outline-none focus:border-accent/60">
            {MIN_SIZE_OPTIONS.map((mb) => <option key={mb} value={mb}>Min {mb} MB</option>)}
          </select>
          <button className="btn-primary" onClick={() => void scan(drive, minSizeMb * 1024 * 1024)} disabled={loading}><span>{loading ? "Scanning..." : "Scan Duplicates"}</span></button>
          {loading && <button className="rounded-xl border border-danger/30 bg-risk-high-bg px-4 py-2.5 text-sm font-semibold text-danger" onClick={() => void cancel()}>Cancel</button>}
        </div>
      </div>

      {progress && (
        <div className="glass-card p-4 text-sm text-text-secondary">
          <span className="font-semibold text-accent-light">{progress.phase}</span> ? {progress.files_processed} files processed ? {progress.groups_found} groups found
          {progress.current_path && <div className="mt-1 truncate text-xs text-text-muted">{progress.current_path}</div>}
        </div>
      )}
      {error && <div className="rounded-xl border border-red-500/20 bg-risk-high-bg/20 p-3 text-sm text-danger">{error}</div>}

      <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
        <SummaryTile label="Duplicate groups" value={String(groups.length)} />
        <SummaryTile label="Recoverable waste" value={formatSize(wastedBytes)} />
        <SummaryTile label="Cleanup candidates" value={String(cleanupItems.length)} />
      </div>

      {groups.length > 0 && (
        <div className="flex justify-end">
          <button className="rounded-xl border border-warning/25 bg-risk-medium-bg px-4 py-2.5 text-sm font-semibold text-warning transition-colors hover:bg-risk-medium-bg/80" onClick={() => onAddToCleanup(cleanupItems)}>
            Add duplicates to Cleanup Preview
          </button>
        </div>
      )}

      <div className="space-y-4">
        {groups.map((group) => (
          <div key={group.group_id} className="glass-card p-5">
            <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
              <div>
                <div className="text-sm font-semibold text-text-primary">Group {group.group_id}</div>
                <p className="mt-1 text-xs text-text-muted">{group.files.length} identical files ? {formatSize(group.total_size_wasted)} reclaimable if one copy is kept</p>
              </div>
            </div>
            <div className="space-y-2">
              {group.files.map((file) => (
                <div key={file.path} className="flex items-center gap-3 rounded-xl border border-aurora-border/30 bg-aurora-elevated/40 px-3 py-2 text-sm">
                  <span className="min-w-24 font-mono text-xs text-text-muted">{formatSize(file.size_bytes)}</span>
                  <span className="truncate text-text-secondary" title={file.path}>{file.path}</span>
                </div>
              ))}
            </div>
          </div>
        ))}
        {!loading && groups.length === 0 && <div className="glass-card p-16 text-center text-sm text-text-muted">No duplicate scan results yet.</div>}
      </div>
    </div>
  );
}

function SummaryTile({ label, value }: { label: string; value: string }) {
  return <div className="glass-card p-5"><div className="text-xs uppercase tracking-wider text-text-muted">{label}</div><div className="mt-2 text-2xl font-bold text-text-primary">{value}</div></div>;
}
