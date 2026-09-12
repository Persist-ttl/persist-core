//! `missing-ttl-extension`: a storage write on Persistent or Instance
//! storage with no corresponding `extend_ttl` call anywhere in the same
//! function or the same `impl` block (a good proxy for "the same contract's
//! lifecycle-related functions").
//!
//! Persistent/Instance entries are archived once their TTL runs out; if
//! nothing in the contract ever extends it, the entry (and the funds or
//! state it represents) becomes unreachable without manual restoration.

use std::collections::{HashMap, HashSet};

use crate::model::{FileModel, OpKind, StorageKind};
use crate::report::{Finding, Severity};

use super::DetectorInfo;

pub const INFO: DetectorInfo = DetectorInfo {
    slug: "missing-ttl-extension",
    title: "Missing TTL extension",
    description: "A `.set()` on Persistent/Instance storage with no `extend_ttl` call \
                   anywhere in the same function or impl block, so the entry can silently \
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

pub fn detect(file_path: &str, model: &FileModel) -> Vec<Finding> {
    // Group ops by their enclosing impl block (or a synthetic "free
    // functions" bucket), since an extend_ttl call in a sibling lifecycle
    // function (e.g. a dedicated `bump()` method) should still count.
    let mut by_group: HashMap<String, Vec<&crate::model::StorageOp>> = HashMap::new();
    for op in &model.ops {
        let group = op.impl_name.clone().unwrap_or_else(|| "<free-functions>".to_string());
        by_group.entry(group).or_default().push(op);
    }

    let mut findings = Vec::new();

    for ops in by_group.values() {
        let extended_kinds: HashSet<StorageKind> = ops
            .iter()
            .filter(|op| op.op == OpKind::ExtendTtl)
            .map(|op| op.kind)
            .collect();

        for op in ops.iter() {
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
                file: file_path.to_string(),
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
}
