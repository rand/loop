//! Single-target proof session protocol for Lean REPL.
//!
//! Implements the Numina-style single-target proof protocol that prevents
//! combinatorial proof explosion by enforcing focus on exactly one sorry
//! at a time.
//!
//! ## Protocol Rules
//!
//! 1. **Single Target**: Exactly one sorry is selected as the session target
//! 2. **Focus Enforcement**: Tactics on non-target sorries are rejected
//! 3. **Helper Lemmas**: Discovered lemmas are tracked with attribution
//! 4. **NL Prohibition**: Complex comments are rejected in favor of helper lemmas
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::proof::{ProofSession, SorryLocation};
//!
//! // Create a session targeting a specific sorry
//! let target = SorryLocation::new("Foo.lean", 42, 5)
//!     .with_context("theorem foo : ∀ n, n + 0 = n := by");
//!
//! let mut session = ProofSession::new(target);
//!
//! // Attempt tactics
//! session.record_tactic("induction n", TacticOutcome::Success);
//!
//! // Add discovered helper lemmas
//! session.add_helper(HelperLemma::new("nat_zero_add", "∀ n, 0 + n = n"));
//! ```

use crate::error::Error;
use crate::lean::types::{LeanMessage, LeanResponse, MessageSeverity};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Location of a sorry in source code.
///
/// Enhanced version of lean::Sorry with file path and surrounding context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SorryLocation {
    /// File containing the sorry.
    pub file: PathBuf,
    /// 1-indexed line number.
    pub line: u32,
    /// 0-indexed column number.
    pub column: u32,
    /// Surrounding theorem/lemma context.
    pub context: String,
    /// The goal at this sorry position.
    pub goal: Option<String>,
    /// Proof state ID from Lean REPL.
    pub proof_state_id: Option<u64>,
}

impl SorryLocation {
    /// Create a new sorry location.
    pub fn new(file: impl Into<PathBuf>, line: u32, column: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column,
            context: String::new(),
            goal: None,
            proof_state_id: None,
        }
    }

    /// Add context (surrounding theorem/lemma).
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = context.into();
        self
    }

    /// Add the goal at this sorry.
    pub fn with_goal(mut self, goal: impl Into<String>) -> Self {
        self.goal = Some(goal.into());
        self
    }

    /// Add proof state ID.
    pub fn with_proof_state(mut self, proof_state_id: u64) -> Self {
        self.proof_state_id = Some(proof_state_id);
        self
    }

    /// Format as a human-readable location string.
    pub fn format_location(&self) -> String {
        format!("{}:{}:{}", self.file.display(), self.line, self.column)
    }

    /// Check if this location matches another (same file, line, column).
    pub fn matches(&self, other: &SorryLocation) -> bool {
        self.file == other.file && self.line == other.line && self.column == other.column
    }
}

/// Status of a proof session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofSessionStatus {
    /// Session is active, working on target.
    Active,
    /// Target sorry has been proven.
    TargetComplete,
    /// All sorries in the file have been eliminated.
    FileComplete,
    /// Session hit a limit and stopped.
    Limit { reason: LimitReason },
    /// Session was abandoned.
    Abandoned { reason: String },
}

impl ProofSessionStatus {
    /// Check if the session is still active.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Check if the session completed successfully.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::TargetComplete | Self::FileComplete)
    }

    /// Check if the session was terminated due to a limit.
    pub fn is_limited(&self) -> bool {
        matches!(self, Self::Limit { .. })
    }
}

impl std::fmt::Display for ProofSessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::TargetComplete => write!(f, "target_complete"),
            Self::FileComplete => write!(f, "file_complete"),
            Self::Limit { reason } => write!(f, "limit:{}", reason),
            Self::Abandoned { reason } => write!(f, "abandoned:{}", reason),
        }
    }
}

/// Reason why a proof session was limited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LimitReason {
    /// Token budget exhausted.
    TokenBudget(u64),
    /// Time limit reached.
    TimeLimit(Duration),
    /// Maximum retry attempts reached.
    RetryLimit(u32),
    /// User aborted the session.
    UserAbort,
    /// Maximum tactic attempts reached.
    TacticLimit(u32),
}

