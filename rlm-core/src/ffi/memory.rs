//! FFI bindings for memory types.

use std::os::raw::c_char;
use std::path::PathBuf;

use super::error::{cstr_to_str, ffi_try, set_last_error, str_to_cstring};
use super::types::{RlmHyperEdge, RlmMemoryStore, RlmNode, RlmNodeType, RlmTier};
use crate::memory::{EdgeType, HyperEdge, Node, NodeId, NodeQuery, NodeType, SqliteMemoryStore, Tier};

// ============================================================================
// MemoryStore
// ============================================================================

/// Create an in-memory store.
///
/// # Safety
/// The returned pointer must be freed with `rlm_memory_store_free()`.
#[no_mangle]
pub extern "C" fn rlm_memory_store_in_memory() -> *mut RlmMemoryStore {
    match SqliteMemoryStore::in_memory() {
        Ok(store) => Box::into_raw(Box::new(RlmMemoryStore(store))),
        Err(e) => {
            set_last_error(&e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Open or create a memory store at a path.
///
/// # Safety
/// - `path` must be a valid null-terminated string.
/// - The returned pointer must be freed with `rlm_memory_store_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_open(path: *const c_char) -> *mut RlmMemoryStore {
    let path = ffi_try!(cstr_to_str(path));
    let store = ffi_try!(SqliteMemoryStore::open(PathBuf::from(path)));
    Box::into_raw(Box::new(RlmMemoryStore(store)))
}

/// Free a memory store.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_free(store: *mut RlmMemoryStore) {
    if !store.is_null() {
        drop(Box::from_raw(store));
    }
}

/// Add a node to the store.
///
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_add_node(
    store: *const RlmMemoryStore,
    node: *const RlmNode,
) -> i32 {
    if store.is_null() || node.is_null() {
        set_last_error("null pointer");
        return -1;
    }
    ffi_try!((*store).0.add_node(&(*node).0), -1);
    0
}

/// Get a node by ID.
///
/// # Safety
/// - `store` must be a valid pointer.
/// - `node_id` must be a valid null-terminated UUID string.
/// - Returns NULL if not found. Check `rlm_has_error()` to distinguish from error.
/// - The returned pointer must be freed with `rlm_node_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_get_node(
    store: *const RlmMemoryStore,
    node_id: *const c_char,
) -> *mut RlmNode {
    if store.is_null() {
        set_last_error("null store pointer");
        return std::ptr::null_mut();
    }
    let id_str = ffi_try!(cstr_to_str(node_id));
    let id = ffi_try!(NodeId::parse(id_str));
    let node = ffi_try!((*store).0.get_node(&id));
    match node {
        Some(n) => Box::into_raw(Box::new(RlmNode(n))),
        None => std::ptr::null_mut(),
    }
}

/// Update a node in the store.
///
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_update_node(
    store: *const RlmMemoryStore,
    node: *const RlmNode,
) -> i32 {
    if store.is_null() || node.is_null() {
        set_last_error("null pointer");
        return -1;
    }
    ffi_try!((*store).0.update_node(&(*node).0), -1);
    0
}

/// Delete a node from the store.
///
/// Returns 1 if deleted, 0 if not found, -1 on error.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_delete_node(
    store: *const RlmMemoryStore,
    node_id: *const c_char,
) -> i32 {
    if store.is_null() {
        set_last_error("null store pointer");
        return -1;
    }
    let id_str = ffi_try!(cstr_to_str(node_id), -1);
    let id = ffi_try!(NodeId::parse(id_str), -1);
    let deleted = ffi_try!((*store).0.delete_node(&id), -1);
    if deleted { 1 } else { 0 }
}

/// Query nodes by type. Returns a JSON array of node IDs.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_query_by_type(
    store: *const RlmMemoryStore,
    node_type: RlmNodeType,
    limit: i64,
) -> *mut c_char {
    if store.is_null() {
        set_last_error("null store pointer");
        return std::ptr::null_mut();
    }
    let query = NodeQuery::new()
        .node_types(vec![NodeType::from(node_type)])
        .limit(limit as usize);
    let nodes = ffi_try!((*store).0.query_nodes(&query));
    let ids: Vec<String> = nodes.iter().map(|n| n.id.to_string()).collect();
    let json = ffi_try!(serde_json::to_string(&ids));
    str_to_cstring(&json)
}

/// Query nodes by tier. Returns a JSON array of node IDs.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_query_by_tier(
    store: *const RlmMemoryStore,
    tier: RlmTier,
    limit: i64,
) -> *mut c_char {
    if store.is_null() {
        set_last_error("null store pointer");
        return std::ptr::null_mut();
    }
    let query = NodeQuery::new()
        .tiers(vec![Tier::from(tier)])
        .limit(limit as usize);
    let nodes = ffi_try!((*store).0.query_nodes(&query));
    let ids: Vec<String> = nodes.iter().map(|n| n.id.to_string()).collect();
    let json = ffi_try!(serde_json::to_string(&ids));
    str_to_cstring(&json)
}

