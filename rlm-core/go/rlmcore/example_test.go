package rlmcore_test

import (
	"fmt"

	"github.com/rand/rlm-core/go/rlmcore"
)

// Example shows basic usage of the rlmcore package.
func Example() {
	// Initialize the library
	if err := rlmcore.Init(); err != nil {
		panic(err)
	}
	defer rlmcore.Shutdown()

	// Print version
	fmt.Println("rlm-core version:", rlmcore.Version())

	// Create a session context
	ctx := rlmcore.NewSessionContext()
	defer ctx.Free()

	// Add some messages
	ctx.AddUserMessage("Analyze the authentication system")
	ctx.AddAssistantMessage("I'll analyze the auth system for you.")

	// Cache some files
	ctx.CacheFile("/src/auth/login.rs", "pub fn login(user: &str, pass: &str) -> Result<Token> { ... }")
	ctx.CacheFile("/src/auth/session.rs", "pub struct Session { user_id: Uuid, expires: DateTime } ...")

	// Check complexity
	classifier := rlmcore.NewPatternClassifier()
	defer classifier.Free()

	decision := classifier.ShouldActivate("Find all security vulnerabilities in the auth system", ctx)
	defer decision.Free()

	if decision.ShouldActivate() {
		fmt.Println("RLM activated!")
		fmt.Println("Score:", decision.Score())
		fmt.Println("Reason:", decision.Reason())
	}
}

// ExampleMemoryStore shows how to use the hypergraph memory store.
func ExampleMemoryStore() {
	// Create an in-memory store
	store, err := rlmcore.NewMemoryStoreInMemory()
	if err != nil {
		panic(err)
	}
	defer store.Free()

	// Create nodes
	userNode := rlmcore.NewNode(rlmcore.NodeTypeEntity, "User authentication module")
	defer userNode.Free()

	sessionNode := rlmcore.NewNode(rlmcore.NodeTypeEntity, "Session management")
	defer sessionNode.Free()

	factNode := rlmcore.NewNodeFull(
		rlmcore.NodeTypeFact,
		"Sessions expire after 24 hours of inactivity",
		rlmcore.TierSession,
		0.95,
	)
	defer factNode.Free()

	// Add nodes to store
	store.AddNode(userNode)
	store.AddNode(sessionNode)
	store.AddNode(factNode)

	// Create an edge connecting them
	edge, err := rlmcore.NewBinaryEdge("structural", userNode.ID(), sessionNode.ID(), "manages")
	if err != nil {
		panic(err)
	}
	defer edge.Free()
	store.AddEdge(edge)

	// Query nodes
	factIDs, _ := store.QueryByType(rlmcore.NodeTypeFact, 10)
	fmt.Println("Found", len(factIDs), "facts")

	// Search content
	authIDs, _ := store.SearchContent("authentication", 10)
	fmt.Println("Found", len(authIDs), "nodes matching 'authentication'")

	// Get stats
	stats, _ := store.Stats()
	fmt.Println("Total nodes:", stats.TotalNodes)
	fmt.Println("Total edges:", stats.TotalEdges)
}

// ExampleTrajectoryEvent shows how to work with trajectory events.
func ExampleTrajectoryEvent() {
	// Create a collector for events
	collector := rlmcore.NewTrajectoryCollector()

	// Simulate RLM execution
	collector.Add(rlmcore.NewRLMStartEvent("What is the auth flow?"))
	collector.Add(rlmcore.NewAnalyzeEvent(0, "Complexity: high, multiple files involved"))
	collector.Add(rlmcore.NewREPLExecEvent(0, "files = context.files.keys()"))
	collector.Add(rlmcore.NewREPLResultEvent(0, "['/src/auth/login.rs', '/src/auth/session.rs']", true))
	collector.Add(rlmcore.NewReasonEvent(0, "Found 2 auth-related files, analyzing..."))
	collector.Add(rlmcore.NewFinalAnswerEvent(0, "The auth flow starts with login() which creates a Session..."))

	// Print log lines
	for _, event := range collector.Events() {
		fmt.Println(event.LogLine())
	}

	// Check for final answer
	if answer := collector.FinalAnswer(); answer != "" {
		fmt.Println("\nFinal answer:", answer[:50]+"...")
	}

	// Check for errors
	if collector.HasError() {
		fmt.Println("Execution had errors!")
	}
}

// Example_bubbleTeaIntegration shows how to integrate with Bubble Tea TUI.
func Example_bubbleTeaIntegration() {
	// This example shows the pattern for Bubble Tea integration.
	// In a real app, you would use these in your Model's Update method.

	// Create an emitter for streaming events
	emitter := rlmcore.NewTrajectoryEmitter(100)

	// In a goroutine, emit events (simulating RLM execution)
	go func() {
		defer emitter.Close()

		emitter.Emit(rlmcore.NewRLMStartEvent("Analyze code"))
		emitter.Emit(rlmcore.NewAnalyzeEvent(0, "Starting analysis..."))
		// ... more events
		emitter.Emit(rlmcore.NewFinalAnswerEvent(0, "Analysis complete"))
	}()

	// In your Bubble Tea Update, receive events from the channel
	for event := range emitter.Events() {
		// Update your TUI model with the event
		fmt.Printf("Received: %s\n", event.Type())

		// Render based on event type
		switch event.Type() {
		case rlmcore.EventRLMStart:
			fmt.Println("  Starting RLM...")
		case rlmcore.EventAnalyze:
			fmt.Println("  Analyzing:", event.Content())
		case rlmcore.EventFinal:
			fmt.Println("  Complete:", event.Content())
		}

		event.Free() // Don't forget to free!
	}
}
