// Package tui provides a Bubble Tea-based TUI for RLM execution visualization.
//
// This package renders trajectory events in real-time, providing a visual
// representation of RLM's recursive reasoning process.
package tui

import (
	"fmt"
	"strings"
	"time"

	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/rand/rlm-core/go/rlmcore"
)

// EventMsg wraps a trajectory event for the Bubble Tea message system.
type EventMsg struct {
	Event *rlmcore.TrajectoryEvent
}

// ErrorMsg indicates an error occurred.
type ErrorMsg struct {
	Err error
}

// DoneMsg indicates execution is complete.
type DoneMsg struct {
	Duration time.Duration
}

// Model represents the TUI state.
type Model struct {
	// Events collected during execution
	events []*rlmcore.TrajectoryEvent

	// Current execution state
	running   bool
	startTime time.Time
	duration  time.Duration
	err       error

	// UI components
	viewport viewport.Model
	spinner  spinner.Model

	// Dimensions
	width  int
	height int

	// Style configuration
	styles Styles

	// Options
	showTimestamps bool
	maxEvents      int
}

// Styles contains all style configurations for the TUI.
type Styles struct {
	Title         lipgloss.Style
	Subtitle      lipgloss.Style
	Event         lipgloss.Style
	EventType     lipgloss.Style
	Content       lipgloss.Style
	Depth         lipgloss.Style
	Timestamp     lipgloss.Style
	Error         lipgloss.Style
	Success       lipgloss.Style
	Warning       lipgloss.Style
	StatusBar     lipgloss.Style
	Help          lipgloss.Style
	Border        lipgloss.Style
	Hallucination lipgloss.Style
}

// DefaultStyles returns the default style configuration.
func DefaultStyles() Styles {
	return Styles{
		Title: lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("212")).
			MarginBottom(1),
		Subtitle: lipgloss.NewStyle().
			Foreground(lipgloss.Color("241")),
		Event: lipgloss.NewStyle().
			PaddingLeft(1),
		EventType: lipgloss.NewStyle().
			Bold(true).
			Width(18),
		Content: lipgloss.NewStyle().
			Foreground(lipgloss.Color("252")),
		Depth: lipgloss.NewStyle().
			Foreground(lipgloss.Color("241")),
		Timestamp: lipgloss.NewStyle().
			Foreground(lipgloss.Color("241")).
			Width(12),
		Error: lipgloss.NewStyle().
			Foreground(lipgloss.Color("196")).
			Bold(true),
		Success: lipgloss.NewStyle().
			Foreground(lipgloss.Color("82")).
			Bold(true),
		Warning: lipgloss.NewStyle().
			Foreground(lipgloss.Color("214")).
			Bold(true),
		StatusBar: lipgloss.NewStyle().
			Foreground(lipgloss.Color("241")).
			Background(lipgloss.Color("236")).
			Padding(0, 1),
		Help: lipgloss.NewStyle().
			Foreground(lipgloss.Color("241")),
		Border: lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(lipgloss.Color("62")),
		Hallucination: lipgloss.NewStyle().
			Foreground(lipgloss.Color("196")).
			Bold(true).
			Background(lipgloss.Color("52")),
	}
}

// Option is a functional option for configuring the Model.
type Option func(*Model)

// WithTimestamps enables timestamp display for events.
func WithTimestamps(show bool) Option {
	return func(m *Model) {
		m.showTimestamps = show
	}
}

// WithMaxEvents sets the maximum number of events to display.
func WithMaxEvents(max int) Option {
	return func(m *Model) {
		m.maxEvents = max
	}
}

// WithStyles sets custom styles.
func WithStyles(styles Styles) Option {
	return func(m *Model) {
		m.styles = styles
	}
}

// New creates a new TUI model.
func New(opts ...Option) Model {
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = lipgloss.NewStyle().Foreground(lipgloss.Color("205"))

	m := Model{
		events:         make([]*rlmcore.TrajectoryEvent, 0),
		spinner:        s,
		styles:         DefaultStyles(),
		showTimestamps: false,
		maxEvents:      1000,
	}

	for _, opt := range opts {
		opt(&m)
	}

	return m
}

// Init implements tea.Model.
func (m Model) Init() tea.Cmd {
	return m.spinner.Tick
}

// Update implements tea.Model.
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd

	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "q", "ctrl+c", "esc":
			return m, tea.Quit
		case "c":
			// Clear events
			m.events = make([]*rlmcore.TrajectoryEvent, 0)
			m.viewport.SetContent(m.renderEvents())
		}

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height

		headerHeight := 3
		footerHeight := 2
		verticalMargin := headerHeight + footerHeight

		if m.viewport.Width == 0 {
			m.viewport = viewport.New(msg.Width, msg.Height-verticalMargin)
			m.viewport.YPosition = headerHeight
		} else {
			m.viewport.Width = msg.Width
			m.viewport.Height = msg.Height - verticalMargin
		}
		m.viewport.SetContent(m.renderEvents())

	case EventMsg:
		m.events = append(m.events, msg.Event)
		if len(m.events) > m.maxEvents {
			m.events = m.events[1:]
		}
		m.viewport.SetContent(m.renderEvents())
		m.viewport.GotoBottom()

	case ErrorMsg:
		m.err = msg.Err
		m.running = false

	case DoneMsg:
		m.running = false
		m.duration = msg.Duration

	case spinner.TickMsg:
		var cmd tea.Cmd
		m.spinner, cmd = m.spinner.Update(msg)
		cmds = append(cmds, cmd)
	}

	var cmd tea.Cmd
	m.viewport, cmd = m.viewport.Update(msg)
	cmds = append(cmds, cmd)

	return m, tea.Batch(cmds...)
}

