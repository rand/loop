//! Type definitions for Deciduous-style reasoning traces.
//!
//! This module defines the core types for representing decision trees and
//! reasoning traces as memory nodes for provenance tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a reasoning trace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(pub Uuid);

impl TraceId {
    /// Generate a new random trace ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from a UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse from string.
    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TraceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a decision node within a trace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DecisionNodeId(pub Uuid);

impl DecisionNodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn parse(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for DecisionNodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DecisionNodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Type of node in the decision tree (Deciduous-style).
///
/// Based on the Deciduous decision tree format for representing
/// AI reasoning processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionNodeType {
    /// Top-level objective being pursued.
    /// The root of a reasoning trace, representing what we're trying to achieve.
    Goal,

    /// Decision point with multiple options to consider.
    /// Represents a branching point where choices must be made.
    Decision,

    /// A possible choice at a decision point.
    /// One of potentially many alternatives being considered.
    Option,

    /// Concrete action taken to implement a decision.
    /// Represents actual execution steps.
    Action,

    /// Result of an action, whether successful or not.
    /// Captures what happened after an action was taken.
    Outcome,

    /// Observed state or fact during reasoning.
    /// External information gathered during the process.
    Observation,
}

impl DecisionNodeType {
    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Goal => "Top-level objective",
            Self::Decision => "Decision point",
            Self::Option => "Possible choice",
            Self::Action => "Concrete action",
            Self::Outcome => "Result of action",
            Self::Observation => "Observed fact",
        }
    }

    /// Get the Mermaid node shape for this type.
    pub fn mermaid_shape(&self) -> (&'static str, &'static str) {
        match self {
            Self::Goal => ("([", "])"),      // Stadium shape for goals
            Self::Decision => ("{", "}"),    // Diamond/rhombus for decisions
            Self::Option => ("[[", "]]"),    // Subroutine shape for options
            Self::Action => ("[/", "/]"),    // Parallelogram for actions
            Self::Outcome => ("((", "))"),   // Circle for outcomes
            Self::Observation => ("[", "]"), // Rectangle for observations
        }
    }
}

impl std::fmt::Display for DecisionNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Goal => write!(f, "goal"),
            Self::Decision => write!(f, "decision"),
            Self::Option => write!(f, "option"),
            Self::Action => write!(f, "action"),
            Self::Outcome => write!(f, "outcome"),
            Self::Observation => write!(f, "observation"),
        }
    }
}

/// Edge labels connecting nodes in the decision tree.
///
/// These represent the semantic relationships between nodes
/// in the reasoning trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceEdgeLabel {
    /// Goal spawns a decision point.
    /// A goal leads to decisions about how to achieve it.
    Spawns,

    /// Decision considers an option.
    /// An option is being evaluated at a decision point.
    Considers,

    /// Decision chooses an option.
    /// The selected option from available choices.
    Chooses,

    /// Decision rejects an option.
    /// An option that was considered but not selected.
    Rejects,

    /// Option leads to implementing an action.
    /// Moving from choice to execution.
    Implements,

    /// Action produces an outcome.
    /// The result of taking an action.
    Produces,

    /// Outcome leads to an observation or new state.
    /// What we learned from the outcome.
    LeadsTo,

    /// Node references evidence or supporting information.
    /// Links to external sources or prior knowledge.
    References,

    /// Node is a prerequisite for another.
    /// Dependency relationship.
    Requires,

    /// Node invalidates or contradicts another.
    /// For tracking reasoning revisions.
    Invalidates,
}

