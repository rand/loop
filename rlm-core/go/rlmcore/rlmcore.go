// Package rlmcore provides Go bindings for the rlm-core Rust library.
//
// This package enables Go applications (particularly Bubble Tea TUIs) to use
// the RLM orchestration capabilities provided by rlm-core.
//
// # Memory Management
//
// Objects created by New* functions must be freed with their corresponding Free() method.
// The library uses explicit memory management to avoid GC overhead for performance-critical
// paths.
//
// # Thread Safety
//
// All types in this package are safe for concurrent use from multiple goroutines.
//
// # Example
//
//	ctx := rlmcore.NewSessionContext()
//	defer ctx.Free()
//
//	ctx.AddUserMessage("Analyze the auth system")
//
//	classifier := rlmcore.NewPatternClassifier()
//	defer classifier.Free()
//
//	decision := classifier.ShouldActivate("Analyze the auth system", ctx)
//	defer decision.Free()
//
//	if decision.ShouldActivate() {
//	    fmt.Println("RLM activated:", decision.Reason())
//	}
package rlmcore

/*
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -lrlm_core
#cgo darwin LDFLAGS: -framework Security -framework CoreFoundation

#include <stdlib.h>
#include <stdint.h>

// Library functions
char* rlm_version(void);
void rlm_string_free(char* s);
int rlm_init(void);
void rlm_shutdown(void);

// Error handling
const char* rlm_last_error(void);
int rlm_has_error(void);
void rlm_clear_error(void);

// Role enum
typedef enum {
    RLM_ROLE_SYSTEM = 0,
    RLM_ROLE_USER = 1,
    RLM_ROLE_ASSISTANT = 2,
    RLM_ROLE_TOOL = 3
} RlmRole;

// NodeType enum
typedef enum {
    RLM_NODE_TYPE_ENTITY = 0,
    RLM_NODE_TYPE_FACT = 1,
    RLM_NODE_TYPE_EXPERIENCE = 2,
    RLM_NODE_TYPE_DECISION = 3,
    RLM_NODE_TYPE_SNIPPET = 4
} RlmNodeType;

// Tier enum
typedef enum {
    RLM_TIER_TASK = 0,
    RLM_TIER_SESSION = 1,
    RLM_TIER_LONG_TERM = 2,
    RLM_TIER_ARCHIVE = 3
} RlmTier;

// TrajectoryEventType enum
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

// Opaque types
typedef struct RlmSessionContext RlmSessionContext;
typedef struct RlmMessage RlmMessage;
typedef struct RlmToolOutput RlmToolOutput;
typedef struct RlmMemoryStore RlmMemoryStore;
typedef struct RlmNode RlmNode;
typedef struct RlmHyperEdge RlmHyperEdge;
typedef struct RlmTrajectoryEvent RlmTrajectoryEvent;
typedef struct RlmPatternClassifier RlmPatternClassifier;
typedef struct RlmActivationDecision RlmActivationDecision;

// SessionContext functions
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

// Message functions
RlmMessage* rlm_message_new(RlmRole role, const char* content);
RlmMessage* rlm_message_user(const char* content);
RlmMessage* rlm_message_assistant(const char* content);
RlmMessage* rlm_message_system(const char* content);
RlmMessage* rlm_message_tool(const char* content);
void rlm_message_free(RlmMessage* msg);
RlmRole rlm_message_role(const RlmMessage* msg);
char* rlm_message_content(const RlmMessage* msg);
char* rlm_message_timestamp(const RlmMessage* msg);

// ToolOutput functions
RlmToolOutput* rlm_tool_output_new(const char* tool_name, const char* content);
RlmToolOutput* rlm_tool_output_new_with_exit_code(const char* tool_name, const char* content, int exit_code);
void rlm_tool_output_free(RlmToolOutput* output);
char* rlm_tool_output_tool_name(const RlmToolOutput* output);
char* rlm_tool_output_content(const RlmToolOutput* output);
int rlm_tool_output_exit_code(const RlmToolOutput* output);
int rlm_tool_output_has_exit_code(const RlmToolOutput* output);
int rlm_tool_output_is_success(const RlmToolOutput* output);

// PatternClassifier functions
RlmPatternClassifier* rlm_pattern_classifier_new(void);
RlmPatternClassifier* rlm_pattern_classifier_with_threshold(int threshold);
void rlm_pattern_classifier_free(RlmPatternClassifier* classifier);
RlmActivationDecision* rlm_pattern_classifier_should_activate(const RlmPatternClassifier* classifier, const char* query, const RlmSessionContext* ctx);

// ActivationDecision functions
void rlm_activation_decision_free(RlmActivationDecision* decision);
int rlm_activation_decision_should_activate(const RlmActivationDecision* decision);
char* rlm_activation_decision_reason(const RlmActivationDecision* decision);
int rlm_activation_decision_score(const RlmActivationDecision* decision);

// MemoryStore functions
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

// Node functions
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

// HyperEdge functions
RlmHyperEdge* rlm_hyperedge_new(const char* edge_type);
RlmHyperEdge* rlm_hyperedge_binary(const char* edge_type, const char* subject_id, const char* object_id, const char* label);
void rlm_hyperedge_free(RlmHyperEdge* edge);
char* rlm_hyperedge_id(const RlmHyperEdge* edge);
char* rlm_hyperedge_type(const RlmHyperEdge* edge);
char* rlm_hyperedge_label(const RlmHyperEdge* edge);
double rlm_hyperedge_weight(const RlmHyperEdge* edge);
char* rlm_hyperedge_node_ids(const RlmHyperEdge* edge);
int rlm_hyperedge_contains(const RlmHyperEdge* edge, const char* node_id);

// TrajectoryEvent functions
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
*/
import "C"

