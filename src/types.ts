export interface DirInfo {
  name: string;
  path: string;
  size_bytes: number;
  file_count: number;
  dir_count: number;
  risk_level: string | null;
}

export interface DriveInfo {
  drive_letter: string;
  total_bytes: number;
  used_bytes: number;
  free_bytes: number;
  top_dirs: DirInfo[];
}

export interface DriveMeta {
  drive_letter: string;
  total_bytes: number;
  used_bytes: number;
  free_bytes: number;
  cached_top_dirs: DirInfo[] | null;
  cache_age_ms: number | null;
}

export type ScanPhase = "walking" | "measuring" | "complete";

export interface ScanProgress {
  drive_letter: string;
  processed: number;
  total: number;
  current_path: string | null;
  phase: ScanPhase;
  partial_results: DirInfo[] | null;
}

export type RiskLevel = "low" | "medium" | "high";

export interface RiskItem {
  name: string;
  path: string;
  size_bytes: number;
  file_count: number;
  dir_count: number;
  risk_level: RiskLevel;
  category: string;
  explanation: string;
  safe_to_delete: boolean;
}

export interface RiskSummary {
  total_items: number;
  low_risk_count: number;
  medium_risk_count: number;
  high_risk_count: number;
  low_risk_bytes: number;
  medium_risk_bytes: number;
  high_risk_bytes: number;
  safe_deletable_bytes: number;
}

export interface RiskReport {
  drive_letter: string;
  items: RiskItem[];
  summary: RiskSummary;
}

export interface CleanItem {
  name: string;
  path: string;
  size_bytes: number;
  risk_level: RiskLevel;
  safe_to_delete: boolean;
}

export interface CleanValidationResult {
  allowed: boolean;
  valid_items: number;
  blocked_items: number;
  total_bytes: number;
  blocked_reason: string | null;
}

export interface CleanPreview {
  accepted: CleanItem[];
  blocked: CleanItem[];
  validation: CleanValidationResult;
}

export interface CleanExecutionResult {
  attempted: number;
  executed: number;
  blocked: number;
  total_bytes: number;
  messages: string[];
}

export interface CleanItemResult {
  path: string;
  name: string;
  size_bytes: number;
  status: "Success" | "Skipped" | "Failed";
  reason: string | null;
  original_path: string | null;
}

export interface CleanResult {
  total_attempted: number;
  succeeded: number;
  skipped: number;
  failed: number;
  freed_bytes: number;
  items: CleanItemResult[];
}

export interface CleanProgress {
  current: number;
  total: number;
  current_item: string | null;
  status: string | null;
}

export interface Snapshot {
  id: number;
  drive_letter: string;
  total_bytes: number;
  used_bytes: number;
  free_bytes: number;
  snapshot_json: string;
  created_at: string;
}

export interface ForecastPoint {
  created_at: string;
  used_bytes: number;
  free_bytes: number;
  usage_percent: number;
  is_forecast: boolean;
}

export interface Prediction {
  drive_letter: string;
  sample_count: number;
  window_days: number;
  current_usage_percent: number;
  growth_bytes_per_day: number;
  growth_percent_per_day: number;
  days_to_95_percent: number | null;
  projected_95_date: string | null;
  confidence_score: number;
  status: "insufficient_data" | "stable" | "growing" | "shrinking" | "warning" | "critical";
  message: string;
  forecast: ForecastPoint[];
}

export interface CleanupLog {
  id: number;
  created_at: string;
  item_count: number;
  freed_bytes: number;
  succeeded: number;
  skipped: number;
  failed: number;
  items_json: string;
}

export interface AppSettings {
  default_drive: string;
  auto_scan_on_startup: boolean;
  auto_monitor_on_startup: boolean;
  watcher_poll_interval_ms: number;
  watcher_debounce_ms: number;
  alert_enabled: boolean;
  alert_threshold_type: string;
  alert_threshold_value: number;
  alert_growth_enabled: boolean;
  alert_growth_percent: number;
  alert_growth_minutes: number;
}

export interface DiskSpaceAlertPayload {
  alert_type: string;
  drive_letter: string;
  message: string;
  free_bytes: number;
  total_bytes: number;
  usage_percent: number;
  timestamp_ms: number;
}

export interface RiskRule {
  id: string;
  patterns: string[];
  risk_level: RiskLevel;
  category: string;
  explanation: string;
  safe_to_delete: boolean;
  name_match: string | null;
}
