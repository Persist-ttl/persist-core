//! `insufficient-ttl-margin`: an `extend_ttl` call whose `extend_to` value
//! is a hardcoded ledger count below a configurable safety threshold. A
//! margin that's too small means the entry has to be bumped almost every
//! call to avoid archival, and a single missed call cycle can archive it.

use syn::{Expr, ExprLit, Lit};

use crate::model::{FileModel, OpKind, StorageKind};
use crate::report::{Finding, Severity};

use super::{Config, DetectorInfo};

pub const INFO: DetectorInfo = DetectorInfo {
    slug: "insufficient-ttl-margin",
    title: "Insufficient TTL margin",
    description: "An `extend_ttl` call whose extension value is a hardcoded ledger count \
                   below a safe threshold.",
    default_severity: "medium",
};

fn literal_ledger_count(expr: &Expr) -> Option<i64> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Int(lit_int),
        ..
    }) = expr
    {
        return lit_int.base10_parse::<i64>().ok();
    }
    None
}

/// `extend_ttl` on Persistent/Temporary storage takes `(key, threshold,
/// extend_to)`; Instance storage takes `(threshold, extend_to)` since it's
/// tied to the contract instance rather than a specific key.
fn extend_to_arg(kind: StorageKind, args: &[Expr]) -> Option<&Expr> {
    match kind {
        StorageKind::Instance => args.get(1),
        StorageKind::Persistent | StorageKind::Temporary => args.get(2),
    }
}

pub fn detect(file_path: &str, model: &FileModel, config: &Config) -> Vec<Finding> {
    let mut findings = Vec::new();

    for op in &model.ops {
        if op.op != OpKind::ExtendTtl {
            continue;
        }
        let Some(extend_to_expr) = extend_to_arg(op.kind, &op.args) else {
            continue;
        };
        let Some(value) = literal_ledger_count(extend_to_expr) else {
            // Not a literal (named constant / computed expression) - we
            // can't evaluate it statically, so we stay silent rather than
            // risk a false positive.
            continue;
        };

        if value < config.min_ttl_threshold {
            findings.push(Finding {
                detector: INFO.slug,
                severity: Severity::Medium,
                message: format!(
                    "`extend_ttl` in `{}()` extends {} storage to only {} ledgers, below the \
                     configured safety threshold of {} ledgers.",
                    op.fn_name,
                    op.kind.as_str(),
                    value,
                    config.min_ttl_threshold,
                ),
                file: file_path.to_string(),
                line: op.line,
                column: op.column,
                suggestion: Some(format!(
                    "Use an extension value of at least {} ledgers, or make it configurable \
                     instead of hardcoding a small constant.",
                    config.min_ttl_threshold
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

    fn findings_for(src: &str, config: &Config) -> Vec<Finding> {
        let file = syn::parse_str::<syn::File>(src).expect("test fixture must parse");
        let model = build_model(&file);
        detect("test.rs", &model, config)
    }

    #[test]
    fn fires_on_small_persistent_extension() {
        let src = r#"
            impl Contract {
                pub fn bump(env: Env, user: Address) {
                    env.storage().persistent().extend_ttl(&user, 10, 50);
                }
            }
        "#;
        let findings = findings_for(src, &Config::default());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Medium);
    }

    #[test]
    fn silent_on_generous_extension() {
        let src = r#"
            impl Contract {
                pub fn bump(env: Env, user: Address) {
                    env.storage().persistent().extend_ttl(&user, 100_000, 500_000);
                }
            }
        "#;
        assert!(findings_for(src, &Config::default()).is_empty());
    }

    #[test]
    fn silent_when_value_is_not_a_literal() {
        let src = r#"
            impl Contract {
                pub fn bump(env: Env, user: Address) {
                    env.storage().persistent().extend_ttl(&user, THRESHOLD, EXTEND_TO);
                }
            }
        "#;
        assert!(findings_for(src, &Config::default()).is_empty());
    }
}