import (
	"errors"
	"runtime"
	"sync"
	"unsafe"
)

var initOnce sync.Once

// Init initializes the rlm-core library. It is safe to call multiple times.
func Init() error {
	var err error
	initOnce.Do(func() {
		if C.rlm_init() != 0 {
			err = lastError()
		}
	})
	return err
}

// Shutdown cleans up library resources. Should be called before program exit.
func Shutdown() {
	C.rlm_shutdown()
}

// Version returns the library version string.
func Version() string {
	cstr := C.rlm_version()
	if cstr == nil {
		return ""
	}
	defer C.rlm_string_free(cstr)
	return C.GoString(cstr)
}

// lastError returns the last error from the library, or nil if none.
func lastError() error {
	if C.rlm_has_error() == 0 {
		return nil
	}
	cstr := C.rlm_last_error()
	if cstr == nil {
		return errors.New("unknown error")
	}
	return errors.New(C.GoString(cstr))
}

// clearError clears the last error.
func clearError() {
	C.rlm_clear_error()
}

// goString converts a C string to Go string and frees the C string.
func goString(cstr *C.char) string {
	if cstr == nil {
		return ""
	}
	defer C.rlm_string_free(cstr)
	return C.GoString(cstr)
}

// cString converts a Go string to C string. Caller must free with C.free().
func cString(s string) *C.char {
	return C.CString(s)
}

// Role represents a message participant role.
type Role int

const (
	RoleSystem    Role = C.RLM_ROLE_SYSTEM
	RoleUser      Role = C.RLM_ROLE_USER
	RoleAssistant Role = C.RLM_ROLE_ASSISTANT
	RoleTool      Role = C.RLM_ROLE_TOOL
)

func (r Role) String() string {
	switch r {
	case RoleSystem:
		return "system"
	case RoleUser:
		return "user"
	case RoleAssistant:
		return "assistant"
	case RoleTool:
		return "tool"
	default:
		return "unknown"
	}
}

// NodeType represents the type of a memory node.
type NodeType int

