//! RLM skills for Claude Code integration.
//!
//! Skills are discoverable capabilities that can be loaded by Claude Code
//! based on context. This module exposes RLM functionality as skills.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An RLM skill that can be discovered and loaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmSkill {
    /// Unique skill name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Patterns that trigger skill loading
    pub trigger_patterns: Vec<String>,
    /// Keywords that indicate relevance
    pub keywords: Vec<String>,
    /// Skill content/instructions
    pub content: String,
    /// Category for organization
    pub category: Option<String>,
    /// Priority (higher = loaded earlier)
    pub priority: i32,
    /// Dependencies on other skills
    pub dependencies: Vec<String>,
    /// Whether skill is enabled
    pub enabled: bool,
}

impl RlmSkill {
    /// Create a new skill.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            trigger_patterns: Vec::new(),
            keywords: Vec::new(),
            content: String::new(),
            category: None,
            priority: 0,
            dependencies: Vec::new(),
            enabled: true,
        }
    }

    /// Add trigger patterns.
    pub fn with_triggers(mut self, patterns: Vec<&str>) -> Self {
        self.trigger_patterns = patterns.into_iter().map(String::from).collect();
        self
    }

    /// Add keywords.
    pub fn with_keywords(mut self, keywords: Vec<&str>) -> Self {
        self.keywords = keywords.into_iter().map(String::from).collect();
        self
    }

    /// Set the content.
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Set the category.
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Add dependencies.
    pub fn with_dependencies(mut self, deps: Vec<&str>) -> Self {
        self.dependencies = deps.into_iter().map(String::from).collect();
        self
    }

    /// Disable the skill.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// Check if a query matches this skill's triggers.
    pub fn matches(&self, query: &str) -> bool {
        if !self.enabled {
            return false;
        }

        let query_lower = query.to_lowercase();

        // Check trigger patterns
        for pattern in &self.trigger_patterns {
            if query_lower.contains(&pattern.to_lowercase()) {
                return true;
            }
        }

        // Check keywords
        for keyword in &self.keywords {
            if query_lower.contains(&keyword.to_lowercase()) {
                return true;
            }
        }

        false
    }

    // =========================================================================
    // Built-in Skills
    // =========================================================================

    /// Create the rlm_execute skill.
    pub fn rlm_execute() -> Self {
        Self::new(
            "rlm_execute",
            "Execute RLM orchestration for complex multi-step reasoning tasks",
        )
        .with_triggers(vec![
            "/rlm",
            "rlm execute",
            "use rlm",
            "activate rlm",
        ])
        .with_keywords(vec![
            "analyze",
            "architecture",
            "thorough",
            "exhaustive",
            "security review",
            "debug",
            "find all",
        ])
        .with_category("rlm")
        .with_priority(100)
        .with_content(RLM_EXECUTE_SKILL_CONTENT)
    }

    /// Create the rlm_status skill.
    pub fn rlm_status() -> Self {
        Self::new("rlm_status", "Check RLM status, mode, and budget")
            .with_triggers(vec![
                "/rlm status",
                "rlm status",
                "check rlm",
            ])
            .with_keywords(vec![
                "rlm mode",
                "budget",
                "cost",
            ])
            .with_category("rlm")
            .with_priority(90)
            .with_content(RLM_STATUS_SKILL_CONTENT)
    }

    /// Create the rlm_mode skill.
    pub fn rlm_mode() -> Self {
        Self::new("rlm_mode", "Change RLM execution mode")
            .with_triggers(vec![
                "/rlm mode",
                "rlm mode",
                "set mode",
            ])
            .with_keywords(vec![
                "micro",
                "fast",
                "balanced",
                "thorough",
            ])
            .with_category("rlm")
            .with_priority(80)
            .with_content(RLM_MODE_SKILL_CONTENT)
    }

    /// Create the memory_query skill.
    pub fn memory_query() -> Self {
        Self::new("memory_query", "Query the RLM memory store for knowledge")
            .with_triggers(vec![
                "memory query",
                "search memory",
                "recall",
            ])
            .with_keywords(vec![
                "remember",
                "what do you know",
                "previous",
                "earlier",
            ])
            .with_category("memory")
            .with_priority(70)
            .with_content(MEMORY_QUERY_SKILL_CONTENT)
    }

    /// Create the memory_store skill.
    pub fn memory_store() -> Self {
        Self::new("memory_store", "Store knowledge in RLM memory")
            .with_triggers(vec![
                "memory store",
                "remember this",
                "save to memory",
            ])
            .with_keywords(vec![
                "store",
                "save",
                "persist",
            ])
            .with_category("memory")
            .with_priority(70)
            .with_content(MEMORY_STORE_SKILL_CONTENT)
    }
}

