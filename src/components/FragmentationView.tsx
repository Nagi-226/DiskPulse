import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { FragmentationReport } from "../types";
import { formatSize } from "../utils/format";

export default function FragmentationView({ selectedDrive }: { selectedDrive: string }) {
  const [report, setReport] = useState<FragmentationReport | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void run();
  }, [selectedDrive]);

  async function run() {
    setLoading(true);
    setError(null);
    try {
      setReport(await invoke<FragmentationReport>("analyze_fragmentation", { drive: selectedDrive }));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="p-8 space-y-6">
      <div className="glass-card p-6 rounded-2xl">
        <div className="flex items-center justify-between gap-4">
          <div>
            <h2 className="text-xl font-bold text-text-primary">Fragmentation Analysis</h2>
            <p className="mt-1 text-sm text-text-muted">
              Sampled extent health for {selectedDrive}: Drive. Analysis only; DiskPulse never defrags.
            </p>
          </div>
          <button className="btn-primary px-4 py-2" onClick={() => void run()} disabled={loading}>
            {loading ? "Analyzing..." : "Analyze"}
          </button>
        </div>
      </div>

      {error && <div className="rounded-xl border border-danger/30 bg-risk-high-bg p-4 text-sm text-danger">{error}</div>}

      {report && (
        <>
          <div className="grid grid-cols-4 gap-4">
            <Metric label="Files analyzed" value={report.files_analyzed.toLocaleString()} />
            <Metric label="Avg fragmentation" value={`${(report.average_fragmentation * 100).toFixed(1)}%`} />
            <Metric label="High-frag files" value={report.high_fragmentation_files.toLocaleString()} />
            <Metric label="Sampling" value={report.sampled ? "Limited" : "Full"} />
          </div>

          <div className="glass-card rounded-2xl p-6">
            <h3 className="mb-4 text-sm font-semibold uppercase tracking-wider text-text-primary">Top Fragmented Directories</h3>
            <div className="space-y-2">
              {report.top_dirs.map((dir) => (
                <div key={dir.path} className="rounded-xl border border-aurora-border/40 bg-aurora-elevated/40 p-4">
                  <div className="flex items-center justify-between gap-4">
                    <div className="min-w-0">
                      <p className="truncate text-sm font-semibold text-text-primary">{dir.path}</p>
                      <p className="text-xs text-text-muted">
                        {dir.files_analyzed} files sampled · {formatSize(dir.total_bytes)}
                      </p>
                    </div>
                    <span className="font-mono text-sm text-warning">
                      {(dir.average_fragmentation * 100).toFixed(1)}%
                    </span>
                  </div>
                </div>
              ))}
              {report.top_dirs.length === 0 && <p className="text-sm text-text-muted">No file samples found.</p>}
            </div>
          </div>
        </>
      )}
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="glass-card rounded-2xl p-4">
      <p className="text-xs uppercase tracking-wider text-text-muted">{label}</p>
      <p className="mt-2 font-mono text-2xl font-bold text-text-primary">{value}</p>
    </div>
  );
}
