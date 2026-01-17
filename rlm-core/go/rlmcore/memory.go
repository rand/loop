package rlmcore

/*
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -lrlm_core
#cgo darwin LDFLAGS: -framework Security -framework CoreFoundation

#include <stdlib.h>
#include "../../include/rlm_core.h"
*/
import "C"

import (
	"encoding/json"
	"runtime"
	"unsafe"
)

// MemoryStore provides persistent hypergraph memory storage.
type MemoryStore struct {
	ptr *C.RlmMemoryStore
}

// NewMemoryStoreInMemory creates an in-memory store (useful for testing).
func NewMemoryStoreInMemory() (*MemoryStore, error) {
	ptr := C.rlm_memory_store_in_memory()
	if ptr == nil {
		return nil, lastError()
	}
	store := &MemoryStore{ptr: ptr}
	runtime.SetFinalizer(store, (*MemoryStore).Free)
	return store, nil
}

// OpenMemoryStore opens or creates a memory store at the given path.
func OpenMemoryStore(path string) (*MemoryStore, error) {
	cpath := cString(path)
	defer C.free(unsafe.Pointer(cpath))
	ptr := C.rlm_memory_store_open(cpath)
	if ptr == nil {
		return nil, lastError()
	}
	store := &MemoryStore{ptr: ptr}
	runtime.SetFinalizer(store, (*MemoryStore).Free)
	return store, nil
}

// Free releases the memory store resources.
func (s *MemoryStore) Free() {
	if s.ptr != nil {
		C.rlm_memory_store_free(s.ptr)
		s.ptr = nil
	}
}

// AddNode adds a node to the store.
func (s *MemoryStore) AddNode(node *Node) error {
	if C.rlm_memory_store_add_node(s.ptr, node.ptr) != 0 {
		return lastError()
	}
	return nil
}

// GetNode retrieves a node by ID.
func (s *MemoryStore) GetNode(nodeID string) (*Node, error) {
	cid := cString(nodeID)
	defer C.free(unsafe.Pointer(cid))
	ptr := C.rlm_memory_store_get_node(s.ptr, cid)
	if ptr == nil {
		if err := lastError(); err != nil {
			return nil, err
		}
		return nil, nil // Not found
	}
	node := &Node{ptr: ptr}
	runtime.SetFinalizer(node, (*Node).Free)
	return node, nil
}

// UpdateNode updates a node in the store.
func (s *MemoryStore) UpdateNode(node *Node) error {
	if C.rlm_memory_store_update_node(s.ptr, node.ptr) != 0 {
		return lastError()
	}
	return nil
}

// DeleteNode deletes a node from the store.
// Returns true if deleted, false if not found.
func (s *MemoryStore) DeleteNode(nodeID string) (bool, error) {
	cid := cString(nodeID)
	defer C.free(unsafe.Pointer(cid))
	result := C.rlm_memory_store_delete_node(s.ptr, cid)
	if result < 0 {
		return false, lastError()
	}
	return result == 1, nil
}

// QueryByType queries nodes by type.
func (s *MemoryStore) QueryByType(nodeType NodeType, limit int64) ([]string, error) {
	cstr := C.rlm_memory_store_query_by_type(s.ptr, C.RlmNodeType(nodeType), C.int64_t(limit))
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)
	var ids []string
	if err := json.Unmarshal([]byte(jsonStr), &ids); err != nil {
		return nil, err
	}
	return ids, nil
}

// QueryByTier queries nodes by tier.
func (s *MemoryStore) QueryByTier(tier Tier, limit int64) ([]string, error) {
	cstr := C.rlm_memory_store_query_by_tier(s.ptr, C.RlmTier(tier), C.int64_t(limit))
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)
	var ids []string
	if err := json.Unmarshal([]byte(jsonStr), &ids); err != nil {
		return nil, err
	}
	return ids, nil
}

// SearchContent performs full-text search on node content.
func (s *MemoryStore) SearchContent(query string, limit int64) ([]string, error) {
	cquery := cString(query)
	defer C.free(unsafe.Pointer(cquery))
	cstr := C.rlm_memory_store_search_content(s.ptr, cquery, C.int64_t(limit))
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)
	var ids []string
	if err := json.Unmarshal([]byte(jsonStr), &ids); err != nil {
		return nil, err
	}
	return ids, nil
}

