// Package rlmcore provides Go bindings for the rlm-core Rust library.
// This file contains orchestrator bindings for execution mode and configuration.

package rlmcore

/*
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -lrlm_core
#cgo darwin LDFLAGS: -framework Security -framework CoreFoundation

#include <stdlib.h>
#include <stdint.h>

// ExecutionMode enum
typedef enum {
    RLM_EXECUTION_MODE_MICRO = 0,
    RLM_EXECUTION_MODE_FAST = 1,
    RLM_EXECUTION_MODE_BALANCED = 2,
    RLM_EXECUTION_MODE_THOROUGH = 3
} RlmExecutionMode;

// ExecutionMode functions
double rlm_execution_mode_budget_usd(RlmExecutionMode mode);
uint32_t rlm_execution_mode_max_depth(RlmExecutionMode mode);
char* rlm_execution_mode_name(RlmExecutionMode mode);
RlmExecutionMode rlm_execution_mode_from_signals(const char* signals_json);

// OrchestratorConfig opaque type and functions
typedef struct RlmOrchestratorConfig RlmOrchestratorConfig;
RlmOrchestratorConfig* rlm_orchestrator_config_default(void);
void rlm_orchestrator_config_free(RlmOrchestratorConfig* config);
uint32_t rlm_orchestrator_config_max_depth(const RlmOrchestratorConfig* config);
int rlm_orchestrator_config_default_spawn_repl(const RlmOrchestratorConfig* config);
uint64_t rlm_orchestrator_config_repl_timeout_ms(const RlmOrchestratorConfig* config);
uint64_t rlm_orchestrator_config_max_tokens_per_call(const RlmOrchestratorConfig* config);
uint64_t rlm_orchestrator_config_total_token_budget(const RlmOrchestratorConfig* config);
double rlm_orchestrator_config_cost_budget_usd(const RlmOrchestratorConfig* config);
char* rlm_orchestrator_config_to_json(const RlmOrchestratorConfig* config);
RlmOrchestratorConfig* rlm_orchestrator_config_from_json(const char* json);

// OrchestratorBuilder opaque type and functions
typedef struct RlmOrchestratorBuilder RlmOrchestratorBuilder;
RlmOrchestratorBuilder* rlm_orchestrator_builder_new(void);
void rlm_orchestrator_builder_free(RlmOrchestratorBuilder* builder);
RlmOrchestratorBuilder* rlm_orchestrator_builder_max_depth(RlmOrchestratorBuilder* builder, uint32_t depth);
RlmOrchestratorBuilder* rlm_orchestrator_builder_default_spawn_repl(RlmOrchestratorBuilder* builder, int spawn);
RlmOrchestratorBuilder* rlm_orchestrator_builder_repl_timeout_ms(RlmOrchestratorBuilder* builder, uint64_t timeout);
RlmOrchestratorBuilder* rlm_orchestrator_builder_total_token_budget(RlmOrchestratorBuilder* builder, uint64_t budget);
RlmOrchestratorBuilder* rlm_orchestrator_builder_cost_budget_usd(RlmOrchestratorBuilder* builder, double budget);
RlmOrchestratorBuilder* rlm_orchestrator_builder_execution_mode(RlmOrchestratorBuilder* builder, RlmExecutionMode mode);
RlmOrchestratorConfig* rlm_orchestrator_builder_build(RlmOrchestratorBuilder* builder);
RlmExecutionMode rlm_orchestrator_builder_get_mode(const RlmOrchestratorBuilder* builder);

// Complexity signals functions
char* rlm_complexity_signals_parse(const char* json);
int rlm_complexity_signals_score(const char* json);
int rlm_complexity_signals_has_strong_signal(const char* json);
*/
import "C"

import (
	"encoding/json"
	"runtime"
	"unsafe"
)

// ============================================================================
// ExecutionMode
// ============================================================================

// ExecutionMode represents the orchestration execution mode.
type ExecutionMode int

const (
	// ExecutionModeMicro is minimal cost, REPL-only mode (~$0.01)
	ExecutionModeMicro ExecutionMode = 0
	// ExecutionModeFast is quick responses mode (~$0.05)
	ExecutionModeFast ExecutionMode = 1
	// ExecutionModeBalanced is the default for complex tasks (~$0.25)
	ExecutionModeBalanced ExecutionMode = 2
	// ExecutionModeThorough is deep analysis mode (~$1.00)
	ExecutionModeThorough ExecutionMode = 3
)

