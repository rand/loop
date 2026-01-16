//! Type definitions for the proof automation pipeline.
//!
//! This module defines the core types for tiered proof automation,
//! including automation levels, proof attempts, and proof strategies.

use crate::lean::types::Goal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Specification domain for domain-specific tactic selection.
///
/// Different domains have different common patterns and preferred tactics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SpecDomain {
    /// Arithmetic and number theory (omega, linarith, ring, norm_num).
    Arithmetic,
    /// Set theory and membership (simp, ext, set_theory tactics).
    SetTheory,
    /// Order relations and inequalities (linarith, positivity).
    Order,
    /// Algebraic structures (ring, field_simp, norm_num).
    Algebra,
    /// Logic and propositional reasoning (decide, tauto, cases).
    Logic,
    /// Type theory and dependent types (rfl, congr, subst).
    TypeTheory,
    /// Data structures (lists, arrays, maps).
    DataStructures,
    /// Category theory (functor, monad laws).
    CategoryTheory,
    /// General/unknown domain.
    General,
}

impl SpecDomain {
    /// Attempt to infer domain from goal text.
    pub fn infer_from_goal(goal: &str) -> Self {
        let lower = goal.to_lowercase();

        // Arithmetic/number theory patterns - check first as it's common
        // Look for Nat operations, integer operations, or arithmetic operators with numbers
        if lower.contains("nat.")
            || lower.contains("int.")
            || lower.contains("nat ")
            || lower.contains(": nat")
            || (lower.contains('+') && (lower.contains("nat") || lower.contains(" 0 ") || lower.contains(" 1 ")))
            || (lower.contains('*') && (lower.contains("nat") || lower.contains(" 0 ") || lower.contains(" 1 ")))
            || lower.contains("omega")
            || lower.contains(".add")
            || lower.contains(".mul")
            || lower.contains(".sub")
            || lower.contains("div")
            || lower.contains("mod")
        {
            return Self::Arithmetic;
        }

        // Order patterns
        if lower.contains("<=") || lower.contains(">=") || lower.contains(" < ") || lower.contains(" > ")
            || lower.contains("le ") || lower.contains("lt ")
        {
            return Self::Order;
        }

        // Set theory patterns
        if lower.contains("set ") || lower.contains("finset") || lower.contains("mem ") {
            return Self::SetTheory;
        }

        // Algebraic patterns
        if lower.contains("ring") || lower.contains("field") || lower.contains("group") {
            return Self::Algebra;
        }

        // Logic patterns
        if lower.contains("true")
            || lower.contains("false")
            || lower.contains(" or ")
            || lower.contains(" and ")
            || lower.contains("decide")
        {
            return Self::Logic;
        }

        // Data structure patterns
        if lower.contains("list")
            || lower.contains("array")
            || lower.contains("map ")
            || lower.contains("hashmap")
        {
            return Self::DataStructures;
        }

        // Category theory patterns
        if lower.contains("functor")
            || lower.contains("monad")
            || lower.contains("applicative")
            || lower.contains("morphism")
        {
            return Self::CategoryTheory;
        }

        // Type theory patterns - check late as `=` is very common
        if lower.contains("eq ")
            || lower.contains("heq")
            || lower.contains("cast")
        {
            return Self::TypeTheory;
        }

        // Default for simple equality (check last, as it's generic)
        if lower.contains(" = ") {
            return Self::TypeTheory;
        }

        Self::General
    }
}

impl std::fmt::Display for SpecDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Arithmetic => write!(f, "arithmetic"),
            Self::SetTheory => write!(f, "set_theory"),
            Self::Order => write!(f, "order"),
            Self::Algebra => write!(f, "algebra"),
            Self::Logic => write!(f, "logic"),
            Self::TypeTheory => write!(f, "type_theory"),
            Self::DataStructures => write!(f, "data_structures"),
            Self::CategoryTheory => write!(f, "category_theory"),
            Self::General => write!(f, "general"),
        }
    }
}

