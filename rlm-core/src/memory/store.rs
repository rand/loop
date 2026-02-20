//! SQLite-backed memory store implementation.

use crate::error::{Error, Result};
use crate::memory::schema::{initialize_schema, is_initialized};
use crate::memory::types::*;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// SQLite-backed memory store.
pub struct SqliteMemoryStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteMemoryStore {
    /// Open or create a memory store at the given path.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path).map_err(|e| Error::MemoryStorage(e.to_string()))?;

        if !is_initialized(&conn) {
            initialize_schema(&conn).map_err(|e| Error::MemoryStorage(e.to_string()))?;
        }

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create an in-memory store (for testing).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| Error::MemoryStorage(e.to_string()))?;
        initialize_schema(&conn).map_err(|e| Error::MemoryStorage(e.to_string()))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn with_conn<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> rusqlite::Result<T>,
    {
        let conn = self
            .conn
            .lock()
            .map_err(|e| Error::Internal(format!("Failed to lock connection: {}", e)))?;
        f(&conn).map_err(|e| Error::MemoryStorage(e.to_string()))
    }

    // ==================== Node Operations ====================

    /// Add a node to the store.
    pub fn add_node(&self, node: &Node) -> Result<()> {
        self.with_conn(|conn| {
            let embedding_blob = node
                .embedding
                .as_ref()
                .map(|e| e.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>());

            let provenance_context = node
                .provenance
                .as_ref()
                .and_then(|p| p.context.as_ref())
                .map(|c| serde_json::to_string(c).unwrap_or_default());

            let metadata = node
                .metadata
                .as_ref()
                .map(|m| serde_json::to_string(m).unwrap_or_default());

            conn.execute(
                "INSERT INTO nodes (
                    id, node_type, subtype, content, embedding, tier, confidence,
                    provenance_source, provenance_ref, provenance_observed_at, provenance_context,
                    created_at, updated_at, last_accessed, access_count, metadata
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    node.id.to_string(),
                    node.node_type.to_string(),
                    node.subtype,
                    node.content,
                    embedding_blob,
                    node.tier as i32,
                    node.confidence,
                    node.provenance
                        .as_ref()
                        .map(|p| format!("{:?}", p.source_type)),
                    node.provenance.as_ref().and_then(|p| p.source_ref.clone()),
                    node.provenance.as_ref().map(|p| p.observed_at.to_rfc3339()),
                    provenance_context,
                    node.created_at.to_rfc3339(),
                    node.updated_at.to_rfc3339(),
                    node.last_accessed.to_rfc3339(),
                    node.access_count as i64,
                    metadata,
                ],
            )?;
            Ok(())
        })
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: &NodeId) -> Result<Option<Node>> {
        self.with_conn(|conn| {
            conn.query_row(
                "SELECT id, node_type, subtype, content, embedding, tier, confidence,
                        provenance_source, provenance_ref, provenance_observed_at, provenance_context,
                        created_at, updated_at, last_accessed, access_count, metadata
                 FROM nodes WHERE id = ?1",
                params![id.to_string()],
                |row| Self::row_to_node(row),
            )
            .optional()
        })
    }

    /// Update a node.
    pub fn update_node(&self, node: &Node) -> Result<()> {
        self.with_conn(|conn| {
            let embedding_blob = node
                .embedding
                .as_ref()
                .map(|e| e.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<u8>>());

            let metadata = node
                .metadata
                .as_ref()
                .map(|m| serde_json::to_string(m).unwrap_or_default());

            conn.execute(
                "UPDATE nodes SET
                    content = ?2, embedding = ?3, tier = ?4, confidence = ?5,
                    updated_at = ?6, last_accessed = ?7, access_count = ?8, metadata = ?9
                 WHERE id = ?1",
                params![
                    node.id.to_string(),
                    node.content,
                    embedding_blob,
                    node.tier as i32,
                    node.confidence,
                    node.updated_at.to_rfc3339(),
                    node.last_accessed.to_rfc3339(),
                    node.access_count as i64,
                    metadata,
                ],
            )?;
            Ok(())
        })
    }

    /// Delete a node.
    pub fn delete_node(&self, id: &NodeId) -> Result<bool> {
        self.with_conn(|conn| {
            let rows = conn.execute("DELETE FROM nodes WHERE id = ?1", params![id.to_string()])?;
            Ok(rows > 0)
        })
    }

    /// Query nodes.
    pub fn query_nodes(&self, query: &NodeQuery) -> Result<Vec<Node>> {
        self.with_conn(|conn| {
            let mut sql = String::from(
                "SELECT id, node_type, subtype, content, embedding, tier, confidence,
                        provenance_source, provenance_ref, provenance_observed_at, provenance_context,
                        created_at, updated_at, last_accessed, access_count, metadata
                 FROM nodes WHERE 1=1",
            );
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(ref types) = query.node_types {
                let placeholders: Vec<String> = types.iter().map(|_| "?".to_string()).collect();
                sql.push_str(&format!(" AND node_type IN ({})", placeholders.join(",")));
                for t in types {
                    params_vec.push(Box::new(t.to_string()));
                }
            }

            if let Some(ref tiers) = query.tiers {
                let placeholders: Vec<String> = tiers.iter().map(|_| "?".to_string()).collect();
                sql.push_str(&format!(" AND tier IN ({})", placeholders.join(",")));
                for t in tiers {
                    params_vec.push(Box::new(*t as i32));
                }
            }

            if let Some(min_conf) = query.min_confidence {
                sql.push_str(" AND confidence >= ?");
                params_vec.push(Box::new(min_conf));
            }

            sql.push_str(" ORDER BY last_accessed DESC");

            if let Some(limit) = query.limit {
                sql.push_str(&format!(" LIMIT {}", limit));
            }

            if let Some(offset) = query.offset {
                sql.push_str(&format!(" OFFSET {}", offset));
            }

            let params_refs: Vec<&dyn rusqlite::ToSql> =
                params_vec.iter().map(|b| b.as_ref()).collect();

            let mut stmt = conn.prepare(&sql)?;
            let nodes = stmt
                .query_map(params_refs.as_slice(), |row| Self::row_to_node(row))?
                .filter_map(|r| r.ok())
                .collect();

            Ok(nodes)
        })
    }

    /// Full-text search on content.
    pub fn search_content(&self, query: &str, limit: usize) -> Result<Vec<Node>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT n.id, n.node_type, n.subtype, n.content, n.embedding, n.tier, n.confidence,
                        n.provenance_source, n.provenance_ref, n.provenance_observed_at, n.provenance_context,
                        n.created_at, n.updated_at, n.last_accessed, n.access_count, n.metadata
                 FROM nodes n
                 JOIN nodes_fts fts ON n.rowid = fts.rowid
                 WHERE nodes_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2",
            )?;

            let nodes = stmt
                .query_map(params![query, limit as i64], |row| Self::row_to_node(row))?
                .filter_map(|r| r.ok())
                .collect();

            Ok(nodes)
        })
    }

    fn row_to_node(row: &rusqlite::Row) -> rusqlite::Result<Node> {
        let id_str: String = row.get(0)?;
        let node_type_str: String = row.get(1)?;
        let tier_int: i32 = row.get(5)?;

        let embedding: Option<Vec<f32>> = row.get::<_, Option<Vec<u8>>>(4)?.map(|bytes| {
            bytes
                .chunks(4)
                .map(|chunk| {
                    let arr: [u8; 4] = chunk.try_into().unwrap_or([0; 4]);
                    f32::from_le_bytes(arr)
                })
                .collect()
        });

        let metadata: Option<HashMap<String, Value>> = row
            .get::<_, Option<String>>(15)?
            .and_then(|s| serde_json::from_str(&s).ok());

        let node_type = match node_type_str.as_str() {
            "entity" => NodeType::Entity,
            "fact" => NodeType::Fact,
            "experience" => NodeType::Experience,
            "decision" => NodeType::Decision,
            "snippet" => NodeType::Snippet,
            _ => NodeType::Fact,
        };

        let tier = match tier_int {
            0 => Tier::Task,
            1 => Tier::Session,
            2 => Tier::LongTerm,
            3 => Tier::Archive,
            _ => Tier::Task,
        };

        Ok(Node {
            id: NodeId::parse(&id_str).unwrap_or_else(|_| NodeId::new()),
            node_type,
            subtype: row.get(2)?,
            content: row.get(3)?,
            embedding,
            tier,
            confidence: row.get(6)?,
            provenance: None, // Simplified for now
            created_at: parse_datetime(row.get::<_, String>(11)?),
            updated_at: parse_datetime(row.get::<_, String>(12)?),
            last_accessed: parse_datetime(row.get::<_, String>(13)?),
            access_count: row.get::<_, i64>(14)? as u64,
            metadata,
        })
    }

    // ==================== Edge Operations ====================

    /// Add a hyperedge.
    pub fn add_edge(&self, edge: &HyperEdge) -> Result<()> {
        self.with_conn(|conn| {
            let metadata = edge
                .metadata
                .as_ref()
                .map(|m| serde_json::to_string(m).unwrap_or_default());

            conn.execute(
                "INSERT INTO hyperedges (id, edge_type, label, weight, created_at, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    edge.id.to_string(),
                    edge.edge_type.to_string(),
                    edge.label,
                    edge.weight,
                    edge.created_at.to_rfc3339(),
                    metadata,
                ],
            )?;

            // Add memberships
            for member in &edge.members {
                conn.execute(
                    "INSERT INTO membership (hyperedge_id, node_id, role, position)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        edge.id.to_string(),
                        member.node_id.to_string(),
                        member.role,
                        member.position,
                    ],
                )?;
            }

            Ok(())
        })
    }

    /// Get edges connected to a node.
    pub fn get_edges_for_node(&self, node_id: &NodeId) -> Result<Vec<HyperEdge>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT e.id, e.edge_type, e.label, e.weight, e.created_at, e.metadata
                 FROM hyperedges e
                 JOIN membership m ON e.id = m.hyperedge_id
                 WHERE m.node_id = ?1",
            )?;

            let edge_ids: Vec<String> = stmt
                .query_map(params![node_id.to_string()], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();

            let mut edges = Vec::new();
            for edge_id in edge_ids {
                if let Some(edge) = self.get_edge_internal(conn, &edge_id)? {
                    edges.push(edge);
                }
            }

            Ok(edges)
        })
    }

    fn get_edge_internal(
        &self,
        conn: &Connection,
        edge_id: &str,
    ) -> rusqlite::Result<Option<HyperEdge>> {
        let edge_opt = conn
            .query_row(
                "SELECT id, edge_type, label, weight, created_at, metadata
                 FROM hyperedges WHERE id = ?1",
                params![edge_id],
                |row| {
                    let edge_type_str: String = row.get(1)?;
                    let edge_type = match edge_type_str.as_str() {
                        "semantic" => EdgeType::Semantic,
                        "structural" => EdgeType::Structural,
                        "causal" => EdgeType::Causal,
                        "temporal" => EdgeType::Temporal,
                        "reference" => EdgeType::Reference,
                        "reasoning" => EdgeType::Reasoning,
                        _ => EdgeType::Semantic,
                    };

                    Ok(HyperEdge {
                        id: EdgeId::parse(&row.get::<_, String>(0)?)
                            .unwrap_or_else(|_| EdgeId::new()),
                        edge_type,
                        label: row.get(2)?,
                        weight: row.get(3)?,
                        members: Vec::new(),
                        created_at: parse_datetime(row.get::<_, String>(4)?),
                        metadata: row
                            .get::<_, Option<String>>(5)?
                            .and_then(|s| serde_json::from_str(&s).ok()),
                    })
                },
            )
            .optional()?;

        if let Some(mut edge) = edge_opt {
            // Load members
            let mut stmt = conn.prepare(
                "SELECT node_id, role, position FROM membership WHERE hyperedge_id = ?1 ORDER BY position",
            )?;
            edge.members = stmt
                .query_map(params![edge_id], |row| {
                    Ok(EdgeMember {
                        node_id: NodeId::parse(&row.get::<_, String>(0)?)
                            .unwrap_or_else(|_| NodeId::new()),
                        role: row.get(1)?,
                        position: row.get(2)?,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(Some(edge))
        } else {
            Ok(None)
        }
    }

    /// Delete an edge.
    pub fn delete_edge(&self, id: &EdgeId) -> Result<bool> {
        self.with_conn(|conn| {
            let rows = conn.execute(
                "DELETE FROM hyperedges WHERE id = ?1",
                params![id.to_string()],
            )?;
            Ok(rows > 0)
        })
    }

    // ==================== Evolution Operations ====================

    /// Promote nodes to a higher tier.
    pub fn promote(&self, node_ids: &[NodeId], reason: &str) -> Result<Vec<NodeId>> {
        let mut promoted = Vec::new();

        for node_id in node_ids {
            if let Some(mut node) = self.get_node(node_id)? {
                if let Some(next_tier) = node.tier.next() {
                    let from_tier = node.tier;
                    node.tier = next_tier;
                    node.updated_at = Utc::now();
                    self.update_node(&node)?;
                    self.log_evolution(
                        node_id,
                        "promote",
                        Some(from_tier),
                        Some(next_tier),
                        reason,
                    )?;
                    promoted.push(node_id.clone());
                }
            }
        }

        Ok(promoted)
    }

    /// Apply decay to nodes based on time and access patterns.
    pub fn decay(&self, factor: f64, min_confidence: f64) -> Result<Vec<NodeId>> {
        let nodes = self.query_nodes(&NodeQuery::new().min_confidence(min_confidence))?;
        let mut decayed = Vec::new();

        for mut node in nodes {
            // Decay formula: confidence * factor^(hours_since_access / 24)
            let hours = (Utc::now() - node.last_accessed).num_hours() as f64;
            let decay_exponent = hours / 24.0;
            let new_confidence = node.confidence * factor.powf(decay_exponent);

            if new_confidence < node.confidence {
                node.confidence = new_confidence.max(0.0);
                node.updated_at = Utc::now();
                self.update_node(&node)?;
                decayed.push(node.id.clone());
            }
        }

        Ok(decayed)
    }

    /// Consolidate nodes from one tier to another.
    pub fn consolidate(&self, from_tier: Tier, to_tier: Tier) -> Result<ConsolidationResult> {
        let nodes = self.query_nodes(&NodeQuery::new().tiers(vec![from_tier]))?;
        let source_ids: Vec<NodeId> = nodes.iter().map(|n| n.id.clone()).collect();

        // For now, just promote nodes above a confidence threshold
        let promoted = self.promote(
            &source_ids
                .iter()
                .filter(|_| true)
                .cloned()
                .collect::<Vec<_>>(),
            &format!("Consolidation from {} to {}", from_tier, to_tier),
        )?;

        Ok(ConsolidationResult {
            source_nodes: source_ids,
            consolidated_node: None,
            promoted_nodes: promoted,
            archived_nodes: Vec::new(),
            summary: format!("Consolidated from {} to {}", from_tier, to_tier),
        })
    }

    /// Log an evolution event.
    fn log_evolution(
        &self,
        node_id: &NodeId,
        operation: &str,
        from_tier: Option<Tier>,
        to_tier: Option<Tier>,
        reason: &str,
    ) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO evolution_log (node_id, operation, from_tier, to_tier, reason)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    node_id.to_string(),
                    operation,
                    from_tier.map(|t| t as i32),
                    to_tier.map(|t| t as i32),
                    reason,
                ],
            )?;
            Ok(())
        })
    }

    /// Get evolution history for a node.
    pub fn get_evolution_history(&self, node_id: &NodeId) -> Result<Vec<EvolutionEntry>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT operation, from_tier, to_tier, reason, created_at
                 FROM evolution_log WHERE node_id = ?1 ORDER BY created_at DESC",
            )?;

            let entries = stmt
                .query_map(params![node_id.to_string()], |row| {
                    Ok(EvolutionEntry {
                        operation: row.get(0)?,
                        from_tier: row.get::<_, Option<i32>>(1)?.map(int_to_tier),
                        to_tier: row.get::<_, Option<i32>>(2)?.map(int_to_tier),
                        reason: row.get(3)?,
                        timestamp: parse_datetime(row.get::<_, String>(4)?),
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();

            Ok(entries)
        })
    }

    /// Get statistics about the memory store.
    pub fn stats(&self) -> Result<MemoryStats> {
        self.with_conn(|conn| {
            let total_nodes: i64 =
                conn.query_row("SELECT COUNT(*) FROM nodes", [], |row| row.get(0))?;

            let nodes_by_tier: HashMap<Tier, i64> = {
                let mut stmt = conn.prepare("SELECT tier, COUNT(*) FROM nodes GROUP BY tier")?;
                let rows = stmt.query_map([], |row| {
                    let tier_int: i32 = row.get(0)?;
                    let count: i64 = row.get(1)?;
                    Ok((int_to_tier(tier_int), count))
                })?;
                let result: HashMap<Tier, i64> = rows.filter_map(|r| r.ok()).collect();
                result
            };

            let nodes_by_type: HashMap<NodeType, i64> = {
                let mut stmt =
                    conn.prepare("SELECT node_type, COUNT(*) FROM nodes GROUP BY node_type")?;
                let rows = stmt.query_map([], |row| {
                    let type_str: String = row.get(0)?;
                    let count: i64 = row.get(1)?;
                    let node_type = match type_str.as_str() {
                        "entity" => NodeType::Entity,
                        "fact" => NodeType::Fact,
                        "experience" => NodeType::Experience,
                        "decision" => NodeType::Decision,
                        "snippet" => NodeType::Snippet,
                        _ => NodeType::Fact,
                    };
                    Ok((node_type, count))
                })?;
                let result: HashMap<NodeType, i64> = rows.filter_map(|r| r.ok()).collect();
                result
            };

            let total_edges: i64 =
                conn.query_row("SELECT COUNT(*) FROM hyperedges", [], |row| row.get(0))?;

            Ok(MemoryStats {
                total_nodes: total_nodes as u64,
                nodes_by_tier,
                nodes_by_type,
                total_edges: total_edges as u64,
            })
        })
    }
}