impl std::fmt::Display for LimitReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TokenBudget(tokens) => write!(f, "token_budget({})", tokens),
            Self::TimeLimit(duration) => write!(f, "time_limit({:?})", duration),
            Self::RetryLimit(retries) => write!(f, "retry_limit({})", retries),
            Self::UserAbort => write!(f, "user_abort"),
            Self::TacticLimit(tactics) => write!(f, "tactic_limit({})", tactics),
        }
    }
}

/// Status of a helper lemma's proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HelperProofStatus {
    /// Helper lemma has been fully proven.
    Proven,
    /// Helper lemma has a sorry (needs its own proof session).
    Sorry,
    /// Helper lemma proof failed.
    Failed,
    /// Helper lemma proof in progress.
    InProgress,
}

impl std::fmt::Display for HelperProofStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Proven => write!(f, "proven"),
            Self::Sorry => write!(f, "sorry"),
            Self::Failed => write!(f, "failed"),
            Self::InProgress => write!(f, "in_progress"),
        }
    }
}

/// A helper lemma discovered during proof.
///
/// Helper lemmas are intermediate results that support the main proof.
/// They are tracked with attribution to maintain provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelperLemma {
    /// Name of the helper lemma.
    pub name: String,
    /// The statement of the lemma.
    pub statement: String,
    /// Attribution comment (e.g., "(by claude) Helper for foo").
    pub attribution: String,
    /// Current proof status.
    pub proof_status: HelperProofStatus,
    /// The proof (if proven).
    pub proof: Option<String>,
    /// Target sorry this helper was discovered for.
    pub discovered_for: Option<String>,
}

impl HelperLemma {
    /// Create a new helper lemma.
    pub fn new(name: impl Into<String>, statement: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            attribution: format!("-- (by claude) Helper lemma: {}", name),
            name,
            statement: statement.into(),
            proof_status: HelperProofStatus::InProgress,
            proof: None,
            discovered_for: None,
        }
    }

    /// Set attribution for a specific target.
    pub fn with_attribution_for(mut self, target: impl Into<String>) -> Self {
        let target = target.into();
        self.attribution = format!("-- (by claude) Helper for {}", target);
        self.discovered_for = Some(target);
        self
    }

    /// Mark as proven with the given proof.
    pub fn mark_proven(mut self, proof: impl Into<String>) -> Self {
        self.proof_status = HelperProofStatus::Proven;
        self.proof = Some(proof.into());
        self
    }

    /// Mark as having a sorry.
    pub fn mark_sorry(mut self) -> Self {
        self.proof_status = HelperProofStatus::Sorry;
        self
    }

    /// Mark as failed.
    pub fn mark_failed(mut self) -> Self {
        self.proof_status = HelperProofStatus::Failed;
        self
    }

    /// Generate the full Lean declaration.
    pub fn to_lean_declaration(&self) -> String {
        let mut decl = String::new();
        decl.push_str(&self.attribution);
        decl.push('\n');
        decl.push_str(&format!("lemma {} : {} := by\n", self.name, self.statement));
        if let Some(ref proof) = self.proof {
            decl.push_str("  ");
            decl.push_str(proof);
        } else {
            decl.push_str("  sorry");
        }
        decl
    }
}

/// Outcome of a tactic attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TacticOutcome {
    /// Tactic succeeded, proof complete (no remaining goals).
    Complete,
    /// Tactic succeeded, made progress (goals reduced).
    Progress { remaining_goals: u32 },
    /// Tactic failed with error.
    Failed { error: String },
    /// Tactic rejected (not applicable to target).
    Rejected { reason: String },
}

impl TacticOutcome {
    /// Check if this outcome represents success.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Complete | Self::Progress { .. })
    }

    /// Check if the proof is complete.
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// A recorded tactic attempt in the session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticAttempt {
    /// The tactic that was attempted.
    pub tactic: String,
    /// Outcome of the attempt.
    pub outcome: TacticOutcome,
    /// Time taken in milliseconds.
    pub elapsed_ms: u64,
    /// Proof state before the attempt.
    pub pre_state_id: Option<u64>,
    /// Proof state after the attempt (if successful).
    pub post_state_id: Option<u64>,
}