/// Automation tier for proof attempts.
///
/// Proofs are attempted in order of automation tier, starting with
/// decidable tactics and escalating to human assistance if needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AutomationTier {
    /// Decidable tactics: decide, native_decide, omega, simp, rfl.
    /// These are guaranteed to terminate and work for decidable problems.
    Decidable = 0,

    /// Automation tactics: aesop, linarith, ring, norm_num, positivity.
    /// These use search-based automation that may not terminate.
    Automation = 1,

    /// AI-assisted: LLM-generated tactic sequences.
    /// Uses language models to suggest proof strategies.
    AIAssisted = 2,

    /// Human loop: sorry with TODO marker for manual completion.
    /// Fallback when automation fails.
    HumanLoop = 3,
}

impl AutomationTier {
    /// Get the next tier in the escalation chain.
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Decidable => Some(Self::Automation),
            Self::Automation => Some(Self::AIAssisted),
            Self::AIAssisted => Some(Self::HumanLoop),
            Self::HumanLoop => None,
        }
    }

    /// Check if this tier requires human intervention.
    pub fn requires_human(&self) -> bool {
        matches!(self, Self::HumanLoop)
    }

    /// Check if this tier uses AI/LLM.
    pub fn uses_ai(&self) -> bool {
        matches!(self, Self::AIAssisted)
    }

    /// Get the maximum time budget (in ms) for this tier.
    pub fn time_budget_ms(&self) -> u64 {
        match self {
            Self::Decidable => 5_000,     // 5 seconds
            Self::Automation => 30_000,   // 30 seconds
            Self::AIAssisted => 60_000,   // 60 seconds
            Self::HumanLoop => 0,         // No timeout
        }
    }
}

impl std::fmt::Display for AutomationTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Decidable => write!(f, "decidable"),
            Self::Automation => write!(f, "automation"),
            Self::AIAssisted => write!(f, "ai_assisted"),
            Self::HumanLoop => write!(f, "human_loop"),
        }
    }
}

/// Result of attempting a single tactic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticResult {
    /// The tactic that was tried.
    pub tactic: String,

    /// Whether the tactic succeeded (either fully or partially).
    pub success: bool,

    /// New goals after applying the tactic (empty if proof complete).
    pub new_goals: Vec<Goal>,

    /// Error message if the tactic failed.
    pub error: Option<String>,

    /// Time taken to execute the tactic in milliseconds.
    pub elapsed_ms: u64,
}

impl TacticResult {
    /// Create a successful tactic result.
    pub fn success(tactic: impl Into<String>, new_goals: Vec<Goal>, elapsed_ms: u64) -> Self {
        Self {
            tactic: tactic.into(),
            success: true,
            new_goals,
            error: None,
            elapsed_ms,
        }
    }

    /// Create a failed tactic result.
    pub fn failure(tactic: impl Into<String>, error: impl Into<String>, elapsed_ms: u64) -> Self {
        Self {
            tactic: tactic.into(),
            success: false,
            new_goals: Vec::new(),
            error: Some(error.into()),
            elapsed_ms,
        }
    }

    /// Check if this result represents a complete proof (no remaining goals).
    pub fn is_complete(&self) -> bool {
        self.success && self.new_goals.is_empty()
    }

    /// Check if this result made progress (reduced goals or changed them).
    pub fn made_progress(&self, original_goal_count: usize) -> bool {
        self.success && self.new_goals.len() < original_goal_count
    }
}

/// A complete proof attempt including all tactics tried.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofAttempt {
    /// The original goal being proved.
    pub goal: Goal,

    /// The tier that ultimately succeeded (or the highest tier tried).
    pub tier: AutomationTier,

    /// All tactics tried during the attempt.
    pub tactics_tried: Vec<TacticResult>,

    /// Whether the proof succeeded.
    pub success: bool,

    /// The successful tactic sequence (if proof succeeded).
    pub successful_tactics: Vec<String>,

    /// Total time spent on this proof attempt.
    pub total_elapsed_ms: u64,

    /// Inferred domain of the goal.
    pub domain: SpecDomain,
}

