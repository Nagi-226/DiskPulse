use crate::scanner::{DirInfo, DriveInfo};
use serde::{Deserialize, Serialize};

/// Risk level for cleanup safety
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RiskLevel {
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "high")]
    High,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Low => write!(f, "low"),
            RiskLevel::Medium => write!(f, "medium"),
            RiskLevel::High => write!(f, "high"),
        }
    }
}

/// A classified item in the risk report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskItem {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub risk_level: RiskLevel,
    pub category: String,
    pub explanation: String,
    pub safe_to_delete: bool,
}

/// Summary statistics for a risk report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskSummary {
    pub total_items: usize,
    pub low_risk_count: usize,
    pub medium_risk_count: usize,
    pub high_risk_count: usize,
    pub low_risk_bytes: u64,
    pub medium_risk_bytes: u64,
    pub high_risk_bytes: u64,
    pub safe_deletable_bytes: u64,
}

/// Complete risk report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskReport {
    pub drive_letter: String,
    pub items: Vec<RiskItem>,
    pub summary: RiskSummary,
}

/// A risk classification rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskRule {
    pub id: String,
    pub patterns: Vec<String>,
    pub risk_level: RiskLevel,
    pub category: String,
    pub explanation: String,
    pub safe_to_delete: bool,
    pub name_match: Option<String>,
}

impl RiskRule {
    fn matches(&self, dir: &DirInfo) -> bool {
        let lower_name = dir.name.to_lowercase();
        let lower_path = dir.path.to_lowercase();

        // Check name match if specified
        if let Some(ref name_pat) = self.name_match {
            let pat = name_pat.to_lowercase();
            if lower_name == pat || lower_name.contains(&pat) {
                return true;
            }
        }

        // Check path patterns
        for pattern in &self.patterns {
            let pat = pattern.to_lowercase();
            // Support simple glob patterns
            if lower_path.contains(&pat.trim_start_matches("*").trim_end_matches("*"))
                || lower_name.contains(&pat)
            {
                return true;
            }
        }

        false
    }
}

