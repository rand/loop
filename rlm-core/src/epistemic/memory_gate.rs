//! Memory gate integration for epistemic verification.
//!
//! This module provides a gate that rejects ungrounded facts from being
//! stored in long-term memory. Only claims that pass epistemic verification
//! are promoted to persistent storage.

use std::sync::Arc;

use crate::error::Result;
use crate::memory::{Node, NodeType, Tier};
use crate::trajectory::{TrajectoryEvent, TrajectoryEventType};

use super::types::{BudgetResult, Claim, ClaimCategory, GroundingStatus};
use super::verifier::EpistemicVerifier;

/// Configuration for the memory gate.
#[derive(Debug, Clone)]
pub struct MemoryGateConfig {
    /// Budget gap threshold for rejecting facts
    pub rejection_threshold: f64,
    /// Whether to allow weakly grounded facts (with reduced confidence)
    pub allow_weak_grounding: bool,
    /// Confidence reduction factor for weakly grounded facts
    pub weak_grounding_penalty: f64,
    /// Node types that require verification
    pub verified_types: Vec<NodeType>,
    /// Minimum tier that requires verification (lower tiers pass through)
    pub min_verified_tier: Tier,
    /// Whether to verify on promotion (tier upgrade)
    pub verify_on_promotion: bool,
}

impl Default for MemoryGateConfig {
    fn default() -> Self {
        Self {
            rejection_threshold: 0.5,
            allow_weak_grounding: true,
            weak_grounding_penalty: 0.3,
            verified_types: vec![NodeType::Fact, NodeType::Entity],
            min_verified_tier: Tier::Session,
            verify_on_promotion: true,
        }
    }
}

impl MemoryGateConfig {
    /// Strict configuration that rejects all ungrounded facts.
    pub fn strict() -> Self {
        Self {
            rejection_threshold: 0.3,
            allow_weak_grounding: false,
            weak_grounding_penalty: 0.0,
            verified_types: vec![
                NodeType::Fact,
                NodeType::Entity,
                NodeType::Experience,
                NodeType::Decision,
            ],
            min_verified_tier: Tier::Task,
            verify_on_promotion: true,
        }
    }

    /// Permissive configuration that only rejects egregious hallucinations.
    pub fn permissive() -> Self {
        Self {
            rejection_threshold: 1.0,
            allow_weak_grounding: true,
            weak_grounding_penalty: 0.1,
            verified_types: vec![NodeType::Fact],
            min_verified_tier: Tier::LongTerm,
            verify_on_promotion: false,
        }
    }
}

/// Result of a memory gate decision.
#[derive(Debug, Clone)]
pub struct GateDecision {
    /// Whether the node is allowed
    pub allowed: bool,
    /// Reason for the decision
    pub reason: String,
    /// Adjusted confidence (if allowed)
    pub adjusted_confidence: Option<f64>,
    /// Budget result from verification (if performed)
    pub budget_result: Option<BudgetResult>,
    /// Recommendation for the node
    pub recommendation: GateRecommendation,
}

/// Recommendation from the memory gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateRecommendation {
    /// Allow the node as-is
    Allow,
    /// Allow with reduced confidence
    AllowWithPenalty,
    /// Reject the node
    Reject,
    /// Defer decision (needs more context)
    Defer,
    /// Promote to verification queue
    QueueForVerification,
}

impl std::fmt::Display for GateRecommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allow => write!(f, "allow"),
            Self::AllowWithPenalty => write!(f, "allow_with_penalty"),
            Self::Reject => write!(f, "reject"),
            Self::Defer => write!(f, "defer"),
            Self::QueueForVerification => write!(f, "queue_for_verification"),
        }
    }
}

/// Memory gate that filters ungrounded facts.
pub struct MemoryGate<V: EpistemicVerifier> {
    verifier: Arc<V>,
    config: MemoryGateConfig,
}

impl<V: EpistemicVerifier> MemoryGate<V> {
    /// Create a new memory gate.
    pub fn new(verifier: Arc<V>, config: MemoryGateConfig) -> Self {
        Self { verifier, config }
    }

    /// Check if a node requires verification.
    pub fn requires_verification(&self, node: &Node) -> bool {
        // Check node type
        if !self.config.verified_types.contains(&node.node_type) {
            return false;
        }

        // Check tier
        if node.tier < self.config.min_verified_tier {
            return false;
        }

        true
    }

