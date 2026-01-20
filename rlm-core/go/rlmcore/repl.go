package rlmcore

/*
#include <stdlib.h>
#include <stdint.h>

// Opaque types for REPL
typedef struct RlmReplHandle RlmReplHandle;
typedef struct RlmReplPool RlmReplPool;

// REPL configuration
char* rlm_repl_config_default(void);

// ReplHandle functions
RlmReplHandle* rlm_repl_handle_spawn_default(void);
RlmReplHandle* rlm_repl_handle_spawn(const char* config_json);
void rlm_repl_handle_free(RlmReplHandle* handle);
char* rlm_repl_handle_execute(RlmReplHandle* handle, const char* code);
char* rlm_repl_handle_get_variable(RlmReplHandle* handle, const char* name);
int rlm_repl_handle_set_variable(RlmReplHandle* handle, const char* name, const char* value_json);
int rlm_repl_handle_resolve_operation(RlmReplHandle* handle, const char* operation_id, const char* result_json);
char* rlm_repl_handle_list_variables(RlmReplHandle* handle);
char* rlm_repl_handle_status(RlmReplHandle* handle);
int rlm_repl_handle_reset(RlmReplHandle* handle);
int rlm_repl_handle_shutdown(RlmReplHandle* handle);
int rlm_repl_handle_is_alive(RlmReplHandle* handle);

// ReplPool functions
RlmReplPool* rlm_repl_pool_new_default(size_t max_size);
RlmReplPool* rlm_repl_pool_new(const char* config_json, size_t max_size);
void rlm_repl_pool_free(RlmReplPool* pool);
RlmReplHandle* rlm_repl_pool_acquire(const RlmReplPool* pool);
void rlm_repl_pool_release(const RlmReplPool* pool, RlmReplHandle* handle);
*/
import "C"

import (
	"encoding/json"
	"runtime"
	"unsafe"
)

// ReplConfig contains configuration for REPL subprocess.
type ReplConfig struct {
	PythonPath      string  `json:"python_path"`
	ReplPackagePath *string `json:"repl_package_path,omitempty"`
	TimeoutMs       uint64  `json:"timeout_ms"`
	MaxMemoryBytes  *uint64 `json:"max_memory_bytes,omitempty"`
	MaxCPUSeconds   *uint64 `json:"max_cpu_seconds,omitempty"`
}

// DefaultReplConfig returns the default REPL configuration.
func DefaultReplConfig() (*ReplConfig, error) {
	cstr := C.rlm_repl_config_default()
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var config ReplConfig
	if err := json.Unmarshal([]byte(jsonStr), &config); err != nil {
		return nil, err
	}
	return &config, nil
}

// ExecuteResult contains the result of code execution.
type ExecuteResult struct {
	Success           bool     `json:"success"`
	Result            any      `json:"result,omitempty"`
	Stdout            string   `json:"stdout"`
	Stderr            string   `json:"stderr"`
	Error             *string  `json:"error,omitempty"`
	ErrorType         *string  `json:"error_type,omitempty"`
	ExecutionTimeMs   float64  `json:"execution_time_ms"`
	PendingOperations []string `json:"pending_operations"`
}

// ReplStatus contains REPL status information.
type ReplStatus struct {
	Ready             bool    `json:"ready"`
	PendingOperations int     `json:"pending_operations"`
	VariablesCount    int     `json:"variables_count"`
	MemoryUsageBytes  *uint64 `json:"memory_usage_bytes,omitempty"`
}

// ReplHandle is a handle to a running REPL subprocess.
type ReplHandle struct {
	ptr *C.RlmReplHandle
}

// SpawnReplDefault spawns a new REPL subprocess with default configuration.
func SpawnReplDefault() (*ReplHandle, error) {
	ptr := C.rlm_repl_handle_spawn_default()
	if ptr == nil {
		return nil, lastError()
	}
	h := &ReplHandle{ptr: ptr}
	runtime.SetFinalizer(h, (*ReplHandle).Free)
	return h, nil
}

// SpawnRepl spawns a new REPL subprocess with custom configuration.
func SpawnRepl(config *ReplConfig) (*ReplHandle, error) {
	configJSON, err := json.Marshal(config)
	if err != nil {
		return nil, err
	}
	cconfig := cString(string(configJSON))
	defer C.free(unsafe.Pointer(cconfig))

	ptr := C.rlm_repl_handle_spawn(cconfig)
	if ptr == nil {
		return nil, lastError()
	}
	h := &ReplHandle{ptr: ptr}
	runtime.SetFinalizer(h, (*ReplHandle).Free)
	return h, nil
}

// Free releases the REPL handle resources.
func (h *ReplHandle) Free() {
	if h.ptr != nil {
		C.rlm_repl_handle_free(h.ptr)
		h.ptr = nil
	}
}

