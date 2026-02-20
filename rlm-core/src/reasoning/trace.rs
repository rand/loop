//! ReasoningTrace implementation for decision tree operations.
//!
//! Provides the main interface for building and querying reasoning traces.

use crate::reasoning::types::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A complete reasoning trace capturing the decision process.
///
/// The trace is organized as a directed acyclic graph (DAG) with a root goal
/// and branching decision points leading to actions and outcomes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReasoningTrace {
    /// Unique identifier for this trace.
    pub id: TraceId,

    /// The root goal node ID.
    pub root_goal: DecisionNodeId,

    /// Session identifier for grouping traces.
    pub session_id: String,

    /// When this trace was created.
    pub created_at: DateTime<Utc>,

    /// When this trace was last modified.
    pub updated_at: DateTime<Utc>,

    /// All nodes in the trace.
    pub nodes: Vec<DecisionNode>,

    /// All edges connecting nodes.
    pub edges: Vec<TraceEdge>,

    /// Optional git commit SHA this trace is linked to.
    pub git_commit: Option<String>,

    /// Optional branch name.
    pub git_branch: Option<String>,

    /// Additional trace-level metadata.
    pub metadata: Option<HashMap<String, Value>>,
}

impl ReasoningTrace {
    /// Create a new reasoning trace with a root goal.
    pub fn new(goal: impl Into<String>, session_id: impl Into<String>) -> Self {
        let goal_node = DecisionNode::goal(goal);
        let root_id = goal_node.id.clone();
        let now = Utc::now();

        Self {
            id: TraceId::new(),
            root_goal: root_id,
            session_id: session_id.into(),
            created_at: now,
            updated_at: now,
            nodes: vec![goal_node],
            edges: Vec::new(),
            git_commit: None,
            git_branch: None,
            metadata: None,
        }
    }

    /// Link this trace to a git commit.
    pub fn with_git_commit(mut self, commit: impl Into<String>) -> Self {
        self.git_commit = Some(commit.into());
        self
    }

    /// Link this trace to a git branch.
    pub fn with_git_branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Add trace-level metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    // ==================== Node Operations ====================

    /// Add a node to the trace.
    pub fn add_node(&mut self, node: DecisionNode) -> DecisionNodeId {
        let id = node.id.clone();
        self.nodes.push(node);
        self.updated_at = Utc::now();
        id
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: &DecisionNodeId) -> Option<&DecisionNode> {
        self.nodes.iter().find(|n| &n.id == id)
    }

    /// Get a mutable node by ID.
    pub fn get_node_mut(&mut self, id: &DecisionNodeId) -> Option<&mut DecisionNode> {
        self.nodes.iter_mut().find(|n| &n.id == id)
    }

    /// Get the root goal node.
    pub fn root(&self) -> Option<&DecisionNode> {
        self.get_node(&self.root_goal)
    }

    /// Get all nodes of a specific type.
    pub fn nodes_by_type(&self, node_type: DecisionNodeType) -> Vec<&DecisionNode> {
        self.nodes
            .iter()
            .filter(|n| n.node_type == node_type)
            .collect()
    }

    // ==================== Edge Operations ====================

    /// Add an edge between nodes.
    pub fn add_edge(&mut self, from: DecisionNodeId, to: DecisionNodeId, label: TraceEdgeLabel) {
        self.edges.push(TraceEdge::new(from, to, label));
        self.updated_at = Utc::now();
    }

    /// Get edges from a node.
    pub fn edges_from(&self, node_id: &DecisionNodeId) -> Vec<&TraceEdge> {
        self.edges.iter().filter(|e| &e.from == node_id).collect()
    }

    /// Get edges to a node.
    pub fn edges_to(&self, node_id: &DecisionNodeId) -> Vec<&TraceEdge> {
        self.edges.iter().filter(|e| &e.to == node_id).collect()
    }

    /// Get children of a node.
    pub fn children(&self, node_id: &DecisionNodeId) -> Vec<&DecisionNode> {
        self.edges_from(node_id)
            .iter()
            .filter_map(|e| self.get_node(&e.to))
            .collect()
    }

    /// Get parent of a node.
    pub fn parent(&self, node_id: &DecisionNodeId) -> Option<&DecisionNode> {
        self.edges_to(node_id)
            .first()
            .and_then(|e| self.get_node(&e.from))
    }

    // ==================== Decision Logging ====================