const (
	NodeTypeEntity     NodeType = C.RLM_NODE_TYPE_ENTITY
	NodeTypeFact       NodeType = C.RLM_NODE_TYPE_FACT
	NodeTypeExperience NodeType = C.RLM_NODE_TYPE_EXPERIENCE
	NodeTypeDecision   NodeType = C.RLM_NODE_TYPE_DECISION
	NodeTypeSnippet    NodeType = C.RLM_NODE_TYPE_SNIPPET
)

func (t NodeType) String() string {
	switch t {
	case NodeTypeEntity:
		return "entity"
	case NodeTypeFact:
		return "fact"
	case NodeTypeExperience:
		return "experience"
	case NodeTypeDecision:
		return "decision"
	case NodeTypeSnippet:
		return "snippet"
	default:
		return "unknown"
	}
}

// Tier represents a memory tier.
type Tier int

const (
	TierTask     Tier = C.RLM_TIER_TASK
	TierSession  Tier = C.RLM_TIER_SESSION
	TierLongTerm Tier = C.RLM_TIER_LONG_TERM
	TierArchive  Tier = C.RLM_TIER_ARCHIVE
)

func (t Tier) String() string {
	switch t {
	case TierTask:
		return "task"
	case TierSession:
		return "session"
	case TierLongTerm:
		return "longterm"
	case TierArchive:
		return "archive"
	default:
		return "unknown"
	}
}

// TrajectoryEventType represents the type of a trajectory event.
type TrajectoryEventType int

const (
	EventRLMStart         TrajectoryEventType = C.RLM_EVENT_RLM_START
	EventAnalyze          TrajectoryEventType = C.RLM_EVENT_ANALYZE
	EventREPLExec         TrajectoryEventType = C.RLM_EVENT_REPL_EXEC
	EventREPLResult       TrajectoryEventType = C.RLM_EVENT_REPL_RESULT
	EventReason           TrajectoryEventType = C.RLM_EVENT_REASON
	EventRecurseStart     TrajectoryEventType = C.RLM_EVENT_RECURSE_START
	EventRecurseEnd       TrajectoryEventType = C.RLM_EVENT_RECURSE_END
	EventFinal            TrajectoryEventType = C.RLM_EVENT_FINAL
	EventError            TrajectoryEventType = C.RLM_EVENT_ERROR
	EventToolUse          TrajectoryEventType = C.RLM_EVENT_TOOL_USE
	EventCostReport       TrajectoryEventType = C.RLM_EVENT_COST_REPORT
	EventVerifyStart      TrajectoryEventType = C.RLM_EVENT_VERIFY_START
	EventClaimExtracted   TrajectoryEventType = C.RLM_EVENT_CLAIM_EXTRACTED
	EventEvidenceChecked  TrajectoryEventType = C.RLM_EVENT_EVIDENCE_CHECKED
	EventBudgetComputed   TrajectoryEventType = C.RLM_EVENT_BUDGET_COMPUTED
	EventHallucinationFlag TrajectoryEventType = C.RLM_EVENT_HALLUCINATION_FLAG
	EventVerifyComplete   TrajectoryEventType = C.RLM_EVENT_VERIFY_COMPLETE
	EventMemory           TrajectoryEventType = C.RLM_EVENT_MEMORY
	EventExternalize      TrajectoryEventType = C.RLM_EVENT_EXTERNALIZE
	EventDecompose        TrajectoryEventType = C.RLM_EVENT_DECOMPOSE
	EventSynthesize       TrajectoryEventType = C.RLM_EVENT_SYNTHESIZE
)

func (t TrajectoryEventType) String() string {
	cstr := C.rlm_trajectory_event_type_name(C.RlmTrajectoryEventType(t))
	return goString(cstr)
}

// MemoryStats contains statistics about the memory store.
type MemoryStats struct {
	TotalNodes int64 `json:"total_nodes"`
	TotalEdges int64 `json:"total_edges"`
}

