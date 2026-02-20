package tui

import (
	"time"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/rand/rlm-core/go/rlmcore"
)

// EventStream is a channel-based event source for the TUI.
type EventStream struct {
	events chan *rlmcore.TrajectoryEvent
	done   chan struct{}
}

// NewEventStream creates a new event stream.
func NewEventStream() *EventStream {
	return &EventStream{
		events: make(chan *rlmcore.TrajectoryEvent, 100),
		done:   make(chan struct{}),
	}
}

// Send sends an event to the stream.
func (s *EventStream) Send(event *rlmcore.TrajectoryEvent) {
	select {
	case s.events <- event:
	case <-s.done:
	}
}

// Close closes the event stream.
func (s *EventStream) Close() {
	close(s.done)
}

// Listen returns a tea.Cmd that listens for events from the stream.
func (s *EventStream) Listen() tea.Cmd {
	return func() tea.Msg {
		select {
		case event := <-s.events:
			if event == nil {
				return nil
			}
			return EventMsg{Event: event}
		case <-s.done:
			return nil
		}
	}
}

// StreamingModel extends Model with streaming capabilities.
type StreamingModel struct {
	Model
	stream *EventStream
}

// NewStreamingModel creates a new streaming TUI model.
func NewStreamingModel(stream *EventStream, opts ...Option) StreamingModel {
	return StreamingModel{
		Model:  New(opts...),
		stream: stream,
	}
}

// Init implements tea.Model with streaming support.
func (m StreamingModel) Init() tea.Cmd {
	return tea.Batch(
		m.Model.Init(),
		m.stream.Listen(),
	)
}

// Update implements tea.Model with streaming support.
func (m StreamingModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd

	switch msg.(type) {
	case EventMsg:
		// Continue listening for more events
		cmds = append(cmds, m.stream.Listen())
	}

	model, cmd := m.Model.Update(msg)
	m.Model = model.(Model)
	cmds = append(cmds, cmd)

	return m, tea.Batch(cmds...)
}

// TrajectoryRecorder records trajectory events and streams them to the TUI.
type TrajectoryRecorder struct {
	stream    *EventStream
	events    []*rlmcore.TrajectoryEvent
	startTime time.Time
}

// NewTrajectoryRecorder creates a new trajectory recorder.
func NewTrajectoryRecorder(stream *EventStream) *TrajectoryRecorder {
	return &TrajectoryRecorder{
		stream:    stream,
		events:    make([]*rlmcore.TrajectoryEvent, 0),
		startTime: time.Now(),
	}
}

// Record records an event and sends it to the stream.
func (r *TrajectoryRecorder) Record(event *rlmcore.TrajectoryEvent) {
	r.events = append(r.events, event)
	if r.stream != nil {
		r.stream.Send(event)
	}
}

// RlmStart records an RLM start event.
func (r *TrajectoryRecorder) RlmStart(query string) {
	r.Record(rlmcore.NewRLMStartEvent(query))
}

// Analyze records an analyze event.
func (r *TrajectoryRecorder) Analyze(depth int, analysis string) {
	r.Record(rlmcore.NewAnalyzeEvent(uint32(depth), analysis))
}

// ReplExec records a REPL execution event.
func (r *TrajectoryRecorder) ReplExec(depth int, code string) {
	r.Record(rlmcore.NewREPLExecEvent(uint32(depth), code))
}

// Reason records a reasoning event.
func (r *TrajectoryRecorder) Reason(depth int, reasoning string) {
	r.Record(rlmcore.NewReasonEvent(uint32(depth), reasoning))
}

// FinalAnswer records a final answer event.
func (r *TrajectoryRecorder) FinalAnswer(depth int, answer string) {
	r.Record(rlmcore.NewFinalAnswerEvent(uint32(depth), answer))
}

// Error records an error event.
func (r *TrajectoryRecorder) Error(depth int, errMsg string) {
	r.Record(rlmcore.NewErrorEvent(uint32(depth), errMsg))
}

// HallucinationFlag records a hallucination flag event.
func (r *TrajectoryRecorder) HallucinationFlag(depth int, claim string) {
	r.Record(rlmcore.NewTrajectoryEvent(rlmcore.EventHallucinationFlag, uint32(depth), claim))
}

// VerifyStart records a verification start event.
func (r *TrajectoryRecorder) VerifyStart(depth int, description string) {
	r.Record(rlmcore.NewTrajectoryEvent(rlmcore.EventVerifyStart, uint32(depth), description))
}

// VerifyComplete records a verification complete event.
func (r *TrajectoryRecorder) VerifyComplete(depth int, summary string) {
	r.Record(rlmcore.NewTrajectoryEvent(rlmcore.EventVerifyComplete, uint32(depth), summary))
}

// Events returns all recorded events.
func (r *TrajectoryRecorder) Events() []*rlmcore.TrajectoryEvent {
	return r.events
}

// Duration returns the elapsed time since recording started.
func (r *TrajectoryRecorder) Duration() time.Duration {
	return time.Since(r.startTime)
}

// ExportJSON exports all events as JSON.
func (r *TrajectoryRecorder) ExportJSON() []string {
	result := make([]string, len(r.events))
	for i, event := range r.events {
		json, err := event.ToJSON()
		if err != nil {
			result[i] = ""
			continue
		}
		result[i] = json
	}
	return result
}

// ExportLogLines exports all events as log lines.
func (r *TrajectoryRecorder) ExportLogLines() []string {
	result := make([]string, len(r.events))
	for i, event := range r.events {
		result[i] = event.LogLine()
	}
	return result
}
