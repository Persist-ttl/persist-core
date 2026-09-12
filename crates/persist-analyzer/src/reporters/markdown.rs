use crate::report::Report;

fn severity_badge(sev: crate::report::Severity) -> &'static str {
    match sev {
        crate::report::Severity::Critical => "🔴 Critical",
        crate::report::Severity::High => "🟠 High",
        crate::report::Severity::Medium => "🟡 Medium",
        crate::report::Severity::Low => "🔵 Low",
    }
}

/// Renders a Markdown report shaped for a GitHub PR comment.
pub fn render(report: &Report) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "## Persist security report - score {}/100\n\n",
        report.score
    ));

    out.push_str("| Severity | Count |\n|---|---|\n");
    out.push_str(&format!("| 🔴 Critical | {} |\n", report.critical_count));
    out.push_str(&format!("| 🟠 High | {} |\n", report.high_count));
    out.push_str(&format!("| 🟡 Medium | {} |\n", report.medium_count));
    out.push_str(&format!("| 🔵 Low | {} |\n\n", report.low_count));

    if report.findings.is_empty() {
        out.push_str("No storage-lifecycle issues found. ✅\n");
        return out;
    }

    out.push_str("| Severity | Detector | Location | Message |\n|---|---|---|---|\n");
    for f in &report.findings {
        out.push_str(&format!(
            "| {} | `{}` | `{}:{}` | {} |\n",
            severity_badge(f.severity),
            f.detector,
            f.file,
            f.line,
            f.message.replace('|', "\\|"),
        ));
    }

    out
}
