//! Lean-specific types for the REPL integration.
//!
//! These types model the JSON protocol used by leanprover-community/repl.
//! See: https://github.com/leanprover-community/repl

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

/// Lean REPL command types.
///
/// The leanprover-community/repl supports several command types:
/// - `cmd`: Execute a Lean command (definition, theorem, etc.)
/// - `tactic`: Apply a tactic in proof mode
/// - `pickle`: Save environment state to a file
/// - `unpickle`: Restore environment state from a file
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum LeanCommand {
    /// Execute a Lean command (e.g., `def`, `theorem`, `#check`).
    Command {
        /// The Lean code to execute.
        cmd: String,
        /// Optional environment ID to build upon.
        /// If omitted, uses a fresh environment.
        #[serde(skip_serializing_if = "Option::is_none")]
        env: Option<u64>,
    },

    /// Apply a tactic in proof mode.
    Tactic {
        /// The tactic to apply (e.g., "simp", "intro x").
        tactic: String,
        /// The proof state ID to apply the tactic in.
        #[serde(rename = "proofState")]
        proof_state: u64,
    },

    /// Save environment state to a pickle file.
    Pickle {
        /// Path where to save the environment.
        path: PathBuf,
        /// Environment ID to save.
        env: u64,
    },

    /// Restore environment state from a pickle file.
    Unpickle {
        /// Path to the pickle file.
        path: PathBuf,
    },
}

impl LeanCommand {
    /// Create a new command execution request.
    pub fn command(cmd: impl Into<String>) -> Self {
        Self::Command {
            cmd: cmd.into(),
            env: None,
        }
    }

    /// Create a command execution request building on an existing environment.
    pub fn command_with_env(cmd: impl Into<String>, env: u64) -> Self {
        Self::Command {
            cmd: cmd.into(),
            env: Some(env),
        }
    }

    /// Create a tactic application request.
    pub fn tactic(tactic: impl Into<String>, proof_state: u64) -> Self {
        Self::Tactic {
            tactic: tactic.into(),
            proof_state,
        }
    }

    /// Create a pickle (save environment) request.
    pub fn pickle(path: PathBuf, env: u64) -> Self {
        Self::Pickle { path, env }
    }

    /// Create an unpickle (restore environment) request.
    pub fn unpickle(path: PathBuf) -> Self {
        Self::Unpickle { path }
    }
}

/// Response from the Lean REPL.
#[derive(Debug, Clone, Deserialize)]
pub struct LeanResponse {
    /// Environment ID after executing the command.
    /// Can be used to build upon this state in subsequent commands.
    #[serde(default)]
    pub env: Option<u64>,

    /// Compiler messages (info, warning, error).
    #[serde(default)]
    pub messages: Vec<LeanMessage>,

    /// Unfinished goals (sorries) with their positions.
    #[serde(default)]
    pub sorries: Vec<Sorry>,

    /// Current proof goals (in tactic mode).
    #[serde(default)]
    pub goals: Option<Vec<String>>,

    /// Proof state ID (for tactic mode responses).
    #[serde(rename = "proofState")]
    pub proof_state: Option<u64>,
}

impl LeanResponse {
    /// Check if the response indicates success (no errors).
    pub fn is_success(&self) -> bool {
        self.messages
            .iter()
            .all(|m| m.severity != MessageSeverity::Error)
            && self.sorries.is_empty()
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.messages
            .iter()
            .any(|m| m.severity == MessageSeverity::Error)
    }

    /// Get all error messages.
    pub fn errors(&self) -> Vec<&LeanMessage> {
        self.messages
            .iter()
            .filter(|m| m.severity == MessageSeverity::Error)
            .collect()
    }

    /// Get all warning messages.
    pub fn warnings(&self) -> Vec<&LeanMessage> {
        self.messages
            .iter()
            .filter(|m| m.severity == MessageSeverity::Warning)
            .collect()
    }

