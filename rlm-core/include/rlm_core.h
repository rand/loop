/**
 * @file rlm_core.h
 * @brief C FFI bindings for rlm-core library
 *
 * This header provides C-compatible FFI functions for integration with
 * Go (CGO), Swift, and other languages that can call C APIs.
 *
 * ## Memory Management
 *
 * - Objects created by `*_new()` functions must be freed with corresponding `*_free()` functions
 * - Strings returned by the library must be freed with `rlm_string_free()`
 * - Caller-owned strings passed to functions are not freed by the library
 *
 * ## Error Handling
 *
 * - Functions that can fail return NULL for pointers or -1 for integers
 * - Check `rlm_last_error()` for error details (thread-local)
 *
 * ## Thread Safety
 *
 * - The library is thread-safe
 * - Each thread has its own last error state
 */

#ifndef RLM_CORE_H
#define RLM_CORE_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ============================================================================
 * Opaque Types
 * ============================================================================ */

typedef struct RlmSessionContext RlmSessionContext;
typedef struct RlmMessage RlmMessage;
typedef struct RlmToolOutput RlmToolOutput;
typedef struct RlmMemoryStore RlmMemoryStore;
typedef struct RlmNode RlmNode;
typedef struct RlmHyperEdge RlmHyperEdge;
typedef struct RlmTrajectoryEvent RlmTrajectoryEvent;
typedef struct RlmPatternClassifier RlmPatternClassifier;
typedef struct RlmActivationDecision RlmActivationDecision;
typedef struct RlmReplHandle RlmReplHandle;
typedef struct RlmReplPool RlmReplPool;

/* ============================================================================
 * Enumerations
 * ============================================================================ */

/** Role of a message participant */
typedef enum {
    RLM_ROLE_SYSTEM = 0,
    RLM_ROLE_USER = 1,
    RLM_ROLE_ASSISTANT = 2,
    RLM_ROLE_TOOL = 3
} RlmRole;

/** Type of a memory node */
typedef enum {
    RLM_NODE_TYPE_ENTITY = 0,
    RLM_NODE_TYPE_FACT = 1,
    RLM_NODE_TYPE_EXPERIENCE = 2,
    RLM_NODE_TYPE_DECISION = 3,
    RLM_NODE_TYPE_SNIPPET = 4
} RlmNodeType;

/** Memory tier (lifecycle stage) */
typedef enum {
    RLM_TIER_TASK = 0,
    RLM_TIER_SESSION = 1,
    RLM_TIER_LONG_TERM = 2,
    RLM_TIER_ARCHIVE = 3
} RlmTier;

/** Type of trajectory event */
typedef enum {
    RLM_EVENT_RLM_START = 0,
    RLM_EVENT_ANALYZE = 1,
    RLM_EVENT_REPL_EXEC = 2,
    RLM_EVENT_REPL_RESULT = 3,
    RLM_EVENT_REASON = 4,
    RLM_EVENT_RECURSE_START = 5,
    RLM_EVENT_RECURSE_END = 6,
    RLM_EVENT_FINAL = 7,
    RLM_EVENT_ERROR = 8,
    RLM_EVENT_TOOL_USE = 9,
    RLM_EVENT_COST_REPORT = 10,
    RLM_EVENT_VERIFY_START = 11,
    RLM_EVENT_CLAIM_EXTRACTED = 12,
    RLM_EVENT_EVIDENCE_CHECKED = 13,
    RLM_EVENT_BUDGET_COMPUTED = 14,
    RLM_EVENT_HALLUCINATION_FLAG = 15,
    RLM_EVENT_VERIFY_COMPLETE = 16,
    RLM_EVENT_MEMORY = 17,
    RLM_EVENT_EXTERNALIZE = 18,
    RLM_EVENT_DECOMPOSE = 19,
    RLM_EVENT_SYNTHESIZE = 20
} RlmTrajectoryEventType;

/* ============================================================================
 * Library Functions
 * ============================================================================ */

/**
 * Get the library version string.
 * @return Version string (must be freed with rlm_string_free)
 */
char* rlm_version(void);

/**
 * Free a string allocated by the library.
 * @param s String to free (may be NULL)
 */
void rlm_string_free(char* s);

/**
 * Initialize the library.
 * @return 0 on success, -1 on failure
 */
int rlm_init(void);

/**
 * Shutdown the library.
 */
void rlm_shutdown(void);

/* ============================================================================
 * Error Handling
 * ============================================================================ */

/**
 * Get the last error message for the current thread.
 * @return Error string (valid until next rlm_* call on same thread), or NULL
 */
const char* rlm_last_error(void);

/**
 * Check if there is a pending error.
 * @return 1 if error, 0 otherwise
 */
