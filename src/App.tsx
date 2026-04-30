import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import Treemap from "./components/Treemap";
import CleanupPage from "./pages/Cleanup";
import { formatSize } from "./utils/format";
import type { DirInfo, DriveInfo, RiskReport, ScanProgress } from "./types";

// --- Drive ring chart ---
function DriveRing({
  usedPercent,
  driveLetter,
  totalBytes,
  usedBytes,
  freeBytes,
}: {
  usedPercent: number;
  driveLetter: string;
  totalBytes: number;
  usedBytes: number;
  freeBytes: number;
}) {
  const color =
    usedPercent > 90
      ? "var(--color-danger)"
      : usedPercent > 70
        ? "var(--color-warning)"
        : "var(--color-accent)";

  return (
    <div className="glass-card p-8 flex items-center gap-8 min-w-[480px]">
      {/* Ring */}
      <div className="relative flex-shrink-0">
        <svg width="140" height="140" viewBox="0 0 140 140">
          <circle
            cx="70" cy="70" r="62"
            fill="none"
            stroke="var(--color-aurora-border)"
            strokeWidth="10"
          />
          <circle
            cx="70" cy="70" r="62"
            fill="none"
            stroke={color}
            strokeWidth="10"
            strokeLinecap="round"
            strokeDasharray={`${(usedPercent / 100) * 389.6} 389.6`}
            transform="rotate(-90 70 70)"
            style={{
              transition: "stroke-dasharray 1.2s ease-out, stroke 0.5s ease",
              filter: `drop-shadow(0 0 8px ${color}44)`,
            }}
          />
          <circle
            cx="70" cy="70" r="48"
            fill="none"
            stroke="var(--color-aurora-border-light)"
            strokeWidth="0.5"
            strokeDasharray="4 6"
          />
          <text
            x="70" y="62"
            textAnchor="middle"
            fill="var(--color-text-primary)"
            fontSize="22"
            fontWeight="700"
            fontFamily="var(--font-mono)"
          >
            {usedPercent.toFixed(1)}%
          </text>
          <text
            x="70" y="84"
            textAnchor="middle"
            fill="var(--color-text-secondary)"
            fontSize="11"
            fontWeight="500"
          >
            USED
          </text>
        </svg>
        <div
          className="absolute inset-0 rounded-full opacity-20"
          style={{
            background: `radial-gradient(circle, ${color}22 0%, transparent 70%)`,
            filter: "blur(20px)",
          }}
        />
      </div>

      {/* Stats */}
      <div className="flex flex-col gap-3">
        <div className="flex items-center gap-2 mb-1">
          <span className="text-lg font-bold text-text-primary">{driveLetter}:</span>
          <span className="text-xs text-text-muted uppercase tracking-wider">Drive Overview</span>
        </div>
        <StatRow label="Total" value={formatSize(totalBytes)} />
        <StatRow label="Used" value={formatSize(usedBytes)} highlight />
        <StatRow label="Free" value={formatSize(freeBytes)} success />
        <div className="mt-3 pt-3 border-t border-aurora-border-light">
          <div className="flex items-center gap-2 text-xs text-text-muted">
            <span className="live-dot" />
            Free space: <strong className="text-success">{formatSize(freeBytes)}</strong>
          </div>
        </div>
      </div>
    </div>
  );
}

function StatRow({
  label,
  value,
  highlight,
  success,
}: {
  label: string;
  value: string;
  highlight?: boolean;
  success?: boolean;
}) {
  return (
    <div className="flex items-center justify-between gap-12">
      <span className="text-sm text-text-secondary">{label}</span>
      <span
        className={`text-sm font-semibold font-mono stat-number ${
          success ? "text-success" : highlight ? "text-text-primary" : "text-text-secondary"
        }`}
      >
        {value}
      </span>
    </div>
  );
}

// --- Navigation Sidebar ---
const NAV_ITEMS = [
  { id: "dashboard", label: "Dashboard", icon: DashboardIcon },
  { id: "cleanup", label: "Cleanup Report", icon: CleanupIcon },
  { id: "history", label: "History", icon: HistoryIcon },
  { id: "settings", label: "Settings", icon: SettingsIcon },
] as const;

function DashboardIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="3" width="7" height="7" rx="1.5" />
      <rect x="14" y="3" width="7" height="7" rx="1.5" />
      <rect x="3" y="14" width="7" height="7" rx="1.5" />
      <rect x="14" y="14" width="7" height="7" rx="1.5" />
    </svg>
  );
}
function CleanupIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 6h18" />
      <path d="M8 6V4a2 2 0 012-2h4a2 2 0 012 2v2" />
      <path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6" />
      <path d="M10 11v6" />
      <path d="M14 11v6" />
    </svg>
  );
}
function HistoryIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="1 4 1 10 7 10" />
      <path d="M3.5 17.5A9 9 0 102 12" />
    </svg>
  );
}
function SettingsIcon() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="3" />
      <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" />
    </svg>
  );
}

// --- Directory bar item ---
function DirBarItem({
  dir,
  maxSize,
  rank,
}: {
  dir: DirInfo;
  maxSize: number;
  rank: number;
}) {
  const percent = maxSize > 0 ? (dir.size_bytes / maxSize) * 100 : 0;
  const widthPercent = Math.max(percent, 1);

  return (
    <div className="group flex items-center gap-3 px-3 py-2.5 rounded-lg transition-colors hover:bg-aurora-elevated/50">
      <span className="w-6 text-xs text-right text-text-muted font-mono">{rank}</span>
      <div className="w-48 flex-shrink-0 flex items-center justify-between">
        <span className="text-sm text-text-primary truncate" title={dir.path}>
          {dir.name}
        </span>
        <span className="text-xs text-text-secondary font-mono ml-2">
          {formatSize(dir.size_bytes)}
        </span>
      </div>
      <div className="flex-1 h-7 relative">
        <div className="absolute inset-0 rounded-md bg-aurora-border/40 overflow-hidden">
          <div
            className="h-full rounded-md transition-all duration-700 ease-out"
            style={{
              width: `${widthPercent}%`,
              background: `linear-gradient(90deg,
                var(--color-accent) 0%,
                var(--color-accent-light) 40%,
                var(--color-cyan) 100%)`,
              opacity: 0.3 + (rank <= 5 ? 0.5 : 0) + (rank <= 3 ? 0.2 : 0),
            }}
          />
          {rank <= 5 && (
            <div className="absolute inset-0 rounded-md progress-shimmer" style={{ opacity: 0.5 }} />
          )}
        </div>
      </div>
      <span className="w-14 text-right text-xs text-text-muted font-mono">
        {(maxSize > 0 ? (dir.size_bytes / maxSize) * 100 : 0).toFixed(1)}%
      </span>
    </div>
  );
}

// --- Scan Progress Bar ---
function ScanProgressBar({ progress }: { progress: ScanProgress }) {
  const pct = progress.total > 0 ? (progress.processed / progress.total) * 100 : 0;
  const dirName = progress.current_path
    ? progress.current_path.split("\\").pop() || progress.current_path
    : "";

  return (
    <div className="glass-card p-4 mb-6 animate-in fade-in">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2 text-sm">
          <svg className="animate-spin" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
            <circle cx="12" cy="12" r="10" strokeDasharray="32" strokeDashoffset="32" />
          </svg>
          <span className="text-text-primary font-medium">Scanning {progress.drive_letter}: Drive</span>
        </div>
        <span className="text-xs text-text-muted font-mono">
          {progress.processed}/{progress.total} dirs
        </span>
      </div>
      <div className="h-2 rounded-full bg-aurora-border/60 overflow-hidden">
        <div
          className="h-full rounded-full transition-all duration-300 ease-out"
          style={{
            width: `${Math.max(pct, 2)}%`,
            background: "linear-gradient(90deg, var(--color-accent), var(--color-cyan))",
          }}
        />
      </div>
      {dirName && (
        <p className="mt-1.5 text-xs text-text-muted truncate">
          Scanning: {dirName}
        </p>
      )}
    </div>
  );
}