// String returns the mode name.
func (m ExecutionMode) String() string {
	cstr := C.rlm_execution_mode_name(C.RlmExecutionMode(m))
	if cstr == nil {
		return "unknown"
	}
	return goString(cstr)
}

// BudgetUSD returns the typical cost budget for this mode.
func (m ExecutionMode) BudgetUSD() float64 {
	return float64(C.rlm_execution_mode_budget_usd(C.RlmExecutionMode(m)))
}

// MaxDepth returns the max recursion depth for this mode.
func (m ExecutionMode) MaxDepth() uint32 {
	return uint32(C.rlm_execution_mode_max_depth(C.RlmExecutionMode(m)))
}

// ExecutionModeFromSignals selects execution mode based on complexity signals.
func ExecutionModeFromSignals(signals *ComplexitySignals) ExecutionMode {
	if signals == nil {
		return ExecutionModeMicro
	}

	signalsJSON, err := json.Marshal(signals)
	if err != nil {
		return ExecutionModeMicro
	}

	csignals := cString(string(signalsJSON))
	defer C.free(unsafe.Pointer(csignals))

	return ExecutionMode(C.rlm_execution_mode_from_signals(csignals))
}

// ============================================================================
// ComplexitySignals
// ============================================================================

// ComplexitySignals contains signals used for execution mode selection.
type ComplexitySignals struct {
	DebuggingTask          bool `json:"debugging_task,omitempty"`
	MultiFileRefs          bool `json:"multi_file_refs,omitempty"`
	DiscoveryKeywords      bool `json:"discovery_keywords,omitempty"`
	ArchitectureAnalysis   bool `json:"architecture_analysis,omitempty"`
	UserWantsFast          bool `json:"user_wants_fast,omitempty"`
	UserWantsThorough      bool `json:"user_wants_thorough,omitempty"`
	RequiresExhaustive     bool `json:"requires_exhaustive_search,omitempty"`
	LongContext            bool `json:"long_context,omitempty"`
	ComplexQuery           bool `json:"complex_query,omitempty"`
}

// Score returns the complexity score for these signals.
func (s *ComplexitySignals) Score() int {
	signalsJSON, err := json.Marshal(s)
	if err != nil {
		return 0
	}

	csignals := cString(string(signalsJSON))
	defer C.free(unsafe.Pointer(csignals))

	return int(C.rlm_complexity_signals_score(csignals))
}

// HasStrongSignal returns true if any strong signal is present.
func (s *ComplexitySignals) HasStrongSignal() bool {
	signalsJSON, err := json.Marshal(s)
	if err != nil {
		return false
	}

	csignals := cString(string(signalsJSON))
	defer C.free(unsafe.Pointer(csignals))

	return C.rlm_complexity_signals_has_strong_signal(csignals) != 0
}

// ============================================================================
// OrchestratorConfig
// ============================================================================

// OrchestratorConfig contains configuration for the orchestrator.
type OrchestratorConfig struct {
	ptr *C.RlmOrchestratorConfig
}

// NewOrchestratorConfigDefault creates a config with default values.
func NewOrchestratorConfigDefault() *OrchestratorConfig {
	c := &OrchestratorConfig{ptr: C.rlm_orchestrator_config_default()}
	runtime.SetFinalizer(c, (*OrchestratorConfig).Free)
	return c
}

// NewOrchestratorConfigFromJSON creates a config from JSON.
func NewOrchestratorConfigFromJSON(jsonStr string) (*OrchestratorConfig, error) {
	cjson := cString(jsonStr)
	defer C.free(unsafe.Pointer(cjson))

	ptr := C.rlm_orchestrator_config_from_json(cjson)
	if ptr == nil {
		return nil, lastError()
	}
	c := &OrchestratorConfig{ptr: ptr}
	runtime.SetFinalizer(c, (*OrchestratorConfig).Free)
	return c, nil
}

// Free releases the config resources.
func (c *OrchestratorConfig) Free() {
	if c.ptr != nil {
		C.rlm_orchestrator_config_free(c.ptr)
		c.ptr = nil
	}
}

// MaxDepth returns the maximum recursion depth.
func (c *OrchestratorConfig) MaxDepth() uint32 {
	return uint32(C.rlm_orchestrator_config_max_depth(c.ptr))
}

// DefaultSpawnREPL returns whether REPL spawning is enabled by default.
func (c *OrchestratorConfig) DefaultSpawnREPL() bool {
	return C.rlm_orchestrator_config_default_spawn_repl(c.ptr) != 0
}

