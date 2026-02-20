//! Python bindings for memory types.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::memory::{
    EdgeId, EdgeType, HyperEdge, Node, NodeId, NodeQuery, NodeType, Provenance, ProvenanceSource,
    SqliteMemoryStore, Tier,
};

/// Python enum for NodeType.
#[pyclass(name = "NodeType", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyNodeType {
    Entity = 0,
    Fact = 1,
    Experience = 2,
    Decision = 3,
    Snippet = 4,
}

impl From<NodeType> for PyNodeType {
    fn from(nt: NodeType) -> Self {
        match nt {
            NodeType::Entity => PyNodeType::Entity,
            NodeType::Fact => PyNodeType::Fact,
            NodeType::Experience => PyNodeType::Experience,
            NodeType::Decision => PyNodeType::Decision,
            NodeType::Snippet => PyNodeType::Snippet,
        }
    }
}

impl From<PyNodeType> for NodeType {
    fn from(nt: PyNodeType) -> Self {
        match nt {
            PyNodeType::Entity => NodeType::Entity,
            PyNodeType::Fact => NodeType::Fact,
            PyNodeType::Experience => NodeType::Experience,
            PyNodeType::Decision => NodeType::Decision,
            PyNodeType::Snippet => NodeType::Snippet,
        }
    }
}

#[pymethods]
impl PyNodeType {
    fn __repr__(&self) -> &'static str {
        match self {
            PyNodeType::Entity => "NodeType.Entity",
            PyNodeType::Fact => "NodeType.Fact",
            PyNodeType::Experience => "NodeType.Experience",
            PyNodeType::Decision => "NodeType.Decision",
            PyNodeType::Snippet => "NodeType.Snippet",
        }
    }
}

/// Python enum for Tier.
#[pyclass(name = "Tier", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyTier {
    Task = 0,
    Session = 1,
    LongTerm = 2,
    Archive = 3,
}

impl From<Tier> for PyTier {
    fn from(tier: Tier) -> Self {
        match tier {
            Tier::Task => PyTier::Task,
            Tier::Session => PyTier::Session,
            Tier::LongTerm => PyTier::LongTerm,
            Tier::Archive => PyTier::Archive,
        }
    }
}

impl From<PyTier> for Tier {
    fn from(tier: PyTier) -> Self {
        match tier {
            PyTier::Task => Tier::Task,
            PyTier::Session => Tier::Session,
            PyTier::LongTerm => Tier::LongTerm,
            PyTier::Archive => Tier::Archive,
        }
    }
}

#[pymethods]
impl PyTier {
    fn __repr__(&self) -> &'static str {
        match self {
            PyTier::Task => "Tier.Task",
            PyTier::Session => "Tier.Session",
            PyTier::LongTerm => "Tier.LongTerm",
            PyTier::Archive => "Tier.Archive",
        }
    }

    /// Get the next tier (for promotion).
    fn next(&self) -> Option<PyTier> {
        Tier::from(*self).next().map(PyTier::from)
    }

    /// Get the previous tier (for demotion).
    fn previous(&self) -> Option<PyTier> {
        Tier::from(*self).previous().map(PyTier::from)
    }
}

/// Python wrapper for Node.
#[pyclass(name = "Node")]
#[derive(Clone)]
pub struct PyNode {
    pub(crate) inner: Node,
}