// Promote promotes nodes to the next tier.
func (s *MemoryStore) Promote(nodeIDs []string, reason string) ([]string, error) {
	idsJSON, err := json.Marshal(nodeIDs)
	if err != nil {
		return nil, err
	}
	cids := cString(string(idsJSON))
	defer C.free(unsafe.Pointer(cids))
	creason := cString(reason)
	defer C.free(unsafe.Pointer(creason))

	cstr := C.rlm_memory_store_promote(s.ptr, cids, creason)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)
	var promoted []string
	if err := json.Unmarshal([]byte(jsonStr), &promoted); err != nil {
		return nil, err
	}
	return promoted, nil
}

// Decay applies confidence decay to nodes.
func (s *MemoryStore) Decay(factor, minConfidence float64) ([]string, error) {
	cstr := C.rlm_memory_store_decay(s.ptr, C.double(factor), C.double(minConfidence))
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)
	var decayed []string
	if err := json.Unmarshal([]byte(jsonStr), &decayed); err != nil {
		return nil, err
	}
	return decayed, nil
}

// Stats returns statistics about the memory store.
func (s *MemoryStore) Stats() (*MemoryStats, error) {
	cstr := C.rlm_memory_store_stats(s.ptr)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)
	var stats MemoryStats
	if err := json.Unmarshal([]byte(jsonStr), &stats); err != nil {
		return nil, err
	}
	return &stats, nil
}

// AddEdge adds a hyperedge to the store.
func (s *MemoryStore) AddEdge(edge *HyperEdge) error {
	if C.rlm_memory_store_add_edge(s.ptr, edge.ptr) != 0 {
		return lastError()
	}
	return nil
}

// GetEdgesForNode retrieves all edges connected to a node.
func (s *MemoryStore) GetEdgesForNode(nodeID string) ([]EdgeData, error) {
	cid := cString(nodeID)
	defer C.free(unsafe.Pointer(cid))
	cstr := C.rlm_memory_store_get_edges_for_node(s.ptr, cid)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)
	var edges []EdgeData
	if err := json.Unmarshal([]byte(jsonStr), &edges); err != nil {
		return nil, err
	}
	return edges, nil
}

// Node represents a memory node in the hypergraph.
type Node struct {
	ptr *C.RlmNode
}

// NewNode creates a new node with the given type and content.
func NewNode(nodeType NodeType, content string) *Node {
	ccontent := cString(content)
	defer C.free(unsafe.Pointer(ccontent))
	node := &Node{ptr: C.rlm_node_new(C.RlmNodeType(nodeType), ccontent)}
	runtime.SetFinalizer(node, (*Node).Free)
	return node
}

// NewNodeFull creates a node with type, content, tier, and confidence.
func NewNodeFull(nodeType NodeType, content string, tier Tier, confidence float64) *Node {
	ccontent := cString(content)
	defer C.free(unsafe.Pointer(ccontent))
	node := &Node{ptr: C.rlm_node_new_full(C.RlmNodeType(nodeType), ccontent, C.RlmTier(tier), C.double(confidence))}
	runtime.SetFinalizer(node, (*Node).Free)
	return node
}

// Free releases the node resources.
func (n *Node) Free() {
	if n.ptr != nil {
		C.rlm_node_free(n.ptr)
		n.ptr = nil
	}
}

// ID returns the node's unique identifier.
func (n *Node) ID() string {
	return goString(C.rlm_node_id(n.ptr))
}

// Type returns the node type.
func (n *Node) Type() NodeType {
	return NodeType(C.rlm_node_type(n.ptr))
}

// Content returns the node content.
func (n *Node) Content() string {
	return goString(C.rlm_node_content(n.ptr))
}

// Tier returns the node's memory tier.
func (n *Node) Tier() Tier {
	return Tier(C.rlm_node_tier(n.ptr))
}

// Confidence returns the node's confidence score.
func (n *Node) Confidence() float64 {
	return float64(C.rlm_node_confidence(n.ptr))
}

// Subtype returns the node's subtype, or empty string if not set.
func (n *Node) Subtype() string {
	cstr := C.rlm_node_subtype(n.ptr)
	if cstr == nil {
		return ""
	}
	return goString(cstr)
}

// SetSubtype sets the node's subtype.
func (n *Node) SetSubtype(subtype string) error {
	var csubtype *C.char
	if subtype != "" {
		csubtype = cString(subtype)
		defer C.free(unsafe.Pointer(csubtype))
	}
	if C.rlm_node_set_subtype(n.ptr, csubtype) != 0 {
		return lastError()
	}
	return nil
}

// SetTier sets the node's memory tier.
func (n *Node) SetTier(tier Tier) error {
	if C.rlm_node_set_tier(n.ptr, C.RlmTier(tier)) != 0 {
		return lastError()
	}
	return nil
}

