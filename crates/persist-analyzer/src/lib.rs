//! Static analysis engine for Soroban storage-lifecycle (TTL/archival)
//! bugs. See the crate README for the problem statement and the detector
//! catalog.

pub mod detectors;
pub mod model;
pub mod report;
pub mod reporters;

use std::path::Path;

pub use detectors::Config;
pub use report::{Finding, Report, Severity};
pub use reporters::OutputFormat;

#[derive(Debug)]
pub enum AnalyzeError {
    Io(std::io::Error),
    Parse { file: String, error: String },
}

impl std::fmt::Display for AnalyzeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalyzeError::Io(e) => write!(f, "I/O error: {e}"),
            AnalyzeError::Parse { file, error } => write!(f, "failed to parse {file}: {error}"),
        }
    }
}

impl std::error::Error for AnalyzeError {}

impl From<std::io::Error> for AnalyzeError {
    fn from(e: std::io::Error) -> Self {
        AnalyzeError::Io(e)
    }
}

/// Parses one file's source and runs every detector against it.
pub fn analyze_source(
    file_path: &str,
    source: &str,
    config: &Config,
) -> Result<Vec<Finding>, AnalyzeError> {
    let file = syn::parse_file(source).map_err(|e| AnalyzeError::Parse {
        file: file_path.to_string(),
        error: e.to_string(),
    })?;
    let model = model::build_model(&file);
    Ok(detectors::run_all(file_path, &model, config))
}

/// Analyzes a single file or recursively walks a directory for `*.rs`
/// files (skipping `target/` build output), returning a combined report.
pub fn analyze_path(path: &Path, config: &Config) -> Result<Report, AnalyzeError> {
    let mut findings = Vec::new();

    if path.is_file() {
        let source = std::fs::read_to_string(path)?;
        findings.extend(analyze_source(
            &path.display().to_string(),
            &source,
            config,
        )?);
        return Ok(Report::from_findings(findings));
    }

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_entry(|e| e.file_name() != "target")
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if p.is_file() && p.extension().is_some_and(|ext| ext == "rs") {
            let source = std::fs::read_to_string(p)?;
            match analyze_source(&p.display().to_string(), &source, config) {
                Ok(f) => findings.extend(f),
                Err(AnalyzeError::Parse { file, error }) => {
                    eprintln!("warning: skipping {file}: {error}");
                }
                Err(e) => return Err(e),
            }
        }
    }

    Ok(Report::from_findings(findings))
}