impl TacticAttempt {
    /// Create a new tactic attempt record.
    pub fn new(tactic: impl Into<String>, outcome: TacticOutcome, elapsed_ms: u64) -> Self {
        Self {
            tactic: tactic.into(),
            outcome,
            elapsed_ms,
            pre_state_id: None,
            post_state_id: None,
        }
    }

    /// Set pre-state ID.
    pub fn with_pre_state(mut self, state_id: u64) -> Self {
        self.pre_state_id = Some(state_id);
        self
    }

    /// Set post-state ID.
    pub fn with_post_state(mut self, state_id: u64) -> Self {
        self.post_state_id = Some(state_id);
        self
    }
}

/// A single-target proof session.
///
/// Implements the Numina-style protocol where exactly one sorry is
/// selected as the session target, and all work focuses on that target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofSession {
    /// The one sorry being targeted.
    pub target: SorryLocation,
    /// Helper lemmas discovered during proof.
    pub helpers: Vec<HelperLemma>,
    /// Current session status.
    pub status: ProofSessionStatus,
    /// Tactics attempted during the session.
    pub tactic_history: Vec<TacticAttempt>,
    /// Token budget consumed.
    pub tokens_used: u64,
    /// Token budget limit.
    pub token_limit: Option<u64>,
    /// Time limit for the session.
    pub time_limit: Option<Duration>,
    /// Maximum tactic attempts.
    pub tactic_limit: Option<u32>,
    /// Session start time (as unix timestamp ms).
    pub started_at: u64,
    /// Session end time (as unix timestamp ms).
    pub ended_at: Option<u64>,
}

impl ProofSession {
    /// Create a new proof session targeting a specific sorry.
    pub fn new(target: SorryLocation) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            target,
            helpers: Vec::new(),
            status: ProofSessionStatus::Active,
            tactic_history: Vec::new(),
            tokens_used: 0,
            token_limit: None,
            time_limit: None,
            tactic_limit: None,
            started_at: now,
            ended_at: None,
        }
    }

    /// Set a token budget limit.
    pub fn with_token_limit(mut self, limit: u64) -> Self {
        self.token_limit = Some(limit);
        self
    }

    /// Set a time limit.
    pub fn with_time_limit(mut self, limit: Duration) -> Self {
        self.time_limit = Some(limit);
        self
    }

    /// Set a tactic attempt limit.
    pub fn with_tactic_limit(mut self, limit: u32) -> Self {
        self.tactic_limit = Some(limit);
        self
    }

    /// Record a tactic attempt.
    pub fn record_tactic(&mut self, attempt: TacticAttempt) {
        self.tactic_history.push(attempt);

        // Check if we've hit the tactic limit
        if let Some(limit) = self.tactic_limit {
            if self.tactic_history.len() as u32 >= limit {
                self.status = ProofSessionStatus::Limit {
                    reason: LimitReason::TacticLimit(limit),
                };
                self.end_session();
            }
        }
    }

    /// Record token usage.
    pub fn record_tokens(&mut self, tokens: u64) {
        self.tokens_used += tokens;

        // Check if we've hit the token limit
        if let Some(limit) = self.token_limit {
            if self.tokens_used >= limit {
                self.status = ProofSessionStatus::Limit {
                    reason: LimitReason::TokenBudget(limit),
                };
                self.end_session();
            }
        }
    }

    /// Add a helper lemma.
    pub fn add_helper(&mut self, helper: HelperLemma) {
        self.helpers.push(helper);
    }

    /// Mark the target as complete.
    pub fn mark_target_complete(&mut self) {
        self.status = ProofSessionStatus::TargetComplete;
        self.end_session();
    }

    /// Mark the entire file as complete.
    pub fn mark_file_complete(&mut self) {
        self.status = ProofSessionStatus::FileComplete;
        self.end_session();
    }

    /// Abandon the session.
    pub fn abandon(&mut self, reason: impl Into<String>) {
        self.status = ProofSessionStatus::Abandoned {
            reason: reason.into(),
        };
        self.end_session();
    }

    /// End the session (set ended_at timestamp).
    fn end_session(&mut self) {
        if self.ended_at.is_none() {
            self.ended_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            );
        }
    }

    /// Check if a sorry location is the current target.
    pub fn is_target(&self, location: &SorryLocation) -> bool {
        self.target.matches(location)
    }

    /// Get the number of successful tactic attempts.
    pub fn successful_tactics(&self) -> usize {
        self.tactic_history
            .iter()
            .filter(|t| t.outcome.is_success())
            .count()
    }

    /// Get the number of failed tactic attempts.
    pub fn failed_tactics(&self) -> usize {
        self.tactic_history
            .iter()
            .filter(|t| !t.outcome.is_success())
            .count()
    }

    /// Get total elapsed time in milliseconds.
    pub fn elapsed_ms(&self) -> u64 {
        let end = self.ended_at.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0)
        });
        end.saturating_sub(self.started_at)
    }

    /// Generate a summary of the session.
    pub fn summary(&self) -> String {
        let status = &self.status;
        let tactics_tried = self.tactic_history.len();
        let tactics_succeeded = self.successful_tactics();
        let helpers = self.helpers.len();
        let elapsed = self.elapsed_ms();

        format!(
            "[{}] {}: {} tactics ({} succeeded), {} helpers, {}ms",
            self.target.format_location(),
            status,
            tactics_tried,
            tactics_succeeded,
            helpers,
            elapsed
        )
    }
}