// REPLTimeoutMs returns the REPL timeout in milliseconds.
func (c *OrchestratorConfig) REPLTimeoutMs() uint64 {
	return uint64(C.rlm_orchestrator_config_repl_timeout_ms(c.ptr))
}

// MaxTokensPerCall returns the max tokens per call.
func (c *OrchestratorConfig) MaxTokensPerCall() uint64 {
	return uint64(C.rlm_orchestrator_config_max_tokens_per_call(c.ptr))
}

// TotalTokenBudget returns the total token budget.
func (c *OrchestratorConfig) TotalTokenBudget() uint64 {
	return uint64(C.rlm_orchestrator_config_total_token_budget(c.ptr))
}

// CostBudgetUSD returns the cost budget in USD.
func (c *OrchestratorConfig) CostBudgetUSD() float64 {
	return float64(C.rlm_orchestrator_config_cost_budget_usd(c.ptr))
}

// ToJSON serializes the config to JSON.
func (c *OrchestratorConfig) ToJSON() (string, error) {
	cstr := C.rlm_orchestrator_config_to_json(c.ptr)
	if cstr == nil {
		return "", lastError()
	}
	return goString(cstr), nil
}

// ============================================================================
// OrchestratorBuilder
// ============================================================================

// OrchestratorBuilder builds orchestrator configurations.
type OrchestratorBuilder struct {
	ptr *C.RlmOrchestratorBuilder
}

// NewOrchestratorBuilder creates a new builder with default values.
func NewOrchestratorBuilder() *OrchestratorBuilder {
	b := &OrchestratorBuilder{ptr: C.rlm_orchestrator_builder_new()}
	runtime.SetFinalizer(b, (*OrchestratorBuilder).Free)
	return b
}

// Free releases the builder resources.
func (b *OrchestratorBuilder) Free() {
	if b.ptr != nil {
		C.rlm_orchestrator_builder_free(b.ptr)
		b.ptr = nil
	}
}

// MaxDepth sets the maximum recursion depth.
func (b *OrchestratorBuilder) MaxDepth(depth uint32) *OrchestratorBuilder {
	b.ptr = C.rlm_orchestrator_builder_max_depth(b.ptr, C.uint32_t(depth))
	return b
}

// DefaultSpawnREPL sets whether to spawn REPL by default.
func (b *OrchestratorBuilder) DefaultSpawnREPL(spawn bool) *OrchestratorBuilder {
	var cspawn C.int
	if spawn {
		cspawn = 1
	}
	b.ptr = C.rlm_orchestrator_builder_default_spawn_repl(b.ptr, cspawn)
	return b
}

// REPLTimeoutMs sets the REPL timeout in milliseconds.
func (b *OrchestratorBuilder) REPLTimeoutMs(timeout uint64) *OrchestratorBuilder {
	b.ptr = C.rlm_orchestrator_builder_repl_timeout_ms(b.ptr, C.uint64_t(timeout))
	return b
}

// TotalTokenBudget sets the total token budget.
func (b *OrchestratorBuilder) TotalTokenBudget(budget uint64) *OrchestratorBuilder {
	b.ptr = C.rlm_orchestrator_builder_total_token_budget(b.ptr, C.uint64_t(budget))
	return b
}

// CostBudgetUSD sets the cost budget in USD.
func (b *OrchestratorBuilder) CostBudgetUSD(budget float64) *OrchestratorBuilder {
	b.ptr = C.rlm_orchestrator_builder_cost_budget_usd(b.ptr, C.double(budget))
	return b
}

// ExecutionMode sets the execution mode.
func (b *OrchestratorBuilder) ExecutionMode(mode ExecutionMode) *OrchestratorBuilder {
	b.ptr = C.rlm_orchestrator_builder_execution_mode(b.ptr, C.RlmExecutionMode(mode))
	return b
}

// Build creates the config from the builder.
// The builder is consumed and should not be used after this call.
func (b *OrchestratorBuilder) Build() *OrchestratorConfig {
	ptr := C.rlm_orchestrator_builder_build(b.ptr)
	b.ptr = nil // consumed
	runtime.SetFinalizer(b, nil)

	c := &OrchestratorConfig{ptr: ptr}
	runtime.SetFinalizer(c, (*OrchestratorConfig).Free)
	return c
}

// GetMode returns the current execution mode.
func (b *OrchestratorBuilder) GetMode() ExecutionMode {
	return ExecutionMode(C.rlm_orchestrator_builder_get_mode(b.ptr))
}
