//! CLI-facing visualization helpers.
//!
//! This module provides a deterministic command surface that a thin binary
//! wrapper can call to export `ReasoningTrace` artifacts.

use crate::error::{Error, Result};
use crate::reasoning::{HtmlConfig, HtmlTheme, ReasoningTrace};
use std::fs;
use std::path::PathBuf;

/// Supported trace visualization output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceVisualizeFormat {
    Html,
    Dot,
    NetworkXJson,
    Mermaid,
}

impl TraceVisualizeFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Html => "html",
            Self::Dot => "dot",
            Self::NetworkXJson => "json",
            Self::Mermaid => "mmd",
        }
    }
}

/// HTML preset for CLI trace visualization exports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlPreset {
    Default,
    Minimal,
    Presentation,
    Analyst,
}

/// Options for CLI trace visualization.
#[derive(Debug, Clone)]
pub struct TraceVisualizeOptions {
    pub format: TraceVisualizeFormat,
    pub output: Option<PathBuf>,
    pub html_preset: HtmlPreset,
    pub title: Option<String>,
}

impl Default for TraceVisualizeOptions {
    fn default() -> Self {
        Self {
            format: TraceVisualizeFormat::Html,
            output: None,
            html_preset: HtmlPreset::Default,
            title: None,
        }
    }
}

/// Result from trace visualization export.
#[derive(Debug, Clone)]
pub struct TraceVisualizeResult {
    pub format: TraceVisualizeFormat,
    pub artifact: String,
    pub output_path: Option<PathBuf>,
}

/// Export a trace visualization artifact for CLI consumers.
pub fn trace_visualize(
    trace: &ReasoningTrace,
    options: &TraceVisualizeOptions,
) -> Result<TraceVisualizeResult> {
    let artifact = match options.format {
        TraceVisualizeFormat::Html => trace.to_html(resolve_html_config(options)),
        TraceVisualizeFormat::Dot => trace.to_dot(),
        TraceVisualizeFormat::NetworkXJson => trace.to_networkx_json(),
        TraceVisualizeFormat::Mermaid => trace.to_mermaid_enhanced(),
    };

    let output_path = if let Some(path) = &options.output {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                Error::Config(format!(
                    "failed to create output directory '{}': {}",
                    parent.display(),
                    error
                ))
            })?;
        }

        fs::write(path, &artifact).map_err(|error| {
            Error::Config(format!(
                "failed to write visualization artifact to '{}': {}",
                path.display(),
                error
            ))
        })?;
        Some(path.clone())
    } else {
        None
    };

    Ok(TraceVisualizeResult {
        format: options.format,
        artifact,
        output_path,
    })
}

/// Parse trace JSON and export visualization artifact for CLI consumers.
pub fn trace_visualize_from_json(
    trace_json: &str,
    options: &TraceVisualizeOptions,
) -> Result<TraceVisualizeResult> {
    let trace: ReasoningTrace = serde_json::from_str(trace_json)
        .map_err(|error| Error::Config(format!("invalid trace JSON payload: {}", error)))?;
    trace_visualize(&trace, options)
}

/// Suggest a default output path for a trace and format.
pub fn suggested_output_path(trace: &ReasoningTrace, format: TraceVisualizeFormat) -> PathBuf {
    PathBuf::from(format!("trace-{}.{}", trace.id, format.extension()))
}

fn resolve_html_config(options: &TraceVisualizeOptions) -> HtmlConfig {
    let mut config = match options.html_preset {
        HtmlPreset::Default => HtmlConfig::default(),
        HtmlPreset::Minimal => HtmlConfig::minimal(),
        HtmlPreset::Presentation => HtmlConfig::presentation(),
        HtmlPreset::Analyst => HtmlConfig::default()
            .with_theme(HtmlTheme::Light)
            .with_details_panel(true)
            .with_export_controls(true)
            .with_fit_to_view(true)
            .with_expand_repl_history(true),
    };

    if let Some(title) = &options.title {
        config = config.with_title(title.clone());
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_trace_visualize_writes_html_with_advanced_controls() {
        let mut trace = ReasoningTrace::new("CLI visualization", "cli-session");
        let root = trace.root_goal.clone();
        trace.log_decision(&root, "Choose strategy", &["A", "B"], 0, "A is simpler");

        let dir = tempdir().expect("tempdir should be created");
        let output = dir.path().join("trace.html");
        let options = TraceVisualizeOptions {
            format: TraceVisualizeFormat::Html,
            output: Some(output.clone()),
            html_preset: HtmlPreset::Default,
            title: Some("CLI Trace".to_string()),
        };

        let result = trace_visualize(&trace, &options).expect("export should succeed");
        assert_eq!(result.format, TraceVisualizeFormat::Html);
        assert_eq!(result.output_path, Some(output.clone()));

        let html = fs::read_to_string(output).expect("html output should be readable");
        assert!(html.contains("Fit to View"));
        assert!(html.contains("Export PNG"));
        assert!(html.contains("details-panel"));
    }

    #[test]
    fn test_trace_visualize_from_json_mermaid() {
        let trace = ReasoningTrace::new("CLI json import", "cli-json");
        let payload = serde_json::to_string(&trace).expect("trace should serialize");
        let options = TraceVisualizeOptions {
            format: TraceVisualizeFormat::Mermaid,
            ..Default::default()
        };

        let result = trace_visualize_from_json(&payload, &options).expect("json export should work");
        assert_eq!(result.format, TraceVisualizeFormat::Mermaid);
        assert!(result.artifact.contains("%% ReasoningTrace (enhanced)"));
    }

    #[test]
    fn test_trace_visualize_from_json_rejects_invalid_payload() {
        let options = TraceVisualizeOptions::default();
        let result = trace_visualize_from_json("{not-json}", &options);
        assert!(result.is_err());
    }

    #[test]
    fn test_suggested_output_path_uses_expected_extension() {
        let trace = ReasoningTrace::new("Path suggestion", "cli-path");
        assert_eq!(
            suggested_output_path(&trace, TraceVisualizeFormat::Dot)
                .to_string_lossy()
                .ends_with(".dot"),
            true
        );
        assert_eq!(
            suggested_output_path(&trace, TraceVisualizeFormat::NetworkXJson)
                .to_string_lossy()
                .ends_with(".json"),
            true
        );
    }
}