    /// Get all info messages.
    pub fn info(&self) -> Vec<&LeanMessage> {
        self.messages
            .iter()
            .filter(|m| m.severity == MessageSeverity::Info)
            .collect()
    }

    /// Format output as a human-readable string.
    pub fn format_output(&self) -> String {
        let mut parts = Vec::new();

        // Info messages first
        for msg in self.info() {
            parts.push(msg.data.clone());
        }

        // Goals if any
        if let Some(ref goals) = self.goals {
            if !goals.is_empty() {
                parts.push(format!("Goals:\n{}", goals.join("\n\n")));
            }
        }

        parts.join("\n")
    }

    /// Format errors as a string.
    pub fn format_errors(&self) -> String {
        let mut parts = Vec::new();

        for msg in self.errors() {
            let location = if let (Some(start), Some(end)) = (&msg.pos, &msg.end_pos) {
                format!("{}:{}-{}:{}: ", start.line, start.column, end.line, end.column)
            } else if let Some(start) = &msg.pos {
                format!("{}:{}: ", start.line, start.column)
            } else {
                String::new()
            };
            parts.push(format!("{}error: {}", location, msg.data));
        }

        for msg in self.warnings() {
            let location = if let Some(start) = &msg.pos {
                format!("{}:{}: ", start.line, start.column)
            } else {
                String::new()
            };
            parts.push(format!("{}warning: {}", location, msg.data));
        }

        for sorry in &self.sorries {
            parts.push(format!("sorry: {}", sorry.goal));
        }

        parts.join("\n")
    }
}

/// A message from the Lean compiler.
#[derive(Debug, Clone, Deserialize)]
pub struct LeanMessage {
    /// Message severity (error, warning, info).
    pub severity: MessageSeverity,

    /// Start position in the source.
    pub pos: Option<Position>,

    /// End position in the source.
    #[serde(rename = "endPos")]
    pub end_pos: Option<Position>,

    /// Message content.
    pub data: String,
}

/// Message severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageSeverity {
    Error,
    Warning,
    Info,
}

impl std::fmt::Display for MessageSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warning => write!(f, "warning"),
            Self::Info => write!(f, "info"),
        }
    }
}

/// Position in source code.
#[derive(Debug, Clone, Deserialize)]
pub struct Position {
    /// 1-indexed line number.
    pub line: u32,
    /// 0-indexed column number.
    pub column: u32,
}

/// An unfinished proof (sorry) in the response.
#[derive(Debug, Clone, Deserialize)]
pub struct Sorry {
    /// The proof goal that needs to be filled.
    pub goal: String,

    /// Position of the sorry in source.
    pub pos: Option<Position>,

    /// End position of the sorry.
    #[serde(rename = "endPos")]
    pub end_pos: Option<Position>,

    /// Proof state ID for continuing this proof.
    #[serde(rename = "proofState")]
    pub proof_state: Option<u64>,
}

/// A hypothesis in a proof goal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    /// Name of the hypothesis.
    pub name: String,
    /// Type of the hypothesis.
    #[serde(rename = "type")]
    pub ty: String,
    /// Value if this is a let-binding.
    pub value: Option<String>,
}

/// A proof goal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    /// Target type to prove.
    pub target: String,
    /// Local hypotheses available.
    pub hypotheses: Vec<Hypothesis>,
    /// Suggested tactics (populated by AI assistant).
    #[serde(default)]
    pub suggestions: Vec<TacticSuggestion>,
}

