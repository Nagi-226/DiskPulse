#[derive(Debug, Clone, PartialEq)]
pub enum CliCommand {
    Scan { drive: String },
    Duplicates { drive: String },
    Health { drive: String },
    CleanLow { drive: String },
    Export { format: String, report_type: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CliOptions {
    pub json: bool,
    pub quiet: bool,
    pub command: Option<CliCommand>,
}

pub fn parse_cli_args(args: &[String]) -> Result<Option<CliOptions>, String> {
    if args.get(1).map(String::as_str) != Some("--cli") {
        return Ok(None);
    }
    let json = args.iter().any(|arg| arg == "--json");
    let quiet = args.iter().any(|arg| arg == "--quiet");
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
            format: positional_arg(&positional, 1, "format")?,
            report_type: positional_arg(&positional, 2, "type")?,
        }),
        Some(other) => Err(format!("Unknown CLI command: {}", other)),
        None => Err("Missing CLI command".into()),
    }?;
    Ok(Some(CliOptions {
        json,
        quiet,
        command: Some(command),
    }))
}

pub fn execute_cli_command(options: &CliOptions) -> i32 {
    let Some(command) = &options.command else {
        return 3;
    };
    let result = match command {
        CliCommand::Scan { drive } => crate::scanner::scan_drive_meta(drive, None, None)
            .and_then(|value| render(&value, options)),
        CliCommand::Duplicates { drive } => crate::duplicates::scan_duplicates_with_progress_and_cancel(
            drive,
            1_000_000,
            |_| {},
            None,
        )
        .and_then(|value| render(&value, options)),
        CliCommand::Health { drive } => {
            crate::recommendations::get_disk_health(drive).and_then(|value| render(&value, options))
        }
        CliCommand::CleanLow { .. } => Err("CLI cleanup execution is disabled until release hardening completes".into()),
        CliCommand::Export {
            format,
            report_type,
        } => match report_type.as_str() {
            "scan" => crate::report::export_scan_report("C", format).and_then(|value| render(&value, options)),
            "cleanup" => crate::report::export_cleanup_history(format).and_then(|value| render(&value, options)),
            "duplicates" => crate::report::export_duplicates("C", format).and_then(|value| render(&value, options)),
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
            1
        }
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
        let args = vec!["diskpulse".into(), "--cli".into(), "scan".into(), "C".into()];
        assert_eq!(
            parse_cli_args(&args).unwrap(),
            Some(CliOptions {
                json: false,
                quiet: false,
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
        assert_eq!(parsed.command, Some(CliCommand::Health { drive: "D".into() }));
    }
}
