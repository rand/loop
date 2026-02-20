//! Query and traversal operations for reasoning traces.
//!
//! Provides advanced querying capabilities for navigating and analyzing
//! decision trees stored in the memory system.

use crate::error::Result;
use crate::reasoning::store::ReasoningTraceStore;
use crate::reasoning::trace::{DecisionTree, ReasoningTrace};
use crate::reasoning::types::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Query builder for finding traces.
#[derive(Debug, Clone, Default)]
pub struct TraceQuery {
    /// Filter by session ID.
    pub session_id: Option<String>,

    /// Filter by git commit.
    pub git_commit: Option<String>,

    /// Filter by git branch.
    pub git_branch: Option<String>,

    /// Filter traces created after this time.
    pub created_after: Option<DateTime<Utc>>,

    /// Filter traces created before this time.
    pub created_before: Option<DateTime<Utc>>,

    /// Search in goal content.
    pub goal_contains: Option<String>,

    /// Minimum number of decisions.
    pub min_decisions: Option<usize>,

    /// Maximum results.
    pub limit: Option<usize>,
}

impl TraceQuery {
    /// Create a new empty query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by session ID.
    pub fn session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Filter by git commit.
    pub fn commit(mut self, commit: impl Into<String>) -> Self {
        self.git_commit = Some(commit.into());
        self
    }

    /// Filter by git branch.
    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Filter by creation time range.
    pub fn created_between(mut self, after: DateTime<Utc>, before: DateTime<Utc>) -> Self {
        self.created_after = Some(after);
        self.created_before = Some(before);
        self
    }

    /// Filter by goal content.
    pub fn goal_contains(mut self, text: impl Into<String>) -> Self {
        self.goal_contains = Some(text.into());
        self
    }

    /// Filter by minimum decision count.
    pub fn min_decisions(mut self, count: usize) -> Self {
        self.min_decisions = Some(count);
        self
    }

    /// Limit results.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Execute the query against a store.
    pub fn execute(&self, store: &ReasoningTraceStore) -> Result<Vec<ReasoningTrace>> {
        // Start with all traces or filtered by session/commit
        let trace_ids = if let Some(ref session) = self.session_id {
            store.find_by_session(session)?
        } else if let Some(ref commit) = self.git_commit {
            store.find_by_commit(commit)?
        } else {
            store.list_traces()?
        };

        let mut results = Vec::new();

        for trace_id in trace_ids {
            if let Some(trace) = store.load_trace(&trace_id)? {
                // Apply filters
                if !self.matches(&trace) {
                    continue;
                }

                results.push(trace);

                if let Some(limit) = self.limit {
                    if results.len() >= limit {
                        break;
                    }
                }
            }
        }

        Ok(results)
    }

    /// Check if a trace matches this query's filters.
    fn matches(&self, trace: &ReasoningTrace) -> bool {
        // Session filter (already applied in initial query, but double-check)
        if let Some(ref session) = self.session_id {
            if &trace.session_id != session {
                return false;
            }
        }

        // Git commit filter
        if let Some(ref commit) = self.git_commit {
            if trace.git_commit.as_ref() != Some(commit) {
                return false;
            }
        }

        // Git branch filter
        if let Some(ref branch) = self.git_branch {
            if trace.git_branch.as_ref() != Some(branch) {
                return false;
            }
        }

        // Time filters
        if let Some(after) = self.created_after {
            if trace.created_at < after {
                return false;
            }
        }
        if let Some(before) = self.created_before {
            if trace.created_at > before {
                return false;
            }
        }

        // Goal content filter
        if let Some(ref text) = self.goal_contains {
            let goal_matches = trace
                .root()
                .map(|g| g.content.to_lowercase().contains(&text.to_lowercase()))
                .unwrap_or(false);
            if !goal_matches {
                return false;
            }
        }

        // Decision count filter
        if let Some(min) = self.min_decisions {
            let decision_count = trace.nodes_by_type(DecisionNodeType::Decision).len();
            if decision_count < min {
                return false;
            }
        }

        true
    }
}

