//! Static analysis engine for Soroban storage-lifecycle (TTL/archival)
//! bugs. See the crate README for the problem statement and the detector
//! catalog.

pub mod detectors;
pub mod model;
pub mod report;
pub mod reporters;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use detectors::Config;
pub use report::{Finding, Report, Severity};
pub use reporters::OutputFormat;

/// The nearest ancestor directory containing a `Cargo.toml`, used to scope
/// cross-file detectors like `missing-ttl-extension` to "the same contract"
/// (see https://github.com/Persist-ttl/persist-core/issues/4). Falls back
/// to `None` (caller treats the file as its own singleton group) when no
/// package manifest is found anywhere above it.
fn package_root_for(file: &Path) -> Option<PathBuf> {
    let mut dir = file.parent();
    while let Some(d) = dir {
        if d.join("Cargo.toml").is_file() {
            return Some(d.to_path_buf());
        }
        dir = d.parent();
    }
    None
}

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

/// Parses one file's source and runs every detector against it, including
/// `missing-ttl-extension` scoped to just this one file (there's no wider
/// package context available for a single in-memory source string).
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
    let mut findings = detectors::run_all(file_path, &model, config);
    findings.extend(detectors::missing_ttl_extension::detect(file_path, &model));
    Ok(findings)
}

/// Analyzes a single file or recursively walks a directory for `*.rs`
/// files (skipping `target/` build output), returning a combined report.
///
/// `missing-ttl-extension` is run once per Cargo package (not once per
/// file), so an `extend_ttl` call in one file counts for a write in a
/// sibling file of the same contract.
pub fn analyze_path(path: &Path, config: &Config) -> Result<Report, AnalyzeError> {
    if path.is_file() {
        let source = std::fs::read_to_string(path)?;
        let findings = analyze_source(&path.display().to_string(), &source, config)?;
        return Ok(Report::from_findings(findings));
    }

    // Phase 1: parse every eligible file into its own model.
    let mut file_paths: Vec<PathBuf> = Vec::new();
    let mut file_models: Vec<model::FileModel> = Vec::new();

    for entry in walkdir::WalkDir::new(path)
        .into_iter()
        .filter_entry(|e| e.file_name() != "target")
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if !p.is_file() || !p.extension().is_some_and(|ext| ext == "rs") {
            continue;
        }

        let source = std::fs::read_to_string(p)?;
        match syn::parse_file(&source) {
            Ok(file) => {
                file_models.push(model::build_model(&file));
                file_paths.push(p.to_path_buf());
            }
            Err(e) => eprintln!("warning: skipping {}: {e}", p.display()),
        }
    }

    let display_paths: Vec<String> = file_paths.iter().map(|p| p.display().to_string()).collect();

    // Phase 2: run the single-file detectors independently for each file.
    let mut findings = Vec::new();
    for (i, model) in file_models.iter().enumerate() {
        findings.extend(detectors::run_all(&display_paths[i], model, config));
    }

    // Phase 3: group files by Cargo package and run `missing-ttl-extension`
    // once per package, so a write in one file and its `extend_ttl` in a
    // sibling file (a very common Soroban layout) are seen together.
    let mut by_package: HashMap<PathBuf, Vec<usize>> = HashMap::new();
    for (i, file_path) in file_paths.iter().enumerate() {
        let key = package_root_for(file_path).unwrap_or_else(|| file_path.clone());
        by_package.entry(key).or_default().push(i);
    }

    for indices in by_package.values() {
        let file_ops: Vec<detectors::missing_ttl_extension::FileOps<'_>> = indices
            .iter()
            .map(|&i| detectors::missing_ttl_extension::FileOps {
                file_path: &display_paths[i],
                model: &file_models[i],
            })
            .collect();
        findings.extend(detectors::missing_ttl_extension::detect_in_package(
            &file_ops,
        ));
    }

    Ok(Report::from_findings(findings))
}
