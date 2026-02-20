//! Proof automation engine with tiered approach.
//!
//! This module provides the core proof automation engine that attempts
//! to prove goals using a tiered approach:
//!
//! 1. Decidable tactics (guaranteed termination)
//! 2. Automation tactics (search-based)
//! 3. AI-assisted tactics (LLM-generated)
//! 4. Human loop fallback (`sorry` marker for manual completion)

use crate::error::Result;
use crate::lean::repl::LeanRepl;
use crate::lean::types::Goal;
use crate::memory::{Node, NodeType, SqliteMemoryStore, Tier};
use crate::proof::tactics::{
    domain_specific_tactics, sorry_placeholder, tactic_variations, tactics_for_goal,
    tactics_for_tier,
};
use crate::proof::types::{
    AutomationTier, ProofAttempt, ProofContext, ProofStats, ProofStrategy, SpecDomain,
    TacticResult,
};
use std::collections::HashMap;
use std::time::Instant;

/// Configuration for the proof automation engine.
#[derive(Debug, Clone)]
pub struct ProofAutomationConfig {
    /// Maximum tactics to try per tier.
    pub max_tactics_per_tier: usize,

    /// Maximum time (ms) for decidable tier.
    pub decidable_timeout_ms: u64,

    /// Maximum time (ms) for automation tier.
    pub automation_timeout_ms: u64,

    /// Maximum time (ms) for AI-assisted tier.
    pub ai_timeout_ms: u64,

    /// Whether to use AI assistance.
    pub enable_ai: bool,

    /// Whether to learn from successful proofs.
    pub enable_learning: bool,

    /// Whether to try tactic variations.
    pub try_variations: bool,
}

impl Default for ProofAutomationConfig {
    fn default() -> Self {
        Self {
            max_tactics_per_tier: 20,
            decidable_timeout_ms: 5_000,
            automation_timeout_ms: 30_000,
            ai_timeout_ms: 60_000,
            enable_ai: true,
            enable_learning: true,
            try_variations: true,
        }
    }
}

/// Proof automation engine.
///
/// Attempts to prove goals using a tiered approach, learning from
/// successful proofs to improve future attempts.
pub struct ProofAutomation {
    /// Configuration.
    config: ProofAutomationConfig,

    /// Domain-specific strategies learned from successful proofs.
    strategies: HashMap<SpecDomain, Vec<ProofStrategy>>,

    /// Proof statistics.
    stats: ProofStats,

    /// Memory store for persisting learned strategies.
    memory: Option<SqliteMemoryStore>,
}

impl ProofAutomation {
    /// Create a new proof automation engine.
    pub fn new(config: ProofAutomationConfig) -> Self {
        let strategies = Self::initialize_default_strategies();

        Self {
            config,
            strategies,
            stats: ProofStats::default(),
            memory: None,
        }
    }

    /// Create with an in-memory store for learning.
    pub fn with_memory(config: ProofAutomationConfig, memory: SqliteMemoryStore) -> Self {
        let strategies = Self::initialize_default_strategies();

        Self {
            config,
            strategies,
            stats: ProofStats::default(),
            memory: Some(memory),
        }
    }

    /// Initialize default strategies for each domain.
    fn initialize_default_strategies() -> HashMap<SpecDomain, Vec<ProofStrategy>> {
        let mut strategies = HashMap::new();

        // Initialize strategies for each domain with domain-specific defaults
        for domain in [
            SpecDomain::Arithmetic,
            SpecDomain::SetTheory,
            SpecDomain::Order,
            SpecDomain::Algebra,
            SpecDomain::Logic,
            SpecDomain::TypeTheory,
            SpecDomain::DataStructures,
            SpecDomain::CategoryTheory,
            SpecDomain::General,
        ] {
            let tactics: Vec<String> = domain_specific_tactics(domain)
                .into_iter()
                .map(String::from)
                .collect();

            strategies.insert(domain, vec![ProofStrategy::new(domain, tactics)]);
        }

        strategies
    }

