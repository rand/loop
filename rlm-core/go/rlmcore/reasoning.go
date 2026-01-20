// Package rlmcore provides Go bindings for the rlm-core Rust library.
// This file contains reasoning trace bindings for Deciduous-style provenance tracking.

package rlmcore

/*
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -lrlm_core
#cgo darwin LDFLAGS: -framework Security -framework CoreFoundation

#include <stdlib.h>
#include <stdint.h>

// ReasoningTrace opaque type and functions
typedef struct RlmReasoningTrace RlmReasoningTrace;
RlmReasoningTrace* rlm_reasoning_trace_new(const char* goal, const char* session_id);
void rlm_reasoning_trace_free(RlmReasoningTrace* trace);
char* rlm_reasoning_trace_id(const RlmReasoningTrace* trace);
char* rlm_reasoning_trace_log_decision(
    RlmReasoningTrace* trace,
    const char* question,
    const char* options_json,
    size_t chosen_index,
    const char* rationale);
char* rlm_reasoning_trace_log_action(
    RlmReasoningTrace* trace,
    const char* action_description,
    const char* outcome_description,
    const char* parent_id);
int rlm_reasoning_trace_link_commit(RlmReasoningTrace* trace, const char* commit_sha);
char* rlm_reasoning_trace_stats(const RlmReasoningTrace* trace);
char* rlm_reasoning_trace_to_json(const RlmReasoningTrace* trace);
char* rlm_reasoning_trace_to_mermaid(const RlmReasoningTrace* trace);
char* rlm_reasoning_trace_analyze(const RlmReasoningTrace* trace);

// ReasoningTraceStore opaque type and functions
typedef struct RlmReasoningTraceStore RlmReasoningTraceStore;
RlmReasoningTraceStore* rlm_reasoning_trace_store_in_memory(void);
RlmReasoningTraceStore* rlm_reasoning_trace_store_open(const char* path);
void rlm_reasoning_trace_store_free(RlmReasoningTraceStore* store);
int rlm_reasoning_trace_store_save(RlmReasoningTraceStore* store, const RlmReasoningTrace* trace);
RlmReasoningTrace* rlm_reasoning_trace_store_load(RlmReasoningTraceStore* store, const char* trace_id);
char* rlm_reasoning_trace_store_find_by_session(RlmReasoningTraceStore* store, const char* session_id);
char* rlm_reasoning_trace_store_find_by_commit(RlmReasoningTraceStore* store, const char* commit);
char* rlm_reasoning_trace_store_stats(RlmReasoningTraceStore* store);
*/
import "C"

import (
	"encoding/json"
	"runtime"
	"unsafe"
)

// ============================================================================
// ReasoningTrace
// ============================================================================

// ReasoningTrace captures the provenance of decisions during reasoning.
// Based on Deciduous-style decision trees for explainability.
type ReasoningTrace struct {
	ptr *C.RlmReasoningTrace
}

// TraceStats contains statistics about a reasoning trace.
type TraceStats struct {
	DecisionCount int `json:"decision_count"`
	OptionCount   int `json:"option_count"`
	ChosenCount   int `json:"chosen_count"`
	RejectedCount int `json:"rejected_count"`
	TotalNodes    int `json:"total_nodes"`
	TotalEdges    int `json:"total_edges"`
	MaxDepth      int `json:"max_depth"`
}

// DecisionResult contains the ID of the chosen option after logging a decision.
type DecisionResult struct {
	ChosenID string `json:"chosen_id"`
}

// ActionResult contains the IDs of nodes created when logging an action.
type ActionResult struct {
	ActionID  string `json:"action_id"`
	OutcomeID string `json:"outcome_id"`
}

// TraceAnalysis contains the results of analyzing a trace.
type TraceAnalysis struct {
	Confidence       float64  `json:"confidence"`
	Narrative        string   `json:"narrative"`
	DecisionPath     []string `json:"decision_path"`
	KeyInsights      []string `json:"key_insights,omitempty"`
	RejectedCount    int      `json:"rejected_count"`
	AlternativePaths int      `json:"alternative_paths"`
}

