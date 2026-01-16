package rlmcore

import (
	"strings"
	"testing"
)

func TestInit(t *testing.T) {
	if err := Init(); err != nil {
		t.Fatalf("Init failed: %v", err)
	}
}

func TestVersion(t *testing.T) {
	v := Version()
	if v == "" {
		t.Error("Version returned empty string")
	}
	// Should be semver-ish
	if !strings.Contains(v, ".") {
		t.Errorf("Version doesn't look like semver: %s", v)
	}
}

func TestSessionContext(t *testing.T) {
	ctx := NewSessionContext()
	defer ctx.Free()

	// Initially empty
	if ctx.MessageCount() != 0 {
		t.Errorf("Expected 0 messages, got %d", ctx.MessageCount())
	}

	// Add messages
	if err := ctx.AddUserMessage("Hello"); err != nil {
		t.Fatalf("AddUserMessage failed: %v", err)
	}
	if err := ctx.AddAssistantMessage("Hi there!"); err != nil {
		t.Fatalf("AddAssistantMessage failed: %v", err)
	}

	if ctx.MessageCount() != 2 {
		t.Errorf("Expected 2 messages, got %d", ctx.MessageCount())
	}

	// Cache files
	if err := ctx.CacheFile("/src/main.rs", "fn main() {}"); err != nil {
		t.Fatalf("CacheFile failed: %v", err)
	}
	if ctx.FileCount() != 1 {
		t.Errorf("Expected 1 file, got %d", ctx.FileCount())
	}

	content, ok := ctx.GetFile("/src/main.rs")
	if !ok {
		t.Error("GetFile returned false for cached file")
	}
	if content != "fn main() {}" {
		t.Errorf("File content mismatch: %s", content)
	}

	// Non-existent file
	_, ok = ctx.GetFile("/nonexistent")
	if ok {
		t.Error("GetFile returned true for non-existent file")
	}
}

func TestSessionContextSpansDirectories(t *testing.T) {
	ctx := NewSessionContext()
	defer ctx.Free()

	ctx.CacheFile("/src/lib.rs", "")
	if ctx.SpansMultipleDirectories() {
		t.Error("Single directory should not span multiple")
	}

	ctx.CacheFile("/tests/test.rs", "")
	if !ctx.SpansMultipleDirectories() {
		t.Error("Two directories should span multiple")
	}
}

func TestSessionContextJSON(t *testing.T) {
	ctx := NewSessionContext()
	defer ctx.Free()

	ctx.AddUserMessage("Test message")
	ctx.CacheFile("/test.txt", "content")

	json, err := ctx.ToJSON()
	if err != nil {
		t.Fatalf("ToJSON failed: %v", err)
	}

	ctx2, err := SessionContextFromJSON(json)
	if err != nil {
		t.Fatalf("SessionContextFromJSON failed: %v", err)
	}
	defer ctx2.Free()

	if ctx2.MessageCount() != 1 {
		t.Errorf("Deserialized context has wrong message count: %d", ctx2.MessageCount())
	}
	if ctx2.FileCount() != 1 {
		t.Errorf("Deserialized context has wrong file count: %d", ctx2.FileCount())
	}
}

func TestMessage(t *testing.T) {
	msg := NewUserMessage("Hello, world!")
	defer msg.Free()

	if msg.Role() != RoleUser {
		t.Errorf("Expected User role, got %v", msg.Role())
	}
	if msg.Content() != "Hello, world!" {
		t.Errorf("Content mismatch: %s", msg.Content())
	}
	if msg.Timestamp() == "" {
		t.Error("Timestamp should not be empty")
	}
}

func TestToolOutput(t *testing.T) {
	output := NewToolOutputWithExitCode("bash", "hello\n", 0)
	defer output.Free()

	if output.ToolName() != "bash" {
		t.Errorf("ToolName mismatch: %s", output.ToolName())
	}
	if output.Content() != "hello\n" {
		t.Errorf("Content mismatch: %s", output.Content())
	}
	if !output.HasExitCode() {
		t.Error("Should have exit code")
	}
	if output.ExitCode() != 0 {
		t.Errorf("ExitCode mismatch: %d", output.ExitCode())
	}
	if !output.IsSuccess() {
		t.Error("Should be success")
	}
}

func TestPatternClassifier(t *testing.T) {
	classifier := NewPatternClassifier()
	defer classifier.Free()

	ctx := NewSessionContext()
	defer ctx.Free()

	// Simple query should not activate
	decision := classifier.ShouldActivate("What is 2 + 2?", ctx)
	defer decision.Free()

	if decision.ShouldActivate() {
		t.Error("Simple query should not activate RLM")
	}
	if decision.Score() >= 3 {
		t.Errorf("Simple query score too high: %d", decision.Score())
	}
}

