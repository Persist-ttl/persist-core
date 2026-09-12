use serde_json::json;

use crate::detectors::ALL_DETECTORS;
use crate::report::Report;

/// Renders a minimal but valid SARIF 2.1.0 document, suitable for upload to
/// GitHub's Security > Code Scanning tab.
pub fn render(report: &Report) -> String {
    let rules: Vec<_> = ALL_DETECTORS
        .iter()
        .map(|d| {
            json!({
                "id": d.slug,
                "name": d.title,
                "shortDescription": { "text": d.title },
                "fullDescription": { "text": d.description },
                "defaultConfiguration": { "level": "warning" },
                "helpUri": format!("https://github.com/Persist-ttl/persist-core#{}", d.slug),
            })
        })
        .collect();

    let results: Vec<_> = report
        .findings
        .iter()
        .map(|f| {
            json!({
                "ruleId": f.detector,
                "level": f.severity.sarif_level(),
                "message": { "text": f.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": f.file },
                        "region": {
                            "startLine": f.line,
                            "startColumn": f.column,
                        }
                    }
                }]
            })
        })
        .collect();

    let doc = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "persist",
                    "informationUri": "https://github.com/Persist-ttl/persist-core",
                    "version": env!("CARGO_PKG_VERSION"),
                    "rules": rules,
                }
            },
            "results": results,
        }]
    });

    serde_json::to_string_pretty(&doc).expect("SARIF document serialization is infallible")
}