// NewReasoningTrace creates a new reasoning trace for tracking decisions.
// goal: The objective being reasoned about
// sessionID: Optional session identifier (can be empty)
func NewReasoningTrace(goal, sessionID string) *ReasoningTrace {
	cgoal := cString(goal)
	defer C.free(unsafe.Pointer(cgoal))

	var csessionID *C.char
	if sessionID != "" {
		csessionID = cString(sessionID)
		defer C.free(unsafe.Pointer(csessionID))
	}

	t := &ReasoningTrace{ptr: C.rlm_reasoning_trace_new(cgoal, csessionID)}
	runtime.SetFinalizer(t, (*ReasoningTrace).Free)
	return t
}

// Free releases the reasoning trace resources.
func (t *ReasoningTrace) Free() {
	if t.ptr != nil {
		C.rlm_reasoning_trace_free(t.ptr)
		t.ptr = nil
	}
}

// ID returns the trace's unique identifier.
func (t *ReasoningTrace) ID() (string, error) {
	cstr := C.rlm_reasoning_trace_id(t.ptr)
	if cstr == nil {
		return "", lastError()
	}
	jsonStr := goString(cstr)

	var result struct {
		TraceID string `json:"trace_id"`
	}
	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return "", err
	}
	return result.TraceID, nil
}