/// Default risk ruleset — embedded in binary, can be overridden by external config
fn default_rules() -> Vec<RiskRule> {
    vec![
        // === LOW RISK — safe one-click cleanup ===
        RiskRule {
            id: "temp-files".into(),
            patterns: vec!["Temp".into(), "tmp".into()],
            risk_level: RiskLevel::Low,
            category: "temporary_files".into(),
            explanation: "Temporary files — safe to delete, automatically recreated as needed".into(),
            safe_to_delete: true,
            name_match: Some("Temp".into()),
        },
        RiskRule {
            id: "browser-cache".into(),
            patterns: vec![
                "Google/Chrome/User Data/*/Cache".into(),
                "Mozilla/Firefox/Profiles/*/cache2".into(),
                "Microsoft/Edge/User Data/*/Cache".into(),
            ],
            risk_level: RiskLevel::Low,
            category: "browser_cache".into(),
            explanation: "Browser cache files — safe to delete, will be re-downloaded".into(),
            safe_to_delete: true,
            name_match: None,
        },
        RiskRule {
            id: "nvidia-dx-cache".into(),
            patterns: vec!["NVIDIA/DXCache".into(), "AMD/DxCache".into()],
            risk_level: RiskLevel::Low,
            category: "gpu_cache".into(),
            explanation: "GPU shader cache — safe to delete, rebuilt on next game launch".into(),
            safe_to_delete: true,
            name_match: Some("DXCache".into()),
        },
        RiskRule {
            id: "npm-cache".into(),
            patterns: vec!["npm-cache".into(), ".npm".into()],
            risk_level: RiskLevel::Low,
            category: "developer_cache".into(),
            explanation: "npm package cache — safe to delete, re-downloaded on next install".into(),
            safe_to_delete: true,
            name_match: Some("npm-cache".into()),
        },
        RiskRule {
            id: "pip-cache".into(),
            patterns: vec!["pip/cache".into(), ".cache/pip".into()],
            risk_level: RiskLevel::Low,
            category: "developer_cache".into(),
            explanation: "pip package cache — safe to delete, re-downloaded on next install".into(),
            safe_to_delete: true,
            name_match: None,
        },
        RiskRule {
            id: "cargo-cache".into(),
            patterns: vec![".cargo/registry".into(), ".cargo/git".into()],
            risk_level: RiskLevel::Low,
            category: "developer_cache".into(),
            explanation: "Cargo registry cache — safe to delete, re-downloaded on next build".into(),
            safe_to_delete: true,
            name_match: Some(".cargo".into()),
        },
        RiskRule {
            id: "delivery-optimization".into(),
            patterns: vec!["DeliveryOptimization".into()],
            risk_level: RiskLevel::Low,
            category: "windows_cache".into(),
            explanation: "Windows Delivery Optimization files — safe to delete".into(),
            safe_to_delete: true,
            name_match: Some("DeliveryOptimization".into()),
        },
        RiskRule {
            id: "recycle-bin".into(),
            patterns: vec!["$Recycle.Bin".into()],
            risk_level: RiskLevel::Low,
            category: "recycle_bin".into(),
            explanation: "Recycle Bin contents — already marked for deletion by user".into(),
            safe_to_delete: true,
            name_match: Some("$Recycle.Bin".into()),
        },
        // === MEDIUM RISK — confirm before cleanup ===
        RiskRule {
            id: "downloads-old".into(),
            patterns: vec![
                "Downloads".into(),
                "Download".into(),
            ],
            risk_level: RiskLevel::Medium,
            category: "downloads".into(),
            explanation: "Downloads folder — review contents before deleting".into(),
            safe_to_delete: false,
            name_match: Some("Downloads".into()),
        },
        RiskRule {
            id: "large-logs".into(),
            patterns: vec!["logs".into(), "Logs".into()],
            risk_level: RiskLevel::Medium,
            category: "log_files".into(),
            explanation: "Log files — review before deleting, may be needed for debugging".into(),
            safe_to_delete: false,
            name_match: None,
        },
        RiskRule {
            id: "winsxs".into(),
            patterns: vec!["WinSxS".into()],
            risk_level: RiskLevel::Medium,
            category: "windows_components".into(),
            explanation: "Windows component store — use DISM cleanup, not manual deletion".into(),
            safe_to_delete: false,
            name_match: Some("WinSxS".into()),
        },
        // === HIGH RISK — display only, NEVER delete ===
        RiskRule {
            id: "windows-installer".into(),
            patterns: vec!["Windows/Installer".into()],
            risk_level: RiskLevel::High,
            category: "system_installer".into(),
            explanation: "Windows Installer database — DO NOT DELETE, required for uninstalls".into(),
            safe_to_delete: false,
            name_match: Some("Installer".into()),
        },
        RiskRule {
            id: "windows-system".into(),
            patterns: vec!["Windows/System32".into(), "Windows/SysWOW64".into()],
            risk_level: RiskLevel::High,
            category: "system_critical".into(),
            explanation: "Windows system directory — NEVER delete, OS will break".into(),
            safe_to_delete: false,
            name_match: None,
        },
        RiskRule {
            id: "program-files".into(),
            patterns: vec!["Program Files".into(), "Program Files (x86)".into()],
            risk_level: RiskLevel::High,
            category: "installed_applications".into(),
            explanation: "Installed applications — only remove via Settings > Apps".into(),
            safe_to_delete: false,
            name_match: None,
        },
        RiskRule {
            id: "wechat-qq-data".into(),
            patterns: vec![
                "WeChat Files".into(),
                "Tencent Files".into(),
                "QQ Files".into(),
            ],
            risk_level: RiskLevel::High,
            category: "chat_user_data".into(),
            explanation: "Chat application user data — contains personal files and history".into(),
            safe_to_delete: false,
            name_match: None,
        },
        RiskRule {
            id: "appdata-config".into(),
            patterns: vec!["AppData/Roaming".into(), "AppData/Local".into()],
            risk_level: RiskLevel::High,
            category: "user_config".into(),
            explanation: "Application configuration — contains user settings and data".into(),
            safe_to_delete: false,
            name_match: None,
        },
    ]
}

