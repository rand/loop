package tui

import (
	"testing"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/rand/rlm-core/go/rlmcore"
)

func TestNewModel(t *testing.T) {
	m := New()
	if m.running {
		t.Error("New model should not be running")
	}
	if len(m.events) != 0 {
		t.Error("New model should have no events")
	}
}

func TestModelWithOptions(t *testing.T) {
	m := New(
		WithTimestamps(true),
		WithMaxEvents(100),
	)
	if !m.showTimestamps {
		t.Error("showTimestamps should be true")
	}
	if m.maxEvents != 100 {
		t.Error("maxEvents should be 100")
	}
}

func TestModelAddEvent(t *testing.T) {
	m := New()

	event := rlmcore.NewTrajectoryEvent(rlmcore.EventAnalyze, "Test", 0)
	defer event.Close()

	m.AddEvent(event)
	if len(m.events) != 1 {
		t.Errorf("Expected 1 event, got %d", len(m.events))
	}
}

func TestModelMaxEvents(t *testing.T) {
	m := New(WithMaxEvents(3))

	for i := 0; i < 5; i++ {
		event := rlmcore.NewTrajectoryEvent(rlmcore.EventAnalyze, "Test", 0)
		m.AddEvent(event)
	}

	if len(m.events) != 3 {
		t.Errorf("Expected 3 events (max), got %d", len(m.events))
	}
}

func TestModelClear(t *testing.T) {
	m := New()

	event := rlmcore.NewTrajectoryEvent(rlmcore.EventAnalyze, "Test", 0)
	m.AddEvent(event)

	m.Clear()
	if len(m.events) != 0 {
		t.Error("Clear should remove all events")
	}
}

func TestModelStart(t *testing.T) {
	m := New()
	m.Start()

	if !m.running {
		t.Error("Model should be running after Start")
	}
	if m.startTime.IsZero() {
		t.Error("startTime should be set after Start")
	}
}

func TestModelUpdateWindowSize(t *testing.T) {
	m := New()

	msg := tea.WindowSizeMsg{Width: 80, Height: 24}
	newModel, _ := m.Update(msg)
	m = newModel.(Model)

	if m.width != 80 {
		t.Errorf("Expected width 80, got %d", m.width)
	}
	if m.height != 24 {
		t.Errorf("Expected height 24, got %d", m.height)
	}
}

func TestModelUpdateEventMsg(t *testing.T) {
	m := New()

	// First set window size
	msg := tea.WindowSizeMsg{Width: 80, Height: 24}
	newModel, _ := m.Update(msg)
	m = newModel.(Model)

	// Then send event
	event := rlmcore.NewTrajectoryEvent(rlmcore.EventRlmStart, "Test query", 0)
	eventMsg := EventMsg{Event: event}
	newModel, _ = m.Update(eventMsg)
	m = newModel.(Model)

	if len(m.events) != 1 {
		t.Errorf("Expected 1 event, got %d", len(m.events))
	}
}

func TestDefaultStyles(t *testing.T) {
	styles := DefaultStyles()

	// Just verify styles are initialized
	if styles.Title.GetBold() != true {
		t.Error("Title should be bold")
	}
}

func TestEventStream(t *testing.T) {
	stream := NewEventStream()
	defer stream.Close()

	event := rlmcore.NewTrajectoryEvent(rlmcore.EventAnalyze, "Test", 0)
	stream.Send(event)

	// The event should be in the channel
	select {
	case received := <-stream.events:
		if received != event {
			t.Error("Should receive the same event")
		}
	default:
		t.Error("Event should be in channel")
	}
}

func TestTrajectoryRecorder(t *testing.T) {
	stream := NewEventStream()
	defer stream.Close()

	recorder := NewTrajectoryRecorder(stream)

	recorder.RlmStart("Test query")
	recorder.Analyze(0, "Analysis")
	recorder.Reason(0, "Reasoning")
	recorder.FinalAnswer(0, "Answer")

	events := recorder.Events()
	if len(events) != 4 {
		t.Errorf("Expected 4 events, got %d", len(events))
	}

	// Check event types
	expectedTypes := []rlmcore.TrajectoryEventType{
		rlmcore.EventRlmStart,
		rlmcore.EventAnalyze,
		rlmcore.EventReason,
		rlmcore.EventFinal,
	}

	for i, event := range events {
		if event.EventType() != expectedTypes[i] {
			t.Errorf("Event %d: expected %s, got %s", i, expectedTypes[i], event.EventType())
		}
	}
}

func TestTrajectoryRecorderExport(t *testing.T) {
	recorder := NewTrajectoryRecorder(nil) // No stream

	recorder.RlmStart("Test")
	recorder.Analyze(0, "Analysis")

	jsonExport := recorder.ExportJSON()
	if len(jsonExport) != 2 {
		t.Errorf("Expected 2 JSON exports, got %d", len(jsonExport))
	}
	for i, json := range jsonExport {
		if json == "" {
			t.Errorf("JSON export %d should not be empty", i)
		}
	}

	logLines := recorder.ExportLogLines()
	if len(logLines) != 2 {
		t.Errorf("Expected 2 log lines, got %d", len(logLines))
	}
	for i, line := range logLines {
		if line == "" {
			t.Errorf("Log line %d should not be empty", i)
		}
	}
}

func TestTrajectoryRecorderDuration(t *testing.T) {
	recorder := NewTrajectoryRecorder(nil)
	duration := recorder.Duration()

	if duration < 0 {
		t.Error("Duration should not be negative")
	}
}

func TestVerificationStatus(t *testing.T) {
	if rlmcore.StatusGrounded.String() != "GROUNDED" {
		t.Error("StatusGrounded string mismatch")
	}
	if rlmcore.StatusUnsupported.IsFlagged() != true {
		t.Error("StatusUnsupported should be flagged")
	}
	if rlmcore.StatusContradicted.IsFlagged() != true {
		t.Error("StatusContradicted should be flagged")
	}
	if rlmcore.StatusGrounded.IsFlagged() != false {
		t.Error("StatusGrounded should not be flagged")
	}
}