#[pymethods]
impl PyNode {
    #[new]
    #[pyo3(signature = (node_type, content, subtype=None, tier=None, confidence=None, metadata=None, provenance_source=None, provenance_ref=None, embedding=None))]
    fn new(
        py: Python<'_>,
        node_type: PyNodeType,
        content: String,
        subtype: Option<String>,
        tier: Option<PyTier>,
        confidence: Option<f64>,
        metadata: Option<Bound<'_, PyDict>>,
        provenance_source: Option<String>,
        provenance_ref: Option<String>,
        embedding: Option<Vec<f32>>,
    ) -> PyResult<Self> {
        let mut node = Node::new(node_type.into(), content);
        if let Some(st) = subtype {
            node = node.with_subtype(st);
        }
        if let Some(t) = tier {
            node = node.with_tier(t.into());
        }
        if let Some(c) = confidence {
            node = node.with_confidence(c);
        }
        if let Some(dict) = metadata {
            for (key, value) in dict.iter() {
                let k: String = key.extract()?;
                let v = pyobj_to_json_value(py, &value)?;
                node = node.with_metadata(k, v);
            }
        }
        if let Some(src) = provenance_source {
            let source_type = parse_provenance_source(&src)?;
            let prov = Provenance {
                source_type,
                source_ref: provenance_ref,
                observed_at: chrono::Utc::now(),
                context: None,
            };
            node = node.with_provenance(prov);
        }
        if let Some(emb) = embedding {
            node = node.with_embedding(emb);
        }
        Ok(Self { inner: node })
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    #[getter]
    fn node_type(&self) -> PyNodeType {
        self.inner.node_type.into()
    }

    #[getter]
    fn subtype(&self) -> Option<String> {
        self.inner.subtype.clone()
    }

    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    #[getter]
    fn tier(&self) -> PyTier {
        self.inner.tier.into()
    }

    #[getter]
    fn confidence(&self) -> f64 {
        self.inner.confidence
    }

    #[getter]
    fn created_at(&self) -> String {
        self.inner.created_at.to_rfc3339()
    }

    #[getter]
    fn updated_at(&self) -> String {
        self.inner.updated_at.to_rfc3339()
    }

    #[getter]
    fn last_accessed(&self) -> String {
        self.inner.last_accessed.to_rfc3339()
    }

    #[getter]
    fn access_count(&self) -> u64 {
        self.inner.access_count
    }

    #[getter]
    fn metadata(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        match &self.inner.metadata {
            Some(map) => {
                let dict = PyDict::new(py);
                for (k, v) in map {
                    dict.set_item(k, json_value_to_pyobj(py, v)?)?;
                }
                Ok(Some(dict.into_any().unbind()))
            }
            None => Ok(None),
        }
    }

    #[getter]
    fn provenance_source(&self) -> Option<String> {
        self.inner
            .provenance
            .as_ref()
            .map(|p| format!("{:?}", p.source_type).to_lowercase())
    }

    #[getter]
    fn provenance_ref(&self) -> Option<String> {
        self.inner
            .provenance
            .as_ref()
            .and_then(|p| p.source_ref.clone())
    }

    #[getter]
    fn embedding(&self) -> Option<Vec<f32>> {
        self.inner.embedding.clone()
    }

    /// Record an access to this node.
    fn record_access(&mut self) {
        self.inner.record_access();
    }

    /// Check if the node has decayed below a threshold.
    fn is_decayed(&self, min_confidence: f64) -> bool {
        self.inner.is_decayed(min_confidence)
    }

    /// Get approximate age in hours.
    fn age_hours(&self) -> i64 {
        self.inner.age_hours()
    }

    fn __repr__(&self) -> String {
        format!(
            "Node(id={}, type={:?}, tier={:?}, content={:?})",
            &self.inner.id.to_string()[..8],
            self.inner.node_type,
            self.inner.tier,
            truncate(&self.inner.content, 30)
        )
    }
}

/// Python wrapper for HyperEdge.
#[pyclass(name = "HyperEdge")]
#[derive(Clone)]
pub struct PyHyperEdge {
    pub(crate) inner: HyperEdge,
}

impl PyHyperEdge {
    /// Parse edge type string to EdgeType enum.
    fn parse_edge_type(edge_type: &str) -> PyResult<EdgeType> {
        match edge_type.to_lowercase().as_str() {
            "semantic" => Ok(EdgeType::Semantic),
            "structural" => Ok(EdgeType::Structural),
            "causal" => Ok(EdgeType::Causal),
            "temporal" => Ok(EdgeType::Temporal),
            "reference" => Ok(EdgeType::Reference),
            "reasoning" => Ok(EdgeType::Reasoning),
            _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid edge type: {}. Valid types: semantic, structural, causal, temporal, reference, reasoning",
                edge_type
            ))),
        }
    }
}

