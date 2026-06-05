import { lazy, Suspense, useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { NavIcons } from "./components/Icons";
import { formatSize } from "./utils/format";
import { useDriveScan } from "./hooks/useDriveScan";
import { useFsEvents } from "./hooks/useFsEvents";
import { useRemoteDevice } from "./hooks/useRemoteDevice";
import { useTranslation } from "react-i18next";
import type { AnomalyEvent, AppSettings, AutoCleanupStatus as AutoCleanupStatusType, CleanItem, CleanProgress, CleanResult, DeviceInfo, DirInfo, DiskSpaceAlertPayload, DriveInfo, RiskItem, ScanProgress } from "./types";

const EMPTY_RISK_ITEMS: RiskItem[] = [];
const Treemap = lazy(() => import("./components/Treemap"));
const CleanupPage = lazy(() => import("./pages/Cleanup"));
const CleanupPreview = lazy(() => import("./components/CleanupPreview"));
const HistoryPage = lazy(() => import("./pages/History"));
const SettingsPage = lazy(() => import("./pages/Settings"));
const PredictionCard = lazy(() => import("./components/PredictionCard"));
const AnomalyCard = lazy(() => import("./components/AnomalyCard"));
const LargeFileFinder = lazy(() => import("./components/LargeFileFinder"));
const AutoCleanupStatus = lazy(() => import("./components/AutoCleanupStatus"));
const DuplicateFinder = lazy(() => import("./components/DuplicateFinder"));
const AgingAnalysis = lazy(() => import("./components/AgingAnalysis"));
const RecommendationCard = lazy(() => import("./components/RecommendationCard"));
const PredictiveCleanupCard = lazy(() => import("./components/PredictiveCleanupCard"));
const FragmentationView = lazy(() => import("./components/FragmentationView"));
const CleanupWizard = lazy(() => import("./components/CleanupWizard"));
const NotificationCenter = lazy(() => import("./components/NotificationCenter"));
const ThemeSwitcher = lazy(() => import("./components/ThemeSwitcher"));
const RELEASE_API_URL = "https://api.github.com/repos/Nagi-226/DiskPulse/releases/latest";

function PageSkeleton({ label = "Loading module" }: { label?: string }) {
  return (
    <div className="glass-card mx-8 my-6 rounded-2xl border border-aurora-border/40 p-6">
      <div className="mb-4 h-4 w-40 animate-pulse rounded-full bg-aurora-border/50" />
      <div className="grid grid-cols-3 gap-3">
        <div className="h-24 animate-pulse rounded-2xl bg-aurora-elevated/60" />
        <div className="h-24 animate-pulse rounded-2xl bg-aurora-elevated/40" />
        <div className="h-24 animate-pulse rounded-2xl bg-aurora-elevated/30" />
      </div>
      <p className="mt-4 text-xs uppercase tracking-wider text-text-muted">{label}</p>
    </div>
  );
}

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
    <div
      className="glass-card fluent-hover p-8 flex items-center gap-8 min-w-[480px]"
      onMouseMove={(e) => {
        const rect = e.currentTarget.getBoundingClientRect();
        e.currentTarget.style.setProperty('--mouse-x', `${((e.clientX - rect.left) / rect.width) * 100}%`);
        e.currentTarget.style.setProperty('--mouse-y', `${((e.clientY - rect.top) / rect.height) * 100}%`);
      }}
    >
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

function formatCacheAge(ageMs: number | null) {
  if (ageMs == null) return "";
  const minutes = Math.floor(ageMs / 60_000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m old`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h old`;
  return `${Math.floor(hours / 24)}d old`;
}

function CacheBadge({
  dataSource,
  cacheAgeMs,
}: {
  dataSource: "empty" | "meta" | "cached" | "fresh";
  cacheAgeMs: number | null;
}) {
  if (dataSource === "empty") return null;

  const label =
    dataSource === "fresh"
      ? "Live"
      : dataSource === "cached"
        ? `Cached ${formatCacheAge(cacheAgeMs)}`
        : "Metadata loaded";

  const tone =
    dataSource === "fresh"
      ? "border-success/25 bg-risk-low-bg text-success"
      : dataSource === "cached"
        ? "border-warning/25 bg-risk-medium-bg text-warning"
        : "border-accent/25 bg-accent/10 text-accent-light";

  return (
    <span className={`px-2.5 py-1 rounded-full text-[11px] font-semibold uppercase tracking-wider border ${tone}`}>
      {label}
    </span>
  );
}

function compareVersions(left: string, right: string) {
  const parse = (value: string) =>
    value
      .replace(/^v/i, "")
      .split(".")
      .map((part) => Number.parseInt(part, 10) || 0);
  const a = parse(left);
  const b = parse(right);
  for (let i = 0; i < Math.max(a.length, b.length); i += 1) {
    const diff = (a[i] ?? 0) - (b[i] ?? 0);
    if (diff !== 0) return diff;
  }
  return 0;
}

async function notifyUpdate(tagName: string, releaseUrl: string) {
  let granted = await isPermissionGranted();
  if (!granted) {
    granted = (await requestPermission()) === "granted";
  }
  if (granted) {
    sendNotification({
      title: "DiskPulse update available",
      body: `${tagName} is available. Open GitHub Releases to download it: ${releaseUrl}`,
    });
  }
}

function DeviceSelector({
  devices,
  selectedDeviceId,
  isHubRunning,
  loading,
  discoveryHint,
  pairingCode,
  onSelect,
  onStartHub,
  onStopHub,
  onDiscover,
  onCreateToken,
  onPairToken,
}: {
  devices: DeviceInfo[];
  selectedDeviceId: string;
  isHubRunning: boolean;
  loading: boolean;
  discoveryHint: string | null;
  pairingCode: string | null;
  onSelect: (deviceId: string) => void;
  onStartHub: () => void;
  onStopHub: () => void;
  onDiscover: () => void;
  onCreateToken: () => void;
  onPairToken: (token: string) => void;
}) {
  return (
    <div className="flex items-center gap-2 rounded-xl border border-aurora-border/40 bg-aurora-elevated/50 px-2 py-1.5">
      <span className={`w-2 h-2 rounded-full ${isHubRunning ? "bg-success live-dot" : "bg-aurora-border"}`} />
      <select
        value={selectedDeviceId}
        onChange={(e) => onSelect(e.target.value)}
        className="bg-transparent text-xs text-text-primary focus:outline-none min-w-[130px]"
        title={discoveryHint ?? "Local device"}
      >
        <option value="local">Local device</option>
        {devices.map((device) => (
          <option key={device.device_id} value={device.device_id}>
            {device.connected ? "●" : "○"} {device.name}
          </option>
        ))}
      </select>
      <button
        className="px-2 py-1 rounded-lg text-[11px] font-semibold border border-aurora-border/50 text-text-secondary hover:text-accent-light hover:border-accent/40 disabled:opacity-50"
        disabled={loading}
        onClick={isHubRunning ? onStopHub : onStartHub}
      >
        {isHubRunning ? "Stop Hub" : "Start Hub"}
      </button>
      <button
        className="px-2 py-1 rounded-lg text-[11px] font-semibold border border-aurora-border/50 text-text-secondary hover:text-accent-light hover:border-accent/40 disabled:opacity-50"
        disabled={loading}
        onClick={onDiscover}
      >
        Discover
      </button>
      <button
        className="px-2 py-1 rounded-lg text-[11px] font-semibold border border-aurora-border/50 text-text-secondary hover:text-accent-light hover:border-accent/40 disabled:opacity-50"
        disabled={loading || !isHubRunning}
        onClick={onCreateToken}
      >
        {pairingCode ?? "Token"}
      </button>
      <button
        className="px-2 py-1 rounded-lg text-[11px] font-semibold border border-aurora-border/50 text-text-secondary hover:text-accent-light hover:border-accent/40 disabled:opacity-50"
        disabled={loading}
        onClick={() => {
          const token = window.prompt("Enter the 6-digit DiskPulse pairing token");
          if (token) onPairToken(token);
        }}
      >
        Pair
      </button>
    </div>
  );
}

// --- Navigation Sidebar ---
const NAV_ITEMS = [
  { id: "dashboard", labelKey: "nav.dashboard", Icon: NavIcons.Dashboard },
  { id: "cleanup", labelKey: "nav.cleanup", Icon: NavIcons.Cleanup },
  { id: "large-files", labelKey: "nav.largeFiles", Icon: NavIcons.LargeFiles },
  { id: "duplicates", labelKey: "nav.duplicates", Icon: NavIcons.LargeFiles },
  { id: "aging", labelKey: "nav.aging", Icon: NavIcons.History },
  { id: "fragmentation", labelKey: "nav.fragmentation", Icon: NavIcons.History },
  { id: "wizard", labelKey: "nav.wizard", Icon: NavIcons.Cleanup },
  { id: "history", labelKey: "nav.history", Icon: NavIcons.History },
  { id: "settings", labelKey: "nav.settings", Icon: NavIcons.Settings },
] as const;

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
        {dir.is_approximate && (
          <span className="ml-2 rounded-full border border-warning/25 bg-risk-medium-bg px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-warning">
            approx
          </span>
        )}
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
  const phaseLabel =
    progress.phase === "walking"
      ? "Discovering directories"
      : progress.phase === "measuring"
        ? "Measuring space"
        : "Updating treemap";

  return (
    <div className="glass-card p-4 mb-6 animate-in fade-in">
      <div className="flex items-center justify-between mb-2">
        <div className="flex items-center gap-2 text-sm">
          <svg className="animate-spin" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
            <circle cx="12" cy="12" r="10" strokeDasharray="32" strokeDashoffset="32" />
          </svg>
          <span className="text-text-primary font-medium">{phaseLabel} on {progress.drive_letter}: Drive</span>
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
  const { t } = useTranslation();
  const [cleanProgress, setCleanProgress] = useState<CleanProgress | null>(null);
  const [alertToast, setAlertToast] = useState<DiskSpaceAlertPayload | null>(null);
  const [anomalyToast, setAnomalyToast] = useState<AnomalyEvent | null>(null);
  const [autoCleanupToast, setAutoCleanupToast] = useState<string | null>(null);
  const [version, setVersion] = useState("");
  const [activeTab, setActiveTab] = useState("dashboard");
  const [drives, setDrives] = useState<string[]>([]);
  const [externalCleanupItems, setExternalCleanupItems] = useState<CleanItem[]>([]);
  const {
    driveInfo,
    loading,
    progress,
    error,
    setError,
    selectedDrive,
    setSelectedDrive,
    riskReport,
    dataSource,
    cacheAgeMs,
    scanDrive,
    cancelScan,
  } = useDriveScan("C");

  // Drill-down state
  interface Breadcrumb { name: string; path: string }
  const [breadcrumbs, setBreadcrumbs] = useState<Breadcrumb[]>([]);
  const [drillData, setDrillData] = useState<DirInfo[] | null>(null);
  const [drillTotal, setDrillTotal] = useState(0);
  const [drillLoading, setDrillLoading] = useState(false);

  // File system watcher
  const { isWatching, lastBatch, eventCount, startWatching, stopWatching } = useFsEvents();
  const {
    devices,
    pairingToken,
    discoveryInfo,
    isHubRunning,
    loading: remoteLoading,
    error: remoteError,
    setError: setRemoteError,
    startHub,
    stopHub,
    discoverDevices,
    createToken,
    pairDevice,
    queryRemoteDevice,
  } = useRemoteDevice();
  const [selectedDeviceId, setSelectedDeviceId] = useState("local");
  const [remoteDriveInfo, setRemoteDriveInfo] = useState<DriveInfo | null>(null);
  const [remoteScanLoading, setRemoteScanLoading] = useState(false);

  const selectedRemoteDevice = devices.find((device) => device.device_id === selectedDeviceId) ?? null;
  const activeDriveInfo = selectedRemoteDevice ? remoteDriveInfo : driveInfo;
  const activeDeviceLabel = selectedRemoteDevice ? selectedRemoteDevice.name : "Local device";

  const startDriveScan = useCallback(
    async (drive: string) => {
      setBreadcrumbs([]);
      setDrillData(null);
      setRemoteError(null);
      if (selectedRemoteDevice) {
        setRemoteScanLoading(true);
        try {
          const remoteInfo = await queryRemoteDevice<DriveInfo>(
            selectedRemoteDevice,
            { command: "scan_drive", payload: { drive } },
            60_000,
          );
          setRemoteDriveInfo(remoteInfo);
          return remoteInfo;
        } catch (e) {
          setRemoteError(String(e));
          return null;
        } finally {
          setRemoteScanLoading(false);
        }
      }
      return scanDrive(drive);
    },
    [queryRemoteDevice, scanDrive, selectedRemoteDevice, setRemoteError],
  );

  // Refs for tray event closures to access latest state
  const scanDriveRef = useRef(startDriveScan);
  const selectedDriveRef = useRef(selectedDrive);
  const isWatchingRef = useRef(isWatching);
  const alertTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const anomalyTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const autoCleanupTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  scanDriveRef.current = startDriveScan;
  selectedDriveRef.current = selectedDrive;
  isWatchingRef.current = isWatching;

  useEffect(() => {
    invoke<string>("app_version").then(setVersion);
    loadDrives();
    startDriveScan("C");

    const unlistenTrayScan = listen("tray-quick-scan", () => {
      scanDriveRef.current(selectedDriveRef.current);
    });
    const unlistenTrayMonitor = listen("tray-toggle-monitor", () => {
      if (isWatchingRef.current) {
        stopWatching();
      } else {
        startWatching();
      }
    });
    const unlistenAutoScan = listen<string>("auto-scan", (event) => {
      scanDriveRef.current(event.payload);
    });
    const unlistenCleanProgress = listen<CleanProgress>("clean-progress", (event) => {
      setCleanProgress(event.payload);
    });
    const unlistenAlert = listen<DiskSpaceAlertPayload>("disk-space-alert", (event) => {
      if (alertTimerRef.current) clearTimeout(alertTimerRef.current);
      setAlertToast(event.payload);
      alertTimerRef.current = setTimeout(() => setAlertToast(null), 8000);
    });
    const unlistenAnomaly = listen<AnomalyEvent>("anomaly-detected", (event) => {
      if (anomalyTimerRef.current) clearTimeout(anomalyTimerRef.current);
      setAnomalyToast(event.payload);
      anomalyTimerRef.current = setTimeout(() => setAnomalyToast(null), 8000);
    });
    const unlistenAutoCleanupComplete = listen<CleanResult>("auto-cleanup-complete", (event) => {
      if (autoCleanupTimerRef.current) clearTimeout(autoCleanupTimerRef.current);
      setAutoCleanupToast(
        `Auto-cleanup complete: ${event.payload.succeeded} cleaned, ${formatSize(event.payload.freed_bytes)} freed.`
      );
      autoCleanupTimerRef.current = setTimeout(() => setAutoCleanupToast(null), 8000);
    });
    const unlistenAutoCleanupScheduled = listen<AutoCleanupStatusType>("auto-cleanup-scheduled", (event) => {
      const nextRun = event.payload.next_run_epoch_ms
        ? new Date(event.payload.next_run_epoch_ms).toLocaleString()
        : "not scheduled";
      if (autoCleanupTimerRef.current) clearTimeout(autoCleanupTimerRef.current);
      setAutoCleanupToast(`Auto-cleanup scheduled: next run ${nextRun}.`);
      autoCleanupTimerRef.current = setTimeout(() => setAutoCleanupToast(null), 8000);
    });
    return () => {
      if (alertTimerRef.current) clearTimeout(alertTimerRef.current);
      if (anomalyTimerRef.current) clearTimeout(anomalyTimerRef.current);
      if (autoCleanupTimerRef.current) clearTimeout(autoCleanupTimerRef.current);
      unlistenTrayScan.then((fn) => fn());
      unlistenTrayMonitor.then((fn) => fn());
      unlistenAutoScan.then((fn) => fn());
      unlistenCleanProgress.then((fn) => fn());
      unlistenAlert.then((fn) => fn());
      unlistenAnomaly.then((fn) => fn());
      unlistenAutoCleanupComplete.then((fn) => fn());
      unlistenAutoCleanupScheduled.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void (async () => {
        try {
          const settings = await invoke<AppSettings>("get_settings");
          if (!settings.update_check_enabled || cancelled) return;
          const response = await fetch(RELEASE_API_URL, {
            headers: { Accept: "application/vnd.github+json" },
          });
          if (!response.ok || cancelled) return;
          const release = (await response.json()) as { tag_name?: string; html_url?: string };
          const latest = release.tag_name;
          if (latest && compareVersions(latest, __APP_VERSION__) > 0) {
            await notifyUpdate(latest, release.html_url ?? "https://github.com/Nagi-226/DiskPulse/releases");
          }
        } catch (e) {
          console.debug("update check skipped:", e);
        }
      })();
    }, 3000);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, []);

  useEffect(() => {
    const current = breadcrumbs[breadcrumbs.length - 1];
    if (!current || !lastBatch?.events.length) {
      return;
    }

    const currentPath = current.path.toLowerCase();
    const changedInCurrentDir = lastBatch.events.some((event) =>
      event.path.toLowerCase().startsWith(currentPath),
    );
    if (!changedInCurrentDir) {
      return;
    }

    let cancelled = false;
    setDrillLoading(true);
    invoke<DirInfo[]>("scan_directory", { path: current.path })
      .then((dirs) => {
        if (cancelled) {
          return;
        }
        setDrillData(dirs);
        setDrillTotal(dirs.reduce((sum, dir) => sum + dir.size_bytes, 0));
      })
      .catch((e) => {
        if (!cancelled) {
          setError(String(e));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setDrillLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [breadcrumbs, lastBatch, setError]);

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

  const handleDrillDown = useCallback(async (path: string, name: string) => {
    setDrillLoading(true);
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
      setDrillLoading(false);
    }
  }, [setError]);

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

  function handleAddToCleanup(items: CleanItem[]) {
    setExternalCleanupItems((current) => {
      const byPath = new Map(current.map((item) => [item.path, item]));
      for (const item of items) {
        byPath.set(item.path, item);
      }
      return Array.from(byPath.values());
    });
    setActiveTab("cleanup");
  }

  const busy = loading || remoteScanLoading;
  const usedPercent = activeDriveInfo
    ? (activeDriveInfo.used_bytes / Math.max(activeDriveInfo.total_bytes, 1)) * 100
    : 0;

  const currentData = drillData ?? activeDriveInfo?.top_dirs ?? [];
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
              <p className="text-[10px] text-text-muted uppercase tracking-wider">v{version || "0.1.0"}</p>
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
              <item.Icon />
              <span>{t(item.labelKey)}</span>
              {activeTab === item.id && (
                <div className="ml-auto w-1.5 h-1.5 rounded-full bg-accent"
                  style={{ boxShadow: "0 0 6px var(--color-accent-glow)" }}
                />
              )}
            </button>
          ))}
        </nav>

        <div className="px-4 py-4 border-t border-aurora-border/40 space-y-3">
          <Suspense fallback={null}>
            <ThemeSwitcher />
          </Suspense>
          <div className="flex items-center gap-2 text-xs text-text-muted">
            <span className={`${isWatching ? "live-dot" : ""} w-2 h-2 rounded-full ${isWatching ? "bg-success" : "bg-aurora-border"}`} />
            <span>{busy ? (cleanProgress ? "Cleaning..." : "Scanning...") : drillLoading ? "Loading folder..." : isWatching ? `Live · ${eventCount} events` : "Monitor paused"}</span>
          </div>
          <button
            className={`w-full px-3 py-2 rounded-xl text-xs font-medium border transition-colors ${
              isWatching
                ? "bg-success/10 border-success/20 text-success hover:bg-success/20"
                : "bg-aurora-elevated/70 border-aurora-border/60 text-text-secondary hover:text-accent-light hover:border-accent/30"
            }`}
            onClick={() => (isWatching ? stopWatching() : startWatching())}
          >
            {isWatching ? "Pause Monitoring" : "Start Monitoring"}
          </button>
        </div>
      </aside>

      {/* --- Main Content --- */}
      <main className="flex-1 flex flex-col overflow-hidden">
        {/* Header */}
        <header className="flex items-center justify-between px-8 py-4 border-b border-aurora-border/40 bg-aurora-surface/30 backdrop-blur-lg">
          <div>
            <div className="flex items-center gap-3">
              <h2 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
                {activeTab === "dashboard"
                  ? "Drive Overview"
                  : activeTab === "large-files"
                    ? t("nav.largeFiles")
                    : activeTab === "duplicates"
                      ? t("nav.duplicates")
                      : activeTab === "aging"
                        ? t("nav.aging")
                        : activeTab}
              </h2>
              {activeTab === "dashboard" && (
                <CacheBadge dataSource={dataSource} cacheAgeMs={cacheAgeMs} />
              )}
            </div>
            <p className="text-xs text-text-muted mt-0.5">
              {activeDriveInfo
                ? `${activeDeviceLabel} · ${activeDriveInfo.drive_letter}: Drive — ${formatSize(activeDriveInfo.total_bytes)} total`
                : "Ready"}
            </p>
          </div>

          <div className="flex items-center gap-3">
            <Suspense fallback={null}>
              <NotificationCenter />
            </Suspense>

            <DeviceSelector
              devices={devices}
              selectedDeviceId={selectedDeviceId}
              isHubRunning={isHubRunning}
              loading={remoteLoading || remoteScanLoading}
              discoveryHint={discoveryInfo?.address_hint ?? null}
              pairingCode={pairingToken?.code ?? null}
              onSelect={(deviceId) => {
                setSelectedDeviceId(deviceId);
                setBreadcrumbs([]);
                setDrillData(null);
                if (deviceId === "local") {
                  setRemoteDriveInfo(null);
                }
              }}
              onStartHub={() => void startHub()}
              onStopHub={() => void stopHub()}
              onDiscover={() => void discoverDevices()}
              onCreateToken={() => void createToken(activeDeviceLabel)}
              onPairToken={(token) => void pairDevice(token)}
            />

            {/* Drive selector */}
            {drives.length > 0 && (
              <select
                value={selectedDrive}
                onChange={(e) => startDriveScan(e.target.value)}
                disabled={busy}
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
            {activeDriveInfo && !busy && (
              <div className="flex items-center gap-4 mr-4 px-4 py-1.5 rounded-lg bg-aurora-elevated/50 border border-aurora-border/30">
                <div className="flex items-center gap-1.5 text-xs">
                  <span className="text-text-muted">Used</span>
                  <span className="font-mono font-semibold text-text-primary">{usedPercent.toFixed(1)}%</span>
                </div>
                <div className="w-px h-4 bg-aurora-border/60" />
                <div className="flex items-center gap-1.5 text-xs">
                  <span className="text-text-muted">Free</span>
                  <span className="font-mono font-semibold text-success">{formatSize(activeDriveInfo.free_bytes)}</span>
                </div>
              </div>
            )}

            <button
              className="btn-primary"
              onClick={() => startDriveScan(selectedDrive)}
              disabled={busy}
            >
              <span className="flex items-center gap-2">
                {busy ? (
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
                    Scan {selectedRemoteDevice ? selectedRemoteDevice.name : `${selectedDrive}: Drive`}
                  </>
                )}
              </span>
            </button>
            {busy && !selectedRemoteDevice && (
              <button
                className="px-4 py-2.5 rounded-xl text-sm font-semibold border border-danger/30 bg-risk-high-bg text-danger hover:bg-risk-high-bg/80 transition-colors"
                onClick={cancelScan}
              >
                Stop Scan
              </button>
            )}
          </div>
        </header>

        {/* Error banner */}
        {(error || remoteError) && (
          <div className="mx-8 mt-4 px-4 py-3 rounded-xl bg-risk-high-bg border border-red-500/20 text-sm text-danger flex items-center gap-2">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
              <circle cx="12" cy="12" r="10" />
              <path d="M12 8v4M12 16h.01" />
            </svg>
            {error || remoteError}
            <button
              onClick={() => {
                setError(null);
                setRemoteError(null);
              }}
              className="ml-auto text-text-muted hover:text-text-primary"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                <path d="M18 6L6 18M6 6l12 12" />
              </svg>
            </button>
          </div>
        )}

        {/* Content Area */}
        <div className="flex-1 overflow-y-auto page-enter">
          <Suspense fallback={<PageSkeleton />}>
          {activeTab === "dashboard" && (
            <div className="p-8 space-y-8">
              {/* Alert toast */}
              {autoCleanupToast && (
                <div className="glass-card p-4 border border-success/25 bg-risk-low-bg/10 animate-in fade-in">
                  <div className="flex items-start gap-3">
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--color-success)" strokeWidth="2" className="mt-0.5 flex-shrink-0">
                      <path d="M20 6L9 17l-5-5" />
                    </svg>
                    <div className="flex-1 text-sm text-text-secondary">{autoCleanupToast}</div>
                    <button className="text-text-muted hover:text-text-primary" onClick={() => setAutoCleanupToast(null)}>
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
                      </svg>
                    </button>
                  </div>
                </div>
              )}

              {/* Alert toast */}
              {alertToast && (
                <div className="glass-card p-4 border border-warning/30 bg-risk-medium-bg/10 animate-in fade-in">
                  <div className="flex items-start gap-3">
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--color-warning)" strokeWidth="2" className="flex-shrink-0 mt-0.5">
                      <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
                      <line x1="12" y1="9" x2="12" y2="13" />
                      <line x1="12" y1="17" x2="12.01" y2="17" />
                    </svg>
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-semibold text-warning">Disk Space Alert</div>
                      <p className="text-sm text-text-secondary mt-1">{alertToast.message}</p>
                      <p className="text-xs text-text-muted mt-1">
                        {alertToast.drive_letter}: — {alertToast.usage_percent.toFixed(0)}% used
                        &middot; {formatSize(alertToast.free_bytes)} free
                      </p>
                    </div>
                    <button
                      className="text-text-muted hover:text-text-primary flex-shrink-0"
                      onClick={() => setAlertToast(null)}
                    >
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
                      </svg>
                    </button>
                  </div>
                </div>
              )}

              {anomalyToast && (
                <div className="glass-card p-4 border border-danger/25 bg-risk-high-bg/10 animate-in fade-in">
                  <div className="flex items-start gap-3">
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--color-danger)" strokeWidth="2" className="flex-shrink-0 mt-0.5">
                      <path d="M12 2v20M2 12h20" />
                      <circle cx="12" cy="12" r="4" />
                    </svg>
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-semibold text-danger">Anomaly Detected</div>
                      <p className="text-sm text-text-secondary mt-1">{anomalyToast.description}</p>
                      {anomalyToast.path && (
                        <p className="text-xs text-text-muted mt-1 truncate">{anomalyToast.path}</p>
                      )}
                    </div>
                    <button
                      className="text-text-muted hover:text-text-primary flex-shrink-0"
                      onClick={() => setAnomalyToast(null)}
                    >
                      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
                      </svg>
                    </button>
                  </div>
                </div>
              )}

              {/* Scan progress */}
              {progress && <ScanProgressBar progress={progress} />}

              {/* Cleanup progress */}
              {cleanProgress && (
                <div className="glass-card p-4 mb-6 animate-in fade-in">
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2 text-sm">
                      <svg className="animate-spin" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                        <circle cx="12" cy="12" r="10" strokeDasharray="32" strokeDashoffset="32" />
                      </svg>
                      <span className="text-text-primary font-medium">
                        Cleaning... {cleanProgress.current}/{cleanProgress.total}
                      </span>
                    </div>
                    <span className="text-xs text-text-muted font-mono">
                      {cleanProgress.status ?? ""}
                    </span>
                  </div>
                  <div className="h-2 rounded-full bg-aurora-border/60 overflow-hidden">
                    <div
                      className="h-full rounded-full transition-all duration-300 ease-out"
                      style={{
                        width: `${Math.max(cleanProgress.total > 0 ? (cleanProgress.current / cleanProgress.total) * 100 : 0, 2)}%`,
                        background: "linear-gradient(90deg, var(--color-warning), var(--color-danger))",
                      }}
                    />
                  </div>
                  {cleanProgress.current_item && (
                    <p className="mt-1.5 text-xs text-text-muted truncate">
                      {cleanProgress.current_item}
                    </p>
                  )}
                </div>
              )}

              {/* Live event feed */}
              {isWatching && lastBatch && (
                <div className="glass-card p-4 rounded-2xl border border-success/15 bg-risk-low-bg/10">
                  <div className="flex items-center justify-between mb-3">
                    <div className="flex items-center gap-2">
                      <span className="live-dot" />
                      <span className="text-sm font-semibold text-text-primary">Live Changes</span>
                    </div>
                    <span className="text-xs text-text-muted font-mono">
                      {eventCount} event{eventCount !== 1 ? "s" : ""}
                    </span>
                  </div>
                  <div className="space-y-1.5 max-h-40 overflow-y-auto">
                    {lastBatch.events.slice(0, 15).map((ev, i) => (
                      <div key={`${ev.path}-${i}`} className="flex items-center gap-2 text-xs">
                        <span className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${
                          ev.kind === "Added" ? "bg-success" : ev.kind === "Removed" ? "bg-danger" : "bg-warning"
                        }`} />
                        <span className="text-text-muted w-14 flex-shrink-0">{ev.kind}</span>
                        <span className="text-text-primary truncate">{ev.path.split("\\").pop() ?? ev.path}</span>
                        <span className="text-text-muted font-mono ml-auto flex-shrink-0">
                          {ev.size_bytes > 0 ? formatSize(ev.size_bytes) : ""}
                        </span>
                      </div>
                    ))}
                    {lastBatch.events.length > 15 && (
                      <p className="text-xs text-text-muted pt-1">
                        +{lastBatch.events.length - 15} more changes in this batch
                      </p>
                    )}
                  </div>
                </div>
              )}

              {activeDriveInfo ? (
                <>
                  <DriveRing
                    usedPercent={usedPercent}
                    driveLetter={activeDriveInfo.drive_letter}
                    totalBytes={activeDriveInfo.total_bytes}
                    usedBytes={activeDriveInfo.used_bytes}
                    freeBytes={activeDriveInfo.free_bytes}
                  />

                  {!selectedRemoteDevice && <PredictionCard drive={activeDriveInfo.drive_letter} />}

                  {!selectedRemoteDevice && <AnomalyCard drive={activeDriveInfo.drive_letter} />}

                  {!selectedRemoteDevice && <AutoCleanupStatus />}

                  {!selectedRemoteDevice && (
                    <RecommendationCard drive={activeDriveInfo.drive_letter} onAddToCleanup={handleAddToCleanup} />
                  )}

                  {!selectedRemoteDevice && (
                    <PredictiveCleanupCard drive={activeDriveInfo.drive_letter} onAddToCleanup={handleAddToCleanup} />
                  )}

                  {busy && currentData.length === 0 && (
                    <div className="glass-card p-6">
                      <div className="flex items-center justify-between mb-5">
                        <div>
                          <h3 className="text-sm font-semibold text-text-primary uppercase tracking-wider">
                            Preparing Treemap
                          </h3>
                          <p className="text-xs text-text-muted mt-0.5">
                            Capacity is ready; top-level folders are filling in as they complete.
                          </p>
                        </div>
                        <span className="text-xs text-accent-light font-mono">background scan</span>
                      </div>
                      <div className="grid grid-cols-4 gap-3 h-64">
                        {Array.from({ length: 12 }).map((_, i) => (
                          <div
                            key={i}
                            className="rounded-xl bg-aurora-elevated/60 border border-aurora-border/40 progress-shimmer"
                            style={{ opacity: 0.35 + (i % 4) * 0.1 }}
                          />
                        ))}
                      </div>
                      {!drillData && activeDriveInfo.top_dirs.some((dir) => dir.is_approximate) && (
                        <span className="rounded-full border border-warning/25 bg-risk-medium-bg px-3 py-1 text-[11px] font-semibold uppercase tracking-wider text-warning">
                          Approximate MFT
                        </span>
                      )}
                    </div>
                  )}

                  {/* Breadcrumb navigation */}
                  {breadcrumbs.length > 0 && (
                    <div className="flex items-center gap-1.5 text-sm flex-wrap">
                      <button
                        className="text-text-muted hover:text-accent-light transition-colors px-2 py-0.5 rounded-md hover:bg-aurora-elevated/50"
                        onClick={() => handleBreadcrumbClick(-1)}
                      >
                        {activeDriveInfo.drive_letter}:\\
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

                  {currentData.length > 0 ? (
                    <>
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
                                : `${activeDriveInfo.top_dirs.length} top-level directories — click to drill down`}
                            </p>
                          </div>
                        </div>
                        <Treemap
                          data={drillData ?? activeDriveInfo.top_dirs}
                          totalBytes={drillData ? drillTotal : activeDriveInfo.total_bytes}
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
                  ) : !busy ? (
                    <div className="glass-card p-8 text-center">
                      <div className="mx-auto mb-4 flex h-14 w-14 items-center justify-center rounded-full border border-aurora-border/50 bg-aurora-elevated/60 text-text-muted">
                        <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8">
                          <path d="M4 6h16M4 12h16M4 18h16" />
                        </svg>
                      </div>
                      <h3 className="text-base font-semibold text-text-primary">No scannable content</h3>
                      <p className="mt-2 text-sm text-text-muted">
                        DiskPulse found this drive, but there are no readable top-level folders to visualize.
                      </p>
                    </div>
                  ) : null}
                </>
              ) : !error && !remoteError && !busy ? (
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
            <div className="space-y-6">
              <CleanupPage report={riskReport} />
              {(riskReport || externalCleanupItems.length > 0) && (
                <div className="px-8 pb-8">
                  {externalCleanupItems.length > 0 && (
                    <div className="mb-4 rounded-2xl border border-accent/20 bg-accent/10 px-4 py-3 text-sm text-text-secondary">
                      <span className="font-semibold text-accent-light">
                        {externalCleanupItems.length} external candidate(s)
                      </span>{" "}
                      added to this safety preview.
                      <button
                        className="ml-3 text-text-muted hover:text-text-primary"
                        onClick={() => setExternalCleanupItems([])}
                      >
                        Clear
                      </button>
                    </div>
                  )}
                  <CleanupPreview
                    items={riskReport?.items ?? EMPTY_RISK_ITEMS}
                    additionalItems={externalCleanupItems}
                  />
                </div>
              )}
            </div>
          )}
          {activeTab === "large-files" && (
            <LargeFileFinder
              drives={drives}
              selectedDrive={selectedDrive}
              onAddToCleanup={handleAddToCleanup}
            />
          )}
          {activeTab === "duplicates" && (
            <DuplicateFinder
              drives={drives}
              selectedDrive={selectedDrive}
              onAddToCleanup={handleAddToCleanup}
            />
          )}
          {activeTab === "aging" && (
            <AgingAnalysis
              drives={drives}
              selectedDrive={selectedDrive}
              onAddToCleanup={handleAddToCleanup}
            />
          )}
          {activeTab === "fragmentation" && <FragmentationView selectedDrive={selectedDrive} />}
          {activeTab === "wizard" && (
            <CleanupWizard selectedDrive={selectedDrive} onStartScan={(drive) => void startDriveScan(drive)} />
          )}
          {activeTab === "history" && <HistoryPage />}
          {activeTab === "settings" && <SettingsPage />}
          </Suspense>
        </div>
      </main>
    </div>
  );
}
