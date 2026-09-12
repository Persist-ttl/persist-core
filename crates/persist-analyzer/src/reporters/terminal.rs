use owo_colors::OwoColorize;

use crate::report::{Finding, Report, Severity};

fn severity_label(sev: Severity) -> String {
    match sev {
        Severity::Critical => "CRITICAL".red().bold().to_string(),
        Severity::High => "HIGH".red().to_string(),
        Severity::Medium => "MEDIUM".yellow().to_string(),
        Severity::Low => "LOW".blue().to_string(),
    }
}

fn render_finding(f: &Finding) -> String {
    let mut out = format!(
        "{} {}:{}:{}  [{}]\n  {}\n",
        severity_label(f.severity),
        f.file,
        f.line,
        f.column,
        f.detector,
        f.message,
    );
    if let Some(suggestion) = &f.suggestion {
        out.push_str(&format!("  {} {}\n", "help:".green(), suggestion));
    }
    out
}

pub fn render(report: &Report) -> String {
    let mut out = String::new();

    if report.findings.is_empty() {
        out.push_str(&format!("{}\n", "No storage-lifecycle issues found.".green().bold()));
    } else {
        for finding in &report.findings {
            out.push('\n');
            out.push_str(&render_finding(finding));
        }
        out.push('\n');
    }

    out.push_str(&format!(
        "{}: {} critical, {} high, {} medium, {} low\n",
        "Summary".bold(),
        report.critical_count,
        report.high_count,
        report.medium_count,
        report.low_count,
    ));

    let score_colored = match report.score {
        90..=100 => report.score.to_string().green().bold().to_string(),
        70..=89 => report.score.to_string().yellow().bold().to_string(),
        _ => report.score.to_string().red().bold().to_string(),
    };
    out.push_str(&format!("Security score: {score_colored}/100\n"));

    out
}