    /// Log a decision point with considered options.
    ///
    /// Records a decision with multiple options, marking one as chosen
    /// and optionally providing a reason.
    ///
    /// # Arguments
    /// * `parent_id` - The parent node (usually a goal or prior outcome)
    /// * `context` - Description of what decision is being made
    /// * `options` - List of option descriptions
    /// * `chosen_index` - Index of the chosen option (0-based)
    /// * `reason` - Why this option was chosen
    ///
    /// # Returns
    /// The ID of the chosen option node.
    pub fn log_decision(
        &mut self,
        parent_id: &DecisionNodeId,
        context: &str,
        options: &[&str],
        chosen_index: usize,
        reason: &str,
    ) -> DecisionNodeId {
        // Create decision node
        let decision = DecisionNode::decision(context);
        let decision_id = decision.id.clone();
        self.add_node(decision);
        self.add_edge(
            parent_id.clone(),
            decision_id.clone(),
            TraceEdgeLabel::Spawns,
        );

        let mut chosen_id = decision_id.clone();

        // Create option nodes
        for (i, opt) in options.iter().enumerate() {
            let mut option = DecisionNode::option(*opt);
            if i == chosen_index {
                option = option.with_reason(reason);
                chosen_id = option.id.clone();
            }
            let option_id = option.id.clone();
            self.add_node(option);

            // Link with appropriate label
            let label = if i == chosen_index {
                TraceEdgeLabel::Chooses
            } else {
                TraceEdgeLabel::Rejects
            };
            self.add_edge(decision_id.clone(), option_id, label);
        }

        chosen_id
    }

    /// Log an action taken.
    ///
    /// Records an action and its outcome.
    ///
    /// # Arguments
    /// * `parent_id` - The parent node (usually a chosen option)
    /// * `action` - Description of the action taken
    /// * `outcome` - Description of the outcome/result
    ///
    /// # Returns
    /// Tuple of (action_id, outcome_id).
    pub fn log_action(
        &mut self,
        parent_id: &DecisionNodeId,
        action: &str,
        outcome: &str,
    ) -> (DecisionNodeId, DecisionNodeId) {
        // Create action node
        let action_node = DecisionNode::action(action);
        let action_id = action_node.id.clone();
        self.add_node(action_node);
        self.add_edge(
            parent_id.clone(),
            action_id.clone(),
            TraceEdgeLabel::Implements,
        );

        // Create outcome node
        let outcome_node = DecisionNode::outcome(outcome);
        let outcome_id = outcome_node.id.clone();
        self.add_node(outcome_node);
        self.add_edge(
            action_id.clone(),
            outcome_id.clone(),
            TraceEdgeLabel::Produces,
        );

        (action_id, outcome_id)
    }

    /// Log an observation.
    ///
    /// Records an observation that may inform future decisions.
    pub fn log_observation(
        &mut self,
        parent_id: &DecisionNodeId,
        observation: &str,
    ) -> DecisionNodeId {
        let obs_node = DecisionNode::observation(observation);
        let obs_id = obs_node.id.clone();
        self.add_node(obs_node);
        self.add_edge(parent_id.clone(), obs_id.clone(), TraceEdgeLabel::LeadsTo);
        obs_id
    }

    /// Add a reference edge between nodes.
    pub fn add_reference(&mut self, from: &DecisionNodeId, to: &DecisionNodeId) {
        self.add_edge(from.clone(), to.clone(), TraceEdgeLabel::References);
    }

    // ==================== Tree Operations ====================

    /// Get the decision tree rooted at the given node.
    pub fn get_subtree(&self, root_id: &DecisionNodeId) -> DecisionTree {
        let mut tree = DecisionTree {
            root: root_id.clone(),
            nodes: HashMap::new(),
            children: HashMap::new(),
            edge_labels: HashMap::new(),
        };

        self.build_subtree(root_id, &mut tree);
        tree
    }

    /// Get the full decision tree from the root goal.
    pub fn get_tree(&self) -> DecisionTree {
        self.get_subtree(&self.root_goal)
    }

    fn build_subtree(&self, node_id: &DecisionNodeId, tree: &mut DecisionTree) {
        if let Some(node) = self.get_node(node_id) {
            tree.nodes.insert(node_id.clone(), node.clone());

            let edges = self.edges_from(node_id);
            let child_ids: Vec<DecisionNodeId> = edges.iter().map(|e| e.to.clone()).collect();

            for edge in &edges {
                tree.edge_labels
                    .insert((node_id.clone(), edge.to.clone()), edge.label);
            }

            tree.children.insert(node_id.clone(), child_ids.clone());

            for child_id in child_ids {
                self.build_subtree(&child_id, tree);
            }
        }
    }

