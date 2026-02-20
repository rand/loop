# SPEC-22: Single-Target Proof Protocol

> Numina-inspired focused proof strategy for Lean REPL

**Status**: Implemented in `rlm-core` runtime (single-target protocol + deterministic Lean diagnostic-feedback execution path)
**Created**: 2026-01-20
**Epic**: loop-zcx (DSPy-Inspired RLM Improvements)
**Task**: loop-dzv

---

## Overview

Implement Numina-lean-agent's single-target proof protocol to prevent combinatorial proof explosion in Lean integration. The protocol enforces focus on exactly one sorry at a time, tracks helper lemmas, and prohibits excessive natural language comments.

## Implementation Snapshot (2026-02-20)

| Section | Status | Runtime Evidence |
|---|---|---|
| SPEC-22.01 ProofSession and target model | Implemented | `rlm-core/src/proof/session.rs` (`ProofSession`, `SorryLocation`, `select_target`) |
| SPEC-22.02 Session status and limits | Implemented | `rlm-core/src/proof/session.rs` (`ProofSessionStatus`, `LimitReason`, status tests) |
| SPEC-22.03 Helper lemma attribution | Implemented | `rlm-core/src/proof/session.rs` (`HelperLemma` attribution/declaration + tests) |
| SPEC-22.04 Protocol enforcement + diagnostic feedback | Implemented | `ProtocolEnforcer::validate_tactic`, `execute_tactic_with_feedback`, `execute_tactic_with_repl` and tests in `rlm-core/src/proof/session.rs` |
| Proof-engine execution/persistence closure (`M7-T05`) | Implemented | `rlm-core/src/proof/engine.rs` (`try_ai_assisted`, `record_success`, `create_context`) with memory-backed tests |

## Background

Numina-lean-agent successfully proved all 12 Putnam 2025 problems using:
- Single-target focus (one sorry per session)
- Helper lemma decomposition
- Natural language prohibition (enforce code-centric proofs)
- lean_diagnostic_messages for rapid iteration (not lake build)

## Requirements

### SPEC-22.01: ProofSession

Session management for proof work.

```rust
/// A proof session targeting a single sorry
#[derive(Debug, Clone)]
pub struct ProofSession {
    /// Unique session identifier
    pub id: SessionId,
    /// The one sorry being targeted this session
    pub target: SorryLocation,
    /// Helper lemmas created during this session
    pub helpers: Vec<HelperLemma>,
    /// Current session status
    pub status: ProofSessionStatus,
    /// Tactics attempted on target
    pub tactic_history: Vec<TacticAttempt>,
    /// Session start time
    pub started_at: DateTime<Utc>,
    /// Total tokens used
    pub tokens_used: u64,
}

/// Location of a sorry in Lean source
#[derive(Debug, Clone)]
pub struct SorryLocation {
    /// File path
    pub file: PathBuf,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
    /// Name of containing theorem/lemma
    pub theorem_name: String,
    /// Full context (surrounding code)
    pub context: String,
    /// Goal type at sorry
    pub goal_type: String,
}

impl ProofSession {
    /// Create new session targeting a sorry
    pub fn new(target: SorryLocation) -> Self;

    /// Check if target is proven
    pub fn is_target_complete(&self) -> bool;

    /// Get helper lemmas that still have sorries
    pub fn pending_helpers(&self) -> Vec<&HelperLemma>;
}
```

**Acceptance Criteria**:
- [x] ProofSession tracks all required state
- [x] SorryLocation captures full context
- [x] Session properly initializes from Lean diagnostics

### SPEC-22.02: Session Status

Status tracking for proof sessions.

```rust
/// Current status of a proof session
#[derive(Debug, Clone)]
pub enum ProofSessionStatus {
    /// Session is active, work in progress
    Active,
    /// Target sorry has been proven
    TargetComplete,
    /// All sorries in file eliminated (including helpers)
    FileComplete,
    /// Hit a limit, session stopped
    Limit { reason: LimitReason },
    /// Session aborted by user or error
    Aborted { reason: String },
}

/// Reasons for hitting session limits
#[derive(Debug, Clone)]
pub enum LimitReason {
    /// Token budget exhausted
    TokenBudget { used: u64, limit: u64 },
    /// Time limit reached
    TimeLimit { elapsed: Duration, limit: Duration },
    /// Too many tactic retries
    RetryLimit { attempts: u32, limit: u32 },
    /// User requested abort
    UserAbort,
    /// Lean compilation error (not recoverable)
    CompilationError { message: String },
}

impl ProofSessionStatus {
    /// Human-readable status for session end
    pub fn end_reason(&self) -> String {
        match self {
            Self::TargetComplete => "SELECTED_TARGET_COMPLETE".into(),
            Self::FileComplete => "COMPLETE".into(),
            Self::Limit { reason } => format!("LIMIT:{}", reason.code()),
            Self::Aborted { reason } => format!("ABORT:{}", reason),
            Self::Active => "IN_PROGRESS".into(),
        }
    }
}
```