/// Protocol enforcement errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// Attempted to work on a non-target sorry.
    NonTargetSorry {
        attempted: SorryLocation,
        target: SorryLocation,
    },
    /// Comment exceeds maximum line limit.
    CommentTooLong { lines: u32, max_lines: u32 },
    /// Too many consecutive comment blocks.
    TooManyComments { count: u32, max_count: u32 },
    /// Session is not active.
    SessionNotActive { status: ProofSessionStatus },
    /// No proof state ID was available for diagnostic execution.
    MissingProofState { location: String },
    /// Lean diagnostic execution failed before response parsing.
    DiagnosticExecutionFailed { message: String },
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonTargetSorry { attempted, target } => {
                write!(
                    f,
                    "Attempted to work on non-target sorry at {} (target is {})",
                    attempted.format_location(),
                    target.format_location()
                )
            }
            Self::CommentTooLong { lines, max_lines } => {
                write!(
                    f,
                    "Comment too long: {} lines (max {}). Extract to helper lemma.",
                    lines, max_lines
                )
            }
            Self::TooManyComments { count, max_count } => {
                write!(
                    f,
                    "Too many consecutive comments: {} blocks (max {}). Extract logic to helper lemmas.",
                    count, max_count
                )
            }
            Self::SessionNotActive { status } => {
                write!(f, "Session is not active (status: {})", status)
            }
            Self::MissingProofState { location } => {
                write!(f, "No proof state available for target {}", location)
            }
            Self::DiagnosticExecutionFailed { message } => {
                write!(f, "Lean diagnostic execution failed: {}", message)
            }
        }
    }
}

impl std::error::Error for ProtocolError {}

/// Configuration for protocol enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    /// Maximum lines allowed in a comment block.
    pub max_comment_lines: u32,
    /// Maximum consecutive comment blocks allowed.
    pub max_consecutive_comments: u32,
    /// Whether to enforce single-target protocol.
    pub enforce_single_target: bool,
    /// Whether to enforce NL prohibition.
    pub enforce_nl_prohibition: bool,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            max_comment_lines: 42,
            max_consecutive_comments: 5,
            enforce_single_target: true,
            enforce_nl_prohibition: true,
        }
    }
}

/// Protocol enforcer for single-target proof sessions.
#[derive(Debug, Clone)]
pub struct ProtocolEnforcer {
    config: ProtocolConfig,
}

impl ProtocolEnforcer {
    /// Create a new protocol enforcer with default config.
    pub fn new() -> Self {
        Self {
            config: ProtocolConfig::default(),
        }
    }

    /// Create with custom config.
    pub fn with_config(config: ProtocolConfig) -> Self {
        Self { config }
    }