/// Registry of RLM skills.
pub struct SkillRegistry {
    skills: HashMap<String, RlmSkill>,
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Create a registry with default RLM skills.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        registry.register(RlmSkill::rlm_execute());
        registry.register(RlmSkill::rlm_status());
        registry.register(RlmSkill::rlm_mode());
        registry.register(RlmSkill::memory_query());
        registry.register(RlmSkill::memory_store());

        registry
    }

    /// Register a skill.
    pub fn register(&mut self, skill: RlmSkill) {
        self.skills.insert(skill.name.clone(), skill);
    }

    /// Get a skill by name.
    pub fn get(&self, name: &str) -> Option<&RlmSkill> {
        self.skills.get(name)
    }

    /// Get all skills.
    pub fn all(&self) -> Vec<&RlmSkill> {
        self.skills.values().collect()
    }

    /// Get enabled skills.
    pub fn enabled(&self) -> Vec<&RlmSkill> {
        self.skills.values().filter(|s| s.enabled).collect()
    }

    /// Find matching skills for a query.
    pub fn find_matching(&self, query: &str) -> Vec<&RlmSkill> {
        let mut matches: Vec<_> = self.skills.values().filter(|s| s.matches(query)).collect();

        // Sort by priority (descending)
        matches.sort_by(|a, b| b.priority.cmp(&a.priority));

        matches
    }

    /// Get skills by category.
    pub fn by_category(&self, category: &str) -> Vec<&RlmSkill> {
        self.skills
            .values()
            .filter(|s| s.category.as_deref() == Some(category))
            .collect()
    }

    /// Export skills as SKILL.md format for discovery.
    pub fn export_discovery(&self) -> String {
        let mut output = String::from("# RLM Skills\n\n");

        for skill in self.enabled() {
            output.push_str(&format!("## {}\n\n", skill.name));
            output.push_str(&format!("{}\n\n", skill.description));

            if !skill.trigger_patterns.is_empty() {
                output.push_str("**Triggers:** ");
                output.push_str(&skill.trigger_patterns.join(", "));
                output.push_str("\n\n");
            }

            if !skill.keywords.is_empty() {
                output.push_str("**Keywords:** ");
                output.push_str(&skill.keywords.join(", "));
                output.push_str("\n\n");
            }

            output.push_str("---\n\n");
        }

        output
    }
}

// =============================================================================
// Skill Content
// =============================================================================

const RLM_EXECUTE_SKILL_CONTENT: &str = r#"
# RLM Execute

Execute RLM (Recursive Language Model) orchestration for complex tasks.

## Usage

```
/rlm execute <query>
```

Or simply include complexity signals in your query:
- "analyze the architecture"
- "find all security issues"
- "thorough review of..."
- "debug the failing test"

## Execution Modes

| Mode | Cost | Use Case |
|------|------|----------|
| micro | ~$0.01 | Default, REPL-only |
| fast | ~$0.05 | Quick analysis |
| balanced | ~$0.25 | Multi-file reasoning |
| thorough | ~$1.00 | Deep architecture analysis |

## Auto-Escalation

RLM automatically escalates mode based on detected complexity:
- Multi-file references
- Architecture/security keywords
- "thorough", "exhaustive" signals
- Debugging tasks

## Example

```
User: Analyze the authentication system and find security issues

RLM: [Mode: thorough]
1. EXTERNALIZE: Loading auth-related files...
2. ANALYZE: Complexity score 8 (security_review, exhaustive_search)
3. DECOMPOSE: Splitting into auth flow, token handling, permission checks
4. EXECUTE: Running analysis on each component...
5. SYNTHESIZE: Combining findings...

Final Answer: Found 3 potential security concerns...
```
"#;

const RLM_STATUS_SKILL_CONTENT: &str = r#"
# RLM Status

Check current RLM status, execution mode, and budget.

## Usage

```
/rlm status
```

## Output

- Current execution mode
- Budget consumption
- Memory statistics
- Active session info

## Example

```
User: /rlm status

RLM Status:
  Mode: balanced
  Budget: $0.15 / $1.00 (15%)
  Memory: 42 nodes (12 facts, 8 entities, 22 experiences)
  Session: abc123 (started 2h ago)
```
"#;

const RLM_MODE_SKILL_CONTENT: &str = r#"
# RLM Mode

Change the RLM execution mode.

## Usage

```
/rlm mode <mode>
```

## Available Modes

| Mode | Budget | Description |
|------|--------|-------------|
| micro | $0.01 | Minimal, REPL-only |
| fast | $0.05 | Quick responses |
| balanced | $0.25 | Default for complex tasks |
| thorough | $1.00 | Deep analysis |