impl Goal {
    /// Create a goal from a string representation.
    pub fn from_string(s: impl Into<String>) -> Self {
        Self {
            target: s.into(),
            hypotheses: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Add a hypothesis to the goal.
    pub fn with_hypothesis(mut self, name: impl Into<String>, ty: impl Into<String>) -> Self {
        self.hypotheses.push(Hypothesis {
            name: name.into(),
            ty: ty.into(),
            value: None,
        });
        self
    }
}

/// A suggested tactic with confidence score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticSuggestion {
    /// The tactic to try.
    pub tactic: String,
    /// Confidence score (0.0-1.0).
    pub confidence: f64,
    /// Brief explanation of why this might work.
    pub explanation: Option<String>,
}

/// Proof state that can be saved and restored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofState {
    /// Environment ID from Lean REPL.
    pub env: u64,
    /// Proof state ID for tactic mode.
    pub proof_state_id: Option<u64>,
    /// Current proof goals.
    pub goals: Vec<Goal>,
    /// Proof steps taken so far.
    pub history: Vec<ProofStep>,
    /// Checkpoint file path (for persistence via pickle).
    pub checkpoint: Option<PathBuf>,
}

impl ProofState {
    /// Create a new proof state.
    pub fn new(env: u64) -> Self {
        Self {
            env,
            proof_state_id: None,
            goals: Vec::new(),
            history: Vec::new(),
            checkpoint: None,
        }
    }

    /// Set the proof state ID.
    pub fn with_proof_state(mut self, proof_state_id: u64) -> Self {
        self.proof_state_id = Some(proof_state_id);
        self
    }

    /// Set the goals.
    pub fn with_goals(mut self, goals: Vec<Goal>) -> Self {
        self.goals = goals;
        self
    }

    /// Add a proof step.
    pub fn add_step(&mut self, step: ProofStep) {
        self.history.push(step);
    }

    /// Check if the proof is complete (no remaining goals).
    pub fn is_complete(&self) -> bool {
        self.goals.is_empty()
    }
}

/// A single step in a proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStep {
    /// The tactic applied.
    pub tactic: String,
    /// Goals before applying the tactic.
    pub pre_goals: Vec<String>,
    /// Goals after applying the tactic.
    pub post_goals: Vec<String>,
    /// Time taken in milliseconds.
    pub elapsed_ms: u64,
    /// Whether the tactic succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

impl ProofStep {
    /// Create a successful proof step.
    pub fn success(
        tactic: impl Into<String>,
        pre_goals: Vec<String>,
        post_goals: Vec<String>,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            tactic: tactic.into(),
            pre_goals,
            post_goals,
            elapsed_ms,
            success: true,
            error: None,
        }
    }

    /// Create a failed proof step.
    pub fn failure(
        tactic: impl Into<String>,
        pre_goals: Vec<String>,
        error: impl Into<String>,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            tactic: tactic.into(),
            pre_goals,
            post_goals: Vec::new(),
            elapsed_ms,
            success: false,
            error: Some(error.into()),
        }
    }
}

/// Result of type checking a Lean expression or definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCheckResult {
    /// Whether type checking succeeded.
    pub success: bool,
    /// The inferred type (if successful).
    pub inferred_type: Option<String>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// The environment after type checking.
    pub env: Option<u64>,
}

/// Lean project template for initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeanProjectTemplate {
    /// Minimal project with no dependencies.
    Minimal,
    /// Project with Mathlib dependency.
    Mathlib,
    /// Project with std4 only.
    Std4,
}

impl LeanProjectTemplate {
    /// Get the lakefile content for this template.
    pub fn lakefile_content(&self, name: &str) -> String {
        match self {
            Self::Minimal => format!(
                r#"import Lake
open Lake DSL

package «{}» where
  -- add package configuration here

@[default_target]
lean_lib «{}» where
  -- add library configuration here
"#,
                name, name
            ),
            Self::Mathlib => format!(
                r#"import Lake
open Lake DSL

package «{}» where
  leanOptions := #[
    ⟨`pp.unicode.fun, true⟩,
    ⟨`autoImplicit, false⟩
  ]

require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git"

@[default_target]
lean_lib «{}» where
  -- add library configuration here
"#,
                name, name
            ),
            Self::Std4 => format!(
                r#"import Lake
open Lake DSL

package «{}» where
  -- add package configuration here

require std from git "https://github.com/leanprover/std4" @ "main"

@[default_target]
lean_lib «{}» where
  -- add library configuration here
"#,
                name, name
            ),
        }
    }
}

