package rlmcore

/*
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -lrlm_core
#cgo darwin LDFLAGS: -framework Security -framework CoreFoundation

#include <stdlib.h>
#include "../../include/rlm_core.h"
*/
import "C"

import (
	"runtime"
	"unsafe"
)

// TrajectoryEvent represents an event in the RLM execution trajectory.
type TrajectoryEvent struct {
	ptr *C.RlmTrajectoryEvent
}

// NewTrajectoryEvent creates a new trajectory event.
func NewTrajectoryEvent(eventType TrajectoryEventType, depth uint32, content string) *TrajectoryEvent {
	ccontent := cString(content)
	defer C.free(unsafe.Pointer(ccontent))
	event := &TrajectoryEvent{ptr: C.rlm_trajectory_event_new(C.RlmTrajectoryEventType(eventType), C.uint32_t(depth), ccontent)}
	runtime.SetFinalizer(event, (*TrajectoryEvent).Free)
	return event
}

// NewRLMStartEvent creates an RLM start event.
func NewRLMStartEvent(query string) *TrajectoryEvent {
	cquery := cString(query)
	defer C.free(unsafe.Pointer(cquery))
	event := &TrajectoryEvent{ptr: C.rlm_trajectory_event_rlm_start(cquery)}
	runtime.SetFinalizer(event, (*TrajectoryEvent).Free)
	return event
}

// NewAnalyzeEvent creates an analyze event.
func NewAnalyzeEvent(depth uint32, analysis string) *TrajectoryEvent {
	canalysis := cString(analysis)
	defer C.free(unsafe.Pointer(canalysis))
	event := &TrajectoryEvent{ptr: C.rlm_trajectory_event_analyze(C.uint32_t(depth), canalysis)}
	runtime.SetFinalizer(event, (*TrajectoryEvent).Free)
	return event
}

// NewREPLExecEvent creates a REPL execution event.
func NewREPLExecEvent(depth uint32, code string) *TrajectoryEvent {
	ccode := cString(code)
	defer C.free(unsafe.Pointer(ccode))
	event := &TrajectoryEvent{ptr: C.rlm_trajectory_event_repl_exec(C.uint32_t(depth), ccode)}
	runtime.SetFinalizer(event, (*TrajectoryEvent).Free)
	return event
}

// NewREPLResultEvent creates a REPL result event.
func NewREPLResultEvent(depth uint32, result string, success bool) *TrajectoryEvent {
	cresult := cString(result)
	defer C.free(unsafe.Pointer(cresult))
	var csuccess C.int
	if success {
		csuccess = 1
	}
	event := &TrajectoryEvent{ptr: C.rlm_trajectory_event_repl_result(C.uint32_t(depth), cresult, csuccess)}
	runtime.SetFinalizer(event, (*TrajectoryEvent).Free)
	return event
}

// NewReasonEvent creates a reasoning event.
func NewReasonEvent(depth uint32, reasoning string) *TrajectoryEvent {
	creasoning := cString(reasoning)
	defer C.free(unsafe.Pointer(creasoning))
	event := &TrajectoryEvent{ptr: C.rlm_trajectory_event_reason(C.uint32_t(depth), creasoning)}
	runtime.SetFinalizer(event, (*TrajectoryEvent).Free)
	return event
}

// NewRecurseStartEvent creates a recursive call start event.
func NewRecurseStartEvent(depth uint32, query string) *TrajectoryEvent {
	cquery := cString(query)
	defer C.free(unsafe.Pointer(cquery))
	event := &TrajectoryEvent{ptr: C.rlm_trajectory_event_recurse_start(C.uint32_t(depth), cquery)}
	runtime.SetFinalizer(event, (*TrajectoryEvent).Free)
	return event
}

// NewRecurseEndEvent creates a recursive call end event.
func NewRecurseEndEvent(depth uint32, result string) *TrajectoryEvent {
	cresult := cString(result)
	defer C.free(unsafe.Pointer(cresult))
	event := &TrajectoryEvent{ptr: C.rlm_trajectory_event_recurse_end(C.uint32_t(depth), cresult)}
	runtime.SetFinalizer(event, (*TrajectoryEvent).Free)
	return event
}

// NewFinalAnswerEvent creates a final answer event.
func NewFinalAnswerEvent(depth uint32, answer string) *TrajectoryEvent {
	canswer := cString(answer)
	defer C.free(unsafe.Pointer(canswer))
	event := &TrajectoryEvent{ptr: C.rlm_trajectory_event_final_answer(C.uint32_t(depth), canswer)}
	runtime.SetFinalizer(event, (*TrajectoryEvent).Free)
	return event
}

