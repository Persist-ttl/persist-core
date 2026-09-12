//! `missing-ttl-extension`: a storage write on Persistent or Instance
//! storage with no corresponding `extend_ttl` call anywhere in the same
//! contract (i.e. the same Cargo package - see `PackageOps` below), so the
//! entry can silently archive.
//!
//! Persistent/Instance entries are archived once their TTL runs out; if
//! nothing in the contract ever extends it, the entry (and the funds or
//! state it represents) becomes unreachable without manual restoration.
//!
//! Storage helpers are very commonly factored into their own module
//! (`admin.rs`, `storage.rs`) as free functions, with the actual
//! `extend_ttl()` call centralized in the `#[contractimpl]` block in a
//! different file (`contract.rs`). So this detector must look across every
//! file in the same package, not just the file (or `impl` block) the write
//! happens to live in - see https://github.com/Persist-ttl/persist-core/issues/4.

use std::collections::HashSet;

use crate::model::{FileModel, OpKind, StorageKind};
use crate::report::{Finding, Severity};

use super::DetectorInfo;

pub const INFO: DetectorInfo = DetectorInfo {
    slug: "missing-ttl-extension",
    title: "Missing TTL extension",
    description: "A `.set()` on Persistent/Instance storage with no `extend_ttl` call \
                   anywhere in the same contract (package), so the entry can silently \
                   archive.",
    default_severity: "critical/high",
};

fn severity_for(kind: StorageKind) -> Severity {
    match kind {
        StorageKind::Persistent => Severity::Critical,
        StorageKind::Instance => Severity::High,
        StorageKind::Temporary => unreachable!("temporary storage is exempt from this detector"),
    }
}

/// One file's storage ops, as part of a larger package-wide scan.
pub struct FileOps<'a> {
    pub file_path: &'a str,
    pub model: &'a FileModel,
}

/// Runs the detector across every file that belongs to the same Cargo
/// package (see `crate::package::group_by_package`), so an `extend_ttl`
/// call in one file counts for a write in another.
pub fn detect_in_package(files: &[FileOps<'_>]) -> Vec<Finding> {
    let extended_kinds: HashSet<StorageKind> = files
        .iter()
        .flat_map(|f| f.model.ops.iter())
        .filter(|op| op.op == OpKind::ExtendTtl)
        .map(|op| op.kind)
        .collect();

    let mut findings = Vec::new();

    for file in files {
        for op in &file.model.ops {
            if op.op != OpKind::Set {
                continue;
            }
            if op.kind == StorageKind::Temporary {
                continue;
            }
            if extended_kinds.contains(&op.kind) {
                continue;
            }

            findings.push(Finding {
                detector: INFO.slug,
                severity: severity_for(op.kind),
                message: format!(
                    "`{}()` storage is written in `{}()` but no `extend_ttl` call for \
                     {} storage exists anywhere in this contract, so the entry can expire \
                     and be archived.",
                    op.kind.as_str(),
                    op.fn_name,
                    op.kind.as_str(),
                ),
                file: file.file_path.to_string(),
                line: op.line,
                column: op.column,
                suggestion: Some(format!(
                    "Call `.storage().{}().extend_ttl(..)` after writing this entry (or in a \
                     shared lifecycle helper called on every contract invocation).",
                    op.kind.as_str()
                )),
            });
        }
    }

    findings
}

/// Convenience wrapper for analyzing a single file in isolation (no
/// package context available), e.g. `analyze_source`.
pub fn detect(file_path: &str, model: &FileModel) -> Vec<Finding> {
    detect_in_package(&[FileOps { file_path, model }])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::build_model;

    fn findings_for(src: &str) -> Vec<Finding> {
        let file = syn::parse_str::<syn::File>(src).expect("test fixture must parse");
        let model = build_model(&file);
        detect("test.rs", &model)
    }

    #[test]
    fn fires_on_persistent_set_without_extend_ttl() {
        let src = r#"
            impl Contract {
                pub fn deposit(env: Env, user: Address, amount: i128) {
                    env.storage().persistent().set(&user, &amount);
                }
            }
        "#;
        let findings = findings_for(src);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Critical);
    }

    #[test]
    fn silent_when_extend_ttl_present_in_same_impl() {
        let src = r#"
            impl Contract {
                pub fn deposit(env: Env, user: Address, amount: i128) {
                    env.storage().persistent().set(&user, &amount);
                }

                pub fn bump(env: Env, user: Address) {
                    env.storage().persistent().extend_ttl(&user, 100, 200_000);
                }
            }
        "#;
        assert!(findings_for(src).is_empty());
    }

    #[test]
    fn silent_for_temporary_storage() {
        let src = r#"
            impl Contract {
                pub fn cache(env: Env, key: Symbol, val: u32) {
                    env.storage().temporary().set(&key, &val);
                }
            }
        "#;
        assert!(findings_for(src).is_empty());
    }

    // Regression test for https://github.com/Persist-ttl/persist-core/issues/4:
    // storage helpers live in one file (e.g. admin.rs) as free functions,
    // the extend_ttl call lives in a sibling file's #[contractimpl] block
    // (e.g. contract.rs). Scanning them together as one package must not
    // flag the write as unprotected.
    #[test]
    fn silent_when_extend_ttl_present_in_sibling_file_same_package() {
        let admin_src = r#"
            pub fn write_administrator(e: &Env, id: &Address) {
                e.storage().instance().set(&DataKey::Admin, id);
            }
        "#;
        let contract_src = r#"
            impl Contract {
                pub fn initialize(e: Env) {
                    e.storage().instance().extend_ttl(100, 200_000);
                }
            }
        "#;

        let admin_file = syn::parse_str::<syn::File>(admin_src).expect("admin.rs must parse");
        let admin_model = build_model(&admin_file);
        let contract_file =
            syn::parse_str::<syn::File>(contract_src).expect("contract.rs must parse");
        let contract_model = build_model(&contract_file);

        let files = [
            FileOps {
                file_path: "admin.rs",
                model: &admin_model,
            },
            FileOps {
                file_path: "contract.rs",
                model: &contract_model,
            },
        ];

        assert!(detect_in_package(&files).is_empty());
    }

    #[test]
    fn still_fires_when_no_file_in_the_package_extends_ttl() {
        let admin_src = r#"
            pub fn write_administrator(e: &Env, id: &Address) {
                e.storage().instance().set(&DataKey::Admin, id);
            }
        "#;
        let other_src = r#"
            pub fn read_administrator(e: &Env) -> Address {
                e.storage().instance().get(&DataKey::Admin).unwrap()
            }
        "#;

        let admin_file = syn::parse_str::<syn::File>(admin_src).expect("admin.rs must parse");
        let admin_model = build_model(&admin_file);
        let other_file = syn::parse_str::<syn::File>(other_src).expect("other.rs must parse");
        let other_model = build_model(&other_file);

        let files = [
            FileOps {
                file_path: "admin.rs",
                model: &admin_model,
            },
            FileOps {
                file_path: "other.rs",
                model: &other_model,
            },
        ];

        let findings = detect_in_package(&files);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].file, "admin.rs");
    }
}