/// Result of a path query through a decision tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPath {
    /// Nodes in the path from root to target.
    pub nodes: Vec<DecisionNode>,

    /// Edge labels along the path.
    pub edges: Vec<TraceEdgeLabel>,

    /// Summary of the path.
    pub summary: String,
}

impl DecisionPath {
    /// Create from a list of nodes and the tree.
    pub fn from_nodes(nodes: Vec<DecisionNode>, tree: &DecisionTree) -> Self {
        let mut edges = Vec::new();

        for i in 0..nodes.len().saturating_sub(1) {
            if let Some(label) = tree.get_edge_label(&nodes[i].id, &nodes[i + 1].id) {
                edges.push(label);
            }
        }

        let summary = Self::generate_summary(&nodes, &edges);

        Self {
            nodes,
            edges,
            summary,
        }
    }

    fn generate_summary(nodes: &[DecisionNode], edges: &[TraceEdgeLabel]) -> String {
        let mut parts = Vec::new();

        for (i, node) in nodes.iter().enumerate() {
            let content = if node.content.len() > 30 {
                format!("{}...", &node.content[..30])
            } else {
                node.content.clone()
            };

            match node.node_type {
                DecisionNodeType::Goal => parts.push(format!("Goal: {}", content)),
                DecisionNodeType::Decision => parts.push(format!("Decision: {}", content)),
                DecisionNodeType::Option => {
                    let status = if i > 0 {
                        edges
                            .get(i - 1)
                            .map(|e| {
                                if *e == TraceEdgeLabel::Chooses {
                                    " (chosen)"
                                } else {
                                    " (rejected)"
                                }
                            })
                            .unwrap_or("")
                    } else {
                        ""
                    };
                    parts.push(format!("Option: {}{}", content, status));
                }
                DecisionNodeType::Action => parts.push(format!("Action: {}", content)),
                DecisionNodeType::Outcome => parts.push(format!("Outcome: {}", content)),
                DecisionNodeType::Observation => parts.push(format!("Observed: {}", content)),
            }
        }

        parts.join(" -> ")
    }

    /// Get the final node in the path.
    pub fn final_node(&self) -> Option<&DecisionNode> {
        self.nodes.last()
    }

    /// Check if this path includes a chosen option.
    pub fn includes_chosen(&self) -> bool {
        self.edges.contains(&TraceEdgeLabel::Chooses)
    }

    /// Get the depth of this path.
    pub fn depth(&self) -> usize {
        self.nodes.len()
    }
}

/// Analyzer for extracting insights from reasoning traces.
pub struct TraceAnalyzer<'a> {
    trace: &'a ReasoningTrace,
}

impl<'a> TraceAnalyzer<'a> {
    /// Create an analyzer for a trace.
    pub fn new(trace: &'a ReasoningTrace) -> Self {
        Self { trace }
    }

    /// Get all decision paths (paths from goal to leaf nodes).
    pub fn decision_paths(&self) -> Vec<DecisionPath> {
        let tree = self.trace.get_tree();
        let leaves = tree.leaves();

        leaves
            .iter()
            .map(|leaf| {
                let nodes: Vec<DecisionNode> =
                    tree.path_to(&leaf.id).into_iter().cloned().collect();
                DecisionPath::from_nodes(nodes, &tree)
            })
            .collect()
    }

    /// Get the "winning" path (path through chosen options to final outcome).
    pub fn winning_path(&self) -> Option<DecisionPath> {
        self.decision_paths()
            .into_iter()
            .filter(|p| p.includes_chosen())
            .max_by_key(|p| p.depth())
    }

    /// Get all rejected options with their reasons.
    pub fn rejected_options(&self) -> Vec<(&DecisionNode, Option<&DecisionNode>)> {
        let mut rejected = Vec::new();

        for edge in &self.trace.edges {
            if edge.label == TraceEdgeLabel::Rejects {
                if let (Some(decision), Some(option)) = (
                    self.trace.get_node(&edge.from),
                    self.trace.get_node(&edge.to),
                ) {
                    rejected.push((option, Some(decision)));
                }
            }
        }

        rejected
    }

