//! Trajectory event types for observable RLM execution.
//!
//! The trajectory system provides a stream of events that can be rendered
//! differently depending on the deployment context:
//! - Claude Code: Streaming text output
//! - TUI: Bubble Tea panel updates
//! - Analysis: JSON export for replay

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Types of trajectory events emitted during RLM execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrajectoryEventType {
    /// RLM orchestration started
    RlmStart,
    /// Analysis phase (complexity assessment, strategy selection)
    Analyze,
    /// REPL code execution started
    ReplExec,
    /// REPL execution result
    ReplResult,
    /// Reasoning step in the trace
    Reason,
    /// Recursive sub-call started
    RecurseStart,
    /// Recursive sub-call completed
    RecurseEnd,
    /// Final answer/synthesis
    Final,
    /// Error occurred
    Error,
    /// Tool use (external tool, not REPL)
    ToolUse,
    /// Cost report for the operation
    CostReport,
    /// Beginning verification of response/trace (Strawberry)
    VerifyStart,
    /// Atomic claim identified during verification
    ClaimExtracted,
    /// Claim verified against evidence
    EvidenceChecked,
    /// Budget metrics computed (p0, p1, gap)
    BudgetComputed,
    /// Claim flagged as potentially hallucinated
    HallucinationFlag,
    /// Verification report complete
    VerifyComplete,
    /// Memory operation (store, query, evolve)
    Memory,
    /// Context externalization complete
    Externalize,
    /// Decomposition/partitioning step
    Decompose,
    /// Synthesis step
    Synthesize,
}

impl std::fmt::Display for TrajectoryEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::RlmStart => "RLM_START",
            Self::Analyze => "ANALYZE",
            Self::ReplExec => "REPL_EXEC",
            Self::ReplResult => "REPL_RESULT",
            Self::Reason => "REASON",
            Self::RecurseStart => "RECURSE_START",
            Self::RecurseEnd => "RECURSE_END",
            Self::Final => "FINAL",
            Self::Error => "ERROR",
            Self::ToolUse => "TOOL_USE",
            Self::CostReport => "COST_REPORT",
            Self::VerifyStart => "VERIFY_START",
            Self::ClaimExtracted => "CLAIM_EXTRACTED",
            Self::EvidenceChecked => "EVIDENCE_CHECKED",
            Self::BudgetComputed => "BUDGET_COMPUTED",
            Self::HallucinationFlag => "HALLUCINATION_FLAG",
            Self::VerifyComplete => "VERIFY_COMPLETE",
            Self::Memory => "MEMORY",
            Self::Externalize => "EXTERNALIZE",
            Self::Decompose => "DECOMPOSE",
            Self::Synthesize => "SYNTHESIZE",
        };
        write!(f, "{}", s)
    }
}

/// A trajectory event emitted during RLM execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrajectoryEvent {
    /// Type of the event
    pub event_type: TrajectoryEventType,
    /// Current recursion depth (0 = top level)
    pub depth: u32,
    /// Human-readable content describing the event
    pub content: String,
    /// Event-specific metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
}

impl TrajectoryEvent {
    /// Create a new trajectory event.
    pub fn new(event_type: TrajectoryEventType, depth: u32, content: impl Into<String>) -> Self {
        Self {
            event_type,
            depth,
            content: content.into(),
            metadata: None,
            timestamp: Utc::now(),
        }
    }

    /// Add metadata to the event.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Add multiple metadata entries.
    pub fn with_metadata_map(mut self, map: HashMap<String, Value>) -> Self {
        self.metadata = Some(map);
        self
    }

    /// Get a metadata value.
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.as_ref()?.get(key)
    }

    // Convenience constructors for common event types

    /// Create an RLM start event.
    pub fn rlm_start(query: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::RlmStart, 0, query)
    }

    /// Create an analyze event.
    pub fn analyze(depth: u32, analysis: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::Analyze, depth, analysis)
    }

    /// Create a REPL execution event.
    pub fn repl_exec(depth: u32, code: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::ReplExec, depth, code)
    }

    /// Create a REPL result event.
    pub fn repl_result(depth: u32, result: impl Into<String>, success: bool) -> Self {
        Self::new(TrajectoryEventType::ReplResult, depth, result).with_metadata("success", success)
    }

    /// Create a reasoning event.
    pub fn reason(depth: u32, reasoning: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::Reason, depth, reasoning)
    }

    /// Create a recursive call start event.
    pub fn recurse_start(depth: u32, query: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::RecurseStart, depth, query)
    }

    /// Create a recursive call end event.
    pub fn recurse_end(depth: u32, result: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::RecurseEnd, depth, result)
    }

    /// Create a final answer event.
    pub fn final_answer(depth: u32, answer: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::Final, depth, answer)
    }

    /// Create an error event.
    pub fn error(depth: u32, error: impl Into<String>) -> Self {
        Self::new(TrajectoryEventType::Error, depth, error)
    }

    /// Create a cost report event.
    pub fn cost_report(cost: &CostSummary) -> Self {
        Self::new(TrajectoryEventType::CostReport, 0, cost.to_string())
            .with_metadata("input_tokens", cost.input_tokens as i64)
            .with_metadata("output_tokens", cost.output_tokens as i64)
            .with_metadata("total_cost_usd", cost.total_cost_usd)
    }

    /// Create a hallucination flag event.
    pub fn hallucination_flag(
        depth: u32,
        claim: impl Into<String>,
        budget_gap: f64,
        status: impl Into<String>,
    ) -> Self {
        Self::new(TrajectoryEventType::HallucinationFlag, depth, claim)
            .with_metadata("budget_gap", budget_gap)
            .with_metadata("status", status.into())
    }

    /// Check if this is an error event.
    pub fn is_error(&self) -> bool {
        self.event_type == TrajectoryEventType::Error
    }

    /// Check if this is a final answer event.
    pub fn is_final(&self) -> bool {
        self.event_type == TrajectoryEventType::Final
    }

    /// Format as a single-line log entry.
    pub fn as_log_line(&self) -> String {
        let indent = "  ".repeat(self.depth as usize);
        format!(
            "[{}] {}{}: {}",
            self.timestamp.format("%H:%M:%S%.3f"),
            indent,
            self.event_type,
            self.content.lines().next().unwrap_or("")
        )
    }
}

