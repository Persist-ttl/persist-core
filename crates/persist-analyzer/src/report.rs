use serde::Serialize;

/// Severity of a finding. Ordered from most to least severe so that
/// `Vec<Finding>` can be sorted with the default `Ord` impl.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl Severity {
    /// Points deducted from the 0-100 security score for one finding of
    /// this severity.
    pub fn score_weight(self) -> u32 {
        match self {
            Severity::Critical => 25,
            Severity::High => 15,
            Severity::Medium => 8,
            Severity::Low => 3,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
        }
    }

    /// SARIF `level` for this severity (SARIF only has error/warning/note).
    pub fn sarif_level(self) -> &'static str {
        match self {
            Severity::Critical | Severity::High => "error",
            Severity::Medium => "warning",
            Severity::Low => "note",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single detector finding.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    /// Detector slug, e.g. `"missing-ttl-extension"`.
    pub detector: &'static str,
    pub severity: Severity,
    pub message: String,
    /// Path to the source file the finding was raised in, as given on the
    /// command line (may be relative or absolute).
    pub file: String,
    pub line: usize,
    pub column: usize,
    /// Optional human-readable remediation hint.
    pub suggestion: Option<String>,
}

/// The result of scanning one or more files: every finding plus the
/// derived security score.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub findings: Vec<Finding>,
    /// 0-100 security score, 100 being no findings at all.
    pub score: u32,
    pub critical_count: u32,
    pub high_count: u32,
    pub medium_count: u32,
    pub low_count: u32,
}

impl Report {
    pub fn from_findings(mut findings: Vec<Finding>) -> Self {
        findings.sort_by(|a, b| {
            a.severity
                .cmp(&b.severity)
                .then_with(|| a.file.cmp(&b.file))
                .then_with(|| a.line.cmp(&b.line))
        });

        let mut critical_count = 0;
        let mut high_count = 0;
        let mut medium_count = 0;
        let mut low_count = 0;
        let mut deductions: u32 = 0;

        for finding in &findings {
            deductions += finding.severity.score_weight();
            match finding.severity {
                Severity::Critical => critical_count += 1,
                Severity::High => high_count += 1,
                Severity::Medium => medium_count += 1,
                Severity::Low => low_count += 1,
            }
        }

        let score = 100u32.saturating_sub(deductions);

        Report {
            findings,
            score,
            critical_count,
            high_count,
            medium_count,
            low_count,
        }
    }

    pub fn has_severity_at_or_above(&self, min: Severity) -> bool {
        self.findings.iter().any(|f| f.severity <= min)
    }
}
