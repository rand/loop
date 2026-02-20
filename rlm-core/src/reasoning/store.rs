//! Persistence layer for reasoning traces using the hypergraph memory system.
//!
//! Stores reasoning traces as subgraphs within the existing memory hypergraph,
//! enabling provenance tracking and cross-trace queries.

use crate::error::{Error, Result};
use crate::memory::{
    EdgeType, HyperEdge, Node, NodeId, NodeQuery, NodeType, SqliteMemoryStore, Tier,
};
use crate::reasoning::trace::ReasoningTrace;
use crate::reasoning::types::*;
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Store for persisting and retrieving reasoning traces.
///
/// Uses the existing SqliteMemoryStore to store traces as hypergraph subgraphs.
/// Each trace's nodes become memory nodes with type `Decision`, and edges
/// become hyperedges with type `Reasoning`.
pub struct ReasoningTraceStore {
    memory: SqliteMemoryStore,
}

impl ReasoningTraceStore {
    /// Create a new trace store backed by the given memory store.
    pub fn new(memory: SqliteMemoryStore) -> Self {
        Self { memory }
    }

    /// Create an in-memory store for testing.
    pub fn in_memory() -> Result<Self> {
        Ok(Self {
            memory: SqliteMemoryStore::in_memory()?,
        })
    }

    /// Get a reference to the underlying memory store.
    pub fn memory(&self) -> &SqliteMemoryStore {
        &self.memory
    }

    // ==================== Save Operations ====================

    /// Save a reasoning trace to the store.
    ///
    /// Converts the trace to memory nodes and hyperedges, storing them
    /// in the hypergraph with appropriate metadata for later retrieval.
    pub fn save_trace(&self, trace: &ReasoningTrace) -> Result<()> {
        // Map from DecisionNodeId to NodeId for edge creation
        let mut id_map: HashMap<DecisionNodeId, NodeId> = HashMap::new();

        // Save all decision nodes as memory nodes
        for decision_node in &trace.nodes {
            let memory_node = self.decision_node_to_memory_node(decision_node, trace)?;
            self.memory.add_node(&memory_node)?;
            id_map.insert(decision_node.id.clone(), memory_node.id);
        }

        // Save all edges as hyperedges
        for edge in &trace.edges {
            if let (Some(from_id), Some(to_id)) = (id_map.get(&edge.from), id_map.get(&edge.to)) {
                let hyperedge = self.trace_edge_to_hyperedge(edge, from_id, to_id, trace)?;
                self.memory.add_edge(&hyperedge)?;
            }
        }

        // Save trace metadata as a special "trace root" node
        self.save_trace_metadata(trace, &id_map)?;

        Ok(())
    }

    /// Convert a DecisionNode to a memory Node.
    fn decision_node_to_memory_node(
        &self,
        node: &DecisionNode,
        trace: &ReasoningTrace,
    ) -> Result<Node> {
        let mut memory_node = Node::new(NodeType::Decision, &node.content)
            .with_subtype(node.node_type.to_string())
            .with_tier(Tier::Session) // Traces start in session tier
            .with_confidence(node.confidence)
            .with_metadata("trace_id", trace.id.to_string())
            .with_metadata("decision_node_id", node.id.to_string())
            .with_metadata("decision_node_type", node.node_type.to_string())
            .with_metadata("session_id", trace.session_id.clone());

        // Add reason if present
        if let Some(ref reason) = node.reason {
            memory_node = memory_node.with_metadata("reason", reason.clone());
        }

        // Copy any existing metadata
        if let Some(ref meta) = node.metadata {
            for (k, v) in meta {
                memory_node = memory_node.with_metadata(k.clone(), v.clone());
            }
        }

        // Add git provenance if available
        if let Some(ref commit) = trace.git_commit {
            memory_node = memory_node.with_metadata("git_commit", commit.clone());
        }
        if let Some(ref branch) = trace.git_branch {
            memory_node = memory_node.with_metadata("git_branch", branch.clone());
        }

        // Override created_at with the original timestamp
        // Note: The Node::new sets created_at to now, but we want the original
        memory_node.created_at = node.created_at;
        memory_node.updated_at = trace.updated_at;

        Ok(memory_node)
    }