    /// Try to prove a goal using the tiered approach.
    pub fn prove(&mut self, repl: &mut LeanRepl, goal: &Goal) -> Result<ProofAttempt> {
        let mut attempt = ProofAttempt::new(goal.clone());
        let domain = attempt.domain;

        // Tier 1: Decidable tactics
        if let Some(result) = self.try_decidable(repl, goal, &mut attempt)? {
            if result.is_complete() {
                attempt.mark_success(AutomationTier::Decidable);
                self.record_success(goal, &result.tactic, domain);
                self.stats.record(&attempt);
                return Ok(attempt);
            }
        }

        // Tier 2: Automation tactics
        if let Some(result) = self.try_automation(repl, goal, &mut attempt)? {
            if result.is_complete() {
                attempt.mark_success(AutomationTier::Automation);
                self.record_success(goal, &result.tactic, domain);
                self.stats.record(&attempt);
                return Ok(attempt);
            }
        }

        // Tier 3: AI-assisted tactic synthesis and execution.
        if self.config.enable_ai {
            if let Some(result) = self.try_ai_assisted(repl, goal, &mut attempt)? {
                if result.is_complete() {
                    attempt.mark_success(AutomationTier::AIAssisted);
                    self.record_success(goal, &result.tactic, domain);
                    self.stats.record(&attempt);
                    return Ok(attempt);
                }
            }
        }

        // Tier 4: Human fallback
        let sorry = self.mark_for_human(goal);
        attempt.record_tactic(TacticResult::success(sorry, vec![goal.clone()], 0));
        attempt.mark_failure(AutomationTier::HumanLoop);
        self.stats.record(&attempt);

        Ok(attempt)
    }