    /// Validate that a tactic targets the session's target sorry.
    pub fn validate_target(
        &self,
        session: &ProofSession,
        location: &SorryLocation,
    ) -> Result<(), ProtocolError> {
        if !self.config.enforce_single_target {
            return Ok(());
        }

        if !session.status.is_active() {
            return Err(ProtocolError::SessionNotActive {
                status: session.status.clone(),
            });
        }

        if !session.is_target(location) {
            return Err(ProtocolError::NonTargetSorry {
                attempted: location.clone(),
                target: session.target.clone(),
            });
        }

        Ok(())
    }

    /// Check for NL prohibition violations in code.
    pub fn check_nl_prohibition(&self, code: &str) -> Result<(), ProtocolError> {
        if !self.config.enforce_nl_prohibition {
            return Ok(());
        }

        // Check for overly long comments
        let mut in_comment = false;
        let mut comment_lines = 0;
        let mut consecutive_comment_blocks = 0;
        let mut prev_was_comment = false;

        for line in code.lines() {
            let trimmed = line.trim();

            // Track block comments
            if trimmed.starts_with("/-") {
                in_comment = true;
            }
            if trimmed.ends_with("-/") {
                in_comment = false;
            }

            // Count lines in comments
            let is_comment = in_comment
                || trimmed.starts_with("--")
                || trimmed.starts_with("/-")
                || trimmed.starts_with("-/");

            if is_comment {
                comment_lines += 1;

                // Check max comment lines
                if comment_lines > self.config.max_comment_lines {
                    return Err(ProtocolError::CommentTooLong {
                        lines: comment_lines,
                        max_lines: self.config.max_comment_lines,
                    });
                }

                if !prev_was_comment {
                    consecutive_comment_blocks += 1;

                    // Check max consecutive comment blocks
                    if consecutive_comment_blocks > self.config.max_consecutive_comments {
                        return Err(ProtocolError::TooManyComments {
                            count: consecutive_comment_blocks,
                            max_count: self.config.max_consecutive_comments,
                        });
                    }
                }
            } else {
                comment_lines = 0;
            }

            prev_was_comment = is_comment;
        }

        Ok(())
    }

    /// Validate a tactic before execution.
    pub fn validate_tactic(
        &self,
        session: &ProofSession,
        tactic: &str,
        target_location: &SorryLocation,
    ) -> Result<(), ProtocolError> {
        // Validate target
        self.validate_target(session, target_location)?;

        // Check NL prohibition in tactic (for tactic scripts with comments)
        self.check_nl_prohibition(tactic)?;

        Ok(())
    }

    /// Execute a validated tactic through Lean diagnostic feedback.
    ///
    /// This runs target/NL protocol validation, executes the tactic against a
    /// proof state, then records a deterministic `TacticAttempt` in the session.
    pub fn execute_tactic_with_feedback<F>(
        &self,
        session: &mut ProofSession,
        tactic: &str,
        target_location: &SorryLocation,
        mut execute: F,
    ) -> Result<TacticAttempt, ProtocolError>
    where
        F: FnMut(&str, u64) -> std::result::Result<LeanResponse, Error>,
    {
        self.validate_tactic(session, tactic, target_location)?;

        let proof_state = target_location
            .proof_state_id
            .or(session.target.proof_state_id)
            .ok_or_else(|| ProtocolError::MissingProofState {
                location: target_location.format_location(),
            })?;

        let start = std::time::Instant::now();
        let response = execute(tactic, proof_state).map_err(|error| {
            ProtocolError::DiagnosticExecutionFailed {
                message: error.to_string(),
            }
        })?;
        let elapsed_ms = start.elapsed().as_millis() as u64;

        let outcome = outcome_from_diagnostics(&response);
        let mut attempt =
            TacticAttempt::new(tactic, outcome.clone(), elapsed_ms).with_pre_state(proof_state);

        if let Some(post_state) = response.proof_state {
            session.target.proof_state_id = Some(post_state);
            attempt = attempt.with_post_state(post_state);
        }

        session.record_tactic(attempt.clone());
        if outcome.is_complete() {
            // Completion takes precedence over tactic-limit side-effects.
            session.mark_target_complete();
        }

        Ok(attempt)
    }