    /// Convert a TraceEdge to a HyperEdge.
    fn trace_edge_to_hyperedge(
        &self,
        edge: &TraceEdge,
        from_id: &NodeId,
        to_id: &NodeId,
        trace: &ReasoningTrace,
    ) -> Result<HyperEdge> {
        let mut hyperedge = HyperEdge::binary(
            EdgeType::Reasoning,
            from_id.clone(),
            to_id.clone(),
            edge.label.to_string(),
        )
        .with_weight(edge.weight);

        // Store the edge label in metadata for reconstruction
        let metadata = json!({
            "trace_id": trace.id.to_string(),
            "trace_edge_label": edge.label.to_string(),
            "session_id": trace.session_id,
        });

        hyperedge.metadata =
            Some(serde_json::from_value(metadata).map_err(|e| {
                Error::Internal(format!("Failed to serialize edge metadata: {}", e))
            })?);

        Ok(hyperedge)
    }

    /// Save trace-level metadata as a special node.
    fn save_trace_metadata(
        &self,
        trace: &ReasoningTrace,
        id_map: &HashMap<DecisionNodeId, NodeId>,
    ) -> Result<()> {
        let root_memory_id = id_map
            .get(&trace.root_goal)
            .ok_or_else(|| Error::Internal("Root goal not found in id_map".to_string()))?;

        // Create a trace root node that links to the actual root goal
        let trace_root = Node::new(NodeType::Decision, format!("Trace: {}", trace.id))
            .with_subtype("trace_root")
            .with_tier(Tier::Session)
            .with_metadata("trace_id", trace.id.to_string())
            .with_metadata("root_goal_id", root_memory_id.to_string())
            .with_metadata("session_id", trace.session_id.clone())
            .with_metadata("created_at", trace.created_at.to_rfc3339())
            .with_metadata("node_count", trace.nodes.len() as i64)
            .with_metadata("edge_count", trace.edges.len() as i64);

        let trace_root = if let Some(ref commit) = trace.git_commit {
            trace_root.with_metadata("git_commit", commit.clone())
        } else {
            trace_root
        };

        let trace_root = if let Some(ref branch) = trace.git_branch {
            trace_root.with_metadata("git_branch", branch.clone())
        } else {
            trace_root
        };

        self.memory.add_node(&trace_root)?;

        // Link trace root to actual root goal
        let link = HyperEdge::binary(
            EdgeType::Structural,
            trace_root.id,
            root_memory_id.clone(),
            "trace_root",
        );
        self.memory.add_edge(&link)?;

        Ok(())
    }

    // ==================== Load Operations ====================