func TestPatternClassifierComplex(t *testing.T) {
	classifier := NewPatternClassifier()
	defer classifier.Free()

	ctx := NewSessionContext()
	defer ctx.Free()

	// Complex query should activate
	decision := classifier.ShouldActivate("Analyze the architecture and find all security issues", ctx)
	defer decision.Free()

	if !decision.ShouldActivate() {
		t.Error("Complex query should activate RLM")
	}
	if decision.Score() < 3 {
		t.Errorf("Complex query score too low: %d", decision.Score())
	}
	if decision.Reason() == "" {
		t.Error("Reason should not be empty")
	}
}

func TestPatternClassifierWithThreshold(t *testing.T) {
	// Very high threshold - nothing should activate
	classifier := NewPatternClassifierWithThreshold(100)
	defer classifier.Free()

	ctx := NewSessionContext()
	defer ctx.Free()

	decision := classifier.ShouldActivate("Analyze the architecture and find all security issues", ctx)
	defer decision.Free()

	if decision.ShouldActivate() {
		t.Error("High threshold should prevent activation")
	}
}

func TestMemoryStore(t *testing.T) {
	store, err := NewMemoryStoreInMemory()
	if err != nil {
		t.Fatalf("NewMemoryStoreInMemory failed: %v", err)
	}
	defer store.Free()

	// Create and add a node
	node := NewNode(NodeTypeFact, "The sky is blue")
	defer node.Free()

	nodeID := node.ID()
	if nodeID == "" {
		t.Error("Node ID should not be empty")
	}

	if err := store.AddNode(node); err != nil {
		t.Fatalf("AddNode failed: %v", err)
	}

	// Retrieve the node
	retrieved, err := store.GetNode(nodeID)
	if err != nil {
		t.Fatalf("GetNode failed: %v", err)
	}
	if retrieved == nil {
		t.Fatal("GetNode returned nil")
	}
	defer retrieved.Free()

	if retrieved.Content() != "The sky is blue" {
		t.Errorf("Content mismatch: %s", retrieved.Content())
	}
	if retrieved.Type() != NodeTypeFact {
		t.Errorf("Type mismatch: %v", retrieved.Type())
	}

	// Query by type
	ids, err := store.QueryByType(NodeTypeFact, 10)
	if err != nil {
		t.Fatalf("QueryByType failed: %v", err)
	}
	if len(ids) != 1 {
		t.Errorf("Expected 1 result, got %d", len(ids))
	}

	// Stats
	stats, err := store.Stats()
	if err != nil {
		t.Fatalf("Stats failed: %v", err)
	}
	if stats.TotalNodes != 1 {
		t.Errorf("Expected 1 node, got %d", stats.TotalNodes)
	}

	// Delete
	deleted, err := store.DeleteNode(nodeID)
	if err != nil {
		t.Fatalf("DeleteNode failed: %v", err)
	}
	if !deleted {
		t.Error("DeleteNode should return true")
	}

	// Verify deleted
	retrieved, err = store.GetNode(nodeID)
	if err != nil {
		t.Fatalf("GetNode after delete failed: %v", err)
	}
	if retrieved != nil {
		t.Error("Node should be deleted")
	}
}

func TestNode(t *testing.T) {
	node := NewNodeFull(NodeTypeEntity, "User struct", TierSession, 0.9)
	defer node.Free()

	if node.Type() != NodeTypeEntity {
		t.Errorf("Type mismatch: %v", node.Type())
	}
	if node.Content() != "User struct" {
		t.Errorf("Content mismatch: %s", node.Content())
	}
	if node.Tier() != TierSession {
		t.Errorf("Tier mismatch: %v", node.Tier())
	}
	if node.Confidence() != 0.9 {
		t.Errorf("Confidence mismatch: %f", node.Confidence())
	}

	// Set subtype
	if err := node.SetSubtype("struct"); err != nil {
		t.Fatalf("SetSubtype failed: %v", err)
	}
	if node.Subtype() != "struct" {
		t.Errorf("Subtype mismatch: %s", node.Subtype())
	}

	// Record access
	if err := node.RecordAccess(); err != nil {
		t.Fatalf("RecordAccess failed: %v", err)
	}
	if node.AccessCount() != 1 {
		t.Errorf("AccessCount mismatch: %d", node.AccessCount())
	}
}

func TestNodeJSON(t *testing.T) {
	node := NewNode(NodeTypeFact, "Test fact")
	defer node.Free()

	json, err := node.ToJSON()
	if err != nil {
		t.Fatalf("ToJSON failed: %v", err)
	}

	node2, err := NodeFromJSON(json)
	if err != nil {
		t.Fatalf("NodeFromJSON failed: %v", err)
	}
	defer node2.Free()

	if node2.Content() != "Test fact" {
		t.Errorf("Deserialized content mismatch: %s", node2.Content())
	}
}