**Acceptance Criteria**:
- [x] All status transitions valid
- [x] Limit reasons capture relevant details
- [x] end_reason() matches Numina format

### SPEC-22.03: Helper Lemma Management

Track and attribute helper lemmas.

```rust
/// A helper lemma created during proof
#[derive(Debug, Clone)]
pub struct HelperLemma {
    /// Lemma name (unique within file)
    pub name: String,
    /// Attribution comment
    pub attribution: String,
    /// Full lemma statement
    pub statement: String,
    /// Proof status
    pub proof_status: ProofStatus,
    /// Location in file
    pub location: Option<SorryLocation>,
    /// Which target this helps
    pub helps_target: String,
}

/// Status of a lemma's proof
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProofStatus {
    /// Fully proven (no sorries)
    Proven,
    /// Has sorry (needs proof)
    Sorry,
    /// Proof attempt failed
    Failed,
    /// Not yet attempted
    Pending,
}

impl HelperLemma {
    /// Create helper with proper attribution
    pub fn new(name: &str, statement: &str, target_name: &str) -> Self {
        Self {
            name: name.to_string(),
            attribution: format!("/- (by claude) Helper for {} -/", target_name),
            statement: statement.to_string(),
            proof_status: ProofStatus::Pending,
            location: None,
            helps_target: target_name.to_string(),
        }
    }

    /// Format as Lean code with attribution
    pub fn to_lean(&self) -> String {
        format!("{}\n{}", self.attribution, self.statement)
    }
}
```

**Acceptance Criteria**:
- [x] Attribution matches Numina format
- [x] Helpers tracked with proof status
- [x] to_lean() produces valid Lean code

### SPEC-22.04: Protocol Enforcement

Rules for single-target focus.

```rust
/// Protocol enforcer for proof sessions
pub struct ProofProtocol {
    session: ProofSession,
    config: ProofProtocolConfig,
}

pub struct ProofProtocolConfig {
    /// Maximum tokens per session
    pub max_tokens: u64,
    /// Maximum time per session
    pub max_duration: Duration,
    /// Maximum tactic retries on target
    pub max_retries: u32,
    /// Maximum comment length (lines)
    pub max_comment_lines: u32,  // Default: 42
    /// Maximum consecutive comment blocks
    pub max_consecutive_comments: u32,  // Default: 5
}

impl ProofProtocol {
    /// Select target from available sorries
    pub fn select_target(&mut self, sorries: &[SorryLocation]) -> Result<(), ProtocolError> {
        if sorries.is_empty() {
            return Err(ProtocolError::NoSorriesFound);
        }
        // Select most tractable sorry (heuristic)
        let target = self.select_most_tractable(sorries);
        self.session.target = target;
        Ok(())
    }

    /// Execute a tactic (validates focus)
    pub fn execute_tactic(
        &mut self,
        tactic: &str,
        location: &SorryLocation,
    ) -> Result<TacticResult, ProtocolError> {
        // Enforce single-target focus
        if !self.is_target_or_helper(location) {
            return Err(ProtocolError::WrongTarget {
                expected: self.session.target.theorem_name.clone(),
                got: location.theorem_name.clone(),
            });
        }

        // Execute via lean_diagnostic_messages (NOT lake build)
        self.execute_via_diagnostics(tactic)
    }

    /// Check if location is target or one of its helpers
    fn is_target_or_helper(&self, location: &SorryLocation) -> bool;

    /// Heuristic for selecting most tractable sorry
    fn select_most_tractable(&self, sorries: &[SorryLocation]) -> SorryLocation;
}

#[derive(Debug)]
pub enum ProtocolError {
    NoSorriesFound,
    WrongTarget { expected: String, got: String },
    LimitReached(LimitReason),
    NLProhibition(NLViolation),
}
```

