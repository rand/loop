//! Core types for epistemic verification.
//!
//! Implements information-theoretic hallucination detection based on the
//! Strawberry/Pythea methodology. The key insight is that claims should only
//! be believed if they are grounded in evidence - the information gain from
//! evidence should account for the claim's specificity.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a claim.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClaimId(pub Uuid);

impl ClaimId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ClaimId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ClaimId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An atomic claim extracted from an LLM response.
///
/// Claims are the unit of epistemic verification. Each claim represents
/// a single factual assertion that can be evaluated for grounding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    /// Unique identifier
    pub id: ClaimId,
    /// The claim text (atomic proposition)
    pub text: String,
    /// Source span in the original response (start, end)
    pub source_span: Option<(usize, usize)>,
    /// Category of the claim
    pub category: ClaimCategory,
    /// Specificity level (higher = more specific = needs more evidence)
    pub specificity: f64,
    /// References to evidence supporting this claim
    pub evidence_refs: Vec<EvidenceRef>,
    /// When the claim was extracted
    pub extracted_at: DateTime<Utc>,
    /// Additional metadata
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl Claim {
    /// Create a new claim.
    pub fn new(text: impl Into<String>, category: ClaimCategory) -> Self {
        Self {
            id: ClaimId::new(),
            text: text.into(),
            source_span: None,
            category,
            specificity: 0.5, // Default medium specificity
            evidence_refs: Vec::new(),
            extracted_at: Utc::now(),
            metadata: None,
        }
    }

    /// Set the source span.
    pub fn with_span(mut self, start: usize, end: usize) -> Self {
        self.source_span = Some((start, end));
        self
    }

    /// Set the specificity.
    pub fn with_specificity(mut self, specificity: f64) -> Self {
        self.specificity = specificity.clamp(0.0, 1.0);
        self
    }

    /// Add an evidence reference.
    pub fn with_evidence(mut self, evidence: EvidenceRef) -> Self {
        self.evidence_refs.push(evidence);
        self
    }

    /// Calculate required bits based on specificity.
    /// More specific claims require more evidence to justify.
    pub fn required_bits(&self) -> f64 {
        // Base requirement scales with specificity
        // A claim with specificity 0.5 requires ~1 bit
        // A claim with specificity 0.9 requires ~3.3 bits
        // A claim with specificity 0.99 requires ~6.6 bits
        // Using -log2(1-s) so higher specificity = more bits required
        let s = self.specificity.clamp(0.01, 0.999);
        -(1.0 - s).log2()
    }
}

/// Category of a claim for different verification strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimCategory {
    /// Factual claim about the world (verifiable)
    Factual,
    /// Claim about code behavior or structure
    CodeBehavior,
    /// Claim about relationships or dependencies
    Relational,
    /// Numerical or quantitative claim
    Numerical,
    /// Temporal claim (ordering, timing)
    Temporal,
    /// Claim about user intent or preferences
    UserIntent,
    /// Meta-level claim about the reasoning process
    MetaReasoning,
    /// Unclassified claim
    Unknown,
}

impl std::fmt::Display for ClaimCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Factual => write!(f, "factual"),
            Self::CodeBehavior => write!(f, "code_behavior"),
            Self::Relational => write!(f, "relational"),
            Self::Numerical => write!(f, "numerical"),
            Self::Temporal => write!(f, "temporal"),
            Self::UserIntent => write!(f, "user_intent"),
            Self::MetaReasoning => write!(f, "meta_reasoning"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Reference to a piece of evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Unique identifier
    pub id: String,
    /// Type of evidence
    pub evidence_type: EvidenceType,
    /// Brief description of the evidence
    pub description: String,
    /// Strength of the evidence (0.0-1.0)
    pub strength: f64,
}

impl EvidenceRef {
    pub fn new(
        id: impl Into<String>,
        evidence_type: EvidenceType,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            evidence_type,
            description: description.into(),
            strength: 1.0,
        }
    }

    pub fn with_strength(mut self, strength: f64) -> Self {
        self.strength = strength.clamp(0.0, 1.0);
        self
    }
}

/// Type of evidence supporting a claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    /// Direct citation from source material
    Citation,
    /// Code snippet or file reference
    CodeRef,
    /// Tool output (REPL result, search, etc.)
    ToolOutput,
    /// User statement in conversation
    UserStatement,
    /// Inference from other verified claims
    Inference,
    /// External knowledge (model prior)
    Prior,
}