    // ==================== Export ====================

    /// Export the trace as a Mermaid diagram.
    pub fn to_mermaid(&self) -> String {
        let mut mermaid = String::from("graph TD\n");

        // Add nodes
        for node in &self.nodes {
            let (open, close) = node.node_type.mermaid_shape();
            let label = node
                .content
                .replace('"', "'")
                .chars()
                .take(50)
                .collect::<String>();
            let label = if node.content.len() > 50 {
                format!("{}...", label)
            } else {
                label
            };
            mermaid.push_str(&format!(
                "    {}{}\"{}\"{}",
                node.id.0.as_simple(),
                open,
                label,
                close
            ));
            mermaid.push('\n');
        }

        mermaid.push('\n');

        // Add edges
        for edge in &self.edges {
            let arrow = match edge.label {
                TraceEdgeLabel::Chooses => "==>",
                TraceEdgeLabel::Rejects => "-.->",
                _ => "-->",
            };
            mermaid.push_str(&format!(
                "    {} {}|{}| {}\n",
                edge.from.0.as_simple(),
                arrow,
                edge.label,
                edge.to.0.as_simple()
            ));
        }

        // Add styling
        mermaid.push_str("\n    classDef goal fill:#90EE90\n");
        mermaid.push_str("    classDef decision fill:#FFD700\n");
        mermaid.push_str("    classDef chosen fill:#87CEEB\n");
        mermaid.push_str("    classDef rejected fill:#FFA07A\n");
        mermaid.push_str("    classDef action fill:#DDA0DD\n");
        mermaid.push_str("    classDef outcome fill:#98FB98\n");

        // Apply classes
        for node in &self.nodes {
            let class = match node.node_type {
                DecisionNodeType::Goal => "goal",
                DecisionNodeType::Decision => "decision",
                DecisionNodeType::Option => {
                    // Check if chosen or rejected
                    let is_chosen = self
                        .edges_to(&node.id)
                        .iter()
                        .any(|e| e.label == TraceEdgeLabel::Chooses);
                    if is_chosen {
                        "chosen"
                    } else {
                        "rejected"
                    }
                }
                DecisionNodeType::Action => "action",
                DecisionNodeType::Outcome => "outcome",
                DecisionNodeType::Observation => "outcome",
            };
            mermaid.push_str(&format!("    class {} {}\n", node.id.0.as_simple(), class));
        }

        mermaid
    }

    /// Export as JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Get trace statistics.
    pub fn stats(&self) -> TraceStats {
        let mut node_counts: HashMap<DecisionNodeType, usize> = HashMap::new();
        for node in &self.nodes {
            *node_counts.entry(node.node_type).or_default() += 1;
        }

        let decisions = self.nodes_by_type(DecisionNodeType::Decision).len();
        let options = self.nodes_by_type(DecisionNodeType::Option).len();

        let chosen_count = self
            .edges
            .iter()
            .filter(|e| e.label == TraceEdgeLabel::Chooses)
            .count();

        TraceStats {
            total_nodes: self.nodes.len(),
            total_edges: self.edges.len(),
            node_counts,
            max_depth: self.calculate_depth(&self.root_goal, 0),
            decision_count: decisions,
            option_count: options,
            chosen_count,
            rejected_count: options - chosen_count,
        }
    }

    fn calculate_depth(&self, node_id: &DecisionNodeId, current: usize) -> usize {
        let children = self.children(node_id);
        if children.is_empty() {
            current
        } else {
            children
                .iter()
                .map(|c| self.calculate_depth(&c.id, current + 1))
                .max()
                .unwrap_or(current)
        }
    }
}

/// A decision tree extracted from a reasoning trace.
#[derive(Debug, Clone)]
pub struct DecisionTree {
    /// Root node ID.
    pub root: DecisionNodeId,

    /// All nodes by ID.
    pub nodes: HashMap<DecisionNodeId, DecisionNode>,

    /// Children for each node.
    pub children: HashMap<DecisionNodeId, Vec<DecisionNodeId>>,

    /// Edge labels.
    pub edge_labels: HashMap<(DecisionNodeId, DecisionNodeId), TraceEdgeLabel>,
}

