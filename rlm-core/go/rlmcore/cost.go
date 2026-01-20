// Package rlmcore provides Go bindings for the rlm-core Rust library.
// This file contains cost tracking bindings.

package rlmcore

/*
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -lrlm_core
#cgo darwin LDFLAGS: -framework Security -framework CoreFoundation

#include <stdlib.h>
#include <stdint.h>

// CostTracker opaque type and functions
typedef struct RlmCostTracker RlmCostTracker;
RlmCostTracker* rlm_cost_tracker_new(void);
void rlm_cost_tracker_free(RlmCostTracker* tracker);
int rlm_cost_tracker_record(
    RlmCostTracker* tracker,
    const char* model,
    uint64_t input_tokens,
    uint64_t output_tokens,
    uint64_t cache_read_tokens,
    uint64_t cache_creation_tokens,
    double cost);
int rlm_cost_tracker_merge(RlmCostTracker* tracker, const RlmCostTracker* other);
uint64_t rlm_cost_tracker_total_input_tokens(const RlmCostTracker* tracker);
uint64_t rlm_cost_tracker_total_output_tokens(const RlmCostTracker* tracker);
uint64_t rlm_cost_tracker_total_cache_read_tokens(const RlmCostTracker* tracker);
uint64_t rlm_cost_tracker_total_cache_creation_tokens(const RlmCostTracker* tracker);
double rlm_cost_tracker_total_cost(const RlmCostTracker* tracker);
uint64_t rlm_cost_tracker_request_count(const RlmCostTracker* tracker);
char* rlm_cost_tracker_by_model_json(const RlmCostTracker* tracker);
char* rlm_cost_tracker_to_json(const RlmCostTracker* tracker);
RlmCostTracker* rlm_cost_tracker_from_json(const char* json);

// Cost calculation helpers
double rlm_calculate_cost(const char* model_json, uint64_t input_tokens, uint64_t output_tokens);
double rlm_calculate_cost_by_name(const char* model_name, uint64_t input_tokens, uint64_t output_tokens);
char* rlm_model_spec_json(const char* model_name);
uint64_t rlm_effective_input_tokens(uint64_t input_tokens, uint64_t cache_read_tokens);
*/
import "C"

import (
	"encoding/json"
	"runtime"
	"unsafe"
)

// ============================================================================
// CostTracker
// ============================================================================

// CostTracker tracks token usage and costs across LLM requests.
type CostTracker struct {
	ptr *C.RlmCostTracker
}

// ModelCost represents cost breakdown for a specific model.
type ModelCost struct {
	InputTokens         uint64  `json:"input_tokens"`
	OutputTokens        uint64  `json:"output_tokens"`
	CacheReadTokens     uint64  `json:"cache_read_tokens"`
	CacheCreationTokens uint64  `json:"cache_creation_tokens"`
	TotalCost           float64 `json:"total_cost"`
	RequestCount        uint64  `json:"request_count"`
}

// NewCostTracker creates a new cost tracker.
func NewCostTracker() *CostTracker {
	ct := &CostTracker{ptr: C.rlm_cost_tracker_new()}
	runtime.SetFinalizer(ct, (*CostTracker).Free)
	return ct
}

// Free releases the cost tracker resources.
func (ct *CostTracker) Free() {
	if ct.ptr != nil {
		C.rlm_cost_tracker_free(ct.ptr)
		ct.ptr = nil
	}
}

// Record records token usage from a completion.
// Pass a negative cost value if cost is unknown.
func (ct *CostTracker) Record(model string, inputTokens, outputTokens, cacheReadTokens, cacheCreationTokens uint64, cost float64) error {
	cmodel := cString(model)
	defer C.free(unsafe.Pointer(cmodel))

	result := C.rlm_cost_tracker_record(
		ct.ptr,
		cmodel,
		C.uint64_t(inputTokens),
		C.uint64_t(outputTokens),
		C.uint64_t(cacheReadTokens),
		C.uint64_t(cacheCreationTokens),
		C.double(cost),
	)
	if result != 0 {
		return lastError()
	}
	return nil
}

// Merge merges another tracker into this one.
func (ct *CostTracker) Merge(other *CostTracker) error {
	if other == nil || other.ptr == nil {
		return nil
	}
	result := C.rlm_cost_tracker_merge(ct.ptr, other.ptr)
	if result != 0 {
		return lastError()
	}
	return nil
}