/// Full-text search on content. Returns a JSON array of node IDs.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_search_content(
    store: *const RlmMemoryStore,
    query: *const c_char,
    limit: i64,
) -> *mut c_char {
    if store.is_null() {
        set_last_error("null store pointer");
        return std::ptr::null_mut();
    }
    let query_str = ffi_try!(cstr_to_str(query));
    let nodes = ffi_try!((*store).0.search_content(query_str, limit as usize));
    let ids: Vec<String> = nodes.iter().map(|n| n.id.to_string()).collect();
    let json = ffi_try!(serde_json::to_string(&ids));
    str_to_cstring(&json)
}

/// Promote nodes to the next tier. Returns a JSON array of promoted node IDs.
///
/// # Safety
/// - `node_ids_json` must be a JSON array of UUID strings.
/// - `reason` must be a valid null-terminated string.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_promote(
    store: *const RlmMemoryStore,
    node_ids_json: *const c_char,
    reason: *const c_char,
) -> *mut c_char {
    if store.is_null() {
        set_last_error("null store pointer");
        return std::ptr::null_mut();
    }
    let ids_json = ffi_try!(cstr_to_str(node_ids_json));
    let reason = ffi_try!(cstr_to_str(reason));
    let id_strings: Vec<String> = ffi_try!(serde_json::from_str(ids_json));
    let ids: Result<Vec<NodeId>, _> = id_strings.iter().map(|s| NodeId::parse(s)).collect();
    let ids = ffi_try!(ids);
    let promoted = ffi_try!((*store).0.promote(&ids, reason));
    let promoted_strings: Vec<String> = promoted.iter().map(|id| id.to_string()).collect();
    let json = ffi_try!(serde_json::to_string(&promoted_strings));
    str_to_cstring(&json)
}

/// Apply decay to nodes. Returns a JSON array of decayed node IDs.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_decay(
    store: *const RlmMemoryStore,
    factor: f64,
    min_confidence: f64,
) -> *mut c_char {
    if store.is_null() {
        set_last_error("null store pointer");
        return std::ptr::null_mut();
    }
    let decayed = ffi_try!((*store).0.decay(factor, min_confidence));
    let decayed_strings: Vec<String> = decayed.iter().map(|id| id.to_string()).collect();
    let json = ffi_try!(serde_json::to_string(&decayed_strings));
    str_to_cstring(&json)
}

/// Get store statistics as JSON.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_stats(store: *const RlmMemoryStore) -> *mut c_char {
    if store.is_null() {
        set_last_error("null store pointer");
        return std::ptr::null_mut();
    }
    let stats = ffi_try!((*store).0.stats());
    // Convert to a simple JSON object
    let json = format!(
        r#"{{"total_nodes":{},"total_edges":{}}}"#,
        stats.total_nodes, stats.total_edges
    );
    str_to_cstring(&json)
}

// ============================================================================
// Node
// ============================================================================

/// Create a new node.
///
/// # Safety
/// - `content` must be a valid null-terminated string.
/// - The returned pointer must be freed with `rlm_node_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_new(
    node_type: RlmNodeType,
    content: *const c_char,
) -> *mut RlmNode {
    let content = ffi_try!(cstr_to_str(content));
    let node = Node::new(NodeType::from(node_type), content);
    Box::into_raw(Box::new(RlmNode(node)))
}

/// Create a node with tier and confidence.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_new_full(
    node_type: RlmNodeType,
    content: *const c_char,
    tier: RlmTier,
    confidence: f64,
) -> *mut RlmNode {
    let content = ffi_try!(cstr_to_str(content));
    let node = Node::new(NodeType::from(node_type), content)
        .with_tier(Tier::from(tier))
        .with_confidence(confidence);
    Box::into_raw(Box::new(RlmNode(node)))
}

/// Free a node.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_free(node: *mut RlmNode) {
    if !node.is_null() {
        drop(Box::from_raw(node));
    }
}

/// Get the node ID.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_id(node: *const RlmNode) -> *mut c_char {
    if node.is_null() {
        set_last_error("null node pointer");
        return std::ptr::null_mut();
    }
    str_to_cstring(&(*node).0.id.to_string())
}

/// Get the node type.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_type(node: *const RlmNode) -> RlmNodeType {
    if node.is_null() {
        return RlmNodeType::Fact;
    }
    RlmNodeType::from((*node).0.node_type)
}

/// Get the node content.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_content(node: *const RlmNode) -> *mut c_char {
    if node.is_null() {
        set_last_error("null node pointer");
        return std::ptr::null_mut();
    }
    str_to_cstring(&(*node).0.content)
}