    /// Load a reasoning trace by its ID.
    pub fn load_trace(&self, trace_id: &TraceId) -> Result<Option<ReasoningTrace>> {
        // Find the trace root node
        let trace_root = self.find_trace_root(trace_id)?;
        let trace_root = match trace_root {
            Some(node) => node,
            None => return Ok(None),
        };

        // Get session_id from trace root
        let session_id = trace_root
            .metadata
            .as_ref()
            .and_then(|m| m.get("session_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Find all nodes belonging to this trace
        let memory_nodes = self.find_trace_nodes(trace_id)?;
        if memory_nodes.is_empty() {
            return Ok(None);
        }

        // Find root goal node (first goal node, or use trace root reference)
        let root_memory_id = trace_root
            .metadata
            .as_ref()
            .and_then(|m| m.get("root_goal_id"))
            .and_then(|v| v.as_str())
            .and_then(|s| NodeId::parse(s).ok());

        // Convert memory nodes back to decision nodes
        let mut decision_nodes = Vec::new();
        let mut memory_to_decision: HashMap<NodeId, DecisionNodeId> = HashMap::new();

        for memory_node in &memory_nodes {
            let decision_node = self.memory_node_to_decision_node(memory_node)?;
            memory_to_decision.insert(memory_node.id.clone(), decision_node.id.clone());
            decision_nodes.push(decision_node);
        }

        // Determine root goal
        let root_goal = root_memory_id
            .and_then(|id| memory_to_decision.get(&id).cloned())
            .or_else(|| {
                decision_nodes
                    .iter()
                    .find(|n| n.node_type == DecisionNodeType::Goal)
                    .map(|n| n.id.clone())
            })
            .unwrap_or_else(|| {
                decision_nodes
                    .first()
                    .map(|n| n.id.clone())
                    .unwrap_or_default()
            });

        // Load edges
        let edges = self.load_trace_edges(trace_id, &memory_to_decision)?;

        // Extract git info from trace root
        let git_commit = trace_root
            .metadata
            .as_ref()
            .and_then(|m| m.get("git_commit"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let git_branch = trace_root
            .metadata
            .as_ref()
            .and_then(|m| m.get("git_branch"))
            .and_then(|v| v.as_str())
            .map(String::from);

        // Get created_at from trace root
        let created_at = trace_root
            .metadata
            .as_ref()
            .and_then(|m| m.get("created_at"))
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| trace_root.created_at);

        Ok(Some(ReasoningTrace {
            id: trace_id.clone(),
            root_goal,
            session_id,
            created_at,
            updated_at: trace_root.updated_at,
            nodes: decision_nodes,
            edges,
            git_commit,
            git_branch,
            metadata: trace_root.metadata.clone(),
        }))
    }

    /// Find the trace root node.
    fn find_trace_root(&self, trace_id: &TraceId) -> Result<Option<Node>> {
        let nodes = self
            .memory
            .query_nodes(&NodeQuery::new().node_types(vec![NodeType::Decision]))?;

        Ok(nodes.into_iter().find(|n| {
            n.subtype.as_deref() == Some("trace_root")
                && n.metadata
                    .as_ref()
                    .and_then(|m| m.get("trace_id"))
                    .and_then(|v| v.as_str())
                    == Some(&trace_id.to_string())
        }))
    }

    /// Find all nodes belonging to a trace.
    fn find_trace_nodes(&self, trace_id: &TraceId) -> Result<Vec<Node>> {
        let nodes = self
            .memory
            .query_nodes(&NodeQuery::new().node_types(vec![NodeType::Decision]))?;

        Ok(nodes
            .into_iter()
            .filter(|n| {
                n.subtype.as_deref() != Some("trace_root")
                    && n.metadata
                        .as_ref()
                        .and_then(|m| m.get("trace_id"))
                        .and_then(|v| v.as_str())
                        == Some(&trace_id.to_string())
            })
            .collect())
    }

    /// Convert a memory node back to a decision node.
    fn memory_node_to_decision_node(&self, node: &Node) -> Result<DecisionNode> {
        // Get the decision node type from subtype
        let node_type = node
            .subtype
            .as_ref()
            .map(|s| match s.as_str() {
                "goal" => DecisionNodeType::Goal,
                "decision" => DecisionNodeType::Decision,
                "option" => DecisionNodeType::Option,
                "action" => DecisionNodeType::Action,
                "outcome" => DecisionNodeType::Outcome,
                "observation" => DecisionNodeType::Observation,
                _ => DecisionNodeType::Observation,
            })
            .unwrap_or(DecisionNodeType::Observation);

        // Get original decision node ID
        let id = node
            .metadata
            .as_ref()
            .and_then(|m| m.get("decision_node_id"))
            .and_then(|v| v.as_str())
            .and_then(|s| DecisionNodeId::parse(s).ok())
            .unwrap_or_else(DecisionNodeId::new);

        // Get reason
        let reason = node
            .metadata
            .as_ref()
            .and_then(|m| m.get("reason"))
            .and_then(|v| v.as_str())
            .map(String::from);

        // Filter out our internal metadata keys
        let internal_keys = [
            "trace_id",
            "decision_node_id",
            "decision_node_type",
            "session_id",
            "reason",
            "git_commit",
            "git_branch",
        ];
        let metadata: Option<HashMap<String, Value>> = node.metadata.as_ref().map(|m| {
            m.iter()
                .filter(|(k, _)| !internal_keys.contains(&k.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        });
        let metadata = metadata.filter(|m| !m.is_empty());

        Ok(DecisionNode {
            id,
            node_type,
            content: node.content.clone(),
            reason,
            confidence: node.confidence,
            created_at: node.created_at,
            metadata,
        })
    }

    /// Load edges for a trace.
    fn load_trace_edges(
        &self,
        trace_id: &TraceId,
        memory_to_decision: &HashMap<NodeId, DecisionNodeId>,
    ) -> Result<Vec<TraceEdge>> {
        let mut edges = Vec::new();

        // We need to find edges by querying nodes and their connected edges
        for memory_id in memory_to_decision.keys() {
            let hyperedges = self.memory.get_edges_for_node(memory_id)?;

            for hyperedge in hyperedges {
                // Check if this edge belongs to our trace
                let is_trace_edge = hyperedge
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("trace_id"))
                    .and_then(|v| v.as_str())
                    == Some(&trace_id.to_string());

                if !is_trace_edge {
                    continue;
                }

                // Get the edge label
                let label = hyperedge
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("trace_edge_label"))
                    .and_then(|v| v.as_str())
                    .map(|s| match s {
                        "spawns" => TraceEdgeLabel::Spawns,
                        "considers" => TraceEdgeLabel::Considers,
                        "chooses" => TraceEdgeLabel::Chooses,
                        "rejects" => TraceEdgeLabel::Rejects,
                        "implements" => TraceEdgeLabel::Implements,
                        "produces" => TraceEdgeLabel::Produces,
                        "leads_to" => TraceEdgeLabel::LeadsTo,
                        "references" => TraceEdgeLabel::References,
                        "requires" => TraceEdgeLabel::Requires,
                        "invalidates" => TraceEdgeLabel::Invalidates,
                        _ => TraceEdgeLabel::References,
                    })
                    .unwrap_or(TraceEdgeLabel::References);

                // Get from and to node IDs
                if hyperedge.members.len() >= 2 {
                    let from_memory = &hyperedge.members[0].node_id;
                    let to_memory = &hyperedge.members[1].node_id;

                    if let (Some(from), Some(to)) = (
                        memory_to_decision.get(from_memory),
                        memory_to_decision.get(to_memory),
                    ) {
                        // Only add if we haven't seen this edge before
                        let edge = TraceEdge::new(from.clone(), to.clone(), label)
                            .with_weight(hyperedge.weight);

                        if !edges
                            .iter()
                            .any(|e: &TraceEdge| e.from == edge.from && e.to == edge.to)
                        {
                            edges.push(edge);
                        }
                    }
                }
            }
        }

        Ok(edges)
    }

    // ==================== Query Operations ====================

    /// List all trace IDs in the store.
    pub fn list_traces(&self) -> Result<Vec<TraceId>> {
        let nodes = self
            .memory
            .query_nodes(&NodeQuery::new().node_types(vec![NodeType::Decision]))?;

        let trace_ids: Vec<TraceId> = nodes
            .iter()
            .filter(|n| n.subtype.as_deref() == Some("trace_root"))
            .filter_map(|n| {
                n.metadata
                    .as_ref()
                    .and_then(|m| m.get("trace_id"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| TraceId::parse(s).ok())
            })
            .collect();

        Ok(trace_ids)
    }

    /// Find traces by session ID.
    pub fn find_by_session(&self, session_id: &str) -> Result<Vec<TraceId>> {
        let nodes = self
            .memory
            .query_nodes(&NodeQuery::new().node_types(vec![NodeType::Decision]))?;

        let trace_ids: Vec<TraceId> = nodes
            .iter()
            .filter(|n| n.subtype.as_deref() == Some("trace_root"))
            .filter(|n| {
                n.metadata
                    .as_ref()
                    .and_then(|m| m.get("session_id"))
                    .and_then(|v| v.as_str())
                    == Some(session_id)
            })
            .filter_map(|n| {
                n.metadata
                    .as_ref()
                    .and_then(|m| m.get("trace_id"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| TraceId::parse(s).ok())
            })
            .collect();

        Ok(trace_ids)
    }

    /// Find traces linked to a git commit.
    pub fn find_by_commit(&self, commit: &str) -> Result<Vec<TraceId>> {
        let nodes = self
            .memory
            .query_nodes(&NodeQuery::new().node_types(vec![NodeType::Decision]))?;

        let trace_ids: Vec<TraceId> = nodes
            .iter()
            .filter(|n| n.subtype.as_deref() == Some("trace_root"))
            .filter(|n| {
                n.metadata
                    .as_ref()
                    .and_then(|m| m.get("git_commit"))
                    .and_then(|v| v.as_str())
                    == Some(commit)
            })
            .filter_map(|n| {
                n.metadata
                    .as_ref()
                    .and_then(|m| m.get("trace_id"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| TraceId::parse(s).ok())
            })
            .collect();

        Ok(trace_ids)
    }

    /// Delete a trace and all its nodes/edges.
    pub fn delete_trace(&self, trace_id: &TraceId) -> Result<bool> {
        let nodes = self.find_trace_nodes(trace_id)?;
        let trace_root = self.find_trace_root(trace_id)?;

        if nodes.is_empty() && trace_root.is_none() {
            return Ok(false);
        }

        // Delete all trace nodes
        for node in nodes {
            self.memory.delete_node(&node.id)?;
        }

        // Delete trace root
        if let Some(root) = trace_root {
            self.memory.delete_node(&root.id)?;
        }

        Ok(true)
    }

    /// Get statistics about stored traces.
    pub fn stats(&self) -> Result<TraceStoreStats> {
        let memory_stats = self.memory.stats()?;
        let trace_ids = self.list_traces()?;

        let decision_nodes = *memory_stats
            .nodes_by_type
            .get(&NodeType::Decision)
            .unwrap_or(&0);

        Ok(TraceStoreStats {
            total_traces: trace_ids.len(),
            total_decision_nodes: decision_nodes as usize,
            total_memory_nodes: memory_stats.total_nodes as usize,
            total_edges: memory_stats.total_edges as usize,
        })
    }
}

/// Statistics about the trace store.
#[derive(Debug, Clone)]
pub struct TraceStoreStats {
    /// Total number of traces stored.
    pub total_traces: usize,

    /// Total number of decision nodes (across all traces).
    pub total_decision_nodes: usize,

    /// Total number of memory nodes (all types).
    pub total_memory_nodes: usize,

    /// Total number of edges.
    pub total_edges: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_trace() {
        let store = ReasoningTraceStore::in_memory().unwrap();

        // Create a trace
        let mut trace = ReasoningTrace::new("Test goal", "session-test");
        let root_id = trace.root_goal.clone();
        let chosen = trace.log_decision(
            &root_id,
            "Choose approach",
            &["Option A", "Option B"],
            0,
            "Better fit",
        );
        trace.log_action(&chosen, "Implement A", "Success");

        let trace_id = trace.id.clone();

        // Save
        store.save_trace(&trace).unwrap();

        // Load
        let loaded = store.load_trace(&trace_id).unwrap().unwrap();

        assert_eq!(loaded.id, trace_id);
        assert_eq!(loaded.session_id, "session-test");
        assert_eq!(loaded.nodes.len(), trace.nodes.len());
        assert_eq!(loaded.edges.len(), trace.edges.len());
    }

    #[test]
    fn test_list_traces() {
        let store = ReasoningTraceStore::in_memory().unwrap();

        // Create and save multiple traces
        let trace1 = ReasoningTrace::new("Goal 1", "session-1");
        let trace2 = ReasoningTrace::new("Goal 2", "session-2");

        store.save_trace(&trace1).unwrap();
        store.save_trace(&trace2).unwrap();

        let traces = store.list_traces().unwrap();
        assert_eq!(traces.len(), 2);
    }

    #[test]
    fn test_find_by_session() {
        let store = ReasoningTraceStore::in_memory().unwrap();

        let trace1 = ReasoningTrace::new("Goal 1", "session-a");
        let trace2 = ReasoningTrace::new("Goal 2", "session-a");
        let trace3 = ReasoningTrace::new("Goal 3", "session-b");

        store.save_trace(&trace1).unwrap();
        store.save_trace(&trace2).unwrap();
        store.save_trace(&trace3).unwrap();

        let session_a_traces = store.find_by_session("session-a").unwrap();
        assert_eq!(session_a_traces.len(), 2);

        let session_b_traces = store.find_by_session("session-b").unwrap();
        assert_eq!(session_b_traces.len(), 1);
    }

    #[test]
    fn test_find_by_commit() {
        let store = ReasoningTraceStore::in_memory().unwrap();

        let trace1 = ReasoningTrace::new("Feature 1", "session-1").with_git_commit("abc123");
        let trace2 = ReasoningTrace::new("Feature 2", "session-2").with_git_commit("abc123");
        let trace3 = ReasoningTrace::new("Feature 3", "session-3").with_git_commit("def456");

        store.save_trace(&trace1).unwrap();
        store.save_trace(&trace2).unwrap();
        store.save_trace(&trace3).unwrap();

        let commit_traces = store.find_by_commit("abc123").unwrap();
        assert_eq!(commit_traces.len(), 2);
    }

    #[test]
    fn test_delete_trace() {
        let store = ReasoningTraceStore::in_memory().unwrap();

        let trace = ReasoningTrace::new("To delete", "session-del");
        let trace_id = trace.id.clone();

        store.save_trace(&trace).unwrap();
        assert!(store.load_trace(&trace_id).unwrap().is_some());

        let deleted = store.delete_trace(&trace_id).unwrap();
        assert!(deleted);

        assert!(store.load_trace(&trace_id).unwrap().is_none());
    }

    #[test]
    fn test_store_stats() {
        let store = ReasoningTraceStore::in_memory().unwrap();

        let mut trace = ReasoningTrace::new("Stats test", "session-stats");
        let root_id = trace.root_goal.clone();
        trace.log_decision(&root_id, "Decision", &["A", "B"], 0, "Reason");

        store.save_trace(&trace).unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.total_traces, 1);
        assert!(stats.total_decision_nodes > 0);
    }

    #[test]
    fn test_git_info_roundtrip() {
        let store = ReasoningTraceStore::in_memory().unwrap();

        let trace = ReasoningTrace::new("Git test", "session-git")
            .with_git_commit("abc123def")
            .with_git_branch("feature/test");

        let trace_id = trace.id.clone();
        store.save_trace(&trace).unwrap();

        let loaded = store.load_trace(&trace_id).unwrap().unwrap();
        assert_eq!(loaded.git_commit, Some("abc123def".to_string()));
        assert_eq!(loaded.git_branch, Some("feature/test".to_string()));
    }

    #[test]
    fn test_node_types_preserved() {
        let store = ReasoningTraceStore::in_memory().unwrap();

        let mut trace = ReasoningTrace::new("Types test", "session-types");
        let root_id = trace.root_goal.clone();

        let chosen = trace.log_decision(&root_id, "Decision", &["Option"], 0, "Only choice");
        let (action_id, outcome_id) = trace.log_action(&chosen, "Action", "Outcome");
        trace.log_observation(&outcome_id, "Observation");

        let trace_id = trace.id.clone();
        store.save_trace(&trace).unwrap();

        let loaded = store.load_trace(&trace_id).unwrap().unwrap();

        // Verify all node types
        let goal_count = loaded
            .nodes
            .iter()
            .filter(|n| n.node_type == DecisionNodeType::Goal)
            .count();
        let decision_count = loaded
            .nodes
            .iter()
            .filter(|n| n.node_type == DecisionNodeType::Decision)
            .count();
        let option_count = loaded
            .nodes
            .iter()
            .filter(|n| n.node_type == DecisionNodeType::Option)
            .count();
        let action_count = loaded
            .nodes
            .iter()
            .filter(|n| n.node_type == DecisionNodeType::Action)
            .count();
        let outcome_count = loaded
            .nodes
            .iter()
            .filter(|n| n.node_type == DecisionNodeType::Outcome)
            .count();
        let obs_count = loaded
            .nodes
            .iter()
            .filter(|n| n.node_type == DecisionNodeType::Observation)
            .count();

        assert_eq!(goal_count, 1);
        assert_eq!(decision_count, 1);
        assert_eq!(option_count, 1);
        assert_eq!(action_count, 1);
        assert_eq!(outcome_count, 1);
        assert_eq!(obs_count, 1);
    }
}