impl ProofAttempt {
    /// Create a new proof attempt.
    pub fn new(goal: Goal) -> Self {
        let domain = SpecDomain::infer_from_goal(&goal.target);
        Self {
            goal,
            tier: AutomationTier::Decidable,
            tactics_tried: Vec::new(),
            success: false,
            successful_tactics: Vec::new(),
            total_elapsed_ms: 0,
            domain,
        }
    }

    /// Record a tactic attempt.
    pub fn record_tactic(&mut self, result: TacticResult) {
        self.total_elapsed_ms += result.elapsed_ms;
        if result.success {
            self.successful_tactics.push(result.tactic.clone());
        }
        self.tactics_tried.push(result);
    }

    /// Mark the proof as successful.
    pub fn mark_success(&mut self, tier: AutomationTier) {
        self.success = true;
        self.tier = tier;
    }

    /// Mark the proof as failed at the given tier.
    pub fn mark_failure(&mut self, tier: AutomationTier) {
        self.tier = tier;
    }

    /// Get the number of goals remaining.
    pub fn remaining_goals(&self) -> usize {
        self.tactics_tried
            .last()
            .map(|t| t.new_goals.len())
            .unwrap_or(1) // Original goal if no tactics tried
    }

    /// Generate a summary of the proof attempt.
    pub fn summary(&self) -> String {
        let status = if self.success { "SUCCESS" } else { "FAILED" };
        let tactics = self.tactics_tried.len();
        let successful = self.successful_tactics.len();

        format!(
            "[{}] {}: {} tactics tried, {} succeeded, {}ms total (domain: {})",
            self.tier, status, tactics, successful, self.total_elapsed_ms, self.domain
        )
    }
}

/// A proof strategy for a specific domain.
///
/// Strategies capture learned patterns about which tactics work
/// well for different types of goals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStrategy {
    /// The domain this strategy applies to.
    pub domain: SpecDomain,

    /// Ordered list of preferred tactics for this domain.
    pub preferred_tactics: Vec<String>,

    /// Success rate of this strategy (0.0 - 1.0).
    pub success_rate: f64,

    /// Number of times this strategy has been used.
    pub usage_count: u64,

    /// Number of successful uses.
    pub success_count: u64,
}

impl ProofStrategy {
    /// Create a new strategy with default tactics for a domain.
    pub fn new(domain: SpecDomain, tactics: Vec<String>) -> Self {
        Self {
            domain,
            preferred_tactics: tactics,
            success_rate: 0.0,
            usage_count: 0,
            success_count: 0,
        }
    }

    /// Record a usage of this strategy.
    pub fn record_usage(&mut self, success: bool) {
        self.usage_count += 1;
        if success {
            self.success_count += 1;
        }
        // Update success rate
        self.success_rate = self.success_count as f64 / self.usage_count as f64;
    }

    /// Add a tactic to the preferred list.
    pub fn add_tactic(&mut self, tactic: impl Into<String>) {
        let tactic = tactic.into();
        if !self.preferred_tactics.contains(&tactic) {
            self.preferred_tactics.push(tactic);
        }
    }

    /// Boost a tactic to the front of the list (after a success).
    pub fn boost_tactic(&mut self, tactic: &str) {
        if let Some(pos) = self.preferred_tactics.iter().position(|t| t == tactic) {
            let t = self.preferred_tactics.remove(pos);
            self.preferred_tactics.insert(0, t);
        }
    }
}

/// Context for AI-assisted proof generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofContext {
    /// The current proof goal.
    pub goal: Goal,

    /// History of tactics tried (and their results).
    pub history: Vec<TacticResult>,

    /// Available lemmas/theorems that might be relevant.
    pub available_lemmas: Vec<String>,

    /// The inferred domain.
    pub domain: SpecDomain,

    /// Previous successful proofs for similar goals.
    pub similar_proofs: Vec<ProofAttempt>,
}