    /// Evaluate a node for storage.
    pub async fn evaluate(&self, node: &Node, context: &str) -> Result<GateDecision> {
        // Skip verification for non-verified types/tiers
        if !self.requires_verification(node) {
            return Ok(GateDecision {
                allowed: true,
                reason: "Node type/tier does not require verification".to_string(),
                adjusted_confidence: Some(node.confidence),
                budget_result: None,
                recommendation: GateRecommendation::Allow,
            });
        }

        // Convert node content to a claim
        let claim = self.node_to_claim(node);

        // Verify the claim
        let evidence: Vec<String> = Vec::new(); // Could extract from node metadata
        let budget_result = self
            .verifier
            .verify_claim(&claim, context, &evidence)
            .await?;

        // Make decision based on budget result
        self.make_decision(node, budget_result)
    }

    /// Evaluate a promotion (tier upgrade).
    pub async fn evaluate_promotion(
        &self,
        node: &Node,
        new_tier: Tier,
        context: &str,
    ) -> Result<GateDecision> {
        if !self.config.verify_on_promotion {
            return Ok(GateDecision {
                allowed: true,
                reason: "Promotion verification disabled".to_string(),
                adjusted_confidence: Some(node.confidence),
                budget_result: None,
                recommendation: GateRecommendation::Allow,
            });
        }

        // Only verify if promoting to or above verified tier
        if new_tier < self.config.min_verified_tier {
            return Ok(GateDecision {
                allowed: true,
                reason: "Target tier below verification threshold".to_string(),
                adjusted_confidence: Some(node.confidence),
                budget_result: None,
                recommendation: GateRecommendation::Allow,
            });
        }

        self.evaluate(node, context).await
    }

    /// Convert a memory node to a claim for verification.
    fn node_to_claim(&self, node: &Node) -> Claim {
        let category = match node.node_type {
            NodeType::Fact => ClaimCategory::Factual,
            NodeType::Entity => ClaimCategory::CodeBehavior,
            NodeType::Experience => ClaimCategory::UserIntent,
            NodeType::Decision => ClaimCategory::MetaReasoning,
            NodeType::Snippet => ClaimCategory::CodeBehavior,
        };

        // Estimate specificity from content
        let specificity = self.estimate_node_specificity(node);

        Claim::new(&node.content, category).with_specificity(specificity)
    }

    /// Estimate the specificity of a node's content.
    fn estimate_node_specificity(&self, node: &Node) -> f64 {
        let content = &node.content;
        let mut specificity = 0.5;

        // Identifiers increase specificity
        let identifier_count = content
            .chars()
            .filter(|c| c.is_uppercase())
            .count();
        specificity += (identifier_count as f64 * 0.02).min(0.2);

        // Numbers increase specificity
        let number_count = content
            .chars()
            .filter(|c| c.is_numeric())
            .count();
        specificity += (number_count as f64 * 0.01).min(0.15);

        // Length affects specificity (longer = potentially more specific)
        if content.len() > 100 {
            specificity += 0.1;
        }

        specificity.clamp(0.2, 0.9)
    }

    /// Make a gate decision based on the budget result.
    fn make_decision(&self, node: &Node, budget_result: BudgetResult) -> Result<GateDecision> {
        let budget_gap = budget_result.budget_gap;

        // Grounded: allow as-is
        if budget_result.status == GroundingStatus::Grounded {
            return Ok(GateDecision {
                allowed: true,
                reason: format!(
                    "Claim grounded (budget_gap={:.2}, observed={:.2} bits)",
                    budget_gap, budget_result.observed_bits
                ),
                adjusted_confidence: Some(node.confidence),
                budget_result: Some(budget_result),
                recommendation: GateRecommendation::Allow,
            });
        }

        // Weakly grounded: allow with penalty if configured
        if budget_result.status == GroundingStatus::WeaklyGrounded {
            if self.config.allow_weak_grounding {
                let adjusted = node.confidence * (1.0 - self.config.weak_grounding_penalty);
                return Ok(GateDecision {
                    allowed: true,
                    reason: format!(
                        "Claim weakly grounded (budget_gap={:.2}), confidence reduced",
                        budget_gap
                    ),
                    adjusted_confidence: Some(adjusted),
                    budget_result: Some(budget_result),
                    recommendation: GateRecommendation::AllowWithPenalty,
                });
            }
        }

        // Ungrounded: check threshold
        if budget_gap > self.config.rejection_threshold {
            return Ok(GateDecision {
                allowed: false,
                reason: format!(
                    "Claim ungrounded (budget_gap={:.2} > threshold={:.2})",
                    budget_gap, self.config.rejection_threshold
                ),
                adjusted_confidence: None,
                budget_result: Some(budget_result),
                recommendation: GateRecommendation::Reject,
            });
        }

        // Uncertain: defer
        if budget_result.status == GroundingStatus::Uncertain {
            return Ok(GateDecision {
                allowed: false,
                reason: "Verification uncertain, deferring decision".to_string(),
                adjusted_confidence: None,
                budget_result: Some(budget_result),
                recommendation: GateRecommendation::Defer,
            });
        }

        // Default: allow with penalty
        let adjusted = node.confidence * 0.7;
        Ok(GateDecision {
            allowed: true,
            reason: "Marginal grounding, allowing with reduced confidence".to_string(),
            adjusted_confidence: Some(adjusted),
            budget_result: Some(budget_result),
            recommendation: GateRecommendation::AllowWithPenalty,
        })
    }