    /// Get all chosen options.
    pub fn chosen_options(&self) -> Vec<&DecisionNode> {
        self.trace
            .edges
            .iter()
            .filter(|e| e.label == TraceEdgeLabel::Chooses)
            .filter_map(|e| self.trace.get_node(&e.to))
            .collect()
    }

    /// Get the decision that led to a specific node.
    pub fn decision_for(&self, node_id: &DecisionNodeId) -> Option<&DecisionNode> {
        // Walk up the tree to find the decision
        let mut current = node_id.clone();

        for _ in 0..100 {
            // Safety limit
            if let Some(parent) = self.trace.parent(&current) {
                if parent.node_type == DecisionNodeType::Decision {
                    return Some(parent);
                }
                current = parent.id.clone();
            } else {
                break;
            }
        }

        None
    }

    /// Get all actions and their outcomes.
    pub fn action_outcomes(&self) -> Vec<(&DecisionNode, Option<&DecisionNode>)> {
        self.trace
            .nodes_by_type(DecisionNodeType::Action)
            .into_iter()
            .map(|action| {
                let outcome = self
                    .trace
                    .edges_from(&action.id)
                    .iter()
                    .find(|e| e.label == TraceEdgeLabel::Produces)
                    .and_then(|e| self.trace.get_node(&e.to));
                (action, outcome)
            })
            .collect()
    }

    /// Calculate confidence score for the overall trace.
    ///
    /// Based on the confidence of chosen options and outcomes.
    pub fn overall_confidence(&self) -> f64 {
        let chosen: Vec<f64> = self.chosen_options().iter().map(|n| n.confidence).collect();

        let outcomes: Vec<f64> = self
            .trace
            .nodes_by_type(DecisionNodeType::Outcome)
            .iter()
            .map(|n| n.confidence)
            .collect();

        let all_confidences: Vec<f64> = chosen.into_iter().chain(outcomes).collect();

        if all_confidences.is_empty() {
            1.0
        } else {
            all_confidences.iter().sum::<f64>() / all_confidences.len() as f64
        }
    }

    /// Generate a narrative summary of the reasoning process.
    pub fn narrative(&self) -> String {
        let mut narrative = String::new();

        // Start with goal
        if let Some(goal) = self.trace.root() {
            narrative.push_str(&format!("Goal: {}\n\n", goal.content));
        }

        // Describe each decision
        for decision in self.trace.nodes_by_type(DecisionNodeType::Decision) {
            narrative.push_str(&format!("Decision: {}\n", decision.content));

            // Find options for this decision
            let options = self.trace.edges_from(&decision.id);
            for edge in options {
                if let Some(option) = self.trace.get_node(&edge.to) {
                    let status = match edge.label {
                        TraceEdgeLabel::Chooses => "CHOSEN",
                        TraceEdgeLabel::Rejects => "rejected",
                        _ => "considered",
                    };
                    narrative.push_str(&format!("  - {} [{}]", option.content, status));
                    if let Some(ref reason) = option.reason {
                        narrative.push_str(&format!(" - {}", reason));
                    }
                    narrative.push('\n');
                }
            }
            narrative.push('\n');
        }

        // Describe actions and outcomes
        for (action, outcome) in self.action_outcomes() {
            narrative.push_str(&format!("Action: {}\n", action.content));
            if let Some(outcome) = outcome {
                narrative.push_str(&format!("Result: {}\n", outcome.content));
            }
            narrative.push('\n');
        }

        narrative
    }
}

/// Comparison result between two traces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceComparison {
    /// Trace IDs being compared.
    pub trace_a: TraceId,
    pub trace_b: TraceId,

    /// Common decisions (by content similarity).
    pub common_decisions: Vec<String>,

    /// Decisions unique to trace A.
    pub unique_to_a: Vec<String>,

    /// Decisions unique to trace B.
    pub unique_to_b: Vec<String>,

    /// Whether the same option was chosen for common decisions.
    pub choice_agreement: f64,

    /// Summary of differences.
    pub summary: String,
}

