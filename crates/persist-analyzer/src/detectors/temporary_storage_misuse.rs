//! `temporary-storage-misuse`: data is read from Temporary storage for a
//! key that's never (re)written to Temporary storage anywhere else in the
//! contract. Temporary entries are permanently deleted (not archived) once
//! their TTL expires, so a read path that assumes the value persists is a
//! genuine correctness bug, not just a lifecycle inconvenience.

use std::collections::HashSet;

use crate::model::{FileModel, OpKind, StorageKind};
use crate::report::{Finding, Severity};

use super::DetectorInfo;

pub const INFO: DetectorInfo = DetectorInfo {
    slug: "temporary-storage-misuse",
    title: "Temporary storage misuse",
    description: "Temporary storage is read for a key that's never written back to \
                   Temporary storage anywhere in the contract, suggesting the code assumes \
                   persistence Temporary storage doesn't provide.",
    default_severity: "medium/high",
};

fn looks_like_a_read_facing_fn(fn_name: &str) -> bool {
    let lower = fn_name.to_lowercase();
    lower.starts_with("get") || lower.starts_with("view") || lower.starts_with("query")
}

pub fn detect(file_path: &str, model: &FileModel) -> Vec<Finding> {
    let written_keys: HashSet<&str> = model
        .ops
        .iter()
        .filter(|op| op.kind == StorageKind::Temporary && op.op == OpKind::Set)
        .filter_map(|op| op.key_repr.as_deref())
        .collect();

    let mut findings = Vec::new();

    for op in &model.ops {
        if op.kind != StorageKind::Temporary || op.op != OpKind::Get {
            continue;
        }
        let Some(key) = op.key_repr.as_deref() else {
            continue;
        };
        if written_keys.contains(key) {
            continue;
        }

        let severity = if looks_like_a_read_facing_fn(&op.fn_name) {
            Severity::High
        } else {
            Severity::Medium
        };

        findings.push(Finding {
            detector: INFO.slug,
            severity,
            message: format!(
                "`{}()` reads Temporary storage for key `{}`, but nothing in this contract \
                 ever writes that key back to Temporary storage - once it expires it's gone \
                 for good, not archived.",
                op.fn_name, key,
            ),
            file: file_path.to_string(),
            line: op.line,
            column: op.column,
            suggestion: Some(
                "Either re-write this key on every access that should keep it alive, or move \
                 it to Persistent storage if it's meant to survive between calls."
                    .to_string(),
            ),
        });
    }

    findings
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
    fn fires_when_key_is_never_rewritten() {
        let src = r#"
            impl Contract {
                pub fn get_session(env: Env, id: Symbol) -> u32 {
                    env.storage().temporary().get(&id).unwrap_or(0)
                }
            }
        "#;
        let findings = findings_for(src);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn silent_when_key_is_rewritten_elsewhere() {
        let src = r#"
            impl Contract {
                pub fn touch(env: Env, id: Symbol) {
                    env.storage().temporary().set(&id, &1u32);
                }

                pub fn get_session(env: Env, id: Symbol) -> u32 {
                    env.storage().temporary().get(&id).unwrap_or(0)
                }
            }
        "#;
        assert!(findings_for(src).is_empty());
    }
}
