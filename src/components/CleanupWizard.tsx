import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useDriveScan } from "../hooks/useDriveScan";
import type { CleanItem, CleanProgress, CleanResult, DiskHealth, Recommendation, RiskLevel } from "../types";
import { formatSize } from "../utils/format";

const STEPS = [
  "Select drive",
  "Scan",
  "Review results",
  "Confirm cleanup",
  "Execution summary",
];

function toRiskLevel(value: string): RiskLevel {
  return value === "low" || value === "high" ? value : "medium";
}

function toCleanItem(recommendation: Recommendation): CleanItem {
  return {
    name: recommendation.item.name,
    path: recommendation.item.path,
    size_bytes: recommendation.estimated_size || recommendation.item.size_bytes,
    risk_level: toRiskLevel(recommendation.item.risk_level),
    safe_to_delete: recommendation.item.safe_to_delete,
  };
}

export default function CleanupWizard({ selectedDrive, onStartScan }: { selectedDrive: string; onStartScan: (drive: string) => void }) {
  const [step, setStep] = useState(0);
  const [wizardDrive, setWizardDrive] = useState(selectedDrive);
  const [drives, setDrives] = useState<string[]>([selectedDrive]);
  const [recommendations, setRecommendations] = useState<Recommendation[]>([]);
  const [health, setHealth] = useState<DiskHealth | null>(null);
  const [reviewLoading, setReviewLoading] = useState(false);
  const [reviewError, setReviewError] = useState<string | null>(null);
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [cleanResult, setCleanResult] = useState<CleanResult | null>(null);
  const [cleanProgress, setCleanProgress] = useState<CleanProgress | null>(null);
  const [executing, setExecuting] = useState(false);
  const [executeError, setExecuteError] = useState<string | null>(null);

  const scan = useDriveScan(selectedDrive);

  useEffect(() => {
    setWizardDrive(selectedDrive);
  }, [selectedDrive]);

  useEffect(() => {
    invoke<string[]>("list_drives").then(setDrives).catch(() => setDrives([selectedDrive]));
  }, [selectedDrive]);

  useEffect(() => {
    const unlisten = listen<CleanProgress>("clean-progress", (event) => setCleanProgress(event.payload));
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const safeCandidates = useMemo(() => {
    const fromRisk = (scan.riskReport?.items ?? [])
      .filter((item) => item.risk_level === "low" && item.safe_to_delete)
      .map<CleanItem>((item) => ({
        name: item.name,
        path: item.path,
        size_bytes: item.size_bytes,
        risk_level: item.risk_level,
        safe_to_delete: item.safe_to_delete,
      }));
    const fromRecommendations = recommendations.filter((item) => item.item.safe_to_delete).map(toCleanItem);
    const byPath = new Map<string, CleanItem>();
    for (const item of [...fromRisk, ...fromRecommendations]) byPath.set(item.path, item);
    return Array.from(byPath.values()).sort((a, b) => b.size_bytes - a.size_bytes);
  }, [recommendations, scan.riskReport]);

  const selectedItems = useMemo(() => safeCandidates.filter((item) => selectedPaths.has(item.path)), [safeCandidates, selectedPaths]);
  const selectedBytes = selectedItems.reduce((sum, item) => sum + item.size_bytes, 0);

  async function runScan() {
    setStep(1);
    onStartScan(wizardDrive);
    const result = await scan.scanDrive(wizardDrive);
    if (result) {
      await loadReview();
      setStep(2);
    }
  }

  async function loadReview() {
    setReviewLoading(true);
    setReviewError(null);
    try {
      const [nextRecommendations, nextHealth] = await Promise.all([
        invoke<Recommendation[]>("get_recommendations", { drive: wizardDrive }),
        invoke<DiskHealth>("get_disk_health", { drive: wizardDrive }),
      ]);
      setRecommendations(nextRecommendations);
      setHealth(nextHealth);
      const nextSafe = nextRecommendations.filter((item) => item.item.safe_to_delete).map((item) => item.item.path);
      setSelectedPaths(new Set(nextSafe));
    } catch (e) {
      setReviewError(String(e));
    } finally {
      setReviewLoading(false);
    }
  }

  async function executeCleanup() {
    setExecuting(true);
    setExecuteError(null);
    setCleanResult(null);
    setStep(4);
    try {
      const result = await invoke<CleanResult>("clean_items", { items: selectedItems });
      setCleanResult(result);
    } catch (e) {
      setExecuteError(String(e));
    } finally {
      setExecuting(false);
    }
  }

  function togglePath(path: string) {
    setSelectedPaths((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }

  return (
    <div className="p-8 space-y-6">
      <div>
        <h2 className="text-sm font-semibold uppercase tracking-wider text-text-primary">Cleanup Wizard</h2>
        <p className="mt-1 text-xs text-text-muted">Guided safe cleanup: select, scan, review, confirm, then move eligible files to Recycle Bin.</p>
      </div>

      <div className="glass-card p-6">
        <div className="grid grid-cols-1 gap-3 md:grid-cols-5">
          {STEPS.map((label, index) => (
            <button key={label} className={`rounded-2xl border p-4 text-left transition-colors ${index === step ? "border-accent/40 bg-accent/15 text-accent-light" : "border-aurora-border/40 bg-aurora-elevated/50 text-text-secondary"}`} onClick={() => setStep(index)}>
              <div className="font-mono text-xs">Step {index + 1}</div>
              <div className="mt-2 text-sm font-semibold">{label}</div>
            </button>
          ))}
        </div>

        <div className="mt-8 rounded-2xl border border-aurora-border/40 bg-aurora-elevated/40 p-6">
          {step === 0 && (
            <div className="space-y-5">
              <h3 className="text-lg font-semibold text-text-primary">Select drive</h3>
              <select value={wizardDrive} onChange={(e) => setWizardDrive(e.target.value)} className="rounded-xl border border-aurora-border/60 bg-aurora-elevated px-4 py-2 text-sm text-text-primary">
                {drives.map((drive) => <option key={drive} value={drive}>{drive}: Drive</option>)}
              </select>
              <div><button className="btn-primary" onClick={() => void runScan()}><span>Scan {wizardDrive}: Drive</span></button></div>
            </div>
          )}

          {step === 1 && (
            <div className="space-y-5">
              <h3 className="text-lg font-semibold text-text-primary">Scanning {wizardDrive}: Drive</h3>
              <div className="h-3 overflow-hidden rounded-full bg-aurora-border/50">
                <div className="h-full rounded-full bg-accent transition-all" style={{ width: `${scan.progress && scan.progress.total > 0 ? Math.min(100, (scan.progress.processed / scan.progress.total) * 100) : 15}%` }} />
              </div>
              <p className="text-sm text-text-secondary">Phase: {scan.progress?.phase ?? "starting"} | {scan.progress?.processed ?? 0}/{scan.progress?.total ?? 0}</p>
              {scan.error && <div className="rounded-xl border border-red-500/20 bg-risk-high-bg/20 p-3 text-sm text-danger">{scan.error}</div>}
              <button className="rounded-xl border border-aurora-border/60 px-4 py-2 text-sm text-text-secondary" onClick={() => void scan.cancelScan()}>Cancel scan</button>
            </div>
          )}

          {step === 2 && (
            <div className="space-y-5">
              <div className="flex items-center justify-between gap-4">
                <h3 className="text-lg font-semibold text-text-primary">Review results</h3>
                <button className="rounded-xl border border-aurora-border/60 px-4 py-2 text-xs text-text-secondary" onClick={() => void loadReview()} disabled={reviewLoading}>{reviewLoading ? "Loading..." : "Refresh"}</button>
              </div>
              {reviewError && <div className="rounded-xl border border-red-500/20 bg-risk-high-bg/20 p-3 text-sm text-danger">{reviewError}</div>}
              <div className="grid grid-cols-1 gap-4 md:grid-cols-4">
                <SummaryTile label="Low risk" value={String(scan.riskReport?.summary.low_risk_count ?? 0)} />
                <SummaryTile label="Safe space" value={formatSize(scan.riskReport?.summary.safe_deletable_bytes ?? 0)} />
                <SummaryTile label="Health" value={health ? `${health.score} / 100` : "--"} />
                <SummaryTile label="Recommendations" value={String(recommendations.length)} />
              </div>
              <div className="space-y-2">
                {recommendations.slice(0, 6).map((item) => (
                  <div key={item.item.path} className="rounded-xl border border-aurora-border/40 bg-aurora-elevated/50 p-3 text-sm text-text-secondary">
                    <div className="flex items-center justify-between gap-3"><span className="truncate font-semibold text-text-primary">#{item.rank} {item.item.name}</span><span>{formatSize(item.estimated_size)}</span></div>
                    <p className="mt-1 truncate text-xs text-text-muted">{item.reason}</p>
                  </div>
                ))}
                {!reviewLoading && recommendations.length === 0 && <div className="py-8 text-center text-sm text-text-muted">No recommendations found.</div>}
              </div>
            </div>
          )}

          {step === 3 && (
            <div className="space-y-5">
              <h3 className="text-lg font-semibold text-text-primary">Confirm cleanup</h3>
              <div className="rounded-xl border border-success/20 bg-risk-low-bg/10 p-4 text-sm text-success">{selectedItems.length} safe LOW-risk items selected, estimated {formatSize(selectedBytes)}.</div>
              <div className="max-h-96 space-y-2 overflow-y-auto">
                {safeCandidates.map((item) => (
                  <label key={item.path} className="flex cursor-pointer items-center gap-3 rounded-xl border border-aurora-border/40 bg-aurora-elevated/40 p-3 text-sm text-text-secondary">
                    <input type="checkbox" checked={selectedPaths.has(item.path)} onChange={() => togglePath(item.path)} />
                    <span className="min-w-0 flex-1 truncate" title={item.path}>{item.name}</span>
                    <span className="font-mono text-xs">{formatSize(item.size_bytes)}</span>
                  </label>
                ))}
                {safeCandidates.length === 0 && <div className="py-10 text-center text-sm text-text-muted">No safe LOW-risk cleanup candidates.</div>}
              </div>
            </div>
          )}

          {step === 4 && (
            <div className="space-y-5">
              <h3 className="text-lg font-semibold text-text-primary">Execution summary</h3>
              {executing && <p className="text-sm text-text-secondary">Cleaning {cleanProgress?.current_item ?? "items"} ({cleanProgress?.current ?? 0}/{cleanProgress?.total ?? selectedItems.length})...</p>}
              {executeError && <div className="rounded-xl border border-red-500/20 bg-risk-high-bg/20 p-3 text-sm text-danger">{executeError}</div>}
              {cleanResult && (
                <div className="grid grid-cols-1 gap-4 md:grid-cols-4">
                  <SummaryTile label="Succeeded" value={String(cleanResult.succeeded)} />
                  <SummaryTile label="Skipped" value={String(cleanResult.skipped)} />
                  <SummaryTile label="Failed" value={String(cleanResult.failed)} />
                  <SummaryTile label="Freed" value={formatSize(cleanResult.freed_bytes)} />
                </div>
              )}
            </div>
          )}

          <div className="mt-8 flex flex-wrap justify-between gap-3 border-t border-aurora-border/30 pt-5">
            <button className="rounded-xl border border-aurora-border/60 px-4 py-2 text-sm text-text-secondary" onClick={() => setStep((value) => Math.max(0, value - 1))} disabled={step === 0 || executing}>Back</button>
            {step === 2 && <button className="btn-primary" onClick={() => { setSelectedPaths(new Set(safeCandidates.map((item) => item.path))); setStep(3); }}><span>Review safe items</span></button>}
            {step === 3 && <button className="btn-primary" onClick={() => void executeCleanup()} disabled={selectedItems.length === 0 || executing}><span>Clean selected items</span></button>}
            {step < 2 && <button className="rounded-xl border border-aurora-border/60 px-4 py-2 text-sm text-text-secondary" onClick={() => setStep((value) => Math.min(4, value + 1))}>Next</button>}
          </div>
        </div>
      </div>
    </div>
  );
}

function SummaryTile({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-2xl border border-aurora-border/40 bg-aurora-elevated/60 p-4">
      <div className="text-xs uppercase tracking-wider text-text-muted">{label}</div>
      <div className="mt-2 text-lg font-semibold text-text-primary">{value}</div>
    </div>
  );
}
