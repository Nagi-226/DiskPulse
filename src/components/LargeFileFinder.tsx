import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { useLargeFileFinder } from "../hooks/useLargeFileFinder";
import type { CleanItem, FileEntry } from "../types";
import { formatSize } from "../utils/format";

type SortKey = "size" | "modified" | "name" | "path";

const MIN_SIZE_OPTIONS = [
  { label: "100 MB", value: 100 * 1024 * 1024 },
  { label: "500 MB", value: 500 * 1024 * 1024 },
  { label: "1 GB", value: 1024 * 1024 * 1024 },
  { label: "5 GB", value: 5 * 1024 * 1024 * 1024 },
];

const LIMIT_OPTIONS = [25, 50, 100, 250];

function fileAge(modifiedEpochMs: number) {
  if (modifiedEpochMs <= 0) return "Unknown";
  const days = Math.max(0, Math.floor((Date.now() - modifiedEpochMs) / 86_400_000));
  if (days === 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 30) return `${days} days`;
  if (days < 365) return `${Math.floor(days / 30)} months`;
  return `${Math.floor(days / 365)} years`;
}

function cleanupReadiness(path: string) {
  const normalized = `/${path.replace(/\\/g, "/").toLowerCase()}/`;
  if (
    normalized.includes("/windows/") ||
    normalized.includes("/program files/") ||
    normalized.includes("/program files (x86)/") ||
    normalized.includes("/system volume information/") ||
    normalized.includes("/$recycle.bin/")
  ) {
    return { label: "Protected", tone: "danger" as const };
  }
  if (
    normalized.includes("/downloads/") ||
    normalized.includes("/temp/") ||
    normalized.includes("/tmp/") ||
    normalized.includes("/cache/") ||
    normalized.includes("/logs/")
  ) {
    return { label: "Preview ready", tone: "success" as const };
  }
  return { label: "Review first", tone: "warning" as const };
}

function toCleanItems(files: FileEntry[]): CleanItem[] {
  return files.map((file) => ({
    name: file.name,
    path: file.path,
    size_bytes: file.size_bytes,
    risk_level: "medium",
    safe_to_delete: true,
  }));
}