    /// Batch evaluate multiple nodes.
    pub async fn evaluate_batch(
        &self,
        nodes: &[Node],
        context: &str,
    ) -> Vec<Result<GateDecision>> {
        let mut results = Vec::new();

        for node in nodes {
            results.push(self.evaluate(node, context).await);
        }

        results
    }

    /// Create a trajectory event for a gate decision.
    pub fn create_event(&self, node: &Node, decision: &GateDecision) -> TrajectoryEvent {
        let content = format!(
            "Memory gate: {} - {} ({})",
            if decision.allowed { "ALLOWED" } else { "REJECTED" },
            &node.content[..node.content.len().min(50)],
            decision.reason
        );

        let event_type = if decision.allowed {
            TrajectoryEventType::Memory
        } else {
            TrajectoryEventType::HallucinationFlag
        };

        let mut event = TrajectoryEvent::new(event_type, 0, content);

        if let Some(ref budget) = decision.budget_result {
            event = event
                .with_metadata("budget_gap", budget.budget_gap)
                .with_metadata("status", budget.status.to_string());
        }

        event = event.with_metadata("recommendation", decision.recommendation.to_string());

        event
    }
}

/// Simple gate that uses threshold-based filtering without full verification.
///
/// Useful for high-throughput scenarios where full LLM-based verification
/// is too expensive.
pub struct ThresholdGate {
    config: MemoryGateConfig,
}

impl ThresholdGate {
    /// Create a new threshold gate.
    pub fn new(config: MemoryGateConfig) -> Self {
        Self { config }
    }

    /// Evaluate a node based on heuristics (no LLM calls).
    pub fn evaluate(&self, node: &Node) -> GateDecision {
        // Skip non-verified types
        if !self.config.verified_types.contains(&node.node_type) {
            return GateDecision {
                allowed: true,
                reason: "Type not verified".to_string(),
                adjusted_confidence: Some(node.confidence),
                budget_result: None,
                recommendation: GateRecommendation::Allow,
            };
        }

        // Check existing confidence
        if node.confidence < 0.3 {
            return GateDecision {
                allowed: false,
                reason: "Low confidence".to_string(),
                adjusted_confidence: None,
                budget_result: None,
                recommendation: GateRecommendation::Reject,
            };
        }

        // Check for hedge words
        let content_lower = node.content.to_lowercase();
        let hedge_words = ["might", "could", "possibly", "perhaps", "maybe", "uncertain"];
        let has_hedge = hedge_words.iter().any(|w| content_lower.contains(w));

        if has_hedge {
            let adjusted = node.confidence * 0.7;
            return GateDecision {
                allowed: true,
                reason: "Contains hedge words".to_string(),
                adjusted_confidence: Some(adjusted),
                budget_result: None,
                recommendation: GateRecommendation::AllowWithPenalty,
            };
        }

        // Check for unsupported universal claims
        let universal_words = ["always", "never", "all", "none", "every"];
        let has_universal = universal_words.iter().any(|w| content_lower.contains(w));

        if has_universal && node.confidence < 0.8 {
            return GateDecision {
                allowed: false,
                reason: "Universal claim with insufficient confidence".to_string(),
                adjusted_confidence: None,
                budget_result: None,
                recommendation: GateRecommendation::QueueForVerification,
            };
        }

        GateDecision {
            allowed: true,
            reason: "Passed heuristic checks".to_string(),
            adjusted_confidence: Some(node.confidence),
            budget_result: None,
            recommendation: GateRecommendation::Allow,
        }
    }
}