    /// Execute a tactic directly via `LeanRepl` using diagnostic feedback.
    pub fn execute_tactic_with_repl(
        &self,
        session: &mut ProofSession,
        repl: &mut crate::lean::repl::LeanRepl,
        tactic: &str,
        target_location: &SorryLocation,
    ) -> Result<TacticAttempt, ProtocolError> {
        self.execute_tactic_with_feedback(
            session,
            tactic,
            target_location,
            |candidate, state_id| repl.apply_tactic(candidate, state_id),
        )
    }
}

impl Default for ProtocolEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

/// Select the best target from multiple sorries.
///
/// Selection heuristics:
/// 1. Prefer sorries with proof state IDs (can interact via REPL)
/// 2. Prefer sorries earlier in the file
/// 3. Prefer sorries with smaller goals (simpler to prove)
pub fn select_target(sorries: &[SorryLocation]) -> Option<&SorryLocation> {
    if sorries.is_empty() {
        return None;
    }

    // Sort by selection criteria
    let mut candidates: Vec<_> = sorries.iter().collect();
    candidates.sort_by(|a, b| {
        // 1. Prefer sorries with proof state IDs
        let a_has_state = a.proof_state_id.is_some();
        let b_has_state = b.proof_state_id.is_some();
        if a_has_state != b_has_state {
            return b_has_state.cmp(&a_has_state);
        }

        // 2. Prefer earlier in file (by line, then column)
        match a.line.cmp(&b.line) {
            std::cmp::Ordering::Equal => a.column.cmp(&b.column),
            ord => ord,
        }
    });

    candidates.first().copied()
}

fn outcome_from_diagnostics(response: &LeanResponse) -> TacticOutcome {
    if response.has_errors() {
        return TacticOutcome::Failed {
            error: deterministic_error_message(response),
        };
    }

    let remaining_goals = response
        .goals
        .as_ref()
        .map(|goals| goals.len() as u32)
        .unwrap_or(response.sorries.len() as u32);

    if remaining_goals == 0 {
        TacticOutcome::Complete
    } else {
        TacticOutcome::Progress { remaining_goals }
    }
}

fn deterministic_error_message(response: &LeanResponse) -> String {
    let mut rendered: Vec<String> = response
        .messages
        .iter()
        .filter(|message| message.severity == MessageSeverity::Error)
        .map(render_message)
        .collect();
    rendered.sort();

    if rendered.is_empty() {
        "lean diagnostic reported an unknown error".to_string()
    } else {
        rendered.join(" | ")
    }
}