impl TraceEdgeLabel {
    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Spawns => "spawns decision",
            Self::Considers => "considers option",
            Self::Chooses => "chooses option",
            Self::Rejects => "rejects option",
            Self::Implements => "implements via",
            Self::Produces => "produces outcome",
            Self::LeadsTo => "leads to",
            Self::References => "references",
            Self::Requires => "requires",
            Self::Invalidates => "invalidates",
        }
    }

    /// Get valid source and target node types for this edge.
    pub fn valid_connections(&self) -> (Vec<DecisionNodeType>, Vec<DecisionNodeType>) {
        use DecisionNodeType::*;
        match self {
            Self::Spawns => (vec![Goal], vec![Decision]),
            Self::Considers => (vec![Decision], vec![Option]),
            Self::Chooses => (vec![Decision], vec![Option]),
            Self::Rejects => (vec![Decision], vec![Option]),
            Self::Implements => (vec![Option], vec![Action]),
            Self::Produces => (vec![Action], vec![Outcome]),
            Self::LeadsTo => (vec![Outcome], vec![Observation, Goal, Decision]),
            Self::References => (
                vec![Goal, Decision, Option, Action, Outcome, Observation],
                vec![Goal, Decision, Option, Action, Outcome, Observation],
            ),
            Self::Requires => (
                vec![Goal, Decision, Action],
                vec![Goal, Decision, Action, Observation],
            ),
            Self::Invalidates => (vec![Outcome, Observation], vec![Option, Decision, Action]),
        }
    }
}

impl std::fmt::Display for TraceEdgeLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawns => write!(f, "spawns"),
            Self::Considers => write!(f, "considers"),
            Self::Chooses => write!(f, "chooses"),
            Self::Rejects => write!(f, "rejects"),
            Self::Implements => write!(f, "implements"),
            Self::Produces => write!(f, "produces"),
            Self::LeadsTo => write!(f, "leads_to"),
            Self::References => write!(f, "references"),
            Self::Requires => write!(f, "requires"),
            Self::Invalidates => write!(f, "invalidates"),
        }
    }
}

/// A node in the decision tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionNode {
    /// Unique identifier for this node.
    pub id: DecisionNodeId,

    /// Type of this node.
    pub node_type: DecisionNodeType,

    /// Human-readable content/description.
    pub content: String,

    /// Optional reason or rationale for this node.
    pub reason: Option<String>,

    /// Confidence score (0.0 - 1.0) for this decision/action.
    pub confidence: f64,

    /// When this node was created.
    pub created_at: DateTime<Utc>,

    /// Additional metadata (e.g., tool outputs, file paths).
    pub metadata: Option<HashMap<String, Value>>,
}

impl DecisionNode {
    /// Create a new decision node.
    pub fn new(node_type: DecisionNodeType, content: impl Into<String>) -> Self {
        Self {
            id: DecisionNodeId::new(),
            node_type,
            content: content.into(),
            reason: None,
            confidence: 1.0,
            created_at: Utc::now(),
            metadata: None,
        }
    }

    /// Create a goal node.
    pub fn goal(content: impl Into<String>) -> Self {
        Self::new(DecisionNodeType::Goal, content)
    }

    /// Create a decision node.
    pub fn decision(content: impl Into<String>) -> Self {
        Self::new(DecisionNodeType::Decision, content)
    }

    /// Create an option node.
    pub fn option(content: impl Into<String>) -> Self {
        Self::new(DecisionNodeType::Option, content)
    }

    /// Create an action node.
    pub fn action(content: impl Into<String>) -> Self {
        Self::new(DecisionNodeType::Action, content)
    }

    /// Create an outcome node.
    pub fn outcome(content: impl Into<String>) -> Self {
        Self::new(DecisionNodeType::Outcome, content)
    }

    /// Create an observation node.
    pub fn observation(content: impl Into<String>) -> Self {
        Self::new(DecisionNodeType::Observation, content)
    }

    /// Set the reason for this node.
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Set the confidence score.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Get a metadata value.
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.as_ref()?.get(key)
    }
}

/// An edge connecting two nodes in the decision tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceEdge {
    /// Source node ID.
    pub from: DecisionNodeId,

    /// Target node ID.
    pub to: DecisionNodeId,

    /// Type of relationship.
    pub label: TraceEdgeLabel,

    /// Optional weight/strength (0.0 - 1.0).
    pub weight: f64,

    /// When this edge was created.
    pub created_at: DateTime<Utc>,

    /// Additional metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl TraceEdge {
    /// Create a new edge.
    pub fn new(from: DecisionNodeId, to: DecisionNodeId, label: TraceEdgeLabel) -> Self {
        Self {
            from,
            to,
            label,
            weight: 1.0,
            created_at: Utc::now(),
            metadata: None,
        }
    }

    /// Set the weight.
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }
}

/// Status of an option in a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionStatus {
    /// Option is being considered.
    Considering,
    /// Option was chosen.
    Chosen,
    /// Option was rejected.
    Rejected,
}