/// A piece of evidence that can support claims.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Evidence {
    /// Reference info
    pub reference: EvidenceRef,
    /// Full content of the evidence
    pub content: String,
    /// Source location (file path, URL, etc.)
    pub source: Option<String>,
    /// When the evidence was observed
    pub observed_at: DateTime<Utc>,
}

impl Evidence {
    pub fn new(reference: EvidenceRef, content: impl Into<String>) -> Self {
        Self {
            reference,
            content: content.into(),
            source: None,
            observed_at: Utc::now(),
        }
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

/// Result of epistemic budget computation.
///
/// The key insight: claims should only be believed if the information
/// gained from evidence (KL divergence) exceeds the information needed
/// to specify the claim (required_bits based on specificity).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetResult {
    /// Claim being evaluated
    pub claim_id: ClaimId,
    /// Prior probability P(claim | context_without_evidence)
    pub p0: Probability,
    /// Posterior probability P(claim | context_with_evidence)
    pub p1: Probability,
    /// Information gained in bits (KL divergence)
    pub observed_bits: f64,
    /// Information required based on claim specificity
    pub required_bits: f64,
    /// Budget gap (positive = exceeds budget = potential hallucination)
    pub budget_gap: f64,
    /// Overall grounding status
    pub status: GroundingStatus,
    /// Confidence in this assessment
    pub confidence: f64,
    /// Breakdown of evidence contributions
    pub evidence_breakdown: Vec<EvidenceContribution>,
}

impl BudgetResult {
    /// Create a new budget result.
    pub fn new(claim_id: ClaimId, p0: Probability, p1: Probability, required_bits: f64) -> Self {
        let observed_bits = p0.kl_divergence(&p1);
        let budget_gap = required_bits - observed_bits;

        let status = if budget_gap > 0.5 {
            GroundingStatus::Ungrounded
        } else if budget_gap > 0.0 {
            GroundingStatus::WeaklyGrounded
        } else {
            GroundingStatus::Grounded
        };

        Self {
            claim_id,
            p0,
            p1,
            observed_bits,
            required_bits,
            budget_gap,
            status,
            confidence: 1.0,
            evidence_breakdown: Vec::new(),
        }
    }

    /// Check if the claim is within epistemic budget.
    pub fn is_grounded(&self) -> bool {
        self.budget_gap <= 0.0
    }

    /// Check if this should trigger a hallucination flag.
    pub fn should_flag(&self, threshold: f64) -> bool {
        self.budget_gap > threshold
    }

    /// Add evidence contribution.
    pub fn with_evidence_contribution(mut self, contribution: EvidenceContribution) -> Self {
        self.evidence_breakdown.push(contribution);
        self
    }

    /// Set confidence level.
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// Grounding status of a claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroundingStatus {
    /// Claim is well-supported by evidence
    Grounded,
    /// Claim has some support but marginal
    WeaklyGrounded,
    /// Claim exceeds epistemic budget
    Ungrounded,
    /// Unable to assess (insufficient samples, etc.)
    Uncertain,
}

impl std::fmt::Display for GroundingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Grounded => write!(f, "grounded"),
            Self::WeaklyGrounded => write!(f, "weakly_grounded"),
            Self::Ungrounded => write!(f, "ungrounded"),
            Self::Uncertain => write!(f, "uncertain"),
        }
    }
}

/// Contribution of a piece of evidence to the budget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceContribution {
    /// Evidence reference ID
    pub evidence_id: String,
    /// Bits contributed by this evidence
    pub bits_contributed: f64,
    /// How the evidence affects the probability
    pub effect: EvidenceEffect,
}

/// How evidence affects claim probability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceEffect {
    /// Evidence supports the claim
    Supporting,
    /// Evidence contradicts the claim
    Contradicting,
    /// Evidence is neutral/irrelevant
    Neutral,
}

/// A probability estimate with uncertainty bounds.
///
/// Uses interval arithmetic to track uncertainty in probability estimates.
/// This is crucial because we're estimating p0 via sampling, which introduces
/// uncertainty.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Probability {
    /// Point estimate
    pub estimate: f64,
    /// Lower bound (confidence interval)
    pub lower: f64,
    /// Upper bound (confidence interval)
    pub upper: f64,
    /// Number of samples used for estimation
    pub n_samples: u32,
}

impl Probability {
    /// Create a probability from a point estimate with no uncertainty.
    pub fn point(p: f64) -> Self {
        let p = p.clamp(0.001, 0.999); // Avoid infinities in KL
        Self {
            estimate: p,
            lower: p,
            upper: p,
            n_samples: 0,
        }
    }