impl ProofContext {
    /// Create a new proof context.
    pub fn new(goal: Goal) -> Self {
        let domain = SpecDomain::infer_from_goal(&goal.target);
        Self {
            goal,
            history: Vec::new(),
            available_lemmas: Vec::new(),
            domain,
            similar_proofs: Vec::new(),
        }
    }

    /// Add a tactic result to the history.
    pub fn add_history(&mut self, result: TacticResult) {
        self.history.push(result);
    }

    /// Add available lemmas.
    pub fn with_lemmas(mut self, lemmas: Vec<String>) -> Self {
        self.available_lemmas = lemmas;
        self
    }

    /// Add similar proofs for reference.
    pub fn with_similar_proofs(mut self, proofs: Vec<ProofAttempt>) -> Self {
        self.similar_proofs = proofs;
        self
    }
}

/// Statistics about proof automation performance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProofStats {
    /// Total number of proof attempts.
    pub total_attempts: u64,

    /// Number of successful proofs.
    pub successful_proofs: u64,

    /// Breakdown by tier.
    pub by_tier: HashMap<AutomationTier, TierStats>,

    /// Breakdown by domain.
    pub by_domain: HashMap<SpecDomain, DomainStats>,

    /// Total time spent proving.
    pub total_time_ms: u64,
}

impl ProofStats {
    /// Record a proof attempt.
    pub fn record(&mut self, attempt: &ProofAttempt) {
        self.total_attempts += 1;
        self.total_time_ms += attempt.total_elapsed_ms;

        if attempt.success {
            self.successful_proofs += 1;
        }

        // Update tier stats
        let tier_stats = self.by_tier.entry(attempt.tier).or_default();
        tier_stats.attempts += 1;
        if attempt.success {
            tier_stats.successes += 1;
        }
        tier_stats.total_time_ms += attempt.total_elapsed_ms;

        // Update domain stats
        let domain_stats = self.by_domain.entry(attempt.domain).or_default();
        domain_stats.attempts += 1;
        if attempt.success {
            domain_stats.successes += 1;
        }
        domain_stats.total_time_ms += attempt.total_elapsed_ms;
    }

    /// Get overall success rate.
    pub fn success_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            0.0
        } else {
            self.successful_proofs as f64 / self.total_attempts as f64
        }
    }
}

/// Statistics for a specific automation tier.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TierStats {
    pub attempts: u64,
    pub successes: u64,
    pub total_time_ms: u64,
}

impl TierStats {
    pub fn success_rate(&self) -> f64 {
        if self.attempts == 0 {
            0.0
        } else {
            self.successes as f64 / self.attempts as f64
        }
    }
}

/// Statistics for a specific domain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomainStats {
    pub attempts: u64,
    pub successes: u64,
    pub total_time_ms: u64,
}