// EdgeMember represents a member of a hyperedge.
type EdgeMember struct {
	NodeID   string `json:"node_id"`
	Role     string `json:"role"`
	Position int    `json:"position"`
}

// EdgeData represents hyperedge data returned from queries.
type EdgeData struct {
	ID       string       `json:"id"`
	EdgeType string       `json:"edge_type"`
	Label    *string      `json:"label"`
	Weight   float64      `json:"weight"`
	Members  []EdgeMember `json:"members"`
}

// SessionContext holds conversation state for RLM orchestration.
type SessionContext struct {
	ptr *C.RlmSessionContext
}

// NewSessionContext creates a new empty session context.
func NewSessionContext() *SessionContext {
	ctx := &SessionContext{ptr: C.rlm_session_context_new()}
	runtime.SetFinalizer(ctx, (*SessionContext).Free)
	return ctx
}

// Free releases the session context resources.
func (c *SessionContext) Free() {
	if c.ptr != nil {
		C.rlm_session_context_free(c.ptr)
		c.ptr = nil
	}
}

// AddMessage adds a message to the context.
func (c *SessionContext) AddMessage(msg *Message) error {
	if C.rlm_session_context_add_message(c.ptr, msg.ptr) != 0 {
		return lastError()
	}
	return nil
}

// AddUserMessage adds a user message to the context.
func (c *SessionContext) AddUserMessage(content string) error {
	cs := cString(content)
	defer C.free(unsafe.Pointer(cs))
	if C.rlm_session_context_add_user_message(c.ptr, cs) != 0 {
		return lastError()
	}
	return nil
}

// AddAssistantMessage adds an assistant message to the context.
func (c *SessionContext) AddAssistantMessage(content string) error {
	cs := cString(content)
	defer C.free(unsafe.Pointer(cs))
	if C.rlm_session_context_add_assistant_message(c.ptr, cs) != 0 {
		return lastError()
	}
	return nil
}

// CacheFile caches a file's contents in the context.
func (c *SessionContext) CacheFile(path, content string) error {
	cpath := cString(path)
	defer C.free(unsafe.Pointer(cpath))
	ccontent := cString(content)
	defer C.free(unsafe.Pointer(ccontent))
	if C.rlm_session_context_cache_file(c.ptr, cpath, ccontent) != 0 {
		return lastError()
	}
	return nil
}

// GetFile retrieves a cached file's contents.
func (c *SessionContext) GetFile(path string) (string, bool) {
	cpath := cString(path)
	defer C.free(unsafe.Pointer(cpath))
	cstr := C.rlm_session_context_get_file(c.ptr, cpath)
	if cstr == nil {
		return "", false
	}
	return goString(cstr), true
}

// AddToolOutput adds a tool output to the context.
func (c *SessionContext) AddToolOutput(output *ToolOutput) error {
	if C.rlm_session_context_add_tool_output(c.ptr, output.ptr) != 0 {
		return lastError()
	}
	return nil
}

// MessageCount returns the number of messages in the context.
func (c *SessionContext) MessageCount() int64 {
	return int64(C.rlm_session_context_message_count(c.ptr))
}

// FileCount returns the number of cached files in the context.
func (c *SessionContext) FileCount() int64 {
	return int64(C.rlm_session_context_file_count(c.ptr))
}

// ToolOutputCount returns the number of tool outputs in the context.
func (c *SessionContext) ToolOutputCount() int64 {
	return int64(C.rlm_session_context_tool_output_count(c.ptr))
}

// SpansMultipleDirectories returns true if cached files span multiple directories.
func (c *SessionContext) SpansMultipleDirectories() bool {
	return C.rlm_session_context_spans_multiple_directories(c.ptr) != 0
}