/// Classify a scan result into risk levels
pub fn classify_risks(drive_info: &DriveInfo) -> RiskReport {
    let rules = default_rules();
    let mut items: Vec<RiskItem> = Vec::new();

    for dir in &drive_info.top_dirs {
        let (risk_level, category, explanation, safe_to_delete) =
            match_rule(dir, &rules);

        items.push(RiskItem {
            name: dir.name.clone(),
            path: dir.path.clone(),
            size_bytes: dir.size_bytes,
            file_count: dir.file_count,
            dir_count: dir.dir_count,
            risk_level,
            category,
            explanation,
            safe_to_delete,
        });
    }

    // Sort: high risk first (for visibility), then by size
    items.sort_by(|a, b| {
        let risk_order = |r: &RiskLevel| match r {
            RiskLevel::High => 0,
            RiskLevel::Medium => 1,
            RiskLevel::Low => 2,
        };
        risk_order(&a.risk_level)
            .cmp(&risk_order(&b.risk_level))
            .then(b.size_bytes.cmp(&a.size_bytes))
    });

    let summary = RiskSummary {
        total_items: items.len(),
        low_risk_count: items.iter().filter(|i| i.risk_level == RiskLevel::Low).count(),
        medium_risk_count: items.iter().filter(|i| i.risk_level == RiskLevel::Medium).count(),
        high_risk_count: items.iter().filter(|i| i.risk_level == RiskLevel::High).count(),
        low_risk_bytes: items.iter().filter(|i| i.risk_level == RiskLevel::Low).map(|i| i.size_bytes).sum(),
        medium_risk_bytes: items.iter().filter(|i| i.risk_level == RiskLevel::Medium).map(|i| i.size_bytes).sum(),
        high_risk_bytes: items.iter().filter(|i| i.risk_level == RiskLevel::High).map(|i| i.size_bytes).sum(),
        safe_deletable_bytes: items.iter().filter(|i| i.safe_to_delete).map(|i| i.size_bytes).sum(),
    };

    RiskReport {
        drive_letter: drive_info.drive_letter.clone(),
        items,
        summary,
    }
}

fn match_rule(dir: &DirInfo, rules: &[RiskRule]) -> (RiskLevel, String, String, bool) {
    // Check for developer project detection (medium risk)
    if is_developer_project(&dir.name) {
        return (
            RiskLevel::Medium,
            "developer_project".into(),
            "Active developer project detected — contains .git or package files".into(),
            false,
        );
    }

    for rule in rules {
        if rule.matches(dir) {
            return (
                rule.risk_level.clone(),
                rule.category.clone(),
                rule.explanation.clone(),
                rule.safe_to_delete,
            );
        }
    }

    // Default: unknown directories are medium risk
    (
        RiskLevel::Medium,
        "unknown".into(),
        "Unknown directory — review before taking action".into(),
        false,
    )
}

/// Check if a path looks like an active developer project
fn is_developer_project(_name: &str) -> bool {
    // This is a heuristic — the full implementation in v0.0.6 will
    // check for .git, Cargo.toml, package.json, etc. inside the directory.
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::DirInfo;

    fn make_dir(name: &str, path: &str) -> DirInfo {
        DirInfo {
            name: name.into(),
            path: path.into(),
            size_bytes: 1_000_000_000,
            file_count: 100,
            dir_count: 10,
            risk_level: None,
        }
    }

    #[test]
    fn test_temp_is_low_risk() {
        let dir = make_dir("Temp", "C:\\Windows\\Temp");
        let rules = default_rules();
        let (level, category, _, safe) = match_rule(&dir, &rules);
        assert_eq!(level, RiskLevel::Low);
        assert_eq!(category, "temporary_files");
        assert!(safe);
    }

    #[test]
    fn test_windows_is_high_risk() {
        // Program Files match
        let dir = make_dir("Program Files", "C:\\Program Files");
        let rules = default_rules();
        let (level, category, _, safe) = match_rule(&dir, &rules);
        assert_eq!(level, RiskLevel::High);
        assert_eq!(category, "installed_applications");
        assert!(!safe);
    }

    #[test]
    fn test_npm_cache_is_low_risk() {
        let dir = make_dir("npm-cache", "C:\\Users\\user\\AppData\\Local\\npm-cache");
        let rules = default_rules();
        let (level, _, _, safe) = match_rule(&dir, &rules);
        assert_eq!(level, RiskLevel::Low);
        assert!(safe);
    }

    #[test]
    fn test_classify_risks_generates_report() {
        let drive = DriveInfo {
            drive_letter: "C".into(),
            total_bytes: 500_000_000_000,
            used_bytes: 300_000_000_000,
            free_bytes: 200_000_000_000,
            top_dirs: vec![
                make_dir("Windows", "C:\\Windows"),
                make_dir("Temp", "C:\\Windows\\Temp"),
                make_dir("Users", "C:\\Users"),
                make_dir("Program Files", "C:\\Program Files"),
            ],
        };
        let report = classify_risks(&drive);
        assert_eq!(report.summary.total_items, 4);
        assert!(report.summary.high_risk_count >= 1);
        assert!(report.summary.low_risk_count >= 1);
        assert!(report.summary.safe_deletable_bytes > 0);
    }

    #[test]
    fn test_risk_level_serialization() {
        let level = RiskLevel::Low;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, r#""low""#);

        let parsed: RiskLevel = serde_json::from_str(r#""high""#).unwrap();
        assert_eq!(parsed, RiskLevel::High);
    }
}
