#[derive(Debug, Clone, PartialEq)]
pub enum CliCommand {
    Scan {
        drive: String,
    },
    Duplicates {
        drive: String,
    },
    Health {
        drive: String,
    },
    CleanLow {
        drive: String,
    },
    Export {
        drive: String,
        format: String,
        report_type: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CliOptions {
    pub json: bool,
    pub quiet: bool,
    pub dry_run: bool,
    pub command: Option<CliCommand>,
}

#[derive(Debug, serde::Serialize)]
struct CliCleanPreview {
    drive: String,
    dry_run: bool,
    candidate_count: usize,
    estimated_bytes: u64,
    items: Vec<crate::cleaner::CleanItem>,
}

struct CliCleanOutcome {
    output: String,
    exit_code: i32,
}

pub fn parse_cli_args(args: &[String]) -> Result<Option<CliOptions>, String> {
    if args.get(1).map(String::as_str) != Some("--cli") {
        return Ok(None);
    }
    let json = args.iter().any(|arg| arg == "--json");
    let quiet = args.iter().any(|arg| arg == "--quiet");
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let positional = args
        .iter()
        .skip(2)
        .filter(|arg| !arg.starts_with("--"))
        .cloned()
        .collect::<Vec<_>>();

    let command = match positional.first().map(String::as_str) {
        Some("scan") => Ok(CliCommand::Scan {
            drive: positional_arg(&positional, 1, "drive")?,
        }),
        Some("duplicates") => Ok(CliCommand::Duplicates {
            drive: positional_arg(&positional, 1, "drive")?,
        }),
        Some("health") => Ok(CliCommand::Health {
            drive: positional_arg(&positional, 1, "drive")?,
        }),
        Some("clean") => Ok(CliCommand::CleanLow {
            drive: positional_arg(&positional, 1, "drive")?,
        }),
        Some("export") => Ok(CliCommand::Export {
            drive: positional_arg(&positional, 1, "drive")?,
            format: positional_arg(&positional, 2, "format")?,
            report_type: positional_arg(&positional, 3, "type")?,
        }),
        Some(other) => Err(format!("Unknown CLI command: {}", other)),
        None => Err("Missing CLI command".into()),
    }?;
    Ok(Some(CliOptions {
        json,
        quiet,
        dry_run,
        command: Some(command),
    }))
}

pub fn execute_cli_command(options: &CliOptions) -> i32 {
    let Some(command) = &options.command else {
        return 3;
    };
    if let CliCommand::CleanLow { drive } = command {
        return match execute_clean_low(drive, options) {
            Ok(outcome) => {
                if !options.quiet {
                    println!("{}", outcome.output);
                }
                outcome.exit_code
            }
            Err(err) => {
                eprintln!("{}", err);
                2
            }
        };
    }
    let result = match command {
        CliCommand::Scan { drive } => crate::scanner::scan_drive_meta(drive, None, None)
            .and_then(|value| render(&value, options)),
        CliCommand::Duplicates { drive } => {
            crate::duplicates::scan_duplicates_with_progress_and_cancel(
                drive,
                1_000_000,
                |_| {},
                None,
            )
            .and_then(|value| render(&value, options))
        }
        CliCommand::Health { drive } => {
            crate::recommendations::get_disk_health(drive).and_then(|value| render(&value, options))
        }
        CliCommand::CleanLow { .. } => unreachable!("clean handled before generic CLI dispatch"),
        CliCommand::Export {
            drive,
            format,
            report_type,
        } => match report_type.as_str() {
            "scan" => crate::report::export_scan_report(drive, format)
                .and_then(|value| render(&value, options)),
            "cleanup" => crate::report::export_cleanup_history(format)
                .and_then(|value| render(&value, options)),
            "duplicates" => crate::report::export_duplicates(drive, format)
                .and_then(|value| render(&value, options)),
            other => Err(format!("Unsupported export type: {}", other)),
        },
    };

    match result {
        Ok(output) => {
            if !options.quiet {
                println!("{}", output);
            }
            0
        }
        Err(err) => {
            eprintln!("{}", err);
            if matches!(
                &options.command,
                Some(CliCommand::Scan { .. })
                    | Some(CliCommand::Health { .. })
                    | Some(CliCommand::CleanLow { .. })
            ) {
                2
            } else {
                1
            }
        }
    }
}

fn execute_clean_low(drive: &str, options: &CliOptions) -> Result<CliCleanOutcome, String> {
    let scan = crate::scanner::scan_drive(drive)?;
    let report = crate::risk::classify_risks(&scan);
    let items = report
        .items
        .iter()
        .filter(|item| item.safe_to_delete && item.risk_level == crate::risk::RiskLevel::Low)
        .map(crate::cleaner::CleanItem::from)
        .collect::<Vec<_>>();

    if options.dry_run {
        let preview = crate::cleaner::preview_cleanup(items);
        let payload = CliCleanPreview {
            drive: drive.to_uppercase(),
            dry_run: true,
            candidate_count: preview.accepted.len(),
            estimated_bytes: preview.validation.total_bytes,
            items: preview.accepted,
        };
        render(&payload, options).map(|output| CliCleanOutcome {
            output,
            exit_code: 0,
        })
    } else {
        let result = crate::cleaner::clean_items_with_progress(items, None, |_| {});
        let exit_code = if result.failed > 0 { 1 } else { 0 };
        render(&result, options).map(|output| CliCleanOutcome { output, exit_code })
    }
}

fn render<T: serde::Serialize + std::fmt::Debug>(
    value: &T,
    options: &CliOptions,
) -> Result<String, String> {
    if options.json {
        serde_json::to_string_pretty(value).map_err(|e| format!("CLI JSON error: {}", e))
    } else {
        Ok(format!("{:?}", value))
    }
}

fn positional_arg(args: &[String], index: usize, name: &str) -> Result<String, String> {
    args.get(index)
        .cloned()
        .ok_or_else(|| format!("Missing required argument: {}", name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scan_cli_command() {
        let args = vec![
            "diskpulse".into(),
            "--cli".into(),
            "scan".into(),
            "C".into(),
        ];
        assert_eq!(
            parse_cli_args(&args).unwrap(),
            Some(CliOptions {
                json: false,
                quiet: false,
                dry_run: false,
                command: Some(CliCommand::Scan { drive: "C".into() })
            })
        );
    }

    #[test]
    fn ignores_non_cli_startup() {
        let args = vec!["diskpulse".into()];
        assert_eq!(parse_cli_args(&args).unwrap(), None);
    }

    #[test]
    fn parse_json_health_cli_command() {
        let args = vec![
            "diskpulse".into(),
            "--cli".into(),
            "health".into(),
            "D".into(),
            "--json".into(),
        ];
        let parsed = parse_cli_args(&args).unwrap().expect("cli options");
        assert!(parsed.json);
        assert!(!parsed.dry_run);
        assert_eq!(
            parsed.command,
            Some(CliCommand::Health { drive: "D".into() })
        );
    }

    #[test]
    fn parse_export_cli_command_includes_drive() {
        let args = vec![
            "diskpulse".into(),
            "--cli".into(),
            "export".into(),
            "D".into(),
            "json".into(),
            "duplicates".into(),
        ];

        assert_eq!(
            parse_cli_args(&args).unwrap().expect("cli options").command,
            Some(CliCommand::Export {
                drive: "D".into(),
                format: "json".into(),
                report_type: "duplicates".into()
            })
        );
    }

    #[test]
    fn parse_clean_dry_run_cli_command() {
        let args = vec![
            "diskpulse".into(),
            "--cli".into(),
            "clean".into(),
            "C".into(),
            "--dry-run".into(),
        ];

        let parsed = parse_cli_args(&args).unwrap().expect("cli options");
        assert!(parsed.dry_run);
        assert_eq!(
            parsed.command,
            Some(CliCommand::CleanLow { drive: "C".into() })
        );
    }
}