export default function LargeFileFinder({
  drives,
  selectedDrive,
  onAddToCleanup,
}: {
  drives: string[];
  selectedDrive: string;
  onAddToCleanup: (items: CleanItem[]) => void;
}) {
  const [drive, setDrive] = useState(selectedDrive);
  const [minSize, setMinSize] = useState(MIN_SIZE_OPTIONS[1].value);
  const [limit, setLimit] = useState(50);
  const [sortKey, setSortKey] = useState<SortKey>("size");
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const { files, progress, loading, error, setError, scan, cancel } = useLargeFileFinder();

  useEffect(() => {
    if (!loading) {
      setDrive(selectedDrive);
    }
  }, [loading, selectedDrive]);

  const sortedFiles = useMemo(() => {
    return [...files].sort((a, b) => {
      if (sortKey === "modified") return b.modified_epoch_ms - a.modified_epoch_ms;
      if (sortKey === "name") return a.name.localeCompare(b.name);
      if (sortKey === "path") return a.path.localeCompare(b.path);
      return b.size_bytes - a.size_bytes;
    });
  }, [files, sortKey]);

  const selectedFiles = useMemo(
    () => sortedFiles.filter((file) => selectedPaths.has(file.path)),
    [selectedPaths, sortedFiles],
  );

  const totalSelectedBytes = selectedFiles.reduce((sum, file) => sum + file.size_bytes, 0);
  const totalFoundBytes = files.reduce((sum, file) => sum + file.size_bytes, 0);
  const progressPct =
    progress && progress.dirs_total > 0
      ? Math.min(100, (progress.dirs_processed / progress.dirs_total) * 100)
      : loading
        ? 4
        : 0;

  async function handleScan() {
    setSelectedPaths(new Set());
    await scan({ drive, minSize, limit });
  }

  function toggleSelection(path: string) {
    setSelectedPaths((current) => {
      const next = new Set(current);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return next;
    });
  }

  function toggleAllVisible() {
    setSelectedPaths((current) => {
      if (sortedFiles.length > 0 && sortedFiles.every((file) => current.has(file.path))) {
        return new Set();
      }
      return new Set(sortedFiles.map((file) => file.path));
    });
  }

  function handleAddToCleanup() {
    onAddToCleanup(toCleanItems(selectedFiles));
  }

  return (
    <div className="p-8 space-y-6">
      <section className="relative overflow-hidden rounded-[28px] border border-aurora-border/60 bg-aurora-surface/70 p-6">
        <div
          className="absolute inset-0 pointer-events-none opacity-70"
          style={{
            background:
              "radial-gradient(circle at 12% 20%, rgba(6, 182, 212, 0.16), transparent 32%), radial-gradient(circle at 82% 12%, rgba(99, 102, 241, 0.2), transparent 28%)",
          }}
        />
        <div className="relative grid gap-6 xl:grid-cols-[1fr_360px]">
          <div>
            <div className="text-xs font-semibold uppercase tracking-[0.24em] text-accent-light">
              Large File Finder
            </div>
            <h2 className="mt-3 max-w-3xl text-3xl font-bold tracking-tight text-text-primary">
              Find oversized files before they become cleanup decisions.
            </h2>
            <p className="mt-3 max-w-2xl text-sm leading-6 text-text-secondary">
              Scan individual files by size, review age and location, then send selected candidates through the same safety preview used by cleanup reports.
            </p>

            <div className="mt-6 flex flex-wrap items-end gap-3">
              <Field label="Drive">
                <select
                  value={drive}
                  onChange={(event) => setDrive(event.target.value)}
                  disabled={loading}
                  className="h-10 min-w-28 rounded-xl border border-aurora-border/60 bg-aurora-elevated/80 px-3 text-sm text-text-primary outline-none focus:border-accent/60"
                >
                  {(drives.length > 0 ? drives : [selectedDrive]).map((item) => (
                    <option key={item} value={item}>
                      {item}: Drive
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="Minimum size">
                <select
                  value={minSize}
                  onChange={(event) => setMinSize(Number(event.target.value))}
                  disabled={loading}
                  className="h-10 min-w-32 rounded-xl border border-aurora-border/60 bg-aurora-elevated/80 px-3 text-sm text-text-primary outline-none focus:border-accent/60"
                >
                  {MIN_SIZE_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="Limit">
                <select
                  value={limit}
                  onChange={(event) => setLimit(Number(event.target.value))}
                  disabled={loading}
                  className="h-10 min-w-24 rounded-xl border border-aurora-border/60 bg-aurora-elevated/80 px-3 text-sm text-text-primary outline-none focus:border-accent/60"
                >
                  {LIMIT_OPTIONS.map((option) => (
                    <option key={option} value={option}>
                      Top {option}
                    </option>
                  ))}
                </select>
              </Field>
              <button className="btn-primary h-10" onClick={handleScan} disabled={loading}>
                <span>{loading ? "Scanning..." : `Scan ${drive}:`}</span>
              </button>
              {loading && (
                <button
                  className="h-10 rounded-xl border border-danger/30 bg-risk-high-bg px-4 text-sm font-semibold text-danger transition-colors hover:bg-risk-high-bg/80"
                  onClick={cancel}
                >
                  Stop
                </button>
              )}
            </div>
          </div>

          <div className="rounded-3xl border border-aurora-border/50 bg-aurora-bg/35 p-5">
            <div className="flex items-center justify-between">
              <span className="text-xs uppercase tracking-wider text-text-muted">Scan window</span>
              <span className="font-mono text-xs text-accent-light">
                {loading ? `${progress?.dirs_processed ?? 0}/${progress?.dirs_total ?? 0}` : `${files.length} files`}
              </span>
            </div>
            <div className="mt-4 h-2 overflow-hidden rounded-full bg-aurora-border/60">
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{
                  width: `${Math.max(progressPct, loading ? 4 : 0)}%`,
                  background: "linear-gradient(90deg, var(--color-cyan), var(--color-accent-light))",
                }}
              />
            </div>
            <div className="mt-5 grid grid-cols-2 gap-3">
              <MiniStat label="Found" value={files.length.toLocaleString()} />
              <MiniStat label="Visible bytes" value={formatSize(totalFoundBytes)} />
              <MiniStat label="Selected" value={selectedFiles.length.toLocaleString()} />
              <MiniStat label="Selected bytes" value={formatSize(totalSelectedBytes)} />
            </div>
            {progress?.current_path && (
              <p className="mt-4 truncate text-xs text-text-muted" title={progress.current_path}>
                {progress.current_path}
              </p>
            )}
          </div>
        </div>
      </section>

      {error && (
        <div className="rounded-2xl border border-danger/20 bg-risk-high-bg px-4 py-3 text-sm text-danger">
          {error}
          <button className="ml-3 text-text-muted hover:text-text-primary" onClick={() => setError(null)}>
            Dismiss
          </button>
        </div>
      )}

      <section className="glass-card overflow-hidden rounded-3xl border border-aurora-border/50">
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-aurora-border/50 px-5 py-4">
          <div>
            <h3 className="text-sm font-semibold uppercase tracking-wider text-text-primary">Large Files</h3>
            <p className="mt-1 text-xs text-text-muted">
              Select files to pass into cleanup safety validation. Protected and non-whitelisted paths remain blocked.
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <select
              value={sortKey}
              onChange={(event) => setSortKey(event.target.value as SortKey)}
              className="h-9 rounded-xl border border-aurora-border/60 bg-aurora-elevated/80 px-3 text-sm text-text-primary outline-none focus:border-accent/60"
            >
              <option value="size">Sort by size</option>
              <option value="modified">Sort by modified</option>
              <option value="name">Sort by name</option>
              <option value="path">Sort by path</option>
            </select>
            <button
              className="h-9 rounded-xl border border-aurora-border/60 bg-aurora-elevated/70 px-3 text-sm text-text-primary transition-colors hover:border-accent/40 hover:text-accent-light"
              onClick={toggleAllVisible}
              disabled={sortedFiles.length === 0}
            >
              {sortedFiles.length > 0 && sortedFiles.every((file) => selectedPaths.has(file.path))
                ? "Clear"
                : "Select visible"}
            </button>
            <button className="btn-primary h-9" onClick={handleAddToCleanup} disabled={selectedFiles.length === 0}>
              <span>Add to Cleanup</span>
            </button>
          </div>
        </div>

        {sortedFiles.length === 0 ? (
          <div className="flex min-h-80 flex-col items-center justify-center px-8 py-16 text-center">
            <div className="rounded-3xl border border-aurora-border/60 bg-aurora-elevated/50 p-5 text-3xl">
              ⌕
            </div>
            <h3 className="mt-5 text-lg font-semibold text-text-primary">
              {loading ? "Scanning for large files" : "No large-file scan yet"}
            </h3>
            <p className="mt-2 max-w-md text-sm leading-6 text-text-secondary">
              {loading
                ? "Results appear after the backend finishes walking the selected drive."
                : "Choose a minimum size and scan a drive to build a ranked file-level view."}
            </p>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full min-w-[920px] text-left text-sm">
              <thead className="bg-aurora-elevated/45 text-xs uppercase tracking-wider text-text-muted">
                <tr>
                  <th className="w-12 px-5 py-3"></th>
                  <th className="px-3 py-3">File</th>
                  <th className="px-3 py-3">Size</th>
                  <th className="px-3 py-3">Modified</th>
                  <th className="px-3 py-3">Cleanup status</th>
                  <th className="px-5 py-3">Path</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-aurora-border/40">
                {sortedFiles.map((file) => {
                  const readiness = cleanupReadiness(file.path);
                  const selected = selectedPaths.has(file.path);
                  return (
                    <tr
                      key={file.path}
                      className={`transition-colors hover:bg-aurora-elevated/35 ${selected ? "bg-accent/5" : ""}`}
                    >
                      <td className="px-5 py-3">
                        <input
                          type="checkbox"
                          checked={selected}
                          onChange={() => toggleSelection(file.path)}
                          className="h-4 w-4 accent-[var(--color-accent)]"
                        />
                      </td>
                      <td className="max-w-56 px-3 py-3">
                        <div className="truncate font-medium text-text-primary" title={file.name}>
                          {file.name}
                        </div>
                      </td>
                      <td className="px-3 py-3 font-mono text-text-primary">
                        <div>{formatSize(file.size_bytes)}</div>
                        {file.size_on_disk_bytes != null && file.size_on_disk_bytes !== file.size_bytes && (
                          <div className="mt-1 text-[10px] text-warning">
                            {formatSize(file.size_on_disk_bytes)} on disk (sparse)
                          </div>
                        )}
                        {file.hard_link_count > 1 && (
                          <div className="mt-1 text-[10px] text-text-muted">Links: {file.hard_link_count}</div>
                        )}
                      </td>
                      <td className="px-3 py-3 text-text-secondary">{fileAge(file.modified_epoch_ms)}</td>
                      <td className="px-3 py-3">
                        <span
                          className={`rounded-full border px-2.5 py-1 text-xs font-medium ${
                            readiness.tone === "success"
                              ? "border-success/20 bg-risk-low-bg text-success"
                              : readiness.tone === "danger"
                                ? "border-danger/20 bg-risk-high-bg text-danger"
                                : "border-warning/20 bg-risk-medium-bg text-warning"
                          }`}
                        >
                          {readiness.label}
                        </span>
                      </td>
                      <td className="max-w-xl px-5 py-3">
                        <div className="truncate text-xs text-text-muted" title={file.path}>
                          {file.path}
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </section>
    </div>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <label className="space-y-1.5">
      <span className="block text-xs font-medium uppercase tracking-wider text-text-muted">{label}</span>
      {children}
    </label>
  );
}

function MiniStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-aurora-border/40 bg-aurora-elevated/50 p-3">
      <div className="text-[11px] uppercase tracking-wider text-text-muted">{label}</div>
      <div className="mt-1 font-mono text-sm font-semibold text-text-primary">{value}</div>
    </div>
  );
}
