//! `no-archival-handling`: a storage read (`.get()`) whose result is
//! immediately `.unwrap()`ed or `.expect()`ed, with no branch for the
//! "entry does not exist / was archived" case. `.get()` returns `Option<T>`
//! precisely because the entry may be missing or archived; unwrapping it
//! directly turns that into a contract panic.

use crate::model::FileModel;
use crate::report::{Finding, Severity};

use super::DetectorInfo;

pub const INFO: DetectorInfo = DetectorInfo {
    slug: "no-archival-handling",
    title: "No archival handling",
    description: "A storage `.get()` result is immediately `.unwrap()`/`.expect()`-ed with no \
                   handling for a missing or archived entry.",
    default_severity: "high",
};

pub fn detect(file_path: &str, model: &FileModel) -> Vec<Finding> {
    model
        .unwrapped_gets
        .iter()
        .map(|hit| Finding {
            detector: INFO.slug,
            severity: Severity::High,
            message: format!(
                "`{}()` calls `.storage().{}().get(..).{}()` with no handling for a missing \
                 or archived entry - this will panic the contract instead of failing \
                 gracefully.",
                hit.fn_name,
                hit.kind.as_str(),
                hit.via,
            ),
            file: file_path.to_string(),
            line: hit.line,
            column: hit.column,
            suggestion: Some(
                "Use `if let Some(value) = ...get(..)`, `.unwrap_or(..)`/`.unwrap_or_else(..)`, \
                 or return an explicit error instead of unwrapping directly."
                    .to_string(),
            ),
        })
        .collect()
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
    fn fires_on_unwrap() {
        let src = r#"
            impl Contract {
                pub fn balance_of(env: Env, user: Address) -> i128 {
                    env.storage().persistent().get(&user).unwrap()
                }
            }
        "#;
        let findings = findings_for(src);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn fires_on_expect() {
        let src = r#"
            impl Contract {
                pub fn balance_of(env: Env, user: Address) -> i128 {
                    env.storage().persistent().get(&user).expect("no balance")
                }
            }
        "#;
        assert_eq!(findings_for(src).len(), 1);
    }

    #[test]
    fn silent_on_unwrap_or() {
        let src = r#"
            impl Contract {
                pub fn balance_of(env: Env, user: Address) -> i128 {
                    env.storage().persistent().get(&user).unwrap_or(0)
                }
            }
        "#;
        assert!(findings_for(src).is_empty());
    }

    #[test]
    fn silent_on_if_let_some() {
        let src = r#"
            impl Contract {
                pub fn balance_of(env: Env, user: Address) -> i128 {
                    if let Some(balance) = env.storage().persistent().get(&user) {
                        balance
                    } else {
                        0
                    }
                }
            }
        "#;
        assert!(findings_for(src).is_empty());
    }
}