/// Get the node tier.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_tier(node: *const RlmNode) -> RlmTier {
    if node.is_null() {
        return RlmTier::Task;
    }
    RlmTier::from((*node).0.tier)
}

/// Get the node confidence.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_confidence(node: *const RlmNode) -> f64 {
    if node.is_null() {
        return 0.0;
    }
    (*node).0.confidence
}

/// Get the node subtype.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
/// Returns NULL if no subtype is set.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_subtype(node: *const RlmNode) -> *mut c_char {
    if node.is_null() {
        return std::ptr::null_mut();
    }
    match &(*node).0.subtype {
        Some(s) => str_to_cstring(s),
        None => std::ptr::null_mut(),
    }
}

/// Set the node subtype.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_set_subtype(
    node: *mut RlmNode,
    subtype: *const c_char,
) -> i32 {
    if node.is_null() {
        set_last_error("null node pointer");
        return -1;
    }
    if subtype.is_null() {
        (*node).0.subtype = None;
    } else {
        let subtype = ffi_try!(cstr_to_str(subtype), -1);
        (*node).0.subtype = Some(subtype.to_string());
    }
    0
}

/// Set the node tier.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_set_tier(node: *mut RlmNode, tier: RlmTier) -> i32 {
    if node.is_null() {
        set_last_error("null node pointer");
        return -1;
    }
    (*node).0.tier = Tier::from(tier);
    0
}

/// Set the node confidence.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_set_confidence(node: *mut RlmNode, confidence: f64) -> i32 {
    if node.is_null() {
        set_last_error("null node pointer");
        return -1;
    }
    (*node).0.confidence = confidence.clamp(0.0, 1.0);
    0
}

/// Record an access to the node.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_record_access(node: *mut RlmNode) -> i32 {
    if node.is_null() {
        set_last_error("null node pointer");
        return -1;
    }
    (*node).0.record_access();
    0
}

/// Get the access count.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_access_count(node: *const RlmNode) -> u64 {
    if node.is_null() {
        return 0;
    }
    (*node).0.access_count
}

/// Check if the node has decayed below a threshold.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_is_decayed(node: *const RlmNode, min_confidence: f64) -> i32 {
    if node.is_null() {
        return 0;
    }
    if (*node).0.is_decayed(min_confidence) { 1 } else { 0 }
}

/// Get the node age in hours.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_age_hours(node: *const RlmNode) -> i64 {
    if node.is_null() {
        return 0;
    }
    (*node).0.age_hours()
}

/// Serialize node to JSON.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_to_json(node: *const RlmNode) -> *mut c_char {
    if node.is_null() {
        set_last_error("null node pointer");
        return std::ptr::null_mut();
    }
    let json = ffi_try!(serde_json::to_string(&(*node).0));
    str_to_cstring(&json)
}

/// Deserialize node from JSON.
///
/// # Safety
/// - `json` must be a valid null-terminated string.
/// - The returned pointer must be freed with `rlm_node_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_node_from_json(json: *const c_char) -> *mut RlmNode {
    let json = ffi_try!(cstr_to_str(json));
    let node: Node = ffi_try!(serde_json::from_str(json));
    Box::into_raw(Box::new(RlmNode(node)))
}

// ============================================================================
// HyperEdge
// ============================================================================

/// Create a new hyperedge.
///
/// # Safety
/// - `edge_type` must be one of: "semantic", "structural", "causal", "temporal", "reference", "reasoning"
/// - The returned pointer must be freed with `rlm_hyperedge_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_hyperedge_new(edge_type: *const c_char) -> *mut RlmHyperEdge {
    let type_str = ffi_try!(cstr_to_str(edge_type));
    let et = match type_str.to_lowercase().as_str() {
        "semantic" => EdgeType::Semantic,
        "structural" => EdgeType::Structural,
        "causal" => EdgeType::Causal,
        "temporal" => EdgeType::Temporal,
        "reference" => EdgeType::Reference,
        "reasoning" => EdgeType::Reasoning,
        _ => {
            set_last_error(&format!("invalid edge type: {}", type_str));
            return std::ptr::null_mut();
        }
    };
    Box::into_raw(Box::new(RlmHyperEdge(HyperEdge::new(et))))
}