// --- Main App ---
export default function App() {
  const [driveInfo, setDriveInfo] = useState<DriveInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [version, setVersion] = useState("");
  const [activeTab, setActiveTab] = useState("dashboard");
  const [drives, setDrives] = useState<string[]>([]);
  const [selectedDrive, setSelectedDrive] = useState("C");
  const [riskReport, setRiskReport] = useState<RiskReport | null>(null);

  // Drill-down state
  interface Breadcrumb { name: string; path: string }
  const [breadcrumbs, setBreadcrumbs] = useState<Breadcrumb[]>([]);
  const [drillData, setDrillData] = useState<DirInfo[] | null>(null);
  const [drillTotal, setDrillTotal] = useState(0);

  useEffect(() => {
    invoke<string>("app_version").then(setVersion);
    loadDrives();
    scanDrive("C");

    // Listen for scan progress events
    const unlisten = listen<ScanProgress>("scan-progress", (event) => {
      setProgress(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  async function loadDrives() {
    try {
      const driveList = await invoke<string[]>("list_drives");
      setDrives(driveList);
      if (driveList.length > 0 && !driveList.includes("C")) {
        setSelectedDrive(driveList[0]);
      }
    } catch {
      // Fallback: just use C:
      setDrives(["C"]);
    }
  }

  async function scanDrive(drive: string) {
    setLoading(true);
    setError(null);
    setProgress(null);
    setSelectedDrive(drive);
    setBreadcrumbs([]);
    setDrillData(null);
    setRiskReport(null);
    try {
      const info = await invoke<DriveInfo>("scan_drive", { drive });
      setDriveInfo(info);
      try {
        const report = await invoke<RiskReport>("classify_risks", { scan: info });
        setRiskReport(report);
      } catch {
        setRiskReport(null);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
      setProgress(null);
    }
  }

  const handleDrillDown = useCallback(async (path: string, name: string) => {
    setLoading(true);
    try {
      const dirs = await invoke<DirInfo[]>("scan_directory", { path });
      setDrillData(dirs);
      setBreadcrumbs((prev) => [...prev, { name, path }]);
      // Use the parent item's size as approximation for total in this view
      const total = dirs.reduce((sum, d) => sum + d.size_bytes, 0);
      setDrillTotal(total);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  function handleBreadcrumbClick(index: number) {
    if (index === -1) {
      // Back to root
      setBreadcrumbs([]);
      setDrillData(null);
      return;
    }
    const newBreadcrumbs = breadcrumbs.slice(0, index + 1);
    setBreadcrumbs(newBreadcrumbs);
    // Re-scan the selected directory
    const last = newBreadcrumbs[newBreadcrumbs.length - 1];
    if (last) {
      handleDrillDown(last.path, last.name);
    }
  }

  const usedPercent = driveInfo
    ? (driveInfo.used_bytes / driveInfo.total_bytes) * 100
    : 0;

  const currentData = drillData ?? driveInfo?.top_dirs ?? [];
  const maxDirSize = currentData[0]?.size_bytes ?? 1;

  return (
    <div className="h-full flex bg-aurora-bg">
      {/* --- Sidebar --- */}
      <aside className="w-60 flex-shrink-0 flex flex-col border-r border-aurora-border/60 bg-aurora-surface/50 backdrop-blur-xl">
        <div className="px-5 pt-6 pb-5 border-b border-aurora-border/40">
          <div className="flex items-center gap-3">
            <div className="relative w-9 h-9 rounded-xl flex items-center justify-center"
              style={{
                background: "linear-gradient(135deg, var(--color-accent), #7c3aed)",
                boxShadow: "0 4px 15px var(--color-accent-glow)",
              }}
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="white" strokeWidth="2" strokeLinecap="round">
                <circle cx="12" cy="12" r="10" />
                <circle cx="12" cy="12" r="4" />
                <path d="M12 2v4M12 18v4M2 12h4M18 12h4" />
              </svg>
            </div>
            <div>
              <h1 className="text-base font-bold text-text-primary tracking-tight">DiskPulse</h1>
              <p className="text-[10px] text-text-muted uppercase tracking-wider">v{version || "0.0.1"}</p>
            </div>
          </div>
        </div>

        <nav className="flex-1 px-3 py-4 space-y-1">
          {NAV_ITEMS.map((item) => (
            <button
              key={item.id}
              className={`nav-item w-full text-left ${activeTab === item.id ? "active" : ""}`}
              onClick={() => setActiveTab(item.id)}
            >
              <item.icon />
              <span>{item.label}</span>
              {activeTab === item.id && (
                <div className="ml-auto w-1.5 h-1.5 rounded-full bg-accent"
                  style={{ boxShadow: "0 0 6px var(--color-accent-glow)" }}
                />
              )}
            </button>
          ))}
        </nav>

        <div className="px-4 py-4 border-t border-aurora-border/40">
          <div className="flex items-center gap-2 text-xs text-text-muted">
            <span className="live-dot" />
            <span>{loading ? "Scanning..." : "Monitoring active"}</span>
          </div>
        </div>
      </aside>

      {/* --- Main Content --- */}
      <main className="flex-1 flex flex-col overflow-hidden">
        {/* Header */}
        <header className="flex items-center justify-between px-8 py-4 border-b border-aurora-border/40 bg-aurora-surface/30 backdrop-blur-lg">
          <div>
            <h2 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
              {activeTab === "dashboard" ? "Drive Overview" : activeTab}
            </h2>
            <p className="text-xs text-text-muted mt-0.5">
              {driveInfo
                ? `${driveInfo.drive_letter}: Drive — ${formatSize(driveInfo.total_bytes)} total`
                : "Ready"}
            </p>
          </div>

          <div className="flex items-center gap-3">
            {/* Drive selector */}
            {drives.length > 0 && (
              <select
                value={selectedDrive}
                onChange={(e) => scanDrive(e.target.value)}
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
            )}

            {/* Quick stats */}
            {driveInfo && !loading && (
              <div className="flex items-center gap-4 mr-4 px-4 py-1.5 rounded-lg bg-aurora-elevated/50 border border-aurora-border/30">
                <div className="flex items-center gap-1.5 text-xs">
                  <span className="text-text-muted">Used</span>
                  <span className="font-mono font-semibold text-text-primary">{usedPercent.toFixed(1)}%</span>
                </div>
                <div className="w-px h-4 bg-aurora-border/60" />
                <div className="flex items-center gap-1.5 text-xs">
                  <span className="text-text-muted">Free</span>
                  <span className="font-mono font-semibold text-success">{formatSize(driveInfo.free_bytes)}</span>
                </div>
              </div>
            )}

            <button
              className="btn-primary"
              onClick={() => scanDrive(selectedDrive)}
              disabled={loading}
            >
              <span className="flex items-center gap-2">
                {loading ? (
                  <>
                    <svg className="animate-spin" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                      <circle cx="12" cy="12" r="10" strokeDasharray="32" strokeDashoffset="32" />
                    </svg>
                    Scanning...
                  </>
                ) : (
                  <>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round">
                      <circle cx="11" cy="11" r="8" />
                      <path d="M21 21l-4.35-4.35" />
                    </svg>
                    Scan {selectedDrive}: Drive
                  </>
                )}
              </span>
            </button>
          </div>
        </header>

        {/* Error banner */}
        {error && (
          <div className="mx-8 mt-4 px-4 py-3 rounded-xl bg-risk-high-bg border border-red-500/20 text-sm text-danger flex items-center gap-2">
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

        {/* Content Area */}
        <div className="flex-1 overflow-y-auto page-enter">
          {activeTab === "dashboard" && (
            <div className="p-8 space-y-8">
              {/* Scan progress */}
              {progress && <ScanProgressBar progress={progress} />}

              {driveInfo ? (
                <>
                  <DriveRing
                    usedPercent={usedPercent}
                    driveLetter={driveInfo.drive_letter}
                    totalBytes={driveInfo.total_bytes}
                    usedBytes={driveInfo.used_bytes}
                    freeBytes={driveInfo.free_bytes}
                  />

                  {/* Breadcrumb navigation */}
                  {breadcrumbs.length > 0 && (
                    <div className="flex items-center gap-1.5 text-sm flex-wrap">
                      <button
                        className="text-text-muted hover:text-accent-light transition-colors px-2 py-0.5 rounded-md hover:bg-aurora-elevated/50"
                        onClick={() => handleBreadcrumbClick(-1)}
                      >
                        {driveInfo.drive_letter}:\\
                      </button>
                      {breadcrumbs.map((crumb, i) => (
                        <span key={crumb.path} className="flex items-center gap-1.5">
                          <span className="text-text-muted">/</span>
                          <button
                            className={`px-2 py-0.5 rounded-md transition-colors ${
                              i === breadcrumbs.length - 1
                                ? "text-accent-light font-medium"
                                : "text-text-secondary hover:text-accent-light hover:bg-aurora-elevated/50"
                            }`}
                            onClick={() => handleBreadcrumbClick(i)}
                          >
                            {crumb.name}
                          </button>
                        </span>
                      ))}
                    </div>
                  )}

                  {/* Treemap Visualization */}
                  <div className="glass-card p-4">
                    <div className="flex items-center justify-between mb-4 px-2">
                      <div>
                        <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
                          {breadcrumbs.length > 0
                            ? breadcrumbs[breadcrumbs.length - 1].name
                            : "Disk Space Treemap"}
                        </h3>
                        <p className="text-xs text-text-muted mt-0.5">
                          {drillData
                            ? `${drillData.length} items — click to explore deeper`
                            : `${driveInfo.top_dirs.length} top-level directories — click to drill down`}
                        </p>
                      </div>
                    </div>
                    <Treemap
                      data={drillData ?? driveInfo.top_dirs}
                      totalBytes={drillData ? drillTotal : driveInfo.total_bytes}
                      onDrillDown={handleDrillDown}
                    />
                  </div>

                  {/* Directory List (detailed view below treemap) */}
                  <div className="glass-card p-6">
                    <div className="flex items-center justify-between mb-6">
                      <div>
                        <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
                          Directory List
                        </h3>
                        <p className="text-xs text-text-muted mt-0.5">
                          Ranked by size — showing top 20
                        </p>
                      </div>
                    </div>
                    <div className="space-y-0.5">
                      {currentData.slice(0, 20).map((dir, i) => (
                        <DirBarItem
                          key={dir.path}
                          dir={dir}
                          maxSize={maxDirSize}
                          rank={i + 1}
                        />
                      ))}
                    </div>
                  </div>
                </>
              ) : !error && !loading ? (
                <div className="flex flex-col items-center justify-center py-32 text-text-muted">
                  <svg width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1" strokeLinecap="round" opacity="0.3">
                    <circle cx="11" cy="11" r="8" />
                    <path d="M21 21l-4.35-4.35" />
                  </svg>
                  <p className="mt-4 text-sm">Select a drive and click Scan to begin</p>
                </div>
              ) : null}
            </div>
          )}

          {activeTab === "cleanup" && (
            <CleanupPage report={riskReport} />
          )}
          {activeTab === "history" && (
            <div className="flex items-center justify-center py-32">
              <div className="text-center">
                <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-aurora-elevated border border-aurora-border/40 flex items-center justify-center">
                  <HistoryIcon />
                </div>
                <p className="text-text-secondary text-sm">History & Trends — Coming in v0.0.8</p>
                <p className="text-text-muted text-xs mt-1">SQLite snapshots + ECharts trend charts</p>
              </div>
            </div>
          )}
          {activeTab === "settings" && (
            <div className="flex items-center justify-center py-32">
              <div className="text-center">
                <div className="w-16 h-16 mx-auto mb-4 rounded-2xl bg-aurora-elevated border border-aurora-border/40 flex items-center justify-center">
                  <SettingsIcon />
                </div>
                <p className="text-text-secondary text-sm">Settings — Coming in v0.0.9</p>
                <p className="text-text-muted text-xs mt-1">Preferences, rules config, about</p>
              </div>
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