impl DomainStats {
    pub fn success_rate(&self) -> f64 {
        if self.attempts == 0 {
            0.0
        } else {
            self.successes as f64 / self.attempts as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_domain_inference() {
        // Arithmetic detection
        assert_eq!(SpecDomain::infer_from_goal("Nat.add_comm"), SpecDomain::Arithmetic);
        assert_eq!(SpecDomain::infer_from_goal("x : Nat |- x + 0 = x"), SpecDomain::Arithmetic);
        assert_eq!(SpecDomain::infer_from_goal("n : Nat, m : Nat |- n + m = m + n"), SpecDomain::Arithmetic);

        // Order detection
        assert_eq!(SpecDomain::infer_from_goal("x < y"), SpecDomain::Order);
        assert_eq!(SpecDomain::infer_from_goal("a <= b"), SpecDomain::Order);

        // Set theory detection
        assert_eq!(SpecDomain::infer_from_goal("a : Set Nat"), SpecDomain::SetTheory);

        // Logic detection
        assert_eq!(SpecDomain::infer_from_goal("true and false"), SpecDomain::Logic);

        // Type theory detection (equality without Nat context)
        assert_eq!(SpecDomain::infer_from_goal("a = b"), SpecDomain::TypeTheory);

        // Data structures detection
        assert_eq!(SpecDomain::infer_from_goal("List.length xs"), SpecDomain::DataStructures);

        // General fallback
        assert_eq!(SpecDomain::infer_from_goal("something_else"), SpecDomain::General);
    }

    #[test]
    fn test_automation_tier_escalation() {
        let tier = AutomationTier::Decidable;
        assert_eq!(tier.next(), Some(AutomationTier::Automation));

        let tier = AutomationTier::Automation;
        assert_eq!(tier.next(), Some(AutomationTier::AIAssisted));

        let tier = AutomationTier::AIAssisted;
        assert_eq!(tier.next(), Some(AutomationTier::HumanLoop));

        let tier = AutomationTier::HumanLoop;
        assert_eq!(tier.next(), None);
    }

    #[test]
    fn test_tactic_result() {
        let success = TacticResult::success("simp", vec![], 100);
        assert!(success.success);
        assert!(success.is_complete());

        let partial = TacticResult::success(
            "intro x",
            vec![Goal::from_string("x -> P x")],
            50,
        );
        assert!(partial.success);
        assert!(!partial.is_complete());
        assert!(partial.made_progress(2));

        let failure = TacticResult::failure("omega", "goal is not in omega fragment", 10);
        assert!(!failure.success);
        assert!(failure.error.is_some());
    }

    #[test]
    fn test_proof_attempt() {
        let goal = Goal::from_string("x : Nat |- x + 0 = x");
        let mut attempt = ProofAttempt::new(goal);

        assert_eq!(attempt.domain, SpecDomain::Arithmetic);
        assert!(!attempt.success);

        attempt.record_tactic(TacticResult::failure("decide", "not decidable", 10));
        attempt.record_tactic(TacticResult::success("simp", vec![], 50));
        attempt.mark_success(AutomationTier::Decidable);

        assert!(attempt.success);
        assert_eq!(attempt.tactics_tried.len(), 2);
        assert_eq!(attempt.successful_tactics, vec!["simp".to_string()]);
    }

    #[test]
    fn test_proof_strategy() {
        let mut strategy = ProofStrategy::new(
            SpecDomain::Arithmetic,
            vec!["omega".to_string(), "linarith".to_string()],
        );

        assert_eq!(strategy.success_rate, 0.0);

        strategy.record_usage(true);
        strategy.record_usage(true);
        strategy.record_usage(false);

        assert!((strategy.success_rate - 0.666).abs() < 0.01);
        assert_eq!(strategy.usage_count, 3);
        assert_eq!(strategy.success_count, 2);
    }

    #[test]
    fn test_boost_tactic() {
        let mut strategy = ProofStrategy::new(
            SpecDomain::Arithmetic,
            vec!["omega".to_string(), "linarith".to_string(), "ring".to_string()],
        );

        strategy.boost_tactic("ring");
        assert_eq!(strategy.preferred_tactics[0], "ring");
        assert_eq!(strategy.preferred_tactics[1], "omega");
        assert_eq!(strategy.preferred_tactics[2], "linarith");
    }

    #[test]
    fn test_proof_stats() {
        let mut stats = ProofStats::default();

        let goal = Goal::from_string("x + 0 = x");
        let mut attempt = ProofAttempt::new(goal);
        attempt.mark_success(AutomationTier::Decidable);
        attempt.total_elapsed_ms = 100;

        stats.record(&attempt);

        assert_eq!(stats.total_attempts, 1);
        assert_eq!(stats.successful_proofs, 1);
        assert_eq!(stats.success_rate(), 1.0);
    }
}