// TotalMessageTokens returns the approximate token count of all messages.
func (c *SessionContext) TotalMessageTokens() int64 {
	return int64(C.rlm_session_context_total_message_tokens(c.ptr))
}

// ToJSON serializes the context to JSON.
func (c *SessionContext) ToJSON() (string, error) {
	cstr := C.rlm_session_context_to_json(c.ptr)
	if cstr == nil {
		return "", lastError()
	}
	return goString(cstr), nil
}

// SessionContextFromJSON deserializes a context from JSON.
func SessionContextFromJSON(jsonStr string) (*SessionContext, error) {
	cs := cString(jsonStr)
	defer C.free(unsafe.Pointer(cs))
	ptr := C.rlm_session_context_from_json(cs)
	if ptr == nil {
		return nil, lastError()
	}
	ctx := &SessionContext{ptr: ptr}
	runtime.SetFinalizer(ctx, (*SessionContext).Free)
	return ctx, nil
}

// Message represents a conversation message.
type Message struct {
	ptr *C.RlmMessage
}

// NewMessage creates a new message with the given role and content.
func NewMessage(role Role, content string) *Message {
	cs := cString(content)
	defer C.free(unsafe.Pointer(cs))
	msg := &Message{ptr: C.rlm_message_new(C.RlmRole(role), cs)}
	runtime.SetFinalizer(msg, (*Message).Free)
	return msg
}

// NewUserMessage creates a new user message.
func NewUserMessage(content string) *Message {
	cs := cString(content)
	defer C.free(unsafe.Pointer(cs))
	msg := &Message{ptr: C.rlm_message_user(cs)}
	runtime.SetFinalizer(msg, (*Message).Free)
	return msg
}

// NewAssistantMessage creates a new assistant message.
func NewAssistantMessage(content string) *Message {
	cs := cString(content)
	defer C.free(unsafe.Pointer(cs))
	msg := &Message{ptr: C.rlm_message_assistant(cs)}
	runtime.SetFinalizer(msg, (*Message).Free)
	return msg
}

// NewSystemMessage creates a new system message.
func NewSystemMessage(content string) *Message {
	cs := cString(content)
	defer C.free(unsafe.Pointer(cs))
	msg := &Message{ptr: C.rlm_message_system(cs)}
	runtime.SetFinalizer(msg, (*Message).Free)
	return msg
}

// NewToolMessage creates a new tool message.
func NewToolMessage(content string) *Message {
	cs := cString(content)
	defer C.free(unsafe.Pointer(cs))
	msg := &Message{ptr: C.rlm_message_tool(cs)}
	runtime.SetFinalizer(msg, (*Message).Free)
	return msg
}

// Free releases the message resources.
func (m *Message) Free() {
	if m.ptr != nil {
		C.rlm_message_free(m.ptr)
		m.ptr = nil
	}
}

// Role returns the message role.
func (m *Message) Role() Role {
	return Role(C.rlm_message_role(m.ptr))
}

// Content returns the message content.
func (m *Message) Content() string {
	return goString(C.rlm_message_content(m.ptr))
}

// Timestamp returns the message timestamp in RFC3339 format.
func (m *Message) Timestamp() string {
	cstr := C.rlm_message_timestamp(m.ptr)
	if cstr == nil {
		return ""
	}
	return goString(cstr)
}

// ToolOutput represents the output of a tool execution.
type ToolOutput struct {
	ptr *C.RlmToolOutput
}

// NewToolOutput creates a new tool output.
func NewToolOutput(toolName, content string) *ToolOutput {
	cname := cString(toolName)
	defer C.free(unsafe.Pointer(cname))
	ccontent := cString(content)
	defer C.free(unsafe.Pointer(ccontent))
	output := &ToolOutput{ptr: C.rlm_tool_output_new(cname, ccontent)}
	runtime.SetFinalizer(output, (*ToolOutput).Free)
	return output
}