fn render_message(message: &LeanMessage) -> String {
    let location = if let (Some(start), Some(end)) = (&message.pos, &message.end_pos) {
        format!(
            "{}:{}-{}:{}: ",
            start.line, start.column, end.line, end.column
        )
    } else if let Some(start) = &message.pos {
        format!("{}:{}: ", start.line, start.column)
    } else {
        String::new()
    };

    format!("{}{}", location, message.data.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::lean::types::{LeanMessage, LeanResponse, MessageSeverity, Position};

    #[test]
    fn test_sorry_location() {
        let loc = SorryLocation::new("Foo.lean", 42, 5)
            .with_context("theorem foo : ∀ n, n + 0 = n := by")
            .with_goal("∀ n, n + 0 = n");

        assert_eq!(loc.format_location(), "Foo.lean:42:5");
        assert!(loc.goal.is_some());

        let same_loc = SorryLocation::new("Foo.lean", 42, 5);
        assert!(loc.matches(&same_loc));

        let diff_loc = SorryLocation::new("Foo.lean", 43, 5);
        assert!(!loc.matches(&diff_loc));
    }

    #[test]
    fn test_proof_session_status() {
        assert!(ProofSessionStatus::Active.is_active());
        assert!(!ProofSessionStatus::TargetComplete.is_active());
        assert!(ProofSessionStatus::TargetComplete.is_success());
        assert!(ProofSessionStatus::FileComplete.is_success());
        assert!(!ProofSessionStatus::Limit {
            reason: LimitReason::UserAbort
        }
        .is_success());
    }

    #[test]
    fn test_helper_lemma() {
        let helper = HelperLemma::new("nat_zero_add", "∀ n, 0 + n = n")
            .with_attribution_for("foo")
            .mark_proven("induction n <;> simp");

        assert_eq!(helper.proof_status, HelperProofStatus::Proven);
        assert!(helper.proof.is_some());
        assert!(helper.attribution.contains("foo"));

        let decl = helper.to_lean_declaration();
        assert!(decl.contains("-- (by claude)"));
        assert!(decl.contains("nat_zero_add"));
        assert!(decl.contains("induction n"));
    }

    #[test]
    fn test_proof_session() {
        let target = SorryLocation::new("Foo.lean", 10, 0);
        let mut session = ProofSession::new(target.clone())
            .with_token_limit(1000)
            .with_tactic_limit(10);

        assert!(session.status.is_active());

        // Record some tactics
        session.record_tactic(TacticAttempt::new(
            "intro n",
            TacticOutcome::Progress { remaining_goals: 1 },
            50,
        ));
        session.record_tactic(TacticAttempt::new(
            "simp",
            TacticOutcome::Failed {
                error: "no progress".into(),
            },
            20,
        ));
        session.record_tactic(TacticAttempt::new("rfl", TacticOutcome::Complete, 10));

        assert_eq!(session.tactic_history.len(), 3);
        assert_eq!(session.successful_tactics(), 2);
        assert_eq!(session.failed_tactics(), 1);

        // Add helper
        session.add_helper(HelperLemma::new("helper1", "P"));
        assert_eq!(session.helpers.len(), 1);

        // Mark complete
        session.mark_target_complete();
        assert!(session.status.is_success());
        assert!(session.ended_at.is_some());
    }

    #[test]
    fn test_protocol_enforcer_target() {
        let enforcer = ProtocolEnforcer::new();
        let target = SorryLocation::new("Foo.lean", 10, 0);
        let session = ProofSession::new(target.clone());

        // Valid target
        assert!(enforcer.validate_target(&session, &target).is_ok());

        // Invalid target
        let other = SorryLocation::new("Foo.lean", 20, 0);
        let result = enforcer.validate_target(&session, &other);
        assert!(matches!(result, Err(ProtocolError::NonTargetSorry { .. })));
    }

    #[test]
    fn test_protocol_enforcer_nl_prohibition() {
        let enforcer = ProtocolEnforcer::new();

        // Short comment is fine
        let code = "-- This is a short comment\ntheorem foo := sorry";
        assert!(enforcer.check_nl_prohibition(code).is_ok());

        // Too many consecutive comments
        let many_comments = (0..6)
            .map(|i| format!("-- Comment block {}\nx", i))
            .collect::<Vec<_>>()
            .join("\n");
        let result = enforcer.check_nl_prohibition(&many_comments);
        assert!(matches!(result, Err(ProtocolError::TooManyComments { .. })));
    }

    #[test]
    fn test_execute_tactic_with_feedback_marks_target_complete() {
        let enforcer = ProtocolEnforcer::new();
        let target = SorryLocation::new("Foo.lean", 10, 0).with_proof_state(41);
        let mut session = ProofSession::new(target.clone());

        let attempt = enforcer
            .execute_tactic_with_feedback(&mut session, "simp", &target, |_, _| {
                Ok(LeanResponse {
                    env: Some(2),
                    messages: vec![],
                    sorries: vec![],
                    goals: Some(vec![]),
                    proof_state: Some(42),
                })
            })
            .expect("diagnostic execution should succeed");

        assert!(matches!(attempt.outcome, TacticOutcome::Complete));
        assert_eq!(attempt.pre_state_id, Some(41));
        assert_eq!(attempt.post_state_id, Some(42));
        assert_eq!(session.target.proof_state_id, Some(42));
        assert!(matches!(session.status, ProofSessionStatus::TargetComplete));
        assert_eq!(session.tactic_history.len(), 1);
    }

    #[test]
    fn test_execute_tactic_with_feedback_produces_deterministic_errors() {
        let enforcer = ProtocolEnforcer::new();
        let target = SorryLocation::new("Foo.lean", 10, 0).with_proof_state(11);
        let mut session = ProofSession::new(target.clone());

        let attempt = enforcer
            .execute_tactic_with_feedback(&mut session, "aesop", &target, |_, _| {
                Ok(LeanResponse {
                    env: Some(2),
                    messages: vec![
                        LeanMessage {
                            severity: MessageSeverity::Error,
                            pos: Some(Position { line: 9, column: 1 }),
                            end_pos: None,
                            data: "second failure".to_string(),
                        },
                        LeanMessage {
                            severity: MessageSeverity::Error,
                            pos: Some(Position { line: 3, column: 5 }),
                            end_pos: None,
                            data: "first failure".to_string(),
                        },
                    ],
                    sorries: vec![],
                    goals: Some(vec!["goal".to_string()]),
                    proof_state: Some(11),
                })
            })
            .expect("execution should return a failed tactic attempt");

        let error = match attempt.outcome {
            TacticOutcome::Failed { error } => error,
            outcome => panic!("expected failed outcome, got {:?}", outcome),
        };
        assert_eq!(error, "3:5: first failure | 9:1: second failure");
        assert!(session.status.is_active());
        assert_eq!(session.tactic_history.len(), 1);
    }

    #[test]
    fn test_execute_tactic_with_feedback_requires_proof_state() {
        let enforcer = ProtocolEnforcer::new();
        let target = SorryLocation::new("Foo.lean", 10, 0);
        let mut session = ProofSession::new(target.clone());

        let err = enforcer
            .execute_tactic_with_feedback(&mut session, "simp", &target, |_, _| {
                panic!("executor should not run without proof state")
            })
            .expect_err("missing proof state should fail deterministically");

        assert!(matches!(err, ProtocolError::MissingProofState { .. }));
        assert_eq!(session.tactic_history.len(), 0);
    }

    #[test]
    fn test_execute_tactic_with_feedback_maps_execution_error() {
        let enforcer = ProtocolEnforcer::new();
        let target = SorryLocation::new("Foo.lean", 10, 0).with_proof_state(7);
        let mut session = ProofSession::new(target.clone());

        let err = enforcer
            .execute_tactic_with_feedback(&mut session, "simp", &target, |_, _| {
                Err(Error::repl_execution("simulated diagnostic failure"))
            })
            .expect_err("executor failures should map to protocol error");

        assert!(matches!(
            err,
            ProtocolError::DiagnosticExecutionFailed { .. }
        ));
        assert_eq!(session.tactic_history.len(), 0);
    }

    #[test]
    fn test_select_target() {
        let sorries = vec![
            SorryLocation::new("Foo.lean", 20, 0),
            SorryLocation::new("Foo.lean", 10, 0).with_proof_state(1),
            SorryLocation::new("Foo.lean", 15, 0),
        ];

        // Should prefer the one with proof state ID
        let selected = select_target(&sorries).unwrap();
        assert_eq!(selected.line, 10);
        assert!(selected.proof_state_id.is_some());

        // Without proof states, prefer earlier
        let sorries_no_state = vec![
            SorryLocation::new("Foo.lean", 20, 0),
            SorryLocation::new("Foo.lean", 10, 0),
            SorryLocation::new("Foo.lean", 15, 0),
        ];
        let selected = select_target(&sorries_no_state).unwrap();
        assert_eq!(selected.line, 10);
    }

    #[test]
    fn test_tactic_limit() {
        let target = SorryLocation::new("Foo.lean", 10, 0);
        let mut session = ProofSession::new(target).with_tactic_limit(3);

        for i in 0..3 {
            session.record_tactic(TacticAttempt::new(
                format!("tactic{}", i),
                TacticOutcome::Progress { remaining_goals: 1 },
                10,
            ));
        }

        assert!(matches!(
            session.status,
            ProofSessionStatus::Limit {
                reason: LimitReason::TacticLimit(3)
            }
        ));
    }

    #[test]
    fn test_token_limit() {
        let target = SorryLocation::new("Foo.lean", 10, 0);
        let mut session = ProofSession::new(target).with_token_limit(100);

        session.record_tokens(50);
        assert!(session.status.is_active());

        session.record_tokens(60);
        assert!(matches!(
            session.status,
            ProofSessionStatus::Limit {
                reason: LimitReason::TokenBudget(100)
            }
        ));
    }
}