    /// Try decidable tactics (Tier 1).
    fn try_decidable(
        &self,
        repl: &mut LeanRepl,
        goal: &Goal,
        attempt: &mut ProofAttempt,
    ) -> Result<Option<TacticResult>> {
        let start = Instant::now();
        let mut tactics = tactics_for_tier(AutomationTier::Decidable);

        // Add learned tactics from strategies
        if let Some(strategies) = self.strategies.get(&attempt.domain) {
            for strategy in strategies {
                for tactic in &strategy.preferred_tactics {
                    if !tactics.iter().any(|t| *t == tactic.as_str()) {
                        tactics.push(Box::leak(tactic.clone().into_boxed_str()));
                    }
                }
            }
        }

        // Limit tactics
        tactics.truncate(self.config.max_tactics_per_tier);

        for tactic in tactics {
            // Check timeout
            if start.elapsed().as_millis() as u64 > self.config.decidable_timeout_ms {
                break;
            }

            let result = self.try_single_tactic(repl, goal, tactic)?;
            attempt.record_tactic(result.clone());

            if result.is_complete() {
                return Ok(Some(result));
            }

            // Also try variations if enabled
            if self.config.try_variations {
                for variant in tactic_variations(tactic, goal) {
                    if start.elapsed().as_millis() as u64 > self.config.decidable_timeout_ms {
                        break;
                    }

                    let result = self.try_single_tactic(repl, goal, &variant)?;
                    attempt.record_tactic(result.clone());

                    if result.is_complete() {
                        return Ok(Some(result));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Try automation tactics (Tier 2).
    fn try_automation(
        &self,
        repl: &mut LeanRepl,
        goal: &Goal,
        attempt: &mut ProofAttempt,
    ) -> Result<Option<TacticResult>> {
        let start = Instant::now();
        let mut tactics = tactics_for_tier(AutomationTier::Automation);

        // Add goal-specific tactics
        for tactic in tactics_for_goal(goal) {
            if !tactics.contains(&tactic) {
                tactics.push(tactic);
            }
        }

        // Limit tactics
        tactics.truncate(self.config.max_tactics_per_tier);

        for tactic in tactics {
            // Check timeout
            if start.elapsed().as_millis() as u64 > self.config.automation_timeout_ms {
                break;
            }

            let result = self.try_single_tactic(repl, goal, tactic)?;
            attempt.record_tactic(result.clone());

            if result.is_complete() {
                return Ok(Some(result));
            }

            // Try variations
            if self.config.try_variations {
                for variant in tactic_variations(tactic, goal) {
                    if start.elapsed().as_millis() as u64 > self.config.automation_timeout_ms {
                        break;
                    }

                    let result = self.try_single_tactic(repl, goal, &variant)?;
                    attempt.record_tactic(result.clone());

                    if result.is_complete() {
                        return Ok(Some(result));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Try AI-assisted tactics (Tier 3).
    ///
    /// This tier synthesizes a broader candidate pool from domain tactics,
    /// goal-shape tactics, and learned strategy order, then executes them
    /// with the same Lean feedback loop used by other tiers.
    fn try_ai_assisted(
        &self,
        repl: &mut LeanRepl,
        goal: &Goal,
        attempt: &mut ProofAttempt,
    ) -> Result<Option<TacticResult>> {
        let start = Instant::now();
        let candidates = self.build_ai_tactic_candidates(goal, attempt);
        let mut best_progress: Option<TacticResult> = None;

        for tactic in candidates {
            if start.elapsed().as_millis() as u64 > self.config.ai_timeout_ms {
                break;
            }

            let result = self.try_single_tactic(repl, goal, &tactic)?;
            attempt.record_tactic(result.clone());

            if result.is_complete() {
                return Ok(Some(result));
            }

            if result.success {
                let should_replace = best_progress
                    .as_ref()
                    .map(|best| result.new_goals.len() < best.new_goals.len())
                    .unwrap_or(true);
                if should_replace {
                    best_progress = Some(result);
                }
            }
        }

        Ok(best_progress)
    }

    /// Mark a goal for human intervention (Tier 4).
    fn mark_for_human(&self, goal: &Goal) -> String {
        sorry_placeholder(goal)
    }

    /// Try a single tactic and return the result.
    fn try_single_tactic(
        &self,
        repl: &mut LeanRepl,
        _goal: &Goal,
        tactic: &str,
    ) -> Result<TacticResult> {
        let start = Instant::now();

        // We need a proof state to apply the tactic
        // If we don't have one, we need to create a theorem with the goal
        let proof_state_id = match repl.current_env() {
            Some(_) => {
                // Try to get or create a proof state for this goal
                // This is a simplified version - in practice, we'd track proof states
                0u64 // Placeholder - actual implementation would track this
            }
            None => {
                // No environment, need to initialize
                0u64
            }
        };

        // Apply the tactic
        let response = repl.apply_tactic(tactic, proof_state_id);
        let elapsed_ms = start.elapsed().as_millis() as u64;

        match response {
            Ok(resp) => {
                if resp.has_errors() {
                    let error = resp.format_errors();
                    Ok(TacticResult::failure(tactic, error, elapsed_ms))
                } else {
                    // Parse the remaining goals
                    let new_goals: Vec<Goal> = resp
                        .goals
                        .map(|goals| goals.into_iter().map(Goal::from_string).collect())
                        .unwrap_or_default();

                    Ok(TacticResult::success(tactic, new_goals, elapsed_ms))
                }
            }
            Err(e) => Ok(TacticResult::failure(tactic, e.to_string(), elapsed_ms)),
        }
    }

    /// Record a successful proof for learning.
    pub fn record_success(&mut self, goal: &Goal, tactic: &str, domain: SpecDomain) {
        if !self.config.enable_learning {
            return;
        }

        // Update strategy for this domain
        let strategies = self.strategies.entry(domain).or_insert_with(Vec::new);

        if let Some(strategy) = strategies.first_mut() {
            strategy.record_usage(true);
            strategy.boost_tactic(tactic);
        }

        // Persist to memory if available.
        self.persist_success_pattern(goal, tactic, domain);
    }

    /// Get the current proof statistics.
    pub fn stats(&self) -> &ProofStats {
        &self.stats
    }

    /// Get strategies for a domain.
    pub fn strategies_for_domain(&self, domain: SpecDomain) -> Option<&Vec<ProofStrategy>> {
        self.strategies.get(&domain)
    }

    /// Create a proof context for AI-assisted proving.
    pub fn create_context(&self, goal: &Goal, attempt: &ProofAttempt) -> ProofContext {
        let mut context = ProofContext::new(goal.clone());

        // Add history from the current attempt
        for result in &attempt.tactics_tried {
            context.add_history(result.clone());
        }

        // Include strategy-ordered tactic hints as available lemmas/hints.
        if let Some(strategies) = self.strategies.get(&attempt.domain) {
            if let Some(strategy) = strategies.first() {
                context.available_lemmas = strategy
                    .preferred_tactics
                    .iter()
                    .take(8)
                    .map(|t| format!("tactic_hint:{t}"))
                    .collect();
            }
        }

        // Load similar proof patterns from memory when available.
        if let Some(memory) = &self.memory {
            if let Ok(nodes) = memory.search_content("proof_pattern", 20) {
                let mut similar = Vec::new();
                for node in nodes {
                    let is_pattern = node
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("kind"))
                        .and_then(|v| v.as_str())
                        .map(|k| k == "proof_pattern")
                        .unwrap_or(false);
                    if !is_pattern {
                        continue;
                    }

                    let same_goal = node
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("goal"))
                        .and_then(|v| v.as_str())
                        .map(|g| g == goal.target)
                        .unwrap_or(false);
                    if !same_goal {
                        continue;
                    }

                    let mut past = ProofAttempt::new(goal.clone());
                    let tactic = node
                        .metadata
                        .as_ref()
                        .and_then(|m| m.get("tactic"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("simp");
                    past.record_tactic(TacticResult::success(tactic, vec![goal.clone()], 0));
                    past.mark_success(AutomationTier::AIAssisted);
                    similar.push(past);
                }
                context.similar_proofs = similar;
            }
        }

        context
    }

    fn build_ai_tactic_candidates(&self, goal: &Goal, attempt: &ProofAttempt) -> Vec<String> {
        let mut candidates: Vec<String> = Vec::new();

        // Start with tier-specific suggestions if defined.
        for tactic in tactics_for_tier(AutomationTier::AIAssisted) {
            candidates.push(tactic.to_string());
        }

        // Add domain-specific tactics.
        for tactic in domain_specific_tactics(attempt.domain) {
            candidates.push(tactic.to_string());
        }

        // Add goal-shape tactics.
        for tactic in tactics_for_goal(goal) {
            candidates.push(tactic.to_string());
        }

        // Prefer tactics already boosted by learned strategies.
        if let Some(strategies) = self.strategies.get(&attempt.domain) {
            for strategy in strategies {
                for tactic in &strategy.preferred_tactics {
                    candidates.push(tactic.clone());
                }
            }
        }

        // Deduplicate while preserving order and cap by tier budget.
        let mut seen = std::collections::HashSet::new();
        let mut unique = Vec::new();
        for tactic in candidates {
            if seen.insert(tactic.clone()) {
                unique.push(tactic);
            }
            if unique.len() >= self.config.max_tactics_per_tier {
                break;
            }
        }

        unique
    }

    fn persist_success_pattern(&self, goal: &Goal, tactic: &str, domain: SpecDomain) {
        let Some(memory) = &self.memory else {
            return;
        };

        let node = Node::new(
            NodeType::Experience,
            format!("proof_pattern:{}:{}:{}", domain, tactic, goal.target),
        )
        .with_tier(Tier::Session)
        .with_confidence(0.9)
        .with_metadata("kind", "proof_pattern")
        .with_metadata("domain", domain.to_string())
        .with_metadata("goal", goal.target.clone())
        .with_metadata("tactic", tactic.to_string());

        let _ = memory.add_node(&node);
    }
}

/// Builder for ProofAutomation with fluent API.
pub struct ProofAutomationBuilder {
    config: ProofAutomationConfig,
    memory: Option<SqliteMemoryStore>,
}

impl ProofAutomationBuilder {
    /// Create a new builder with default config.
    pub fn new() -> Self {
        Self {
            config: ProofAutomationConfig::default(),
            memory: None,
        }
    }

    /// Set the maximum tactics per tier.
    pub fn max_tactics_per_tier(mut self, max: usize) -> Self {
        self.config.max_tactics_per_tier = max;
        self
    }

    /// Set the decidable tier timeout.
    pub fn decidable_timeout(mut self, timeout_ms: u64) -> Self {
        self.config.decidable_timeout_ms = timeout_ms;
        self
    }

    /// Set the automation tier timeout.
    pub fn automation_timeout(mut self, timeout_ms: u64) -> Self {
        self.config.automation_timeout_ms = timeout_ms;
        self
    }

    /// Enable or disable AI assistance.
    pub fn enable_ai(mut self, enable: bool) -> Self {
        self.config.enable_ai = enable;
        self
    }

    /// Enable or disable learning.
    pub fn enable_learning(mut self, enable: bool) -> Self {
        self.config.enable_learning = enable;
        self
    }

    /// Enable or disable tactic variations.
    pub fn try_variations(mut self, enable: bool) -> Self {
        self.config.try_variations = enable;
        self
    }

    /// Set the memory store for learning.
    pub fn with_memory(mut self, memory: SqliteMemoryStore) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Build the proof automation engine.
    pub fn build(self) -> ProofAutomation {
        match self.memory {
            Some(memory) => ProofAutomation::with_memory(self.config, memory),
            None => ProofAutomation::new(self.config),
        }
    }
}

impl Default for ProofAutomationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ProofAutomationConfig::default();
        assert_eq!(config.max_tactics_per_tier, 20);
        assert_eq!(config.decidable_timeout_ms, 5_000);
        assert!(config.enable_ai);
        assert!(config.enable_learning);
    }

    #[test]
    fn test_automation_creation() {
        let automation = ProofAutomation::new(ProofAutomationConfig::default());
        assert!(automation.strategies.contains_key(&SpecDomain::Arithmetic));
        assert!(automation.strategies.contains_key(&SpecDomain::Logic));
    }

    #[test]
    fn test_builder() {
        let automation = ProofAutomationBuilder::new()
            .max_tactics_per_tier(10)
            .decidable_timeout(1000)
            .enable_ai(false)
            .build();

        assert_eq!(automation.config.max_tactics_per_tier, 10);
        assert_eq!(automation.config.decidable_timeout_ms, 1000);
        assert!(!automation.config.enable_ai);
    }

    #[test]
    fn test_record_success() {
        let mut automation = ProofAutomation::new(ProofAutomationConfig::default());
        let goal = Goal::from_string("x + 0 = x");

        automation.record_success(&goal, "simp", SpecDomain::Arithmetic);

        let strategies = automation.strategies_for_domain(SpecDomain::Arithmetic).unwrap();
        assert!(!strategies.is_empty());
        assert!(strategies[0].success_count > 0);
    }

    #[test]
    fn test_mark_for_human() {
        let automation = ProofAutomation::new(ProofAutomationConfig::default());
        let goal = Goal::from_string("complex_theorem");

        let sorry = automation.mark_for_human(&goal);
        assert!(sorry.contains("sorry"));
        assert!(sorry.contains("TODO"));
    }

    #[test]
    fn test_create_context() {
        let automation = ProofAutomation::new(ProofAutomationConfig::default());
        let goal = Goal::from_string("n : Nat |- n + 0 = n");
        let attempt = ProofAttempt::new(goal.clone());

        let context = automation.create_context(&goal, &attempt);
        assert_eq!(context.domain, SpecDomain::Arithmetic);
    }

    #[test]
    fn test_ai_candidate_tactics_not_empty() {
        let automation = ProofAutomation::new(ProofAutomationConfig::default());
        let goal = Goal::from_string("n + 0 = n");
        let attempt = ProofAttempt::new(goal.clone());

        let candidates = automation.build_ai_tactic_candidates(&goal, &attempt);
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_record_success_persists_pattern_when_memory_enabled() {
        let memory = SqliteMemoryStore::in_memory().expect("memory store should be created");
        let mut automation = ProofAutomation::with_memory(ProofAutomationConfig::default(), memory);
        let goal = Goal::from_string("x + 0 = x");

        automation.record_success(&goal, "simp", SpecDomain::Arithmetic);

        let store = automation.memory.as_ref().expect("memory should be enabled");
        let nodes = store
            .search_content("proof_pattern", 10)
            .expect("search should succeed");
        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_create_context_loads_similar_proofs_from_memory() {
        let memory = SqliteMemoryStore::in_memory().expect("memory store should be created");
        let mut automation = ProofAutomation::with_memory(ProofAutomationConfig::default(), memory);
        let goal = Goal::from_string("x + 0 = x");
        let attempt = ProofAttempt::new(goal.clone());

        automation.record_success(&goal, "simp", SpecDomain::Arithmetic);
        let context = automation.create_context(&goal, &attempt);

        assert!(!context.similar_proofs.is_empty());
        assert!(!context.available_lemmas.is_empty());
    }
}