/// Compare two reasoning traces.
pub fn compare_traces(trace_a: &ReasoningTrace, trace_b: &ReasoningTrace) -> TraceComparison {
    let decisions_a: Vec<&str> = trace_a
        .nodes_by_type(DecisionNodeType::Decision)
        .iter()
        .map(|n| n.content.as_str())
        .collect();

    let decisions_b: Vec<&str> = trace_b
        .nodes_by_type(DecisionNodeType::Decision)
        .iter()
        .map(|n| n.content.as_str())
        .collect();

    let common: Vec<String> = decisions_a
        .iter()
        .filter(|d| decisions_b.contains(d))
        .map(|s| s.to_string())
        .collect();

    let unique_a: Vec<String> = decisions_a
        .iter()
        .filter(|d| !decisions_b.contains(d))
        .map(|s| s.to_string())
        .collect();

    let unique_b: Vec<String> = decisions_b
        .iter()
        .filter(|d| !decisions_a.contains(d))
        .map(|s| s.to_string())
        .collect();

    // Calculate choice agreement for common decisions
    // (simplified - would need more sophisticated matching in practice)
    let choice_agreement = if common.is_empty() {
        0.0
    } else {
        // For now, just check if number of chosen options is similar
        let analyzer_a = TraceAnalyzer::new(trace_a);
        let analyzer_b = TraceAnalyzer::new(trace_b);

        let chosen_a = analyzer_a.chosen_options().len();
        let chosen_b = analyzer_b.chosen_options().len();

        if chosen_a == 0 && chosen_b == 0 {
            1.0
        } else {
            1.0 - ((chosen_a as f64 - chosen_b as f64).abs()
                / (chosen_a.max(chosen_b) as f64).max(1.0))
        }
    };

    let summary = format!(
        "{} common decisions, {} unique to A, {} unique to B, {:.0}% choice agreement",
        common.len(),
        unique_a.len(),
        unique_b.len(),
        choice_agreement * 100.0
    );

    TraceComparison {
        trace_a: trace_a.id.clone(),
        trace_b: trace_b.id.clone(),
        common_decisions: common,
        unique_to_a: unique_a,
        unique_to_b: unique_b,
        choice_agreement,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning::store::ReasoningTraceStore;

    fn create_test_trace() -> ReasoningTrace {
        let mut trace = ReasoningTrace::new("Implement feature X", "test-session");
        let root_id = trace.root_goal.clone();

        let chosen = trace.log_decision(
            &root_id,
            "Choose implementation approach",
            &["Simple", "Complex", "Hybrid"],
            0,
            "Fastest to implement",
        );

        let (_, outcome_id) =
            trace.log_action(&chosen, "Write simple implementation", "Code works");

        trace.log_observation(&outcome_id, "Performance is acceptable");

        trace
    }

    #[test]
    fn test_trace_query_builder() {
        let query = TraceQuery::new()
            .session("session-1")
            .commit("abc123")
            .min_decisions(2)
            .limit(10);

        assert_eq!(query.session_id, Some("session-1".to_string()));
        assert_eq!(query.git_commit, Some("abc123".to_string()));
        assert_eq!(query.min_decisions, Some(2));
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn test_query_execution() {
        let store = ReasoningTraceStore::in_memory().unwrap();

        let trace1 = ReasoningTrace::new("Goal 1", "session-a");
        let trace2 = ReasoningTrace::new("Goal 2", "session-b");

        store.save_trace(&trace1).unwrap();
        store.save_trace(&trace2).unwrap();

        let results = TraceQuery::new()
            .session("session-a")
            .execute(&store)
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, "session-a");
    }

    #[test]
    fn test_analyzer_decision_paths() {
        let trace = create_test_trace();
        let analyzer = TraceAnalyzer::new(&trace);

        let paths = analyzer.decision_paths();
        assert!(!paths.is_empty());

        // Should have path to the observation (deepest leaf)
        let deepest = paths.iter().max_by_key(|p| p.depth()).unwrap();
        assert!(deepest.depth() >= 4); // goal -> decision -> option -> action -> outcome -> observation
    }

    #[test]
    fn test_analyzer_winning_path() {
        let trace = create_test_trace();
        let analyzer = TraceAnalyzer::new(&trace);

        let winning = analyzer.winning_path();
        assert!(winning.is_some());

        let path = winning.unwrap();
        assert!(path.includes_chosen());
    }

    #[test]
    fn test_analyzer_rejected_options() {
        let trace = create_test_trace();
        let analyzer = TraceAnalyzer::new(&trace);

        let rejected = analyzer.rejected_options();
        assert_eq!(rejected.len(), 2); // "Complex" and "Hybrid"
    }

    #[test]
    fn test_analyzer_chosen_options() {
        let trace = create_test_trace();
        let analyzer = TraceAnalyzer::new(&trace);

        let chosen = analyzer.chosen_options();
        assert_eq!(chosen.len(), 1);
        assert!(chosen[0].content.contains("Simple"));
    }

    #[test]
    fn test_analyzer_action_outcomes() {
        let trace = create_test_trace();
        let analyzer = TraceAnalyzer::new(&trace);

        let actions = analyzer.action_outcomes();
        assert_eq!(actions.len(), 1);
        assert!(actions[0].1.is_some());
    }

    #[test]
    fn test_analyzer_narrative() {
        let trace = create_test_trace();
        let analyzer = TraceAnalyzer::new(&trace);

        let narrative = analyzer.narrative();

        assert!(narrative.contains("Goal:"));
        assert!(narrative.contains("Decision:"));
        assert!(narrative.contains("CHOSEN"));
        assert!(narrative.contains("rejected"));
        assert!(narrative.contains("Action:"));
    }

    #[test]
    fn test_analyzer_overall_confidence() {
        let trace = create_test_trace();
        let analyzer = TraceAnalyzer::new(&trace);

        let confidence = analyzer.overall_confidence();
        assert!(confidence >= 0.0 && confidence <= 1.0);
        assert_eq!(confidence, 1.0); // Default confidence is 1.0
    }

    #[test]
    fn test_decision_path_summary() {
        let trace = create_test_trace();
        let tree = trace.get_tree();
        let leaves = tree.leaves();

        let leaf = leaves
            .iter()
            .find(|n| n.node_type == DecisionNodeType::Observation)
            .unwrap();

        let nodes: Vec<DecisionNode> = tree.path_to(&leaf.id).into_iter().cloned().collect();
        let path = DecisionPath::from_nodes(nodes, &tree);

        assert!(path.summary.contains("Goal:"));
        assert!(path.summary.contains("->"));
    }

    #[test]
    fn test_compare_traces() {
        let mut trace_a = ReasoningTrace::new("Build API", "session-a");
        let root_a = trace_a.root_goal.clone();
        trace_a.log_decision(
            &root_a,
            "Choose framework",
            &["Axum", "Actix"],
            0,
            "Performance",
        );

        let mut trace_b = ReasoningTrace::new("Build API", "session-b");
        let root_b = trace_b.root_goal.clone();
        trace_b.log_decision(
            &root_b,
            "Choose framework",
            &["Axum", "Rocket"],
            0,
            "Ergonomics",
        );
        trace_b.log_decision(
            &root_b,
            "Choose database",
            &["Postgres", "SQLite"],
            1,
            "Simplicity",
        );

        let comparison = compare_traces(&trace_a, &trace_b);

        assert_eq!(comparison.common_decisions.len(), 1);
        assert!(comparison
            .common_decisions
            .contains(&"Choose framework".to_string()));
        assert_eq!(comparison.unique_to_b.len(), 1); // "Choose database"
        assert!(comparison.summary.contains("common"));
    }

    #[test]
    fn test_query_goal_contains() {
        let store = ReasoningTraceStore::in_memory().unwrap();

        let trace1 = ReasoningTrace::new("Implement authentication", "session-1");
        let trace2 = ReasoningTrace::new("Fix database bug", "session-2");

        store.save_trace(&trace1).unwrap();
        store.save_trace(&trace2).unwrap();

        let results = TraceQuery::new()
            .goal_contains("auth")
            .execute(&store)
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0]
            .root()
            .unwrap()
            .content
            .contains("authentication"));
    }
}
