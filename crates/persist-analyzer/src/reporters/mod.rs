pub mod json;
pub mod markdown;
pub mod sarif;
pub mod terminal;

use std::fmt;
use std::str::FromStr;

use crate::report::Report;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
    Markdown,
    Sarif,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" | "terminal" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "md" | "markdown" => Ok(OutputFormat::Markdown),
            "sarif" => Ok(OutputFormat::Sarif),
            other => Err(format!(
                "unknown output format `{other}` (expected one of: text, json, md, sarif)"
            )),
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OutputFormat::Text => "text",
            OutputFormat::Json => "json",
            OutputFormat::Markdown => "md",
            OutputFormat::Sarif => "sarif",
        };
        write!(f, "{s}")
    }
}

/// Renders a report in the requested format.
pub fn render(format: OutputFormat, report: &Report) -> String {
    match format {
        OutputFormat::Text => terminal::render(report),
        OutputFormat::Json => json::render(report),
        OutputFormat::Markdown => markdown::render(report),
        OutputFormat::Sarif => sarif::render(report),
    }
}