// Execute executes Python code in the REPL.
func (h *ReplHandle) Execute(code string) (*ExecuteResult, error) {
	ccode := cString(code)
	defer C.free(unsafe.Pointer(ccode))

	cstr := C.rlm_repl_handle_execute(h.ptr, ccode)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var result ExecuteResult
	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// GetVariable gets a variable from the REPL namespace.
func (h *ReplHandle) GetVariable(name string) (any, error) {
	cname := cString(name)
	defer C.free(unsafe.Pointer(cname))

	cstr := C.rlm_repl_handle_get_variable(h.ptr, cname)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var value any
	if err := json.Unmarshal([]byte(jsonStr), &value); err != nil {
		return nil, err
	}
	return value, nil
}

// SetVariable sets a variable in the REPL namespace.
func (h *ReplHandle) SetVariable(name string, value any) error {
	valueJSON, err := json.Marshal(value)
	if err != nil {
		return err
	}

	cname := cString(name)
	defer C.free(unsafe.Pointer(cname))
	cvalue := cString(string(valueJSON))
	defer C.free(unsafe.Pointer(cvalue))

	if C.rlm_repl_handle_set_variable(h.ptr, cname, cvalue) != 0 {
		return lastError()
	}
	return nil
}

// ResolveOperation resolves a deferred operation.
func (h *ReplHandle) ResolveOperation(operationID string, result any) error {
	resultJSON, err := json.Marshal(result)
	if err != nil {
		return err
	}

	copid := cString(operationID)
	defer C.free(unsafe.Pointer(copid))
	cresult := cString(string(resultJSON))
	defer C.free(unsafe.Pointer(cresult))

	if C.rlm_repl_handle_resolve_operation(h.ptr, copid, cresult) != 0 {
		return lastError()
	}
	return nil
}

// ListVariables lists all variables in the REPL namespace.
func (h *ReplHandle) ListVariables() (map[string]string, error) {
	cstr := C.rlm_repl_handle_list_variables(h.ptr)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var vars map[string]string
	if err := json.Unmarshal([]byte(jsonStr), &vars); err != nil {
		return nil, err
	}
	return vars, nil
}

// Status returns the REPL status.
func (h *ReplHandle) Status() (*ReplStatus, error) {
	cstr := C.rlm_repl_handle_status(h.ptr)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var status ReplStatus
	if err := json.Unmarshal([]byte(jsonStr), &status); err != nil {
		return nil, err
	}
	return &status, nil
}

// Reset resets the REPL state.
func (h *ReplHandle) Reset() error {
	if C.rlm_repl_handle_reset(h.ptr) != 0 {
		return lastError()
	}
	return nil
}

// Shutdown shuts down the REPL subprocess.
func (h *ReplHandle) Shutdown() error {
	if C.rlm_repl_handle_shutdown(h.ptr) != 0 {
		return lastError()
	}
	return nil
}

// IsAlive checks if the REPL subprocess is still running.
func (h *ReplHandle) IsAlive() bool {
	return C.rlm_repl_handle_is_alive(h.ptr) == 1
}

// ReplPool is a pool of REPL subprocess handles.
type ReplPool struct {
	ptr *C.RlmReplPool
}

// NewReplPoolDefault creates a new REPL pool with default configuration.
func NewReplPoolDefault(maxSize int) *ReplPool {
	ptr := C.rlm_repl_pool_new_default(C.size_t(maxSize))
	p := &ReplPool{ptr: ptr}
	runtime.SetFinalizer(p, (*ReplPool).Free)
	return p
}

// NewReplPool creates a new REPL pool with custom configuration.
func NewReplPool(config *ReplConfig, maxSize int) (*ReplPool, error) {
	configJSON, err := json.Marshal(config)
	if err != nil {
		return nil, err
	}
	cconfig := cString(string(configJSON))
	defer C.free(unsafe.Pointer(cconfig))

	ptr := C.rlm_repl_pool_new(cconfig, C.size_t(maxSize))
	if ptr == nil {
		return nil, lastError()
	}
	p := &ReplPool{ptr: ptr}
	runtime.SetFinalizer(p, (*ReplPool).Free)
	return p, nil
}

// Free releases the REPL pool resources.
func (p *ReplPool) Free() {
	if p.ptr != nil {
		C.rlm_repl_pool_free(p.ptr)
		p.ptr = nil
	}
}

// Acquire acquires a REPL handle from the pool.
func (p *ReplPool) Acquire() (*ReplHandle, error) {
	ptr := C.rlm_repl_pool_acquire(p.ptr)
	if ptr == nil {
		return nil, lastError()
	}
	h := &ReplHandle{ptr: ptr}
	// Don't set finalizer - handle should be released back to pool
	return h, nil
}

// Release releases a REPL handle back to the pool.
func (p *ReplPool) Release(h *ReplHandle) {
	if h.ptr != nil {
		C.rlm_repl_pool_release(p.ptr, h.ptr)
		h.ptr = nil // Mark as released
	}
}
