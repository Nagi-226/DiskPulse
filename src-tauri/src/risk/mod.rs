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
        let lower_name = normalize_match_text(&dir.name);
        let lower_path = normalize_match_text(&dir.path);

        if let Some(ref name_pat) = self.name_match {
            let pat = normalize_match_text(name_pat);
            if lower_name == pat || lower_name.contains(&pat) {
                return true;
            }
        }

        self.patterns.iter().any(|pattern| {
            let pat = normalize_match_text(pattern);
            lower_path.contains(&pat) || lower_name.contains(&pat)
        })
    }
}

/// Default risk ruleset — embedded in binary, can be overridden later.
fn default_rules() -> Vec<RiskRule> {
    vec![
        RiskRule {
            id: "temp-files".into(),
            patterns: vec!["temp".into(), "tmp".into()],
            risk_level: RiskLevel::Low,
            category: "temporary_files".into(),
            explanation: "Temporary files — safe to delete, automatically recreated as needed".into(),
            safe_to_delete: true,
            name_match: None,
        },
        RiskRule {
            id: "browser-cache".into(),
            patterns: vec!["chrome/user data".into(), "firefox/profiles".into(), "edge/user data".into()],
            risk_level: RiskLevel::Low,
            category: "browser_cache".into(),
            explanation: "Browser cache files — safe to delete, will be re-downloaded".into(),
            safe_to_delete: true,
            name_match: Some("/cache".into()),
        },
        RiskRule {
            id: "nvidia-dx-cache".into(),
            patterns: vec!["nvidia/dxcache".into(), "amd/dxcache".into()],
            risk_level: RiskLevel::Low,
            category: "gpu_cache".into(),
            explanation: "GPU shader cache — safe to delete, rebuilt on next game launch".into(),
            safe_to_delete: true,
            name_match: Some("/dxcache".into()),
        },
        RiskRule {
            id: "npm-cache".into(),
            patterns: vec![".npm".into(), "npm-cache".into()],
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
            name_match: Some("pip".into()),
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
            patterns: vec!["deliveryoptimization".into()],
            risk_level: RiskLevel::Low,
            category: "windows_cache".into(),
            explanation: "Windows Delivery Optimization files — safe to delete".into(),
            safe_to_delete: true,
            name_match: Some("deliveryoptimization".into()),
        },
        RiskRule {
            id: "recycle-bin".into(),
            patterns: vec!["$recycle.bin".into()],
            risk_level: RiskLevel::Low,
            category: "recycle_bin".into(),
            explanation: "Recycle Bin contents — already marked for deletion by user".into(),
            safe_to_delete: true,
            name_match: Some("$recycle.bin".into()),
        },
        RiskRule {
            id: "downloads-old".into(),
            patterns: vec!["downloads".into(), "download".into()],
            risk_level: RiskLevel::Medium,
            category: "downloads".into(),
            explanation: "Downloads folder — review contents before deleting".into(),
            safe_to_delete: false,
            name_match: Some("downloads".into()),
        },
        RiskRule {
            id: "large-logs".into(),
            patterns: vec!["/logs".into(), "\\logs".into(), "logs".into()],
            risk_level: RiskLevel::Medium,
            category: "log_files".into(),
            explanation: "Log files — review before deleting, may be needed for debugging".into(),
            safe_to_delete: false,
            name_match: Some("log".into()),
        },
        RiskRule {
            id: "winsxs".into(),
            patterns: vec!["winsxs".into()],
            risk_level: RiskLevel::Medium,
            category: "windows_components".into(),
            explanation: "Windows component store — use DISM cleanup, not manual deletion".into(),
            safe_to_delete: false,
            name_match: Some("winsxs".into()),
        },
        RiskRule {
            id: "windows-installer".into(),
            patterns: vec!["windows/installer".into()],
            risk_level: RiskLevel::High,
            category: "system_installer".into(),
            explanation: "Windows Installer database — DO NOT DELETE, required for uninstalls".into(),
            safe_to_delete: false,
            name_match: Some("installer".into()),
        },
        RiskRule {
            id: "windows-system".into(),
            patterns: vec!["windows/system32".into(), "windows/syswow64".into()],
            risk_level: RiskLevel::High,
            category: "system_critical".into(),
            explanation: "Windows system directory — NEVER delete, OS will break".into(),
            safe_to_delete: false,
            name_match: None,
        },
        RiskRule {
            id: "program-files".into(),
            patterns: vec!["program files (x86)".into(), "program files".into()],
            risk_level: RiskLevel::High,
            category: "installed_applications".into(),
            explanation: "Installed applications — only remove via Settings > Apps".into(),
            safe_to_delete: false,
            name_match: None,
        },
        RiskRule {
            id: "wechat-qq-data".into(),
            patterns: vec!["wechat files".into(), "tencent files".into(), "qq files".into()],
            risk_level: RiskLevel::High,
            category: "chat_user_data".into(),
            explanation: "Chat application user data — contains personal files and history".into(),
            safe_to_delete: false,
            name_match: None,
        },
        RiskRule {
            id: "appdata-config".into(),
            patterns: vec!["appdata/roaming".into(), "appdata/local".into()],
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

    let mut summary = RiskSummary {
        total_items: items.len(),
        low_risk_count: 0,
        medium_risk_count: 0,
        high_risk_count: 0,
        low_risk_bytes: 0,
        medium_risk_bytes: 0,
        high_risk_bytes: 0,
        safe_deletable_bytes: 0,
    };
    for item in &items {
        match item.risk_level {
            RiskLevel::Low => {
                summary.low_risk_count += 1;
                summary.low_risk_bytes += item.size_bytes;
            }
            RiskLevel::Medium => {
                summary.medium_risk_count += 1;
                summary.medium_risk_bytes += item.size_bytes;
            }
            RiskLevel::High => {
                summary.high_risk_count += 1;
                summary.high_risk_bytes += item.size_bytes;
            }
        }
        if item.safe_to_delete {
            summary.safe_deletable_bytes += item.size_bytes;
        }
    }

    RiskReport {
        drive_letter: drive_info.drive_letter.clone(),
        items,
        summary,
    }
}

fn match_rule(dir: &DirInfo, rules: &[RiskRule]) -> (RiskLevel, String, String, bool) {
    // Check for developer project detection (medium risk)
    if is_developer_project(&dir.path) {
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

/// Check if a path looks like an active developer project.
fn is_developer_project(path: &str) -> bool {
    let lower = normalize_match_text(path);
    lower.contains("/.git")
        || lower.contains("/cargo.toml")
        || lower.contains("/package.json")
        || lower.contains("/pyproject.toml")
        || lower.contains("/go.mod")
        || lower.contains("/node_modules")
        || lower.contains("/target")
}

fn normalize_match_text(input: &str) -> String {
    input
        .replace('\\', "/")
        .to_lowercase()
        .trim()
        .to_string()
}

/// Get all rules with user overrides applied.
pub fn get_rules_with_overrides(
    overrides: &std::collections::HashMap<String, bool>,
) -> Vec<RiskRule> {
    default_rules()
        .into_iter()
        .map(|mut rule| {
            if let Some(&safe) = overrides.get(&rule.id) {
                rule.safe_to_delete = safe;
            }
            rule
        })
        .collect()
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