int rlm_has_error(void);

/**
 * Clear the last error for the current thread.
 */
void rlm_clear_error(void);

/* ============================================================================
 * SessionContext
 * ============================================================================ */

RlmSessionContext* rlm_session_context_new(void);
void rlm_session_context_free(RlmSessionContext* ctx);
int rlm_session_context_add_message(RlmSessionContext* ctx, const RlmMessage* msg);
int rlm_session_context_add_user_message(RlmSessionContext* ctx, const char* content);
int rlm_session_context_add_assistant_message(RlmSessionContext* ctx, const char* content);
int rlm_session_context_cache_file(RlmSessionContext* ctx, const char* path, const char* content);
char* rlm_session_context_get_file(const RlmSessionContext* ctx, const char* path);
int rlm_session_context_add_tool_output(RlmSessionContext* ctx, const RlmToolOutput* output);
int64_t rlm_session_context_message_count(const RlmSessionContext* ctx);
int64_t rlm_session_context_file_count(const RlmSessionContext* ctx);
int64_t rlm_session_context_tool_output_count(const RlmSessionContext* ctx);
int rlm_session_context_spans_multiple_directories(const RlmSessionContext* ctx);
int64_t rlm_session_context_total_message_tokens(const RlmSessionContext* ctx);
char* rlm_session_context_to_json(const RlmSessionContext* ctx);
RlmSessionContext* rlm_session_context_from_json(const char* json);

/* ============================================================================
 * Message
 * ============================================================================ */

RlmMessage* rlm_message_new(RlmRole role, const char* content);
RlmMessage* rlm_message_user(const char* content);
RlmMessage* rlm_message_assistant(const char* content);
RlmMessage* rlm_message_system(const char* content);
RlmMessage* rlm_message_tool(const char* content);
void rlm_message_free(RlmMessage* msg);
RlmRole rlm_message_role(const RlmMessage* msg);
char* rlm_message_content(const RlmMessage* msg);
char* rlm_message_timestamp(const RlmMessage* msg);

/* ============================================================================
 * ToolOutput
 * ============================================================================ */

RlmToolOutput* rlm_tool_output_new(const char* tool_name, const char* content);
RlmToolOutput* rlm_tool_output_new_with_exit_code(const char* tool_name, const char* content, int exit_code);
void rlm_tool_output_free(RlmToolOutput* output);
char* rlm_tool_output_tool_name(const RlmToolOutput* output);
char* rlm_tool_output_content(const RlmToolOutput* output);
int rlm_tool_output_exit_code(const RlmToolOutput* output);
int rlm_tool_output_has_exit_code(const RlmToolOutput* output);
int rlm_tool_output_is_success(const RlmToolOutput* output);

/* ============================================================================
 * PatternClassifier
 * ============================================================================ */

RlmPatternClassifier* rlm_pattern_classifier_new(void);
RlmPatternClassifier* rlm_pattern_classifier_with_threshold(int threshold);
void rlm_pattern_classifier_free(RlmPatternClassifier* classifier);
RlmActivationDecision* rlm_pattern_classifier_should_activate(
    const RlmPatternClassifier* classifier,
    const char* query,
    const RlmSessionContext* ctx);

/* ============================================================================
 * ActivationDecision
 * ============================================================================ */

void rlm_activation_decision_free(RlmActivationDecision* decision);
int rlm_activation_decision_should_activate(const RlmActivationDecision* decision);
char* rlm_activation_decision_reason(const RlmActivationDecision* decision);
int rlm_activation_decision_score(const RlmActivationDecision* decision);

/* ============================================================================
 * MemoryStore
 * ============================================================================ */

RlmMemoryStore* rlm_memory_store_in_memory(void);
RlmMemoryStore* rlm_memory_store_open(const char* path);
void rlm_memory_store_free(RlmMemoryStore* store);
int rlm_memory_store_add_node(const RlmMemoryStore* store, const RlmNode* node);
RlmNode* rlm_memory_store_get_node(const RlmMemoryStore* store, const char* node_id);
int rlm_memory_store_update_node(const RlmMemoryStore* store, const RlmNode* node);
int rlm_memory_store_delete_node(const RlmMemoryStore* store, const char* node_id);
char* rlm_memory_store_query_by_type(const RlmMemoryStore* store, RlmNodeType node_type, int64_t limit);
char* rlm_memory_store_query_by_tier(const RlmMemoryStore* store, RlmTier tier, int64_t limit);
char* rlm_memory_store_search_content(const RlmMemoryStore* store, const char* query, int64_t limit);
char* rlm_memory_store_promote(const RlmMemoryStore* store, const char* node_ids_json, const char* reason);
char* rlm_memory_store_decay(const RlmMemoryStore* store, double factor, double min_confidence);
char* rlm_memory_store_stats(const RlmMemoryStore* store);
int rlm_memory_store_add_edge(const RlmMemoryStore* store, const RlmHyperEdge* edge);
char* rlm_memory_store_get_edges_for_node(const RlmMemoryStore* store, const char* node_id);