/// Entry in the evolution log.
#[derive(Debug, Clone)]
pub struct EvolutionEntry {
    pub operation: String,
    pub from_tier: Option<Tier>,
    pub to_tier: Option<Tier>,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

/// Statistics about the memory store.
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_nodes: u64,
    pub nodes_by_tier: HashMap<Tier, i64>,
    pub nodes_by_type: HashMap<NodeType, i64>,
    pub total_edges: u64,
}

fn parse_datetime(s: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn int_to_tier(i: i32) -> Tier {
    match i {
        0 => Tier::Task,
        1 => Tier::Session,
        2 => Tier::LongTerm,
        3 => Tier::Archive,
        _ => Tier::Task,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_node() {
        let store = SqliteMemoryStore::in_memory().unwrap();
        let node = Node::new(NodeType::Fact, "Test fact");

        store.add_node(&node).unwrap();
        let retrieved = store.get_node(&node.id).unwrap().unwrap();

        assert_eq!(retrieved.content, "Test fact");
        assert_eq!(retrieved.node_type, NodeType::Fact);
    }

    #[test]
    fn test_query_nodes_by_type() {
        let store = SqliteMemoryStore::in_memory().unwrap();

        store
            .add_node(&Node::new(NodeType::Fact, "Fact 1"))
            .unwrap();
        store
            .add_node(&Node::new(NodeType::Fact, "Fact 2"))
            .unwrap();
        store
            .add_node(&Node::new(NodeType::Entity, "Entity 1"))
            .unwrap();

        let facts = store
            .query_nodes(&NodeQuery::new().node_types(vec![NodeType::Fact]))
            .unwrap();

        assert_eq!(facts.len(), 2);
    }

    #[test]
    fn test_full_text_search() {
        let store = SqliteMemoryStore::in_memory().unwrap();

        store
            .add_node(&Node::new(
                NodeType::Fact,
                "The authentication system uses JWT",
            ))
            .unwrap();
        store
            .add_node(&Node::new(NodeType::Fact, "Users can login with OAuth"))
            .unwrap();
        store
            .add_node(&Node::new(NodeType::Fact, "Database uses PostgreSQL"))
            .unwrap();

        let results = store.search_content("authentication", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("authentication"));
    }

    #[test]
    fn test_add_and_get_edge() {
        let store = SqliteMemoryStore::in_memory().unwrap();

        let node1 = Node::new(NodeType::Entity, "User");
        let node2 = Node::new(NodeType::Entity, "Session");
        store.add_node(&node1).unwrap();
        store.add_node(&node2).unwrap();

        let edge = HyperEdge::binary(
            EdgeType::Structural,
            node1.id.clone(),
            node2.id.clone(),
            "has",
        );
        store.add_edge(&edge).unwrap();

        let edges = store.get_edges_for_node(&node1.id).unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].label, Some("has".to_string()));
    }

    #[test]
    fn test_promote() {
        let store = SqliteMemoryStore::in_memory().unwrap();

        let node = Node::new(NodeType::Fact, "Test").with_tier(Tier::Task);
        store.add_node(&node).unwrap();

        let promoted = store.promote(&[node.id.clone()], "Test promotion").unwrap();
        assert_eq!(promoted.len(), 1);

        let updated = store.get_node(&node.id).unwrap().unwrap();
        assert_eq!(updated.tier, Tier::Session);
    }

    #[test]
    fn test_evolution_history() {
        let store = SqliteMemoryStore::in_memory().unwrap();

        let node = Node::new(NodeType::Fact, "Test").with_tier(Tier::Task);
        store.add_node(&node).unwrap();
        store
            .promote(&[node.id.clone()], "First promotion")
            .unwrap();
        store
            .promote(&[node.id.clone()], "Second promotion")
            .unwrap();

        let history = store.get_evolution_history(&node.id).unwrap();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_stats() {
        let store = SqliteMemoryStore::in_memory().unwrap();

        store.add_node(&Node::new(NodeType::Fact, "F1")).unwrap();
        store
            .add_node(&Node::new(NodeType::Fact, "F2").with_tier(Tier::Session))
            .unwrap();
        store.add_node(&Node::new(NodeType::Entity, "E1")).unwrap();

        let stats = store.stats().unwrap();
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.nodes_by_type.get(&NodeType::Fact), Some(&2));
    }
}
