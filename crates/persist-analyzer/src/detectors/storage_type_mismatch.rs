//! `storage-type-mismatch`: a keyword heuristic flagging storage keys that
//! look like contract-wide config/admin data but aren't stored as Instance
//! storage (or vice versa: per-user-looking data stored as Instance).
//!
//! This is the most heuristic of the five detectors - it only looks at
//! identifier names, so it's kept at Low/Medium severity and documented as
//! a hint rather than a hard finding.

use crate::model::{FileModel, OpKind, StorageKind};
use crate::report::{Finding, Severity};

use super::DetectorInfo;

pub const INFO: DetectorInfo = DetectorInfo {
    slug: "storage-type-mismatch",
    title: "Storage type mismatch",
    description: "A storage key's name suggests contract-wide admin/config data stored \
                   outside Instance storage, or per-user data stored as Instance storage.",
    default_severity: "low/medium",
};

const ADMIN_LIKE: &[&str] = &[
    "admin",
    "config",
    "owner",
    "paused",
    "initialized",
    "fee",
    "treasury",
    "governance",
];

const USER_LIKE: &[&str] = &["balance", "allowance", "user", "account", "holder", "nonce"];

fn contains_keyword(haystack: &str, keywords: &'static [&'static str]) -> Option<&'static str> {
    let lower = haystack.to_lowercase();
    keywords.iter().find(|kw| lower.contains(*kw)).copied()
}

pub fn detect(file_path: &str, model: &FileModel) -> Vec<Finding> {
    let mut findings = Vec::new();

    for op in &model.ops {
        if !matches!(op.op, OpKind::Set | OpKind::Get) {
            continue;
        }
        let Some(key) = op.key_repr.as_deref() else {
            continue;
        };

        match op.kind {
            StorageKind::Persistent | StorageKind::Temporary => {
                if let Some(kw) = contains_keyword(key, ADMIN_LIKE) {
                    findings.push(Finding {
                        detector: INFO.slug,
                        severity: Severity::Medium,
                        message: format!(
                            "Key `{}` in `{}()` looks like contract-wide config data (matched \
                             `{}`) but is stored as {} storage, not Instance storage.",
                            key,
                            op.fn_name,
                            kw,
                            op.kind.as_str(),
                        ),
                        file: file_path.to_string(),
                        line: op.line,
                        column: op.column,
                        suggestion: Some(
                            "Contract-wide settings usually belong in Instance storage, so \
                             their TTL is tied to the contract instance itself."
                                .to_string(),
                        ),
                    });
                }
            }
            StorageKind::Instance => {
                if let Some(kw) = contains_keyword(key, USER_LIKE) {
                    findings.push(Finding {
                        detector: INFO.slug,
                        severity: Severity::Low,
                        message: format!(
                            "Key `{}` in `{}()` looks like per-user data (matched `{}`) but is \
                             stored in Instance storage, which doesn't scale per-user and ties \
                             every user's data to one shared TTL.",
                            key, op.fn_name, kw,
                        ),
                        file: file_path.to_string(),
                        line: op.line,
                        column: op.column,
                        suggestion: Some(
                            "Per-user data usually belongs in Persistent storage, keyed by the \
                             user's address."
                                .to_string(),
                        ),
                    });
                }
            }
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
    fn fires_on_admin_key_in_persistent_storage() {
        let src = r#"
            impl Contract {
                pub fn set_admin(env: Env, admin_key: Address) {
                    env.storage().persistent().set(&ADMIN, &admin_key);
                }
            }
        "#;
        let findings = findings_for(src);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Medium);
    }

    #[test]
    fn fires_on_user_key_in_instance_storage() {
        let src = r#"
            impl Contract {
                pub fn set_balance(env: Env, user: Address, amount: i128) {
                    env.storage().instance().set(&BALANCE, &amount);
                }
            }
        "#;
        let findings = findings_for(src);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Low);
    }

    #[test]
    fn silent_on_admin_key_in_instance_storage() {
        let src = r#"
            impl Contract {
                pub fn set_admin(env: Env, admin_key: Address) {
                    env.storage().instance().set(&ADMIN, &admin_key);
                }
            }
        "#;
        assert!(findings_for(src).is_empty());
    }
}