#[pymethods]
impl PyHyperEdge {
    #[new]
    #[pyo3(signature = (edge_type, node_ids=None, label=None, weight=None))]
    fn new(
        edge_type: &str,
        node_ids: Option<Vec<String>>,
        label: Option<String>,
        weight: Option<f64>,
    ) -> PyResult<Self> {
        let et = Self::parse_edge_type(edge_type)?;
        let mut edge = HyperEdge::new(et);

        // Add node members if provided
        if let Some(ids) = node_ids {
            for (i, id_str) in ids.iter().enumerate() {
                let node_id = NodeId::parse(id_str)
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
                edge = edge.with_member(node_id, format!("member_{}", i));
            }
        }

        if let Some(l) = label {
            edge = edge.with_label(l);
        }
        if let Some(w) = weight {
            edge = edge.with_weight(w);
        }
        Ok(Self { inner: edge })
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    #[getter]
    fn edge_type(&self) -> String {
        self.inner.edge_type.to_string()
    }

    #[getter]
    fn label(&self) -> Option<String> {
        self.inner.label.clone()
    }

    #[getter]
    fn weight(&self) -> f64 {
        self.inner.weight
    }

    #[getter]
    fn created_at(&self) -> String {
        self.inner.created_at.to_rfc3339()
    }

    /// Get all node IDs in this edge.
    fn node_ids(&self) -> Vec<String> {
        self.inner
            .node_ids()
            .iter()
            .map(|id| id.to_string())
            .collect()
    }

    /// Check if a node is a member.
    fn contains(&self, node_id: &str) -> PyResult<bool> {
        let id = NodeId::parse(node_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(self.inner.contains(&id))
    }

    /// Add a member node with a role. Returns a new HyperEdge with the member added.
    fn with_member(&self, node_id: &str, role: &str) -> PyResult<Self> {
        let id = NodeId::parse(node_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(Self {
            inner: self.inner.clone().with_member(id, role),
        })
    }

    /// Create a binary edge (subject-object relationship).
    #[staticmethod]
    fn binary(edge_type: &str, subject_id: &str, object_id: &str, label: &str) -> PyResult<Self> {
        let et = Self::parse_edge_type(edge_type)?;
        let subject = NodeId::parse(subject_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let object = NodeId::parse(object_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(Self {
            inner: HyperEdge::binary(et, subject, object, label),
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "HyperEdge(id={}, type={}, members={})",
            &self.inner.id.to_string()[..8],
            self.inner.edge_type,
            self.inner.members.len()
        )
    }
}

/// Python wrapper for SqliteMemoryStore.
#[pyclass(name = "MemoryStore")]
pub struct PyMemoryStore {
    inner: SqliteMemoryStore,
}

#[pymethods]
impl PyMemoryStore {
    /// Create an in-memory store.
    #[staticmethod]
    fn in_memory() -> PyResult<Self> {
        let store = SqliteMemoryStore::in_memory()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(Self { inner: store })
    }

    /// Create or open a store at a path.
    #[staticmethod]
    fn open(path: &str) -> PyResult<Self> {
        let store = SqliteMemoryStore::open(PathBuf::from(path))
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(Self { inner: store })
    }

    /// Add a node to the store. Returns the node's ID.
    fn add_node(&self, node: &PyNode) -> PyResult<String> {
        let id = node.inner.id.to_string();
        self.inner
            .add_node(&node.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(id)
    }

    /// Get a node by ID.
    fn get_node(&self, node_id: &str) -> PyResult<Option<PyNode>> {
        let id = NodeId::parse(node_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let node = self
            .inner
            .get_node(&id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(node.map(|n| PyNode { inner: n }))
    }

    /// Query nodes by type.
    fn query_by_type(&self, node_type: PyNodeType, limit: usize) -> PyResult<Vec<PyNode>> {
        let query = NodeQuery::new()
            .node_types(vec![node_type.into()])
            .limit(limit);
        let nodes = self
            .inner
            .query_nodes(&query)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(nodes.into_iter().map(|n| PyNode { inner: n }).collect())
    }

    /// Query nodes by tier.
    fn query_by_tier(&self, tier: PyTier, limit: usize) -> PyResult<Vec<PyNode>> {
        let query = NodeQuery::new().tiers(vec![tier.into()]).limit(limit);
        let nodes = self
            .inner
            .query_nodes(&query)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(nodes.into_iter().map(|n| PyNode { inner: n }).collect())
    }

    /// Search nodes by content.
    fn search_content(&self, query: &str, limit: usize) -> PyResult<Vec<PyNode>> {
        let nodes = self
            .inner
            .search_content(query, limit)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(nodes.into_iter().map(|n| PyNode { inner: n }).collect())
    }

    /// Update a node.
    fn update_node(&self, node: &PyNode) -> PyResult<()> {
        self.inner
            .update_node(&node.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Delete a node.
    fn delete_node(&self, node_id: &str) -> PyResult<bool> {
        let id = NodeId::parse(node_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        self.inner
            .delete_node(&id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Promote nodes to the next tier. Returns IDs of promoted nodes.
    fn promote(&self, node_ids: Vec<String>, reason: &str) -> PyResult<Vec<String>> {
        let ids: Result<Vec<NodeId>, _> = node_ids.iter().map(|s| NodeId::parse(s)).collect();
        let ids =
            ids.map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let promoted = self
            .inner
            .promote(&ids, reason)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(promoted.into_iter().map(|id| id.to_string()).collect())
    }

    /// Decay node confidence values.
    fn decay(&self, factor: f64, min_confidence: f64) -> PyResult<Vec<String>> {
        let ids = self
            .inner
            .decay(factor, min_confidence)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(ids.into_iter().map(|id| id.to_string()).collect())
    }

    /// Get store statistics.
    fn stats(&self) -> PyResult<PyMemoryStats> {
        let stats = self
            .inner
            .stats()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(PyMemoryStats { inner: stats })
    }

    /// Add an edge to the store. Returns the edge's ID.
    fn add_edge(&self, edge: &PyHyperEdge) -> PyResult<String> {
        let id = edge.inner.id.to_string();
        self.inner
            .add_edge(&edge.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(id)
    }

    /// Get all edges for a node.
    fn get_edges_for_node(&self, node_id: &str) -> PyResult<Vec<PyHyperEdge>> {
        let id = NodeId::parse(node_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        let edges = self
            .inner
            .get_edges_for_node(&id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(edges
            .into_iter()
            .map(|e| PyHyperEdge { inner: e })
            .collect())
    }

    /// Delete an edge by ID.
    fn delete_edge(&self, edge_id: &str) -> PyResult<bool> {
        let id = EdgeId::parse(edge_id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        self.inner
            .delete_edge(&id)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        "MemoryStore()".to_string()
    }
}

/// Python wrapper for MemoryStats.
#[pyclass(name = "MemoryStats")]
pub struct PyMemoryStats {
    inner: crate::memory::MemoryStats,
}

#[pymethods]
impl PyMemoryStats {
    #[getter]
    fn total_nodes(&self) -> u64 {
        self.inner.total_nodes
    }

    #[getter]
    fn total_edges(&self) -> u64 {
        self.inner.total_edges
    }

    #[getter]
    fn nodes_by_tier(&self) -> std::collections::HashMap<String, i64> {
        self.inner
            .nodes_by_tier
            .iter()
            .map(|(t, c)| (t.to_string(), *c))
            .collect()
    }

    #[getter]
    fn nodes_by_type(&self) -> std::collections::HashMap<String, i64> {
        self.inner
            .nodes_by_type
            .iter()
            .map(|(t, c)| (t.to_string(), *c))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "MemoryStats(nodes={}, edges={})",
            self.inner.total_nodes, self.inner.total_edges
        )
    }
}

/// Truncate a string for display.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// Convert a Python object to serde_json::Value.
fn pyobj_to_json_value(_py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    if obj.is_none() {
        Ok(Value::Null)
    } else if let Ok(b) = obj.extract::<bool>() {
        Ok(Value::Bool(b))
    } else if let Ok(i) = obj.extract::<i64>() {
        Ok(Value::Number(i.into()))
    } else if let Ok(f) = obj.extract::<f64>() {
        Ok(serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null))
    } else if let Ok(s) = obj.extract::<String>() {
        Ok(Value::String(s))
    } else if let Ok(list) = obj.downcast::<pyo3::types::PyList>() {
        let items: PyResult<Vec<Value>> = list
            .iter()
            .map(|item| pyobj_to_json_value(_py, &item))
            .collect();
        Ok(Value::Array(items?))
    } else if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            map.insert(key, pyobj_to_json_value(_py, &v)?);
        }
        Ok(Value::Object(map))
    } else {
        Ok(Value::String(obj.str()?.to_string()))
    }
}

/// Convert a serde_json::Value to a Python object.
fn json_value_to_pyobj(py: Python<'_>, val: &Value) -> PyResult<PyObject> {
    match val {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok((*b).into_pyobject(py)?.to_owned().into_any().unbind()),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any().unbind())
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_pyobject(py)?.into_any().unbind())
            } else {
                Ok(py.None())
            }
        }
        Value::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        Value::Array(arr) => {
            let items: PyResult<Vec<PyObject>> =
                arr.iter().map(|v| json_value_to_pyobj(py, v)).collect();
            Ok(pyo3::types::PyList::new(py, items?)?.into_any().unbind())
        }
        Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, json_value_to_pyobj(py, v)?)?;
            }
            Ok(dict.into_any().unbind())
        }
    }
}

/// Parse a provenance source string.
fn parse_provenance_source(s: &str) -> PyResult<ProvenanceSource> {
    match s.to_lowercase().as_str() {
        "user_message" | "user" => Ok(ProvenanceSource::UserMessage),
        "assistant_response" | "assistant" => Ok(ProvenanceSource::AssistantResponse),
        "tool_output" | "tool" => Ok(ProvenanceSource::ToolOutput),
        "file_content" | "file" => Ok(ProvenanceSource::FileContent),
        "consolidation" => Ok(ProvenanceSource::Consolidation),
        "inference" => Ok(ProvenanceSource::Inference),
        "import" => Ok(ProvenanceSource::Import),
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "Invalid provenance source: {}. Valid: user_message, assistant_response, tool_output, file_content, consolidation, inference, import",
            s
        ))),
    }
}
