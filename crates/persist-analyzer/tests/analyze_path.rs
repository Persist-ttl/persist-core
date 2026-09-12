//! Integration tests for `analyze_path`'s package-scoped cross-file
//! awareness for `missing-ttl-extension`
//! (https://github.com/Persist-ttl/persist-core/issues/4).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use persist_analyzer::{analyze_path, Config};

/// Minimal self-cleaning temp directory, to avoid pulling in a `tempfile`
/// dev-dependency just for these tests.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "persist-analyzer-test-{}-{}-{n}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        TempDir(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tempdir() -> TempDir {
    TempDir::new()
}

fn write(dir: &std::path::Path, rel: &str, contents: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

#[test]
fn cross_file_extend_ttl_in_same_package_suppresses_the_finding() {
    let dir = tempdir();
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"c\"\n").unwrap();
    write(
        dir.path(),
        "src/admin.rs",
        r#"
            pub fn write_administrator(e: &Env, id: &Address) {
                e.storage().instance().set(&DataKey::Admin, id);
            }
        "#,
    );
    write(
        dir.path(),
        "src/contract.rs",
        r#"
            impl Contract {
                pub fn initialize(e: Env) {
                    e.storage().instance().extend_ttl(100, 200_000);
                }
            }
        "#,
    );

    let report = analyze_path(dir.path(), &Config::default()).unwrap();
    assert!(
        report
            .findings
            .iter()
            .all(|f| f.detector != "missing-ttl-extension"),
        "expected no missing-ttl-extension findings, got: {:#?}",
        report.findings
    );
}

#[test]
fn different_packages_do_not_share_extend_ttl_calls() {
    let dir = tempdir();

    write(dir.path(), "pkg-a/Cargo.toml", "[package]\nname = \"a\"\n");
    write(
        dir.path(),
        "pkg-a/src/lib.rs",
        r#"
            impl Contract {
                pub fn deposit(e: Env) {
                    e.storage().instance().set(&Key::X, &1);
                }
            }
        "#,
    );

    write(dir.path(), "pkg-b/Cargo.toml", "[package]\nname = \"b\"\n");
    write(
        dir.path(),
        "pkg-b/src/lib.rs",
        r#"
            impl Contract {
                pub fn bump(e: Env) {
                    e.storage().instance().extend_ttl(100, 200_000);
                }
            }
        "#,
    );

    let report = analyze_path(dir.path(), &Config::default()).unwrap();
    let missing_ttl: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.detector == "missing-ttl-extension")
        .collect();
    assert_eq!(
        missing_ttl.len(),
        1,
        "pkg-a's write shouldn't be suppressed by pkg-b's unrelated extend_ttl: {:#?}",
        report.findings
    );
    assert!(missing_ttl[0].file.contains("pkg-a"));
}