// NewToolOutputWithExitCode creates a new tool output with an exit code.
func NewToolOutputWithExitCode(toolName, content string, exitCode int) *ToolOutput {
	cname := cString(toolName)
	defer C.free(unsafe.Pointer(cname))
	ccontent := cString(content)
	defer C.free(unsafe.Pointer(ccontent))
	output := &ToolOutput{ptr: C.rlm_tool_output_new_with_exit_code(cname, ccontent, C.int(exitCode))}
	runtime.SetFinalizer(output, (*ToolOutput).Free)
	return output
}

// Free releases the tool output resources.
func (o *ToolOutput) Free() {
	if o.ptr != nil {
		C.rlm_tool_output_free(o.ptr)
		o.ptr = nil
	}
}

// ToolName returns the tool name.
func (o *ToolOutput) ToolName() string {
	return goString(C.rlm_tool_output_tool_name(o.ptr))
}

// Content returns the output content.
func (o *ToolOutput) Content() string {
	return goString(C.rlm_tool_output_content(o.ptr))
}

// ExitCode returns the exit code, or -1 if not set.
func (o *ToolOutput) ExitCode() int {
	return int(C.rlm_tool_output_exit_code(o.ptr))
}

// HasExitCode returns true if an exit code is set.
func (o *ToolOutput) HasExitCode() bool {
	return C.rlm_tool_output_has_exit_code(o.ptr) != 0
}

// IsSuccess returns true if the tool execution succeeded.
func (o *ToolOutput) IsSuccess() bool {
	return C.rlm_tool_output_is_success(o.ptr) != 0
}

// PatternClassifier classifies task complexity.
type PatternClassifier struct {
	ptr *C.RlmPatternClassifier
}

// NewPatternClassifier creates a new pattern classifier with default settings.
func NewPatternClassifier() *PatternClassifier {
	c := &PatternClassifier{ptr: C.rlm_pattern_classifier_new()}
	runtime.SetFinalizer(c, (*PatternClassifier).Free)
	return c
}

// NewPatternClassifierWithThreshold creates a classifier with a custom threshold.
func NewPatternClassifierWithThreshold(threshold int) *PatternClassifier {
	c := &PatternClassifier{ptr: C.rlm_pattern_classifier_with_threshold(C.int(threshold))}
	runtime.SetFinalizer(c, (*PatternClassifier).Free)
	return c
}

// Free releases the classifier resources.
func (c *PatternClassifier) Free() {
	if c.ptr != nil {
		C.rlm_pattern_classifier_free(c.ptr)
		c.ptr = nil
	}
}

// ShouldActivate checks if RLM should activate for a query.
func (c *PatternClassifier) ShouldActivate(query string, ctx *SessionContext) *ActivationDecision {
	cquery := cString(query)
	defer C.free(unsafe.Pointer(cquery))
	ptr := C.rlm_pattern_classifier_should_activate(c.ptr, cquery, ctx.ptr)
	if ptr == nil {
		return nil
	}
	d := &ActivationDecision{ptr: ptr}
	runtime.SetFinalizer(d, (*ActivationDecision).Free)
	return d
}

// ActivationDecision contains the result of a complexity analysis.
type ActivationDecision struct {
	ptr *C.RlmActivationDecision
}

// Free releases the decision resources.
func (d *ActivationDecision) Free() {
	if d.ptr != nil {
		C.rlm_activation_decision_free(d.ptr)
		d.ptr = nil
	}
}

// ShouldActivate returns true if RLM should be activated.
func (d *ActivationDecision) ShouldActivate() bool {
	return C.rlm_activation_decision_should_activate(d.ptr) != 0
}

// Reason returns the human-readable reason for the decision.
func (d *ActivationDecision) Reason() string {
	return goString(C.rlm_activation_decision_reason(d.ptr))
}

// Score returns the complexity score.
func (d *ActivationDecision) Score() int {
	return int(C.rlm_activation_decision_score(d.ptr))
}