    /// Create a probability from samples (agreement rate).
    pub fn from_samples(agreeing: u32, total: u32) -> Self {
        if total == 0 {
            return Self::point(0.5); // Uniform prior
        }

        let p = agreeing as f64 / total as f64;
        let p = p.clamp(0.001, 0.999);

        // Wilson score interval for confidence bounds
        let z = 1.96; // 95% confidence
        let n = total as f64;
        let phat = p;

        let denominator = 1.0 + z * z / n;
        let center = (phat + z * z / (2.0 * n)) / denominator;
        let margin = z * ((phat * (1.0 - phat) + z * z / (4.0 * n)) / n).sqrt() / denominator;

        Self {
            estimate: p,
            lower: (center - margin).clamp(0.001, 0.999),
            upper: (center + margin).clamp(0.001, 0.999),
            n_samples: total,
        }
    }

    /// Compute KL divergence D_KL(self || other) in bits.
    /// Measures information gained by moving from other (prior) to self (posterior).
    pub fn kl_divergence(&self, other: &Probability) -> f64 {
        let p = self.estimate;
        let q = other.estimate;

        // Bernoulli KL divergence
        let kl = p * (p / q).ln() + (1.0 - p) * ((1.0 - p) / (1.0 - q)).ln();

        // Convert from nats to bits
        kl / std::f64::consts::LN_2
    }

    /// Compute KL divergence with interval arithmetic (returns bounds).
    pub fn kl_divergence_interval(&self, other: &Probability) -> (f64, f64) {
        // Lower bound: use p_lower, q_upper (minimizes divergence)
        let kl_lower = {
            let p = self.lower;
            let q = other.upper;
            (p * (p / q).ln() + (1.0 - p) * ((1.0 - p) / (1.0 - q)).ln()) / std::f64::consts::LN_2
        };

        // Upper bound: use p_upper, q_lower (maximizes divergence)
        let kl_upper = {
            let p = self.upper;
            let q = other.lower;
            (p * (p / q).ln() + (1.0 - p) * ((1.0 - p) / (1.0 - q)).ln()) / std::f64::consts::LN_2
        };

        (kl_lower.max(0.0), kl_upper.max(0.0))
    }

    /// Get the uncertainty range.
    pub fn uncertainty(&self) -> f64 {
        self.upper - self.lower
    }
}

impl Default for Probability {
    fn default() -> Self {
        Self::point(0.5)
    }
}

/// Complete verification result for a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Session or request ID
    pub session_id: String,
    /// All claims extracted
    pub claims: Vec<Claim>,
    /// Budget results for each claim
    pub budget_results: Vec<BudgetResult>,
    /// Overall verdict
    pub verdict: VerificationVerdict,
    /// Summary statistics
    pub stats: VerificationStats,
    /// When verification completed
    pub completed_at: DateTime<Utc>,
    /// Latency in milliseconds
    pub latency_ms: u64,
}

/// Overall verification verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationVerdict {
    /// All claims grounded
    Verified,
    /// Some claims weakly grounded
    PartiallyVerified,
    /// One or more claims ungrounded
    Unverified,
    /// Verification could not complete
    Error,
}

impl std::fmt::Display for VerificationVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Verified => write!(f, "verified"),
            Self::PartiallyVerified => write!(f, "partially_verified"),
            Self::Unverified => write!(f, "unverified"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// Statistics from verification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerificationStats {
    /// Total claims analyzed
    pub total_claims: u32,
    /// Claims that are grounded
    pub grounded_claims: u32,
    /// Claims that are weakly grounded
    pub weakly_grounded_claims: u32,
    /// Claims that are ungrounded
    pub ungrounded_claims: u32,
    /// Claims with uncertain status
    pub uncertain_claims: u32,
    /// Average budget gap
    pub avg_budget_gap: f64,
    /// Maximum budget gap (worst offender)
    pub max_budget_gap: f64,
    /// Total LLM samples used
    pub total_samples: u32,
}

impl VerificationStats {
    /// Calculate hallucination rate (ungrounded / total).
    pub fn hallucination_rate(&self) -> f64 {
        if self.total_claims == 0 {
            0.0
        } else {
            self.ungrounded_claims as f64 / self.total_claims as f64
        }
    }