## Example

```
User: /rlm mode thorough

Mode set to: thorough
Budget: $1.00 per query
Max depth: 5 recursion levels
```
"#;

const MEMORY_QUERY_SKILL_CONTENT: &str = r#"
# Memory Query

Query the RLM memory store for relevant knowledge.

## Usage

```
memory_query text="<search>" node_types=["fact", "entity"] limit=10
```

## Parameters

- **text**: Full-text search query
- **node_types**: Filter by type (entity, fact, experience, decision, snippet)
- **tiers**: Filter by tier (task, session, longterm, archive)
- **min_confidence**: Minimum confidence threshold
- **limit**: Maximum results

## Example

```
User: What do you remember about authentication?

Memory Query Results (3 nodes):
1. [fact] The API uses JWT tokens (confidence: 0.95)
2. [entity] AuthService handles login flow (confidence: 0.90)
3. [experience] User prefers OAuth2 flows (confidence: 0.85)
```
"#;

const MEMORY_STORE_SKILL_CONTENT: &str = r#"
# Memory Store

Store new knowledge in the RLM memory system.

## Usage

```
memory_store content="<content>" node_type="fact" confidence=0.9
```

## Parameters

- **content**: The knowledge to store (required)
- **node_type**: Type of node (required)
  - entity: Code elements (files, functions, types)
  - fact: Knowledge claims
  - experience: Learned patterns
  - decision: Reasoning traces
  - snippet: Verbatim content
- **confidence**: Confidence score 0.0-1.0
- **tier**: Initial tier (task, session, longterm)

## Example

```
User: Remember that the config file is at /etc/app/config.yaml

Stored: [fact] Configuration file location: /etc/app/config.yaml
Node ID: abc-123
Tier: session
```
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_creation() {
        let skill = RlmSkill::new("test", "A test skill")
            .with_triggers(vec!["/test", "test me"])
            .with_keywords(vec!["testing", "check"])
            .with_category("testing")
            .with_priority(50);

        assert_eq!(skill.name, "test");
        assert_eq!(skill.trigger_patterns.len(), 2);
        assert_eq!(skill.keywords.len(), 2);
        assert_eq!(skill.priority, 50);
    }

    #[test]
    fn test_skill_matches_trigger() {
        let skill = RlmSkill::new("test", "test")
            .with_triggers(vec!["/rlm execute"]);

        assert!(skill.matches("/rlm execute the task"));
        assert!(!skill.matches("regular query"));
    }

    #[test]
    fn test_skill_matches_keyword() {
        let skill = RlmSkill::new("test", "test")
            .with_keywords(vec!["analyze", "architecture"]);

        assert!(skill.matches("analyze the codebase"));
        assert!(skill.matches("describe the architecture"));
        assert!(!skill.matches("what is 2 + 2"));
    }

    #[test]
    fn test_skill_disabled() {
        let skill = RlmSkill::new("test", "test")
            .with_keywords(vec!["test"])
            .disabled();

        assert!(!skill.matches("test query"));
    }

    #[test]
    fn test_builtin_skills() {
        let execute = RlmSkill::rlm_execute();
        assert!(execute.matches("analyze the architecture"));
        assert!(execute.matches("/rlm execute"));

        let status = RlmSkill::rlm_status();
        assert!(status.matches("/rlm status"));

        let mode = RlmSkill::rlm_mode();
        assert!(mode.matches("/rlm mode thorough"));
    }

    #[test]
    fn test_registry_defaults() {
        let registry = SkillRegistry::with_defaults();

        assert!(registry.get("rlm_execute").is_some());
        assert!(registry.get("rlm_status").is_some());
        assert!(registry.get("rlm_mode").is_some());
        assert!(registry.get("memory_query").is_some());
        assert!(registry.get("memory_store").is_some());
    }

    #[test]
    fn test_registry_find_matching() {
        let registry = SkillRegistry::with_defaults();

        let matches = registry.find_matching("analyze the architecture");
        assert!(!matches.is_empty());
        assert_eq!(matches[0].name, "rlm_execute"); // Highest priority
    }

    #[test]
    fn test_registry_by_category() {
        let registry = SkillRegistry::with_defaults();

        let rlm_skills = registry.by_category("rlm");
        assert_eq!(rlm_skills.len(), 3);

        let memory_skills = registry.by_category("memory");
        assert_eq!(memory_skills.len(), 2);
    }

    #[test]
    fn test_registry_export() {
        let registry = SkillRegistry::with_defaults();
        let export = registry.export_discovery();

        assert!(export.contains("# RLM Skills"));
        assert!(export.contains("## rlm_execute"));
        assert!(export.contains("**Triggers:**"));
    }
}