func TestHyperEdge(t *testing.T) {
	// Create two nodes
	node1 := NewNode(NodeTypeEntity, "User")
	defer node1.Free()
	node2 := NewNode(NodeTypeEntity, "Session")
	defer node2.Free()

	// Create edge
	edge, err := NewBinaryEdge("structural", node1.ID(), node2.ID(), "has")
	if err != nil {
		t.Fatalf("NewBinaryEdge failed: %v", err)
	}
	defer edge.Free()

	if edge.Type() != "structural" {
		t.Errorf("Type mismatch: %s", edge.Type())
	}
	if edge.Label() != "has" {
		t.Errorf("Label mismatch: %s", edge.Label())
	}

	// Check membership
	if !edge.Contains(node1.ID()) {
		t.Error("Edge should contain node1")
	}
	if !edge.Contains(node2.ID()) {
		t.Error("Edge should contain node2")
	}

	ids, err := edge.NodeIDs()
	if err != nil {
		t.Fatalf("NodeIDs failed: %v", err)
	}
	if len(ids) != 2 {
		t.Errorf("Expected 2 members, got %d", len(ids))
	}
}

func TestTrajectoryEvent(t *testing.T) {
	event := NewRLMStartEvent("What is the auth flow?")
	defer event.Free()

	if event.Type() != EventRLMStart {
		t.Errorf("Type mismatch: %v", event.Type())
	}
	if event.Depth() != 0 {
		t.Errorf("Depth should be 0, got %d", event.Depth())
	}
	if event.Content() != "What is the auth flow?" {
		t.Errorf("Content mismatch: %s", event.Content())
	}
	if event.Timestamp() == "" {
		t.Error("Timestamp should not be empty")
	}
	if event.IsError() {
		t.Error("Should not be error")
	}
	if event.IsFinal() {
		t.Error("Should not be final")
	}
}

func TestTrajectoryEventJSON(t *testing.T) {
	event := NewAnalyzeEvent(1, "Complexity: high")
	defer event.Free()

	json, err := event.ToJSON()
	if err != nil {
		t.Fatalf("ToJSON failed: %v", err)
	}

	event2, err := TrajectoryEventFromJSON(json)
	if err != nil {
		t.Fatalf("TrajectoryEventFromJSON failed: %v", err)
	}
	defer event2.Free()

	if event2.Type() != EventAnalyze {
		t.Errorf("Type mismatch: %v", event2.Type())
	}
	if event2.Depth() != 1 {
		t.Errorf("Depth mismatch: %d", event2.Depth())
	}
}

func TestTrajectoryCollector(t *testing.T) {
	collector := NewTrajectoryCollector()

	collector.Add(NewRLMStartEvent("Test"))
	collector.Add(NewAnalyzeEvent(0, "Analysis"))
	collector.Add(NewFinalAnswerEvent(0, "The answer is 42"))

	if len(collector.Events()) != 3 {
		t.Errorf("Expected 3 events, got %d", len(collector.Events()))
	}
	if collector.HasError() {
		t.Error("Should not have error")
	}
	if collector.FinalAnswer() != "The answer is 42" {
		t.Errorf("FinalAnswer mismatch: %s", collector.FinalAnswer())
	}

	collector.Clear()
	if len(collector.Events()) != 0 {
		t.Error("Should be empty after clear")
	}
}

func TestTrajectoryEventTypes(t *testing.T) {
	// Test all event type string representations
	types := []TrajectoryEventType{
		EventRLMStart,
		EventAnalyze,
		EventREPLExec,
		EventREPLResult,
		EventReason,
		EventRecurseStart,
		EventRecurseEnd,
		EventFinal,
		EventError,
	}

	for _, et := range types {
		s := et.String()
		if s == "" {
			t.Errorf("Event type %d has empty string", et)
		}
	}
}

func TestRoleString(t *testing.T) {
	if RoleUser.String() != "user" {
		t.Errorf("RoleUser string mismatch: %s", RoleUser.String())
	}
	if RoleAssistant.String() != "assistant" {
		t.Errorf("RoleAssistant string mismatch: %s", RoleAssistant.String())
	}
}

func TestNodeTypeString(t *testing.T) {
	if NodeTypeFact.String() != "fact" {
		t.Errorf("NodeTypeFact string mismatch: %s", NodeTypeFact.String())
	}
	if NodeTypeEntity.String() != "entity" {
		t.Errorf("NodeTypeEntity string mismatch: %s", NodeTypeEntity.String())
	}
}

func TestTierString(t *testing.T) {
	if TierTask.String() != "task" {
		t.Errorf("TierTask string mismatch: %s", TierTask.String())
	}
	if TierLongTerm.String() != "longterm" {
		t.Errorf("TierLongTerm string mismatch: %s", TierLongTerm.String())
	}
}