/// Configuration for a Lean project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanProjectConfig {
    /// Project root directory.
    pub root: PathBuf,
    /// Project name.
    pub name: String,
    /// Lean version (e.g., "v4.15.0").
    pub lean_version: String,
    /// Required packages.
    pub packages: Vec<String>,
    /// Template used to create the project.
    pub template: Option<String>,
}

/// Information about a Lean command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanExecutionInfo {
    /// The command that was executed.
    pub command: String,
    /// Environment before execution.
    pub env_before: Option<u64>,
    /// Environment after execution.
    pub env_after: Option<u64>,
    /// Execution time in milliseconds.
    pub elapsed_ms: u64,
    /// Whether execution succeeded.
    pub success: bool,
}

/// Metadata for trajectory events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanEventMetadata {
    /// Environment ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<u64>,
    /// Proof state ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof_state: Option<u64>,
    /// Number of sorries (unfinished proofs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sorries_count: Option<usize>,
    /// Current goals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goals: Option<Vec<String>>,
    /// Execution time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
}

impl LeanEventMetadata {
    /// Create empty metadata.
    pub fn empty() -> Self {
        Self {
            env: None,
            proof_state: None,
            sorries_count: None,
            goals: None,
            elapsed_ms: None,
        }
    }

    /// Set environment ID.
    pub fn with_env(mut self, env: u64) -> Self {
        self.env = Some(env);
        self
    }

    /// Set proof state.
    pub fn with_proof_state(mut self, proof_state: u64) -> Self {
        self.proof_state = Some(proof_state);
        self
    }

    /// Set sorries count.
    pub fn with_sorries(mut self, count: usize) -> Self {
        self.sorries_count = Some(count);
        self
    }

    /// Set goals.
    pub fn with_goals(mut self, goals: Vec<String>) -> Self {
        self.goals = Some(goals);
        self
    }

    /// Set elapsed time.
    pub fn with_elapsed(mut self, elapsed_ms: u64) -> Self {
        self.elapsed_ms = Some(elapsed_ms);
        self
    }

    /// Convert to serde_json::Value for trajectory metadata.
    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lean_command_serialization() {
        let cmd = LeanCommand::command("def foo := 42");
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("cmd"));
        assert!(json.contains("def foo := 42"));

        let cmd_with_env = LeanCommand::command_with_env("def bar := 1", 5);
        let json = serde_json::to_string(&cmd_with_env).unwrap();
        assert!(json.contains("\"env\":5"));
    }

    #[test]
    fn test_lean_response_deserialization() {
        let json = r#"{
            "env": 1,
            "messages": [
                {"severity": "info", "data": "42 : Nat"}
            ],
            "sorries": []
        }"#;

        let response: LeanResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.env, Some(1));
        assert!(response.is_success());
        assert_eq!(response.messages.len(), 1);
    }

    #[test]
    fn test_lean_response_with_error() {
        let json = r#"{
            "env": null,
            "messages": [
                {"severity": "error", "data": "unknown identifier 'foo'", "pos": {"line": 1, "column": 5}}
            ],
            "sorries": []
        }"#;

        let response: LeanResponse = serde_json::from_str(json).unwrap();
        assert!(!response.is_success());
        assert!(response.has_errors());
        assert_eq!(response.errors().len(), 1);
    }

    #[test]
    fn test_proof_state() {
        let mut state = ProofState::new(1);
        assert!(state.goals.is_empty());
        assert!(state.is_complete());

        state.goals.push(Goal::from_string("Nat"));
        assert!(!state.is_complete());

        state.add_step(ProofStep::success(
            "exact 42",
            vec!["Nat".to_string()],
            vec![],
            10,
        ));

        state.goals.clear();
        assert!(state.is_complete());
    }

    #[test]
    fn test_project_template() {
        let content = LeanProjectTemplate::Mathlib.lakefile_content("MyProject");
        assert!(content.contains("mathlib"));
        assert!(content.contains("MyProject"));
    }
}
