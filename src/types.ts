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

export interface ScanProgress {
  drive_letter: string;
  processed: number;
  total: number;
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

export type CleanItemStatus = "Success" | "Skipped" | "Failed";

export interface CleanItemResult {
  path: string;
  name: string;
  size_bytes: number;
  status: CleanItemStatus;
  reason: string | null;
  original_path: string | null;
}

export interface RestoreItemResult {
  original_path: string;
  restored: boolean;
  reason: string | null;
}

export interface RestoreResult {
  attempted: number;
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

export interface CleanResult {
  total_attempted: number;
  succeeded: number;
  skipped: number;
  failed: number;
  freed_bytes: number;
  items: CleanItemResult[];
}