/// Token usage for an LLM call.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
}

impl TokenUsage {
    pub fn new(input: u64, output: u64) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            cache_creation_tokens: 0,
            cache_read_tokens: 0,
        }
    }

    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// Cost tracking for a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostComponent {
    Orchestration,
    Repl,
    Recursion,
    Memory,
    Verification,
    Embedding,
}

/// Summary of costs for an RLM execution.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CostSummary {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    pub total_cost_usd: f64,
    /// Breakdown by component
    pub by_component: HashMap<String, TokenUsage>,
}

impl CostSummary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add token usage for a component.
    pub fn add(&mut self, component: CostComponent, usage: TokenUsage, cost_usd: f64) {
        self.input_tokens += usage.input_tokens;
        self.output_tokens += usage.output_tokens;
        self.cache_creation_tokens += usage.cache_creation_tokens;
        self.cache_read_tokens += usage.cache_read_tokens;
        self.total_cost_usd += cost_usd;

        let key = format!("{:?}", component).to_lowercase();
        self.by_component
            .entry(key)
            .and_modify(|existing| {
                existing.input_tokens += usage.input_tokens;
                existing.output_tokens += usage.output_tokens;
                existing.cache_creation_tokens += usage.cache_creation_tokens;
                existing.cache_read_tokens += usage.cache_read_tokens;
            })
            .or_insert(usage);
    }

    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

impl std::fmt::Display for CostSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tokens: {} in / {} out | Cost: ${:.4}",
            self.input_tokens, self.output_tokens, self.total_cost_usd
        )
    }
}

/// Export format for trajectory data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON Lines format (one event per line)
    JsonLines,
    /// Pretty-printed JSON array
    JsonPretty,
    /// Compact JSON array
    JsonCompact,
    /// Markdown summary
    Markdown,
}

/// Serialize a list of events to the specified format.
pub fn export_events(events: &[TrajectoryEvent], format: ExportFormat) -> String {
    match format {
        ExportFormat::JsonLines => events
            .iter()
            .filter_map(|e| serde_json::to_string(e).ok())
            .collect::<Vec<_>>()
            .join("\n"),
        ExportFormat::JsonPretty => {
            serde_json::to_string_pretty(events).unwrap_or_else(|_| "[]".to_string())
        }
        ExportFormat::JsonCompact => {
            serde_json::to_string(events).unwrap_or_else(|_| "[]".to_string())
        }
        ExportFormat::Markdown => events_to_markdown(events),
    }
}

fn events_to_markdown(events: &[TrajectoryEvent]) -> String {
    let mut md = String::from("# RLM Trajectory\n\n");

    for event in events {
        let indent = "  ".repeat(event.depth as usize);
        md.push_str(&format!(
            "{}**{}** `{}`\n",
            indent, event.event_type, event.timestamp
        ));
        if !event.content.is_empty() {
            md.push_str(&format!("{}> {}\n", indent, event.content));
        }
        md.push('\n');
    }

    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = TrajectoryEvent::rlm_start("What is the auth flow?");
        assert_eq!(event.event_type, TrajectoryEventType::RlmStart);
        assert_eq!(event.depth, 0);
        assert_eq!(event.content, "What is the auth flow?");
    }

    #[test]
    fn test_event_with_metadata() {
        let event = TrajectoryEvent::repl_result(1, "42", true);
        assert_eq!(
            event.get_metadata("success"),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn test_cost_summary() {
        let mut cost = CostSummary::new();
        cost.add(
            CostComponent::Orchestration,
            TokenUsage::new(1000, 500),
            0.05,
        );
        cost.add(CostComponent::Repl, TokenUsage::new(200, 100), 0.01);

        assert_eq!(cost.input_tokens, 1200);
        assert_eq!(cost.output_tokens, 600);
        assert!((cost.total_cost_usd - 0.06).abs() < 0.0001);
    }

    #[test]
    fn test_event_log_line() {
        let event = TrajectoryEvent::analyze(1, "Complexity: high");
        let line = event.as_log_line();
        assert!(line.contains("ANALYZE"));
        assert!(line.contains("  ")); // depth indent
        assert!(line.contains("Complexity: high"));
    }

    #[test]
    fn test_export_json_lines() {
        let events = vec![
            TrajectoryEvent::rlm_start("Test"),
            TrajectoryEvent::final_answer(0, "Done"),
        ];
        let exported = export_events(&events, ExportFormat::JsonLines);
        let lines: Vec<_> = exported.lines().collect();
        assert_eq!(lines.len(), 2);
    }
}