// TotalInputTokens returns the total input tokens tracked.
func (ct *CostTracker) TotalInputTokens() uint64 {
	return uint64(C.rlm_cost_tracker_total_input_tokens(ct.ptr))
}

// TotalOutputTokens returns the total output tokens tracked.
func (ct *CostTracker) TotalOutputTokens() uint64 {
	return uint64(C.rlm_cost_tracker_total_output_tokens(ct.ptr))
}

// TotalCacheReadTokens returns the total cache read tokens tracked.
func (ct *CostTracker) TotalCacheReadTokens() uint64 {
	return uint64(C.rlm_cost_tracker_total_cache_read_tokens(ct.ptr))
}

// TotalCacheCreationTokens returns the total cache creation tokens tracked.
func (ct *CostTracker) TotalCacheCreationTokens() uint64 {
	return uint64(C.rlm_cost_tracker_total_cache_creation_tokens(ct.ptr))
}

// TotalCost returns the total cost in USD.
func (ct *CostTracker) TotalCost() float64 {
	return float64(C.rlm_cost_tracker_total_cost(ct.ptr))
}

// RequestCount returns the total number of requests tracked.
func (ct *CostTracker) RequestCount() uint64 {
	return uint64(C.rlm_cost_tracker_request_count(ct.ptr))
}

// ByModel returns the per-model cost breakdown.
func (ct *CostTracker) ByModel() (map[string]ModelCost, error) {
	cstr := C.rlm_cost_tracker_by_model_json(ct.ptr)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var result map[string]ModelCost
	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return nil, err
	}
	return result, nil
}

// ToJSON serializes the tracker to JSON.
func (ct *CostTracker) ToJSON() (string, error) {
	cstr := C.rlm_cost_tracker_to_json(ct.ptr)
	if cstr == nil {
		return "", lastError()
	}
	return goString(cstr), nil
}

// NewCostTrackerFromJSON deserializes a tracker from JSON.
func NewCostTrackerFromJSON(jsonStr string) (*CostTracker, error) {
	cjson := cString(jsonStr)
	defer C.free(unsafe.Pointer(cjson))

	ptr := C.rlm_cost_tracker_from_json(cjson)
	if ptr == nil {
		return nil, lastError()
	}

	ct := &CostTracker{ptr: ptr}
	runtime.SetFinalizer(ct, (*CostTracker).Free)
	return ct, nil
}

// ============================================================================
// Cost Calculation Helpers
// ============================================================================

// CalculateCost calculates cost for given token usage with a model spec JSON.
// Returns cost in USD, or -1.0 on error.
func CalculateCost(modelJSON string, inputTokens, outputTokens uint64) float64 {
	cmodel := cString(modelJSON)
	defer C.free(unsafe.Pointer(cmodel))

	return float64(C.rlm_calculate_cost(cmodel, C.uint64_t(inputTokens), C.uint64_t(outputTokens)))
}

// CalculateCostByName calculates cost using well-known model names.
// Supported: "claude-opus", "claude-sonnet", "claude-haiku", "gpt-4o", "gpt-4o-mini"
// Returns cost in USD, or -1.0 on error (unknown model).
func CalculateCostByName(modelName string, inputTokens, outputTokens uint64) float64 {
	cmodel := cString(modelName)
	defer C.free(unsafe.Pointer(cmodel))

	return float64(C.rlm_calculate_cost_by_name(cmodel, C.uint64_t(inputTokens), C.uint64_t(outputTokens)))
}

// ModelSpecJSON returns default model spec JSON for a well-known model.
// Supported: "claude-opus", "claude-sonnet", "claude-haiku", "gpt-4o", "gpt-4o-mini"
func ModelSpecJSON(modelName string) (string, error) {
	cmodel := cString(modelName)
	defer C.free(unsafe.Pointer(cmodel))

	cstr := C.rlm_model_spec_json(cmodel)
	if cstr == nil {
		return "", lastError()
	}
	return goString(cstr), nil
}

// EffectiveInputTokens calculates effective input tokens accounting for cache reads.
// Cache reads are typically 90% cheaper, so we count them at 10%.
func EffectiveInputTokens(inputTokens, cacheReadTokens uint64) uint64 {
	return uint64(C.rlm_effective_input_tokens(C.uint64_t(inputTokens), C.uint64_t(cacheReadTokens)))
}