    /// Calculate grounding rate (grounded / total).
    pub fn grounding_rate(&self) -> f64 {
        if self.total_claims == 0 {
            1.0
        } else {
            self.grounded_claims as f64 / self.total_claims as f64
        }
    }
}

/// Configuration for epistemic verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationConfig {
    /// Number of samples for p0 estimation
    pub n_samples: u32,
    /// Temperature for sampling (higher = more diverse)
    pub sample_temperature: f64,
    /// Budget gap threshold for flagging hallucinations
    pub hallucination_threshold: f64,
    /// Maximum latency budget in milliseconds
    pub max_latency_ms: u64,
    /// Whether to use batch mode (all samples in parallel)
    pub batch_mode: bool,
    /// Model to use for verification (None = same as generation)
    pub verification_model: Option<String>,
    /// Whether to verify all claims or sample
    pub verify_all_claims: bool,
    /// Maximum claims to verify if sampling
    pub max_claims: Option<u32>,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            n_samples: 5,
            sample_temperature: 0.7,
            hallucination_threshold: 0.5,
            max_latency_ms: 500,
            batch_mode: true,
            verification_model: None, // Use Haiku by default
            verify_all_claims: false,
            max_claims: Some(10),
        }
    }
}

impl VerificationConfig {
    /// Configuration optimized for low latency.
    pub fn fast() -> Self {
        Self {
            n_samples: 3,
            sample_temperature: 0.8,
            hallucination_threshold: 0.7,
            max_latency_ms: 200,
            batch_mode: true,
            verification_model: Some("claude-3-5-haiku-20241022".to_string()),
            verify_all_claims: false,
            max_claims: Some(5),
        }
    }

    /// Configuration optimized for accuracy.
    pub fn thorough() -> Self {
        Self {
            n_samples: 10,
            sample_temperature: 0.5,
            hallucination_threshold: 0.3,
            max_latency_ms: 2000,
            batch_mode: true,
            verification_model: Some("claude-3-5-sonnet-20241022".to_string()),
            verify_all_claims: true,
            max_claims: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claim_creation() {
        let claim = Claim::new(
            "The function returns an integer",
            ClaimCategory::CodeBehavior,
        )
        .with_specificity(0.7)
        .with_span(10, 50);

        assert_eq!(claim.category, ClaimCategory::CodeBehavior);
        assert_eq!(claim.specificity, 0.7);
        assert_eq!(claim.source_span, Some((10, 50)));
    }

    #[test]
    fn test_probability_from_samples() {
        // 8 out of 10 agree
        let p = Probability::from_samples(8, 10);
        assert!((p.estimate - 0.8).abs() < 0.01);
        assert!(p.lower < p.estimate);
        assert!(p.upper > p.estimate);
    }

    #[test]
    fn test_kl_divergence() {
        let p1 = Probability::point(0.9);
        let p0 = Probability::point(0.5);

        let kl = p0.kl_divergence(&p1);
        // KL should be positive when distributions differ
        assert!(kl > 0.0);
    }

    #[test]
    fn test_budget_result() {
        let claim = Claim::new("Test claim", ClaimCategory::Factual).with_specificity(0.8);
        let p0 = Probability::point(0.5);
        let p1 = Probability::point(0.9);

        let result = BudgetResult::new(claim.id.clone(), p0, p1, claim.required_bits());

        // With p0=0.5, p1=0.9, we gain significant information
        assert!(result.observed_bits > 0.0);
    }

    #[test]
    fn test_grounding_status() {
        // Grounded: budget_gap <= 0
        let claim_id = ClaimId::new();
        let p0 = Probability::point(0.5);
        let p1 = Probability::point(0.95);
        let result = BudgetResult::new(claim_id, p0, p1, 1.0);

        // High posterior with low required bits should be grounded
        assert!(result.observed_bits > 1.0);
        assert!(result.is_grounded());
    }

    #[test]
    fn test_probability_bounds() {
        // Probabilities should be clamped
        let p = Probability::point(1.5);
        assert!(p.estimate <= 0.999);

        let p = Probability::point(-0.5);
        assert!(p.estimate >= 0.001);
    }

    #[test]
    fn test_verification_stats() {
        let mut stats = VerificationStats::default();
        stats.total_claims = 10;
        stats.grounded_claims = 7;
        stats.ungrounded_claims = 2;
        stats.weakly_grounded_claims = 1;

        assert!((stats.hallucination_rate() - 0.2).abs() < 0.01);
        assert!((stats.grounding_rate() - 0.7).abs() < 0.01);
    }
}