impl std::fmt::Display for OptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Considering => write!(f, "considering"),
            Self::Chosen => write!(f, "chosen"),
            Self::Rejected => write!(f, "rejected"),
        }
    }
}

/// A decision point with its considered options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionPoint {
    /// The decision node.
    pub decision: DecisionNode,

    /// Options being considered with their status.
    pub options: Vec<(DecisionNode, OptionStatus)>,

    /// Context that led to this decision.
    pub context: Option<String>,
}

impl DecisionPoint {
    /// Create a new decision point.
    pub fn new(context: impl Into<String>) -> Self {
        Self {
            decision: DecisionNode::decision(context.into()),
            options: Vec::new(),
            context: None,
        }
    }

    /// Set the context.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    /// Add an option being considered.
    pub fn add_option(&mut self, option: DecisionNode) {
        self.options.push((option, OptionStatus::Considering));
    }

    /// Choose an option by index.
    pub fn choose(&mut self, index: usize) -> Option<&DecisionNode> {
        if index < self.options.len() {
            // Mark chosen
            self.options[index].1 = OptionStatus::Chosen;
            // Reject others still considering
            for (i, (_, status)) in self.options.iter_mut().enumerate() {
                if i != index && *status == OptionStatus::Considering {
                    *status = OptionStatus::Rejected;
                }
            }
            Some(&self.options[index].0)
        } else {
            None
        }
    }

    /// Get the chosen option.
    pub fn chosen(&self) -> Option<&DecisionNode> {
        self.options
            .iter()
            .find(|(_, s)| *s == OptionStatus::Chosen)
            .map(|(n, _)| n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_node_creation() {
        let goal = DecisionNode::goal("Implement user authentication");
        assert_eq!(goal.node_type, DecisionNodeType::Goal);
        assert!(goal.content.contains("authentication"));
        assert_eq!(goal.confidence, 1.0);
    }

    #[test]
    fn test_decision_node_builder() {
        let action = DecisionNode::action("Add JWT validation")
            .with_reason("Standard approach for stateless auth")
            .with_confidence(0.9)
            .with_metadata("file", "/src/auth.rs");

        assert_eq!(
            action.reason,
            Some("Standard approach for stateless auth".to_string())
        );
        assert_eq!(action.confidence, 0.9);
        assert!(action.get_metadata("file").is_some());
    }

    #[test]
    fn test_trace_edge_creation() {
        let from = DecisionNodeId::new();
        let to = DecisionNodeId::new();

        let edge = TraceEdge::new(from.clone(), to.clone(), TraceEdgeLabel::Spawns);
        assert_eq!(edge.from, from);
        assert_eq!(edge.to, to);
        assert_eq!(edge.label, TraceEdgeLabel::Spawns);
    }

    #[test]
    fn test_decision_point() {
        let mut dp = DecisionPoint::new("Choose auth strategy");
        dp.add_option(DecisionNode::option("JWT tokens"));
        dp.add_option(DecisionNode::option("Session cookies"));
        dp.add_option(DecisionNode::option("OAuth2 only"));

        assert_eq!(dp.options.len(), 3);

        // Choose first option
        dp.choose(0);
        assert_eq!(dp.options[0].1, OptionStatus::Chosen);
        assert_eq!(dp.options[1].1, OptionStatus::Rejected);
        assert_eq!(dp.options[2].1, OptionStatus::Rejected);

        // Get chosen
        let chosen = dp.chosen().unwrap();
        assert!(chosen.content.contains("JWT"));
    }

    #[test]
    fn test_edge_label_valid_connections() {
        let (sources, targets) = TraceEdgeLabel::Spawns.valid_connections();
        assert!(sources.contains(&DecisionNodeType::Goal));
        assert!(targets.contains(&DecisionNodeType::Decision));
    }

    #[test]
    fn test_node_type_mermaid() {
        let (open, close) = DecisionNodeType::Goal.mermaid_shape();
        assert_eq!(open, "([");
        assert_eq!(close, "])");
    }

    #[test]
    fn test_trace_id_roundtrip() {
        let id = TraceId::new();
        let parsed = TraceId::parse(&id.to_string()).unwrap();
        assert_eq!(id, parsed);
    }
}
