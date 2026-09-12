use crate::model::FileModel;
use crate::report::Finding;

pub mod insufficient_ttl_margin;
pub mod missing_ttl_extension;
pub mod no_archival_handling;
pub mod storage_type_mismatch;
pub mod temporary_storage_misuse;

/// Configuration knobs shared across detectors.
#[derive(Debug, Clone)]
pub struct Config {
    /// Below this many ledgers, an `extend_ttl` call is considered too
    /// small to be a meaningful safety margin. Defaults to roughly one day
    /// of ledgers at Stellar's ~5s average close time.
    pub min_ttl_threshold: i64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            min_ttl_threshold: 17_280,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DetectorInfo {
    pub slug: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub default_severity: &'static str,
}

pub const ALL_DETECTORS: &[DetectorInfo] = &[
    missing_ttl_extension::INFO,
    insufficient_ttl_margin::INFO,
    temporary_storage_misuse::INFO,
    no_archival_handling::INFO,
    storage_type_mismatch::INFO,
];

/// Runs every detector against one parsed file's model and returns the
/// combined, unsorted findings.
pub fn run_all(file_path: &str, model: &FileModel, config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(missing_ttl_extension::detect(file_path, model));
    findings.extend(insufficient_ttl_margin::detect(file_path, model, config));
    findings.extend(temporary_storage_misuse::detect(file_path, model));
    findings.extend(no_archival_handling::detect(file_path, model));
    findings.extend(storage_type_mismatch::detect(file_path, model));
    findings
}
