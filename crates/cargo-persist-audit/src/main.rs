use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use persist_analyzer::{analyze_path, Config, OutputFormat, Severity};

/// `cargo persist-audit` - static analysis for Soroban storage-lifecycle
/// (TTL/archival) bugs.
#[derive(Parser, Debug)]
#[command(name = "cargo-persist-audit", version, about)]
struct Args {
    /// Path to a Soroban contract source file or a directory to scan recursively.
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Output format: text, json, md, sarif
    #[arg(long, default_value = "text")]
    format: String,

    /// Write output to a file instead of stdout.
    #[arg(long)]
    output: Option<PathBuf>,

    /// Minimum severity that causes a non-zero exit code: critical, high, medium, low, none.
    #[arg(long, default_value = "critical")]
    fail_on: String,

    /// Ledger-count threshold below which an `extend_ttl` call is flagged as too small.
    #[arg(long, default_value_t = 17_280)]
    min_ttl_threshold: i64,
}

fn parse_fail_on(value: &str) -> Result<Option<Severity>, String> {
    match value.to_lowercase().as_str() {
        "none" => Ok(None),
        "critical" => Ok(Some(Severity::Critical)),
        "high" => Ok(Some(Severity::High)),
        "medium" => Ok(Some(Severity::Medium)),
        "low" => Ok(Some(Severity::Low)),
        other => Err(format!(
            "unknown --fail-on value `{other}` (expected one of: critical, high, medium, low, none)"
        )),
    }
}

fn main() -> ExitCode {
    // Support both direct invocation (`cargo-persist-audit <args>`) and the
    // cargo-subcommand form (`cargo persist-audit <args>`), where cargo
    // inserts an extra "persist-audit" token as argv[1].
    let mut raw_args: Vec<String> = std::env::args().collect();
    if raw_args.get(1).map(String::as_str) == Some("persist-audit") {
        raw_args.remove(1);
    }

    let args = Args::parse_from(raw_args);

    let format: OutputFormat = match args.format.parse() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    let fail_on = match parse_fail_on(&args.fail_on) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    let config = Config {
        min_ttl_threshold: args.min_ttl_threshold,
    };

    let report = match analyze_path(&args.path, &config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    let rendered = persist_analyzer::reporters::render(format, &report);

    match &args.output {
        Some(output_path) => {
            if let Err(e) = std::fs::write(output_path, &rendered) {
                eprintln!("error: could not write {}: {e}", output_path.display());
                return ExitCode::from(2);
            }
        }
        None => println!("{rendered}"),
    }

    if let Some(threshold) = fail_on {
        if report.has_severity_at_or_above(threshold) {
            return ExitCode::from(1);
        }
    }

    ExitCode::SUCCESS
}
