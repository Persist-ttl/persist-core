//! Integration tests that run every detector against the real, compiling
//! Soroban contracts checked into `test-cases/` (not hand-typed snippets),
//! asserting each detector fires on the vulnerable variant and stays
//! silent on the fixed variant.

use persist_analyzer::{analyze_source, Config};

fn detector_fires(source: &str, detector: &str) -> bool {
    let config = Config::default();
    let findings =
        analyze_source("fixture.rs", source, &config).expect("fixture source must parse");
    findings.iter().any(|f| f.detector == detector)
}

macro_rules! fixture_test {
    ($name:ident, $detector:literal, $vulnerable_path:literal, $fixed_path:literal) => {
        #[test]
        fn $name() {
            let vulnerable = include_str!($vulnerable_path);
            let fixed = include_str!($fixed_path);

            assert!(
                detector_fires(vulnerable, $detector),
                "expected `{}` to fire on {}",
                $detector,
                $vulnerable_path
            );
            assert!(
                !detector_fires(fixed, $detector),
                "expected `{}` to stay silent on {}",
                $detector,
                $fixed_path
            );
        }
    };
}

fixture_test!(
    missing_ttl_extension_fixture,
    "missing-ttl-extension",
    "../../../test-cases/missing-ttl-extension/vulnerable/src/lib.rs",
    "../../../test-cases/missing-ttl-extension/fixed/src/lib.rs"
);

fixture_test!(
    insufficient_ttl_margin_fixture,
    "insufficient-ttl-margin",
    "../../../test-cases/insufficient-ttl-margin/vulnerable/src/lib.rs",
    "../../../test-cases/insufficient-ttl-margin/fixed/src/lib.rs"
);

fixture_test!(
    temporary_storage_misuse_fixture,
    "temporary-storage-misuse",
    "../../../test-cases/temporary-storage-misuse/vulnerable/src/lib.rs",
    "../../../test-cases/temporary-storage-misuse/fixed/src/lib.rs"
);

fixture_test!(
    no_archival_handling_fixture,
    "no-archival-handling",
    "../../../test-cases/no-archival-handling/vulnerable/src/lib.rs",
    "../../../test-cases/no-archival-handling/fixed/src/lib.rs"
);

fixture_test!(
    storage_type_mismatch_fixture,
    "storage-type-mismatch",
    "../../../test-cases/storage-type-mismatch/vulnerable/src/lib.rs",
    "../../../test-cases/storage-type-mismatch/fixed/src/lib.rs"
);