// LogDecision logs a decision point with multiple options.
// question: The decision being made
// options: Available choices
// chosenIndex: Index of the selected option (0-based)
// rationale: Explanation for the choice
func (t *ReasoningTrace) LogDecision(question string, options []string, chosenIndex int, rationale string) (*DecisionResult, error) {
	cquestion := cString(question)
	defer C.free(unsafe.Pointer(cquestion))

	optionsJSON, err := json.Marshal(options)
	if err != nil {
		return nil, err
	}
	coptionsJSON := cString(string(optionsJSON))
	defer C.free(unsafe.Pointer(coptionsJSON))

	crationale := cString(rationale)
	defer C.free(unsafe.Pointer(crationale))

	cstr := C.rlm_reasoning_trace_log_decision(
		t.ptr,
		cquestion,
		coptionsJSON,
		C.size_t(chosenIndex),
		crationale,
	)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var result DecisionResult
	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// LogAction logs an action taken and its outcome.
// actionDescription: What was done
// outcomeDescription: What resulted
// parentID: Optional parent decision node ID (can be empty)
func (t *ReasoningTrace) LogAction(actionDescription, outcomeDescription, parentID string) (*ActionResult, error) {
	caction := cString(actionDescription)
	defer C.free(unsafe.Pointer(caction))

	coutcome := cString(outcomeDescription)
	defer C.free(unsafe.Pointer(coutcome))

	var cparentID *C.char
	if parentID != "" {
		cparentID = cString(parentID)
		defer C.free(unsafe.Pointer(cparentID))
	}

	cstr := C.rlm_reasoning_trace_log_action(t.ptr, caction, coutcome, cparentID)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var result ActionResult
	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// LinkCommit associates the trace with a git commit.
func (t *ReasoningTrace) LinkCommit(commitSHA string) error {
	ccommit := cString(commitSHA)
	defer C.free(unsafe.Pointer(ccommit))

	if C.rlm_reasoning_trace_link_commit(t.ptr, ccommit) != 0 {
		return lastError()
	}
	return nil
}

// Stats returns statistics about the trace.
func (t *ReasoningTrace) Stats() (*TraceStats, error) {
	cstr := C.rlm_reasoning_trace_stats(t.ptr)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var stats TraceStats
	if err := json.Unmarshal([]byte(jsonStr), &stats); err != nil {
		return nil, err
	}
	return &stats, nil
}

// ToJSON exports the trace to JSON format.
func (t *ReasoningTrace) ToJSON() (string, error) {
	cstr := C.rlm_reasoning_trace_to_json(t.ptr)
	if cstr == nil {
		return "", lastError()
	}
	return goString(cstr), nil
}

// ToMermaid exports the trace to Mermaid flowchart format.
func (t *ReasoningTrace) ToMermaid() (string, error) {
	cstr := C.rlm_reasoning_trace_to_mermaid(t.ptr)
	if cstr == nil {
		return "", lastError()
	}
	return goString(cstr), nil
}

// Analyze runs analysis on the trace and returns insights.
func (t *ReasoningTrace) Analyze() (*TraceAnalysis, error) {
	cstr := C.rlm_reasoning_trace_analyze(t.ptr)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var analysis TraceAnalysis
	if err := json.Unmarshal([]byte(jsonStr), &analysis); err != nil {
		return nil, err
	}
	return &analysis, nil
}

// ============================================================================
// ReasoningTraceStore
// ============================================================================

// ReasoningTraceStore provides persistence for reasoning traces.
type ReasoningTraceStore struct {
	ptr *C.RlmReasoningTraceStore
}

// TraceStoreStats contains statistics about the trace store.
type TraceStoreStats struct {
	TotalTraces        int `json:"total_traces"`
	TotalDecisionNodes int `json:"total_decision_nodes"`
	TotalMemoryNodes   int `json:"total_memory_nodes"`
	TotalEdges         int `json:"total_edges"`
}

// NewReasoningTraceStoreInMemory creates an in-memory trace store.
func NewReasoningTraceStoreInMemory() *ReasoningTraceStore {
	s := &ReasoningTraceStore{ptr: C.rlm_reasoning_trace_store_in_memory()}
	runtime.SetFinalizer(s, (*ReasoningTraceStore).Free)
	return s
}

// OpenReasoningTraceStore opens a file-backed trace store.
func OpenReasoningTraceStore(path string) (*ReasoningTraceStore, error) {
	cpath := cString(path)
	defer C.free(unsafe.Pointer(cpath))

	ptr := C.rlm_reasoning_trace_store_open(cpath)
	if ptr == nil {
		return nil, lastError()
	}
	s := &ReasoningTraceStore{ptr: ptr}
	runtime.SetFinalizer(s, (*ReasoningTraceStore).Free)
	return s, nil
}

// Free releases the trace store resources.
func (s *ReasoningTraceStore) Free() {
	if s.ptr != nil {
		C.rlm_reasoning_trace_store_free(s.ptr)
		s.ptr = nil
	}
}

// Save persists a trace to the store.
func (s *ReasoningTraceStore) Save(trace *ReasoningTrace) error {
	if C.rlm_reasoning_trace_store_save(s.ptr, trace.ptr) != 0 {
		return lastError()
	}
	return nil
}

// Load retrieves a trace from the store by ID.
func (s *ReasoningTraceStore) Load(traceID string) (*ReasoningTrace, error) {
	ctraceID := cString(traceID)
	defer C.free(unsafe.Pointer(ctraceID))

	ptr := C.rlm_reasoning_trace_store_load(s.ptr, ctraceID)
	if ptr == nil {
		return nil, lastError()
	}
	t := &ReasoningTrace{ptr: ptr}
	runtime.SetFinalizer(t, (*ReasoningTrace).Free)
	return t, nil
}

// FindBySession finds all traces associated with a session.
func (s *ReasoningTraceStore) FindBySession(sessionID string) ([]string, error) {
	csessionID := cString(sessionID)
	defer C.free(unsafe.Pointer(csessionID))

	cstr := C.rlm_reasoning_trace_store_find_by_session(s.ptr, csessionID)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var traceIDs []string
	if err := json.Unmarshal([]byte(jsonStr), &traceIDs); err != nil {
		return nil, err
	}
	return traceIDs, nil
}

// FindByCommit finds all traces linked to a git commit.
func (s *ReasoningTraceStore) FindByCommit(commit string) ([]string, error) {
	ccommit := cString(commit)
	defer C.free(unsafe.Pointer(ccommit))

	cstr := C.rlm_reasoning_trace_store_find_by_commit(s.ptr, ccommit)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var traceIDs []string
	if err := json.Unmarshal([]byte(jsonStr), &traceIDs); err != nil {
		return nil, err
	}
	return traceIDs, nil
}

// Stats returns statistics about the store.
func (s *ReasoningTraceStore) Stats() (*TraceStoreStats, error) {
	cstr := C.rlm_reasoning_trace_store_stats(s.ptr)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var stats TraceStoreStats
	if err := json.Unmarshal([]byte(jsonStr), &stats); err != nil {
		return nil, err
	}
	return &stats, nil
}