/* ============================================================================
 * Node
 * ============================================================================ */

RlmNode* rlm_node_new(RlmNodeType node_type, const char* content);
RlmNode* rlm_node_new_full(RlmNodeType node_type, const char* content, RlmTier tier, double confidence);
void rlm_node_free(RlmNode* node);
char* rlm_node_id(const RlmNode* node);
RlmNodeType rlm_node_type(const RlmNode* node);
char* rlm_node_content(const RlmNode* node);
RlmTier rlm_node_tier(const RlmNode* node);
double rlm_node_confidence(const RlmNode* node);
char* rlm_node_subtype(const RlmNode* node);
int rlm_node_set_subtype(RlmNode* node, const char* subtype);
int rlm_node_set_tier(RlmNode* node, RlmTier tier);
int rlm_node_set_confidence(RlmNode* node, double confidence);
int rlm_node_record_access(RlmNode* node);
uint64_t rlm_node_access_count(const RlmNode* node);
int rlm_node_is_decayed(const RlmNode* node, double min_confidence);
int64_t rlm_node_age_hours(const RlmNode* node);
char* rlm_node_to_json(const RlmNode* node);
RlmNode* rlm_node_from_json(const char* json);

/* ============================================================================
 * HyperEdge
 * ============================================================================ */

RlmHyperEdge* rlm_hyperedge_new(const char* edge_type);
RlmHyperEdge* rlm_hyperedge_binary(const char* edge_type, const char* subject_id, const char* object_id, const char* label);
void rlm_hyperedge_free(RlmHyperEdge* edge);
char* rlm_hyperedge_id(const RlmHyperEdge* edge);
char* rlm_hyperedge_type(const RlmHyperEdge* edge);
char* rlm_hyperedge_label(const RlmHyperEdge* edge);
double rlm_hyperedge_weight(const RlmHyperEdge* edge);
char* rlm_hyperedge_node_ids(const RlmHyperEdge* edge);
int rlm_hyperedge_contains(const RlmHyperEdge* edge, const char* node_id);

/* ============================================================================
 * TrajectoryEvent
 * ============================================================================ */

RlmTrajectoryEvent* rlm_trajectory_event_new(RlmTrajectoryEventType event_type, uint32_t depth, const char* content);
RlmTrajectoryEvent* rlm_trajectory_event_rlm_start(const char* query);
RlmTrajectoryEvent* rlm_trajectory_event_analyze(uint32_t depth, const char* analysis);
RlmTrajectoryEvent* rlm_trajectory_event_repl_exec(uint32_t depth, const char* code);
RlmTrajectoryEvent* rlm_trajectory_event_repl_result(uint32_t depth, const char* result, int success);
RlmTrajectoryEvent* rlm_trajectory_event_reason(uint32_t depth, const char* reasoning);
RlmTrajectoryEvent* rlm_trajectory_event_recurse_start(uint32_t depth, const char* query);
RlmTrajectoryEvent* rlm_trajectory_event_recurse_end(uint32_t depth, const char* result);
RlmTrajectoryEvent* rlm_trajectory_event_final_answer(uint32_t depth, const char* answer);
RlmTrajectoryEvent* rlm_trajectory_event_error(uint32_t depth, const char* error);
void rlm_trajectory_event_free(RlmTrajectoryEvent* event);
RlmTrajectoryEventType rlm_trajectory_event_type(const RlmTrajectoryEvent* event);
uint32_t rlm_trajectory_event_depth(const RlmTrajectoryEvent* event);
char* rlm_trajectory_event_content(const RlmTrajectoryEvent* event);
char* rlm_trajectory_event_timestamp(const RlmTrajectoryEvent* event);
char* rlm_trajectory_event_log_line(const RlmTrajectoryEvent* event);
int rlm_trajectory_event_is_error(const RlmTrajectoryEvent* event);
int rlm_trajectory_event_is_final(const RlmTrajectoryEvent* event);
char* rlm_trajectory_event_to_json(const RlmTrajectoryEvent* event);
RlmTrajectoryEvent* rlm_trajectory_event_from_json(const char* json);
char* rlm_trajectory_event_type_name(RlmTrajectoryEventType event_type);

/* ============================================================================
 * REPL Configuration
 * ============================================================================ */

