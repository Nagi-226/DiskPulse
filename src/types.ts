export interface DirInfo {
  name: string;
  path: string;
  size_bytes: number;
  file_count: number;
  dir_count: number;
  risk_level: string | null;
  is_approximate: boolean;
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

export type StorageKind = "removable" | "fixed_external" | "network" | "optical" | "unknown";

export interface ExternalStorageInfo {
  id: string;
  name: string;
  mount_path: string;
  filesystem: string;
  total_bytes: number;
  free_bytes: number;
  removable: boolean;
  kind: StorageKind;
  platform: string;
  detection_source: string;
}

export interface StorageEvent {
  event_type: "attached" | "detached";
  storage: ExternalStorageInfo | null;
  mount_path: string | null;
  raw_code: number | null;
  source: string;
}

export interface ModelStatus {
  ae_model_version: string;
  classifier_model_version: string;
  snapshots_available: number;
  min_snapshots_required: number;
  can_fine_tune: boolean;
  fine_tuned: boolean;
  auc_score: number;
  classifier_accuracy: number;
  message: string;
}

export interface RelayStatus {
  connected: boolean;
  url: string | null;
  device_id: string | null;
  device_name: string | null;
  peer_count: number;
  last_error: string | null;
  mode: string;
}

export interface CloudDevice {
  device_id: string;
  name: string;
  relay_url: string;
  connected: boolean;
  last_seen_epoch_ms: number;
}

export interface RelayEnvelope {
  id: string;
  from_device_id: string;
  to_device_id: string | null;
  kind: string;
  payload: unknown;
}

export type ScanPhase = "walking" | "measuring" | "streaming" | "complete";

export interface ScanProgress {
  drive_letter: string;
  processed: number;
  total: number;
  current_path: string | null;
  phase: ScanPhase;
  partial_results: DirInfo[] | null;
}

export interface ScanBatch {
  dirs: DirInfo[];
  batch_index: number;
  is_complete: boolean;
}

export interface FileEntry {
  name: string;
  path: string;
  size_bytes: number;
  modified_epoch_ms: number;
  hard_link_count: number;
  size_on_disk_bytes: number | null;
  file_category?: string | null;
}

export interface LargeFileProgress {
  drive_letter: string;
  dirs_processed: number;
  dirs_total: number;
  files_found: number;
  current_path: string | null;
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
  unsafe_items: CleanItemResult[];
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

export interface RestoreItemResult {
  original_path: string;
  restored: boolean;
  reason: string | null;
}

export interface RestoreResult {
  restored: number;
  failed: number;
  items: RestoreItemResult[];
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
  seasonal_component: number;
  trend_component: number;
  dynamic_confidence_interval: [number, number] | null;
  status: "insufficient_data" | "stable" | "growing" | "shrinking" | "warning" | "critical";
  message: string;
  forecast: ForecastPoint[];
}

export type AnomalyType =
  | "rate_anomaly"
  | "burst_anomaly"
  | "hotspot_anomaly"
  | "pattern_deviation";

export type AnomalySeverity = "warning" | "critical";

export interface AnomalyEvent {
  anomaly_type: AnomalyType;
  severity: AnomalySeverity;
  drive_letter: string;
  created_at: string;
  metric_value: number;
  modified_z_score: number;
  description: string;
  path: string | null;
}

export interface AnomalyReport {
  drive_letter: string;
  sample_count: number;
  events: AnomalyEvent[];
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
  scan_mode: "exact" | "speed";
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
  auto_cleanup_enabled: boolean;
  auto_cleanup_frequency: string;
  auto_cleanup_time: string;
  auto_cleanup_risk_levels: string;
  auto_cleanup_min_free_gb: number;
  language: string;
  theme: string;
  update_check_enabled: boolean;
  scoring_weight_risk: number;
  scoring_weight_age: number;
  scoring_weight_duplicate: number;
  scoring_weight_size: number;
  scoring_weight_safety: number;
  scoring_weight_urgency: number;
  scoring_weight_pattern: number;
  duplicate_min_size_bytes: number;
  aging_zombie_days: number;
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

export interface AutoCleanupStatus {
  enabled: boolean;
  running: boolean;
  drive_letter: string;
  frequency: string;
  next_run_epoch_ms: number | null;
  last_run_at: string | null;
  last_freed_bytes: number;
  message: string;
}

export interface AutoCleanupReport {
  id: number;
  drive_letter: string;
  freed_bytes: number;
  succeeded: number;
  skipped: number;
  failed: number;
  status: string;
  message: string;
  items_json: string;
  created_at: string;
}

export type ServiceState =
  | "not_installed"
  | "stopped"
  | "start_pending"
  | "running"
  | "stop_pending"
  | "paused"
  | "unknown";

export interface ServiceStatus {
  installed: boolean;
  state: ServiceState;
  message: string;
}

export interface RiskRule {
  id: string;
  patterns: string[];
  risk_level: RiskLevel;
  category: string;
  explanation: string;
  safe_to_delete: boolean;
  name_match: string | null;
  file_category?: string | null;
}

export interface CustomRule {
  id: string;
  name: string;
  pattern: string;
  risk_level: Extract<RiskLevel, "low" | "medium">;
  enabled: boolean;
}

export interface DuplicateScanProgress {
  drive_letter: string;
  phase: string;
  files_processed: number;
  groups_found: number;
  current_path: string | null;
  hard_link_count: number;
}

export interface DuplicateGroup {
  group_id: string;
  total_size_wasted: number;
  hard_link_count: number;
  files: FileEntry[];
}

export interface AgeBucket {
  id: string;
  label: string;
  min_days: number;
  max_days: number | null;
  total_bytes: number;
  file_count: number;
}

export interface Hotspot {
  path: string;
  recent_bytes: number;
  file_count: number;
}

export interface FileAge {
  path: string;
  age_days: number;
}

export interface AgingReport {
  drive_letter: string;
  buckets: AgeBucket[];
  zombies_total_size: number;
  zombie_files: FileEntry[];
  hotspots: Hotspot[];
  file_ages?: FileAge[];
}

export interface FileFragmentation {
  path: string;
  size_bytes: number;
  extent_count: number;
  cluster_size: number;
  fragmentation_ratio: number;
}

export interface FragmentationDirSummary {
  path: string;
  files_analyzed: number;
  average_fragmentation: number;
  max_fragmentation: number;
  fragmented_files: number;
  total_bytes: number;
}

export interface FragmentationReport {
  root_path: string;
  files_analyzed: number;
  total_files_seen: number;
  average_fragmentation: number;
  high_fragmentation_files: number;
  top_dirs: FragmentationDirSummary[];
  top_files: FileFragmentation[];
  sampled: boolean;
}

export interface DiskFullPrediction {
  drive_letter: string;
  days_to_full: number | null;
  optimistic_days: number | null;
  pessimistic_days: number | null;
  urgency: string;
  confidence_score: number;
}

export interface CleanupGain {
  drive_letter: string;
  freed_bytes: number;
  estimated_extra_days: number | null;
  new_urgency: string;
}

export interface AgingScanProgress {
  drive_letter: string;
  files_processed: number;
  buckets: AgeBucket[];
  current_path: string | null;
}

export interface RecommendationItem {
  name: string;
  path: string;
  category: string;
  size_bytes: number;
  risk_level: string;
  safe_to_delete: boolean;
}

export interface Recommendation {
  rank: number;
  item: RecommendationItem;
  score: number;
  reason: string;
  estimated_size: number;
  action: string;
  urgency_multiplier: number;
  pattern_boost: number;
  correlation_bonus: number;
  urgency_label: "normal" | "elevated" | "critical";
}

export interface DiskHealth {
  drive_letter: string;
  score: number;
  status: string;
  free_percent: number;
  duplicate_waste_bytes: number;
  zombie_bytes: number;
  space_score: number;
  waste_score: number;
  trend_score: number;
  age_score: number;
  trend_growth_percent_per_day: number;
  message: string;
}

export interface NotificationRecord {
  id: number;
  notification_type: string;
  title: string;
  message: string;
  read: boolean;
  created_at: string;
}

export interface PlatformSystemInfo {
  os_name: string;
  os_version: string;
  cpu_count: number;
  total_ram_bytes: number;
  app_data_dir: string;
}

export interface FileIdentity {
  volume_serial: number;
  file_index: number;
}

export interface FileMeta {
  path: string;
  hard_link_count: number;
  is_sparse: boolean;
  size_on_disk_bytes: number | null;
  identity: FileIdentity | null;
}

export interface DeviceInfo {
  device_id: string;
  name: string;
  address: string;
  paired: boolean;
  connected: boolean;
  last_seen_epoch_ms: number;
}

export interface PairingToken {
  code: string;
  device_name: string;
  expires_at_epoch_ms: number;
}

export interface RemoteAlertPayload {
  device_id: string;
  alert_payload: unknown;
}

export interface HubDiscoveryInfo {
  service_name: string;
  mode: "mdns" | "manual";
  port: number;
  address_hint: string;
}

export type DashboardDevice = { kind: "local" } | { kind: "remote"; device: DeviceInfo };

export interface RemoteDeviceRequest {
  id?: string;
  command: string;
  payload?: unknown;
}

export interface RemoteDeviceResponse<T = unknown> {
  id: string;
  ok: boolean;
  payload: T;
  error: string | null;
}