/// Create a binary edge between two nodes.
///
/// # Safety
/// - `subject_id` and `object_id` must be valid UUID strings.
/// - `label` must be a valid null-terminated string.
/// - The returned pointer must be freed with `rlm_hyperedge_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_hyperedge_binary(
    edge_type: *const c_char,
    subject_id: *const c_char,
    object_id: *const c_char,
    label: *const c_char,
) -> *mut RlmHyperEdge {
    let type_str = ffi_try!(cstr_to_str(edge_type));
    let et = match type_str.to_lowercase().as_str() {
        "semantic" => EdgeType::Semantic,
        "structural" => EdgeType::Structural,
        "causal" => EdgeType::Causal,
        "temporal" => EdgeType::Temporal,
        "reference" => EdgeType::Reference,
        "reasoning" => EdgeType::Reasoning,
        _ => {
            set_last_error(&format!("invalid edge type: {}", type_str));
            return std::ptr::null_mut();
        }
    };
    let subject = ffi_try!(NodeId::parse(ffi_try!(cstr_to_str(subject_id))));
    let object = ffi_try!(NodeId::parse(ffi_try!(cstr_to_str(object_id))));
    let label = ffi_try!(cstr_to_str(label));
    let edge = HyperEdge::binary(et, subject, object, label);
    Box::into_raw(Box::new(RlmHyperEdge(edge)))
}

/// Free a hyperedge.
#[no_mangle]
pub unsafe extern "C" fn rlm_hyperedge_free(edge: *mut RlmHyperEdge) {
    if !edge.is_null() {
        drop(Box::from_raw(edge));
    }
}

/// Get the edge ID.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_hyperedge_id(edge: *const RlmHyperEdge) -> *mut c_char {
    if edge.is_null() {
        set_last_error("null edge pointer");
        return std::ptr::null_mut();
    }
    str_to_cstring(&(*edge).0.id.to_string())
}

/// Get the edge type as string.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_hyperedge_type(edge: *const RlmHyperEdge) -> *mut c_char {
    if edge.is_null() {
        set_last_error("null edge pointer");
        return std::ptr::null_mut();
    }
    str_to_cstring(&(*edge).0.edge_type.to_string())
}

/// Get the edge label.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
/// Returns NULL if no label is set.
#[no_mangle]
pub unsafe extern "C" fn rlm_hyperedge_label(edge: *const RlmHyperEdge) -> *mut c_char {
    if edge.is_null() {
        return std::ptr::null_mut();
    }
    match &(*edge).0.label {
        Some(l) => str_to_cstring(l),
        None => std::ptr::null_mut(),
    }
}

/// Get the edge weight.
#[no_mangle]
pub unsafe extern "C" fn rlm_hyperedge_weight(edge: *const RlmHyperEdge) -> f64 {
    if edge.is_null() {
        return 0.0;
    }
    (*edge).0.weight
}

/// Get member node IDs as JSON array.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_hyperedge_node_ids(edge: *const RlmHyperEdge) -> *mut c_char {
    if edge.is_null() {
        set_last_error("null edge pointer");
        return std::ptr::null_mut();
    }
    let ids: Vec<String> = (*edge).0.node_ids().iter().map(|id| id.to_string()).collect();
    let json = ffi_try!(serde_json::to_string(&ids));
    str_to_cstring(&json)
}

/// Check if a node is a member of the edge.
#[no_mangle]
pub unsafe extern "C" fn rlm_hyperedge_contains(
    edge: *const RlmHyperEdge,
    node_id: *const c_char,
) -> i32 {
    if edge.is_null() {
        return 0;
    }
    let id_str = match cstr_to_str(node_id) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    let id = match NodeId::parse(id_str) {
        Ok(id) => id,
        Err(_) => return 0,
    };
    if (*edge).0.contains(&id) { 1 } else { 0 }
}

/// Add the edge to a memory store.
///
/// Returns 0 on success, -1 on failure.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_add_edge(
    store: *const RlmMemoryStore,
    edge: *const RlmHyperEdge,
) -> i32 {
    if store.is_null() || edge.is_null() {
        set_last_error("null pointer");
        return -1;
    }
    ffi_try!((*store).0.add_edge(&(*edge).0), -1);
    0
}

/// Get edges connected to a node. Returns a JSON array of edge data.
///
/// # Safety
/// The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_memory_store_get_edges_for_node(
    store: *const RlmMemoryStore,
    node_id: *const c_char,
) -> *mut c_char {
    if store.is_null() {
        set_last_error("null store pointer");
        return std::ptr::null_mut();
    }
    let id_str = ffi_try!(cstr_to_str(node_id));
    let id = ffi_try!(NodeId::parse(id_str));
    let edges = ffi_try!((*store).0.get_edges_for_node(&id));
    let edge_data: Vec<serde_json::Value> = edges
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id.to_string(),
                "edge_type": e.edge_type.to_string(),
                "label": e.label,
                "weight": e.weight,
                "members": e.members.iter().map(|m| {
                    serde_json::json!({
                        "node_id": m.node_id.to_string(),
                        "role": m.role,
                        "position": m.position
                    })
                }).collect::<Vec<_>>()
            })
        })
        .collect();
    let json = ffi_try!(serde_json::to_string(&edge_data));
    str_to_cstring(&json)
}