/// Statistics from gate operations.
#[derive(Debug, Clone, Default)]
pub struct GateStats {
    /// Total nodes evaluated
    pub total_evaluated: u64,
    /// Nodes allowed
    pub allowed: u64,
    /// Nodes rejected
    pub rejected: u64,
    /// Nodes allowed with penalty
    pub allowed_with_penalty: u64,
    /// Nodes deferred
    pub deferred: u64,
    /// Average budget gap for evaluated nodes
    pub avg_budget_gap: f64,
}

impl GateStats {
    /// Calculate rejection rate.
    pub fn rejection_rate(&self) -> f64 {
        if self.total_evaluated == 0 {
            0.0
        } else {
            self.rejected as f64 / self.total_evaluated as f64
        }
    }

    /// Update stats from a decision.
    pub fn record(&mut self, decision: &GateDecision) {
        self.total_evaluated += 1;

        match decision.recommendation {
            GateRecommendation::Allow => self.allowed += 1,
            GateRecommendation::AllowWithPenalty => self.allowed_with_penalty += 1,
            GateRecommendation::Reject => self.rejected += 1,
            GateRecommendation::Defer => self.deferred += 1,
            GateRecommendation::QueueForVerification => self.deferred += 1,
        }

        if let Some(ref budget) = decision.budget_result {
            // Running average
            let n = self.total_evaluated as f64;
            self.avg_budget_gap =
                self.avg_budget_gap * (n - 1.0) / n + budget.budget_gap / n;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_node(content: &str, node_type: NodeType, tier: Tier) -> Node {
        Node {
            id: crate::memory::NodeId::new(),
            node_type,
            subtype: None,
            content: content.to_string(),
            embedding: None,
            tier,
            confidence: 0.8,
            provenance: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 0,
            metadata: None,
        }
    }

    #[test]
    fn test_requires_verification() {
        // Create a mock verifier - we'll just test the gate logic
        let config = MemoryGateConfig::default();

        // Fact at Session tier should require verification
        let node = create_test_node("Test fact", NodeType::Fact, Tier::Session);
        assert!(config.verified_types.contains(&node.node_type));
        assert!(node.tier >= config.min_verified_tier);

        // Snippet at Task tier should not require verification
        let node = create_test_node("Test snippet", NodeType::Snippet, Tier::Task);
        assert!(!config.verified_types.contains(&node.node_type));
    }

    #[test]
    fn test_threshold_gate_hedge_words() {
        let gate = ThresholdGate::new(MemoryGateConfig::default());

        let node = create_test_node(
            "The function might return null",
            NodeType::Fact,
            Tier::Session,
        );

        let decision = gate.evaluate(&node);
        assert!(decision.allowed);
        assert_eq!(decision.recommendation, GateRecommendation::AllowWithPenalty);
        assert!(decision.adjusted_confidence.unwrap() < node.confidence);
    }

    #[test]
    fn test_threshold_gate_low_confidence() {
        let gate = ThresholdGate::new(MemoryGateConfig::default());

        let mut node = create_test_node("Some fact", NodeType::Fact, Tier::Session);
        node.confidence = 0.2;

        let decision = gate.evaluate(&node);
        assert!(!decision.allowed);
        assert_eq!(decision.recommendation, GateRecommendation::Reject);
    }

    #[test]
    fn test_threshold_gate_universal_claim() {
        let gate = ThresholdGate::new(MemoryGateConfig::default());

        let mut node = create_test_node(
            "This function always returns true",
            NodeType::Fact,
            Tier::Session,
        );
        node.confidence = 0.6;

        let decision = gate.evaluate(&node);
        assert!(!decision.allowed);
        assert_eq!(
            decision.recommendation,
            GateRecommendation::QueueForVerification
        );
    }

    #[test]
    fn test_gate_stats() {
        let mut stats = GateStats::default();

        stats.record(&GateDecision {
            allowed: true,
            reason: "Test".to_string(),
            adjusted_confidence: Some(0.8),
            budget_result: None,
            recommendation: GateRecommendation::Allow,
        });

        stats.record(&GateDecision {
            allowed: false,
            reason: "Test".to_string(),
            adjusted_confidence: None,
            budget_result: None,
            recommendation: GateRecommendation::Reject,
        });

        assert_eq!(stats.total_evaluated, 2);
        assert_eq!(stats.allowed, 1);
        assert_eq!(stats.rejected, 1);
        assert!((stats.rejection_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_config_presets() {
        let strict = MemoryGateConfig::strict();
        assert!(strict.rejection_threshold < 0.5);
        assert!(!strict.allow_weak_grounding);

        let permissive = MemoryGateConfig::permissive();
        assert!(permissive.rejection_threshold >= 1.0);
        assert!(permissive.allow_weak_grounding);
    }
}