**Acceptance Criteria**:
- [x] select_target() enforces single selection
- [x] execute_tactic() rejects non-target work
- [x] Uses lean_diagnostic_messages (not lake build)

### SPEC-22.05: Natural Language Prohibition

Enforce code-centric proofs.

```rust
/// Natural language violation types
#[derive(Debug, Clone)]
pub enum NLViolation {
    /// Comment exceeds line limit
    CommentTooLong {
        lines: u32,
        limit: u32,
        location: String,
    },
    /// Too many consecutive comment blocks
    ConsecutiveComments {
        count: u32,
        limit: u32,
        start_location: String,
    },
}

impl ProofProtocol {
    /// Check code for NL violations
    pub fn check_nl_prohibition(&self, code: &str) -> Result<(), NLViolation> {
        let mut comment_lines = 0;
        let mut consecutive_comments = 0;
        let mut in_comment = false;

        for (line_num, line) in code.lines().enumerate() {
            if line.trim().starts_with("/-") {
                in_comment = true;
            }
            if in_comment {
                comment_lines += 1;
                if comment_lines > self.config.max_comment_lines {
                    return Err(NLViolation::CommentTooLong {
                        lines: comment_lines,
                        limit: self.config.max_comment_lines,
                        location: format!("line {}", line_num + 1),
                    });
                }
            }
            if line.contains("-/") {
                in_comment = false;
                consecutive_comments += 1;
                comment_lines = 0;

                if consecutive_comments > self.config.max_consecutive_comments {
                    return Err(NLViolation::ConsecutiveComments {
                        count: consecutive_comments,
                        limit: self.config.max_consecutive_comments,
                        start_location: format!("line {}", line_num + 1),
                    });
                }
            }
            if !in_comment && !line.trim().is_empty() && !line.trim().starts_with("--") {
                consecutive_comments = 0;  // Reset on code
            }
        }

        Ok(())
    }

    /// Suggest extracting complex logic to lemma
    pub fn suggest_lemma_extraction(&self, code: &str) -> Option<String> {
        // Heuristic: if code has nested tactics > 3 deep, suggest extraction
    }
}
```

**NL Prohibition Rules**:
- Comments >42 lines MUST be rejected
- 5+ consecutive comment blocks without code MUST trigger warning
- Complex logic MUST be suggested for lemma extraction

**Acceptance Criteria**:
- [x] Long comments detected and rejected
- [x] Consecutive comment detection works
- [x] Lemma extraction suggestions generated

---

## Integration with Lean REPL

### Diagnostic-Based Feedback

```rust
impl LeanRepl {
    /// Execute tactic and get diagnostics (not full build)
    pub async fn execute_with_diagnostics(
        &mut self,
        tactic: &str,
    ) -> Result<DiagnosticResult, LeanError> {
        // Use lean_diagnostic_messages for fast feedback
        // Errors identified by "severity 1" responses
    }
}

pub struct DiagnosticResult {
    pub success: bool,
    pub diagnostics: Vec<LeanDiagnostic>,
    pub remaining_goals: Vec<String>,
}

pub struct LeanDiagnostic {
    pub severity: u32,  // 1 = error
    pub message: String,
    pub range: Option<Range>,
}
```

---

## Test Plan

| Test | Description | Spec |
|------|-------------|------|
| `test_select_single_target` | Must select exactly one | SPEC-22.04 |
| `test_reject_wrong_target` | Rejects non-target tactics | SPEC-22.04 |
| `test_helper_attribution` | Attribution format correct | SPEC-22.03 |
| `test_nl_long_comment` | Rejects >42 line comments | SPEC-22.05 |
| `test_nl_consecutive` | Rejects 5+ consecutive | SPEC-22.05 |
| `test_session_status_transitions` | Valid status changes | SPEC-22.02 |
| `proof::session::tests::test_execute_tactic_with_feedback_*` | Deterministic diagnostic-feedback tactic execution path (success, deterministic failures, missing proof-state, and execution-error mapping) | SPEC-22.04 |

---

## References

- [Numina-lean-agent](https://github.com/project-numina/numina-lean-agent)
- [Numina prompt_medium_mode.txt](https://github.com/project-numina/numina-lean-agent/blob/main/prompts/prompt_medium_mode.txt)
- Existing Lean REPL: `src/lean/repl.rs`