// View implements tea.Model.
func (m Model) View() string {
	if m.width == 0 {
		return "Loading..."
	}

	var b strings.Builder

	// Header
	title := m.styles.Title.Render("RLM Trajectory Viewer")
	var status string
	if m.running {
		status = m.spinner.View() + " Running..."
	} else if m.err != nil {
		status = m.styles.Error.Render("Error: " + m.err.Error())
	} else if m.duration > 0 {
		status = m.styles.Success.Render(fmt.Sprintf("Complete (%s)", m.duration.Round(time.Millisecond)))
	} else {
		status = m.styles.Subtitle.Render("Ready")
	}

	header := lipgloss.JoinHorizontal(
		lipgloss.Center,
		title,
		strings.Repeat(" ", max(0, m.width-lipgloss.Width(title)-lipgloss.Width(status)-4)),
		status,
	)
	b.WriteString(header)
	b.WriteString("\n")
	b.WriteString(strings.Repeat("─", m.width))
	b.WriteString("\n")

	// Viewport
	b.WriteString(m.viewport.View())

	// Footer
	b.WriteString("\n")
	b.WriteString(strings.Repeat("─", m.width))
	b.WriteString("\n")

	eventCount := fmt.Sprintf("%d events", len(m.events))
	scrollInfo := fmt.Sprintf("%3.f%%", m.viewport.ScrollPercent()*100)
	help := m.styles.Help.Render("q: quit • c: clear • ↑/↓: scroll")

	footer := lipgloss.JoinHorizontal(
		lipgloss.Center,
		m.styles.StatusBar.Render(eventCount),
		strings.Repeat(" ", max(0, m.width-len(eventCount)-len(scrollInfo)-len(help)-8)),
		help,
		"  ",
		m.styles.StatusBar.Render(scrollInfo),
	)
	b.WriteString(footer)

	return b.String()
}

// renderEvents renders all events as a formatted string.
func (m Model) renderEvents() string {
	if len(m.events) == 0 {
		return m.styles.Subtitle.Render("No events yet...")
	}

	var b strings.Builder
	for _, event := range m.events {
		b.WriteString(m.renderEvent(event))
		b.WriteString("\n")
	}
	return b.String()
}

// renderEvent renders a single event.
func (m Model) renderEvent(event *rlmcore.TrajectoryEvent) string {
	var parts []string

	// Timestamp (optional)
	if m.showTimestamps {
		ts := event.Timestamp()
		if len(ts) > 19 {
			ts = ts[11:19] // Extract HH:MM:SS
		}
		parts = append(parts, m.styles.Timestamp.Render(ts))
	}

	// Depth indicator
	depth := event.Depth()
	indent := strings.Repeat("  ", int(depth))
	if depth > 0 {
		depthStr := fmt.Sprintf("L%d", depth)
		parts = append(parts, m.styles.Depth.Render(depthStr))
	}

	// Event type with color based on type
	eventType := event.Type()
	typeStyle := m.eventTypeStyle(eventType)
	parts = append(parts, indent+typeStyle.Render(eventType.String()))

	// Content (truncated if needed)
	content := event.Content()
	maxContentLen := m.width - 40
	if maxContentLen < 20 {
		maxContentLen = 20
	}
	if len(content) > maxContentLen {
		content = content[:maxContentLen-3] + "..."
	}
	// Remove newlines for single-line display
	content = strings.ReplaceAll(content, "\n", " ")
	parts = append(parts, m.styles.Content.Render(content))

	return m.styles.Event.Render(strings.Join(parts, " "))
}

// eventTypeStyle returns the appropriate style for an event type.
func (m Model) eventTypeStyle(eventType rlmcore.TrajectoryEventType) lipgloss.Style {
	base := m.styles.EventType

	switch eventType {
	case rlmcore.EventRLMStart:
		return base.Foreground(lipgloss.Color("212"))
	case rlmcore.EventAnalyze:
		return base.Foreground(lipgloss.Color("81"))
	case rlmcore.EventREPLExec:
		return base.Foreground(lipgloss.Color("214"))
	case rlmcore.EventREPLResult:
		return base.Foreground(lipgloss.Color("220"))
	case rlmcore.EventReason:
		return base.Foreground(lipgloss.Color("141"))
	case rlmcore.EventRecurseStart, rlmcore.EventRecurseEnd:
		return base.Foreground(lipgloss.Color("183"))
	case rlmcore.EventFinal:
		return base.Foreground(lipgloss.Color("82"))
	case rlmcore.EventError:
		return base.Foreground(lipgloss.Color("196"))
	case rlmcore.EventVerifyStart, rlmcore.EventVerifyComplete:
		return base.Foreground(lipgloss.Color("117"))
	case rlmcore.EventClaimExtracted, rlmcore.EventEvidenceChecked:
		return base.Foreground(lipgloss.Color("159"))
	case rlmcore.EventBudgetComputed:
		return base.Foreground(lipgloss.Color("122"))
	case rlmcore.EventHallucinationFlag:
		return m.styles.Hallucination
	case rlmcore.EventMemory:
		return base.Foreground(lipgloss.Color("228"))
	default:
		return base.Foreground(lipgloss.Color("252"))
	}
}

// Start marks the model as running.
func (m *Model) Start() {
	m.running = true
	m.startTime = time.Now()
	m.err = nil
}

// AddEvent adds an event to the model (for programmatic use).
func (m *Model) AddEvent(event *rlmcore.TrajectoryEvent) {
	m.events = append(m.events, event)
	if len(m.events) > m.maxEvents {
		m.events = m.events[1:]
	}
}

// Events returns all collected events.
func (m *Model) Events() []*rlmcore.TrajectoryEvent {
	return m.events
}

// Clear removes all events.
func (m *Model) Clear() {
	m.events = make([]*rlmcore.TrajectoryEvent, 0)
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