// SetConfidence sets the node's confidence score.
func (n *Node) SetConfidence(confidence float64) error {
	if C.rlm_node_set_confidence(n.ptr, C.double(confidence)) != 0 {
		return lastError()
	}
	return nil
}

// RecordAccess records an access to the node.
func (n *Node) RecordAccess() error {
	if C.rlm_node_record_access(n.ptr) != 0 {
		return lastError()
	}
	return nil
}

// AccessCount returns the number of times the node has been accessed.
func (n *Node) AccessCount() uint64 {
	return uint64(C.rlm_node_access_count(n.ptr))
}

// IsDecayed returns true if the node's confidence is below the threshold.
func (n *Node) IsDecayed(minConfidence float64) bool {
	return C.rlm_node_is_decayed(n.ptr, C.double(minConfidence)) != 0
}

// AgeHours returns the node's age in hours.
func (n *Node) AgeHours() int64 {
	return int64(C.rlm_node_age_hours(n.ptr))
}

// ToJSON serializes the node to JSON.
func (n *Node) ToJSON() (string, error) {
	cstr := C.rlm_node_to_json(n.ptr)
	if cstr == nil {
		return "", lastError()
	}
	return goString(cstr), nil
}

// NodeFromJSON deserializes a node from JSON.
func NodeFromJSON(jsonStr string) (*Node, error) {
	cs := cString(jsonStr)
	defer C.free(unsafe.Pointer(cs))
	ptr := C.rlm_node_from_json(cs)
	if ptr == nil {
		return nil, lastError()
	}
	node := &Node{ptr: ptr}
	runtime.SetFinalizer(node, (*Node).Free)
	return node, nil
}

// HyperEdge represents a hyperedge connecting multiple nodes.
type HyperEdge struct {
	ptr *C.RlmHyperEdge
}

// NewHyperEdge creates a new hyperedge with the given type.
// Valid types: "semantic", "structural", "causal", "temporal", "reference", "reasoning"
func NewHyperEdge(edgeType string) (*HyperEdge, error) {
	ctype := cString(edgeType)
	defer C.free(unsafe.Pointer(ctype))
	ptr := C.rlm_hyperedge_new(ctype)
	if ptr == nil {
		return nil, lastError()
	}
	edge := &HyperEdge{ptr: ptr}
	runtime.SetFinalizer(edge, (*HyperEdge).Free)
	return edge, nil
}

// NewBinaryEdge creates a binary edge between two nodes.
func NewBinaryEdge(edgeType, subjectID, objectID, label string) (*HyperEdge, error) {
	ctype := cString(edgeType)
	defer C.free(unsafe.Pointer(ctype))
	csubject := cString(subjectID)
	defer C.free(unsafe.Pointer(csubject))
	cobject := cString(objectID)
	defer C.free(unsafe.Pointer(cobject))
	clabel := cString(label)
	defer C.free(unsafe.Pointer(clabel))

	ptr := C.rlm_hyperedge_binary(ctype, csubject, cobject, clabel)
	if ptr == nil {
		return nil, lastError()
	}
	edge := &HyperEdge{ptr: ptr}
	runtime.SetFinalizer(edge, (*HyperEdge).Free)
	return edge, nil
}

// Free releases the hyperedge resources.
func (e *HyperEdge) Free() {
	if e.ptr != nil {
		C.rlm_hyperedge_free(e.ptr)
		e.ptr = nil
	}
}

// ID returns the edge's unique identifier.
func (e *HyperEdge) ID() string {
	return goString(C.rlm_hyperedge_id(e.ptr))
}

// Type returns the edge type.
func (e *HyperEdge) Type() string {
	return goString(C.rlm_hyperedge_type(e.ptr))
}

// Label returns the edge label, or empty string if not set.
func (e *HyperEdge) Label() string {
	cstr := C.rlm_hyperedge_label(e.ptr)
	if cstr == nil {
		return ""
	}
	return goString(cstr)
}

// Weight returns the edge weight.
func (e *HyperEdge) Weight() float64 {
	return float64(C.rlm_hyperedge_weight(e.ptr))
}

// NodeIDs returns the IDs of all member nodes.
func (e *HyperEdge) NodeIDs() ([]string, error) {
	cstr := C.rlm_hyperedge_node_ids(e.ptr)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)
	var ids []string
	if err := json.Unmarshal([]byte(jsonStr), &ids); err != nil {
		return nil, err
	}
	return ids, nil
}

// Contains returns true if the node is a member of this edge.
func (e *HyperEdge) Contains(nodeID string) bool {
	cid := cString(nodeID)
	defer C.free(unsafe.Pointer(cid))
	return C.rlm_hyperedge_contains(e.ptr, cid) != 0
}