impl DecisionTree {
    /// Get the root node.
    pub fn root_node(&self) -> Option<&DecisionNode> {
        self.nodes.get(&self.root)
    }

    /// Get children of a node.
    pub fn get_children(&self, id: &DecisionNodeId) -> Vec<&DecisionNode> {
        self.children
            .get(id)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get the edge label between two nodes.
    pub fn get_edge_label(
        &self,
        from: &DecisionNodeId,
        to: &DecisionNodeId,
    ) -> Option<TraceEdgeLabel> {
        self.edge_labels.get(&(from.clone(), to.clone())).copied()
    }

    /// Iterate over nodes in depth-first order.
    pub fn iter_dfs(&self) -> impl Iterator<Item = &DecisionNode> {
        DfsIterator::new(self)
    }

    /// Get all leaf nodes (nodes with no children).
    pub fn leaves(&self) -> Vec<&DecisionNode> {
        self.nodes
            .iter()
            .filter(|(id, _)| self.children.get(*id).map(|c| c.is_empty()).unwrap_or(true))
            .map(|(_, n)| n)
            .collect()
    }

    /// Get the path from root to a specific node.
    pub fn path_to(&self, target: &DecisionNodeId) -> Vec<&DecisionNode> {
        let mut path = Vec::new();
        self.find_path(&self.root, target, &mut path);
        path
    }

    fn find_path<'a>(
        &'a self,
        current: &DecisionNodeId,
        target: &DecisionNodeId,
        path: &mut Vec<&'a DecisionNode>,
    ) -> bool {
        if let Some(node) = self.nodes.get(current) {
            path.push(node);
            if current == target {
                return true;
            }
            if let Some(children) = self.children.get(current) {
                for child in children {
                    if self.find_path(child, target, path) {
                        return true;
                    }
                }
            }
            path.pop();
        }
        false
    }
}

/// Depth-first iterator over a decision tree.
struct DfsIterator<'a> {
    tree: &'a DecisionTree,
    stack: Vec<&'a DecisionNodeId>,
}

impl<'a> DfsIterator<'a> {
    fn new(tree: &'a DecisionTree) -> Self {
        let mut stack = Vec::new();
        stack.push(&tree.root);
        Self { tree, stack }
    }
}

impl<'a> Iterator for DfsIterator<'a> {
    type Item = &'a DecisionNode;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(id) = self.stack.pop() {
            if let Some(node) = self.tree.nodes.get(id) {
                // Push children in reverse order for correct DFS order
                if let Some(children) = self.tree.children.get(id) {
                    for child_id in children.iter().rev() {
                        self.stack.push(child_id);
                    }
                }
                return Some(node);
            }
        }
        None
    }
}

/// Statistics about a reasoning trace.
#[derive(Debug, Clone)]
pub struct TraceStats {
    /// Total number of nodes.
    pub total_nodes: usize,

    /// Total number of edges.
    pub total_edges: usize,

    /// Count by node type.
    pub node_counts: HashMap<DecisionNodeType, usize>,

    /// Maximum depth of the tree.
    pub max_depth: usize,

    /// Number of decision points.
    pub decision_count: usize,

    /// Number of options considered.
    pub option_count: usize,

    /// Number of chosen options.
    pub chosen_count: usize,

    /// Number of rejected options.
    pub rejected_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_creation() {
        let trace = ReasoningTrace::new("Implement auth system", "session-123");
        assert_eq!(trace.session_id, "session-123");
        assert_eq!(trace.nodes.len(), 1);
        assert_eq!(trace.nodes[0].node_type, DecisionNodeType::Goal);
    }

    #[test]
    fn test_log_decision() {
        let mut trace = ReasoningTrace::new("Build API", "session-1");
        let root_id = trace.root_goal.clone();

        let chosen_id = trace.log_decision(
            &root_id,
            "Choose framework",
            &["Axum", "Actix-web", "Warp"],
            0,
            "Best performance and ergonomics",
        );

        // Should have: goal + decision + 3 options = 5 nodes
        assert_eq!(trace.nodes.len(), 5);

        // Should have: spawns + 1 chooses + 2 rejects = 4 edges
        assert_eq!(trace.edges.len(), 4);

        // Chosen option should have the reason
        let chosen = trace.get_node(&chosen_id).unwrap();
        assert!(chosen.reason.is_some());
        assert!(chosen.reason.as_ref().unwrap().contains("performance"));
    }

