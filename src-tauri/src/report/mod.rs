use crate::{db, duplicates, risk, scanner};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq)]
enum ReportFormat {
    Csv,
    Json,
}

impl ReportFormat {
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "json" => Some(Self::Json),
            _ => None,
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
        }
    }
}

pub fn export_scan_report(drive: &str, format: &str) -> Result<String, String> {
    let format = ReportFormat::parse(format).ok_or_else(|| "Unsupported format".to_string())?;
    let scan = scanner::scan_drive(drive)?;
    let risk_report = risk::classify_risks(&scan);
    let content = match format {
        ReportFormat::Json => serde_json::to_string_pretty(&risk_report)
            .map_err(|e| format!("Serialize report error: {}", e))?,
        ReportFormat::Csv => risk_report_to_csv(&risk_report),
    };
    write_report_file("scan-report", format, content)
}

pub fn export_cleanup_history(format: &str) -> Result<String, String> {
    let format = ReportFormat::parse(format).ok_or_else(|| "Unsupported format".to_string())?;
    let history = db::get_cleanup_history()?;
    let content = match format {
        ReportFormat::Json => json_pretty(&history)?,
        ReportFormat::Csv => cleanup_history_to_csv(&history),
    };
    write_report_file("cleanup-history", format, content)
}

pub fn export_duplicates(drive: &str, format: &str) -> Result<String, String> {
    let format = ReportFormat::parse(format).ok_or_else(|| "Unsupported format".to_string())?;
    let groups = duplicates::scan_duplicates_with_progress_and_cancel(drive, 1_000_000, |_| {}, None)?;
    let content = match format {
        ReportFormat::Json => json_pretty(&groups)?,
        ReportFormat::Csv => duplicates_to_csv(&groups),
    };
    write_report_file("duplicates", format, content)
}

fn json_pretty<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| format!("Serialize report error: {}", e))
}

fn risk_report_to_csv(report: &risk::RiskReport) -> String {
    let mut rows = vec!["name,path,size_bytes,risk_level,safe_to_delete,category".to_string()];
    for item in &report.items {
        rows.push(format!(
            "{},{},{},{},{},{}",
            csv_escape(&item.name),
            csv_escape(&item.path),
            item.size_bytes,
            item.risk_level,
            item.safe_to_delete,
            csv_escape(&item.category)
        ));
    }
    rows.join("\n")
}

fn cleanup_history_to_csv(history: &[db::CleanupLog]) -> String {
    let mut rows = vec!["id,created_at,item_count,freed_bytes,succeeded,skipped,failed".to_string()];
    for item in history {
        rows.push(format!(
            "{},{},{},{},{},{},{}",
            item.id,
            csv_escape(&item.created_at),
            item.item_count,
            item.freed_bytes,
            item.succeeded,
            item.skipped,
            item.failed
        ));
    }
    rows.join("\n")
}

fn duplicates_to_csv(groups: &[duplicates::DuplicateGroup]) -> String {
    let mut rows = vec!["group_id,path,size_bytes,wasted_group_bytes".to_string()];
    for group in groups {
        for file in &group.files {
            rows.push(format!(
                "{},{},{},{}",
                csv_escape(&group.group_id),
                csv_escape(&file.path),
                file.size_bytes,
                group.total_size_wasted
            ));
        }
    }
    rows.join("\n")
}

fn write_report_file(prefix: &str, format: ReportFormat, content: String) -> Result<String, String> {
    let mut path: PathBuf = std::env::temp_dir();
    path.push(format!(
        "diskpulse-{}-{}.{}",
        prefix,
        std::process::id(),
        format.extension()
    ));
    std::fs::write(&path, content).map_err(|e| format!("Write report error: {}", e))?;
    Ok(path.to_string_lossy().to_string())
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_escape_quotes_and_commas() {
        assert_eq!(csv_escape("a,b \"quoted\""), "\"a,b \"\"quoted\"\"\"");
    }

    #[test]
    fn report_format_parses_csv_and_json() {
        assert!(matches!(ReportFormat::parse("csv"), Some(ReportFormat::Csv)));
        assert!(matches!(ReportFormat::parse("json"), Some(ReportFormat::Json)));
        assert!(ReportFormat::parse("xml").is_none());
    }
}