// NewErrorEvent creates an error event.
func NewErrorEvent(depth uint32, errMsg string) *TrajectoryEvent {
	cerr := cString(errMsg)
	defer C.free(unsafe.Pointer(cerr))
	event := &TrajectoryEvent{ptr: C.rlm_trajectory_event_error(C.uint32_t(depth), cerr)}
	runtime.SetFinalizer(event, (*TrajectoryEvent).Free)
	return event
}

// Free releases the event resources.
func (e *TrajectoryEvent) Free() {
	if e.ptr != nil {
		C.rlm_trajectory_event_free(e.ptr)
		e.ptr = nil
	}
}

// Type returns the event type.
func (e *TrajectoryEvent) Type() TrajectoryEventType {
	return TrajectoryEventType(C.rlm_trajectory_event_type(e.ptr))
}

// Depth returns the recursion depth.
func (e *TrajectoryEvent) Depth() uint32 {
	return uint32(C.rlm_trajectory_event_depth(e.ptr))
}

// Content returns the event content.
func (e *TrajectoryEvent) Content() string {
	return goString(C.rlm_trajectory_event_content(e.ptr))
}

// Timestamp returns the event timestamp in RFC3339 format.
func (e *TrajectoryEvent) Timestamp() string {
	return goString(C.rlm_trajectory_event_timestamp(e.ptr))
}

// LogLine formats the event as a single-line log entry.
func (e *TrajectoryEvent) LogLine() string {
	return goString(C.rlm_trajectory_event_log_line(e.ptr))
}

// IsError returns true if this is an error event.
func (e *TrajectoryEvent) IsError() bool {
	return C.rlm_trajectory_event_is_error(e.ptr) != 0
}

// IsFinal returns true if this is a final answer event.
func (e *TrajectoryEvent) IsFinal() bool {
	return C.rlm_trajectory_event_is_final(e.ptr) != 0
}

// ToJSON serializes the event to JSON.
func (e *TrajectoryEvent) ToJSON() (string, error) {
	cstr := C.rlm_trajectory_event_to_json(e.ptr)
	if cstr == nil {
		return "", lastError()
	}
	return goString(cstr), nil
}

// TrajectoryEventFromJSON deserializes an event from JSON.
func TrajectoryEventFromJSON(jsonStr string) (*TrajectoryEvent, error) {
	cs := cString(jsonStr)
	defer C.free(unsafe.Pointer(cs))
	ptr := C.rlm_trajectory_event_from_json(cs)
	if ptr == nil {
		return nil, lastError()
	}
	event := &TrajectoryEvent{ptr: ptr}
	runtime.SetFinalizer(event, (*TrajectoryEvent).Free)
	return event, nil
}

// TrajectoryEmitter provides a channel-based interface for receiving trajectory events.
// This bridges Rust's streaming API with Go channels for use with Bubble Tea.
type TrajectoryEmitter struct {
	events chan *TrajectoryEvent
	done   chan struct{}
}

// NewTrajectoryEmitter creates a new emitter with the given buffer size.
func NewTrajectoryEmitter(bufferSize int) *TrajectoryEmitter {
	return &TrajectoryEmitter{
		events: make(chan *TrajectoryEvent, bufferSize),
		done:   make(chan struct{}),
	}
}

// Events returns the channel for receiving trajectory events.
func (e *TrajectoryEmitter) Events() <-chan *TrajectoryEvent {
	return e.events
}

// Emit sends an event to the channel.
func (e *TrajectoryEmitter) Emit(event *TrajectoryEvent) {
	select {
	case e.events <- event:
	case <-e.done:
	}
}

// Close closes the emitter and its channel.
func (e *TrajectoryEmitter) Close() {
	close(e.done)
	close(e.events)
}

// TrajectoryCollector collects trajectory events into a slice.
type TrajectoryCollector struct {
	events []*TrajectoryEvent
}

// NewTrajectoryCollector creates a new collector.
func NewTrajectoryCollector() *TrajectoryCollector {
	return &TrajectoryCollector{
		events: make([]*TrajectoryEvent, 0),
	}
}

// Add adds an event to the collector.
func (c *TrajectoryCollector) Add(event *TrajectoryEvent) {
	c.events = append(c.events, event)
}

// Events returns all collected events.
func (c *TrajectoryCollector) Events() []*TrajectoryEvent {
	return c.events
}

// Clear removes all collected events.
func (c *TrajectoryCollector) Clear() {
	c.events = c.events[:0]
}

// HasError returns true if any event is an error.
func (c *TrajectoryCollector) HasError() bool {
	for _, e := range c.events {
		if e.IsError() {
			return true
		}
	}
	return false
}

// FinalAnswer returns the content of the final answer event, or empty string if none.
func (c *TrajectoryCollector) FinalAnswer() string {
	for _, e := range c.events {
		if e.IsFinal() {
			return e.Content()
		}
	}
	return ""
}