    #[test]
    fn test_log_action() {
        let mut trace = ReasoningTrace::new("Fix bug", "session-2");
        let root_id = trace.root_goal.clone();

        let chosen = trace.log_decision(
            &root_id,
            "Choose fix approach",
            &["Patch", "Rewrite"],
            0,
            "Less risky",
        );

        let (action_id, outcome_id) = trace.log_action(
            &chosen,
            "Apply patch to validate_input()",
            "Bug fixed, tests pass",
        );

        // Verify action node
        let action = trace.get_node(&action_id).unwrap();
        assert_eq!(action.node_type, DecisionNodeType::Action);

        // Verify outcome node
        let outcome = trace.get_node(&outcome_id).unwrap();
        assert_eq!(outcome.node_type, DecisionNodeType::Outcome);

        // Verify edges
        assert!(trace
            .edges
            .iter()
            .any(|e| e.from == chosen && e.label == TraceEdgeLabel::Implements));
        assert!(trace
            .edges
            .iter()
            .any(|e| e.from == action_id && e.label == TraceEdgeLabel::Produces));
    }

    #[test]
    fn test_get_tree() {
        let mut trace = ReasoningTrace::new("Design system", "session-3");
        let root_id = trace.root_goal.clone();

        trace.log_decision(
            &root_id,
            "Choose architecture",
            &["Monolith", "Microservices"],
            1,
            "Scalability",
        );

        let tree = trace.get_tree();
        assert!(tree.root_node().is_some());
        assert_eq!(tree.nodes.len(), trace.nodes.len());
    }

    #[test]
    fn test_mermaid_export() {
        let mut trace = ReasoningTrace::new("Test goal", "session-4");
        let root_id = trace.root_goal.clone();
        trace.log_decision(&root_id, "Test decision", &["A", "B"], 0, "Reason");

        let mermaid = trace.to_mermaid();
        assert!(mermaid.starts_with("graph TD"));
        assert!(mermaid.contains("chooses"));
        assert!(mermaid.contains("rejects"));
    }

    #[test]
    fn test_trace_stats() {
        let mut trace = ReasoningTrace::new("Stats test", "session-5");
        let root_id = trace.root_goal.clone();

        let chosen = trace.log_decision(&root_id, "Decision 1", &["A", "B", "C"], 1, "Best option");
        trace.log_action(&chosen, "Do something", "It worked");

        let stats = trace.stats();
        assert_eq!(stats.total_nodes, 7); // goal + decision + 3 options + action + outcome
        assert_eq!(stats.decision_count, 1);
        assert_eq!(stats.option_count, 3);
        assert_eq!(stats.chosen_count, 1);
        assert_eq!(stats.rejected_count, 2);
    }

    #[test]
    fn test_git_linking() {
        let trace = ReasoningTrace::new("Feature", "session-6")
            .with_git_commit("abc123")
            .with_git_branch("feature/auth");

        assert_eq!(trace.git_commit, Some("abc123".to_string()));
        assert_eq!(trace.git_branch, Some("feature/auth".to_string()));
    }

    #[test]
    fn test_tree_path_to() {
        let mut trace = ReasoningTrace::new("Root goal", "session-7");
        let root_id = trace.root_goal.clone();

        let chosen = trace.log_decision(&root_id, "Decision", &["A"], 0, "Only option");
        let (_, outcome_id) = trace.log_action(&chosen, "Action", "Outcome");

        let tree = trace.get_tree();
        let path = tree.path_to(&outcome_id);

        // Path should be: goal -> decision -> option -> action -> outcome
        assert_eq!(path.len(), 5);
        assert_eq!(path[0].node_type, DecisionNodeType::Goal);
        assert_eq!(path[4].node_type, DecisionNodeType::Outcome);
    }

    #[test]
    fn test_dfs_iteration() {
        let mut trace = ReasoningTrace::new("DFS test", "session-8");
        let root_id = trace.root_goal.clone();
        trace.log_decision(&root_id, "Decision", &["A", "B"], 0, "Test");

        let tree = trace.get_tree();
        let nodes: Vec<_> = tree.iter_dfs().collect();

        // Should visit all nodes
        assert_eq!(nodes.len(), 4); // goal + decision + 2 options
                                    // First should be root
        assert_eq!(nodes[0].node_type, DecisionNodeType::Goal);
    }
}