/**
 * Get default REPL configuration as JSON.
 * @return JSON string with configuration (must be freed with rlm_string_free)
 */
char* rlm_repl_config_default(void);

/* ============================================================================
 * ReplHandle - Single REPL subprocess
 * ============================================================================ */

/**
 * Spawn a new REPL subprocess with default configuration.
 * @return Handle pointer (must be freed with rlm_repl_handle_free), or NULL on error
 */
RlmReplHandle* rlm_repl_handle_spawn_default(void);

/**
 * Spawn a new REPL subprocess with custom configuration.
 * @param config_json JSON string with configuration options
 * @return Handle pointer (must be freed with rlm_repl_handle_free), or NULL on error
 */
RlmReplHandle* rlm_repl_handle_spawn(const char* config_json);

/**
 * Free a REPL handle.
 * @param handle Handle to free (may be NULL)
 */
void rlm_repl_handle_free(RlmReplHandle* handle);

/**
 * Execute Python code in the REPL.
 * @param handle REPL handle
 * @param code Python code to execute
 * @return JSON string with execution result (must be freed with rlm_string_free), or NULL on error
 */
char* rlm_repl_handle_execute(RlmReplHandle* handle, const char* code);

/**
 * Get a variable from the REPL namespace.
 * @param handle REPL handle
 * @param name Variable name
 * @return JSON string with variable value (must be freed with rlm_string_free), or NULL on error
 */
char* rlm_repl_handle_get_variable(RlmReplHandle* handle, const char* name);

/**
 * Set a variable in the REPL namespace.
 * @param handle REPL handle
 * @param name Variable name
 * @param value_json JSON string with variable value
 * @return 0 on success, -1 on failure
 */
int rlm_repl_handle_set_variable(RlmReplHandle* handle, const char* name, const char* value_json);

/**
 * Resolve a deferred operation.
 * @param handle REPL handle
 * @param operation_id Operation ID
 * @param result_json JSON string with result value
 * @return 0 on success, -1 on failure
 */
int rlm_repl_handle_resolve_operation(RlmReplHandle* handle, const char* operation_id, const char* result_json);

/**
 * List all variables in the REPL namespace.
 * @param handle REPL handle
 * @return JSON object mapping names to types (must be freed with rlm_string_free), or NULL on error
 */
char* rlm_repl_handle_list_variables(RlmReplHandle* handle);

/**
 * Get REPL status.
 * @param handle REPL handle
 * @return JSON string with status info (must be freed with rlm_string_free), or NULL on error
 */
char* rlm_repl_handle_status(RlmReplHandle* handle);

/**
 * Reset the REPL state.
 * @param handle REPL handle
 * @return 0 on success, -1 on failure
 */
int rlm_repl_handle_reset(RlmReplHandle* handle);

/**
 * Shutdown the REPL subprocess.
 * @param handle REPL handle
 * @return 0 on success, -1 on failure
 */
int rlm_repl_handle_shutdown(RlmReplHandle* handle);

/**
 * Check if the REPL subprocess is still running.
 * @param handle REPL handle
 * @return 1 if alive, 0 if not, -1 on error
 */
int rlm_repl_handle_is_alive(RlmReplHandle* handle);

/* ============================================================================
 * ReplPool - Pool of REPL subprocesses
 * ============================================================================ */

/**
 * Create a new REPL pool with default configuration.
 * @param max_size Maximum number of REPL handles to keep in the pool
 * @return Pool pointer (must be freed with rlm_repl_pool_free)
 */
RlmReplPool* rlm_repl_pool_new_default(size_t max_size);

/**
 * Create a new REPL pool with custom configuration.
 * @param config_json JSON string with configuration options
 * @param max_size Maximum number of REPL handles to keep in the pool
 * @return Pool pointer (must be freed with rlm_repl_pool_free), or NULL on error
 */
RlmReplPool* rlm_repl_pool_new(const char* config_json, size_t max_size);

/**
 * Free a REPL pool.
 * @param pool Pool to free (may be NULL)
 */
void rlm_repl_pool_free(RlmReplPool* pool);

/**
 * Acquire a REPL handle from the pool.
 * @param pool REPL pool
 * @return Handle pointer (must be freed with rlm_repl_handle_free or returned with rlm_repl_pool_release), or NULL on error
 */
RlmReplHandle* rlm_repl_pool_acquire(const RlmReplPool* pool);

/**
 * Release a REPL handle back to the pool.
 * @param pool REPL pool
 * @param handle Handle to release (will be invalid after this call)
 */
void rlm_repl_pool_release(const RlmReplPool* pool, RlmReplHandle* handle);

#ifdef __cplusplus
}
#endif

#endif /* RLM_CORE_H */
