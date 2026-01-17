//! Epistemic verification for hallucination detection.
//!
//! This module implements information-theoretic hallucination detection based on
//! the Strawberry/Pythea methodology. The key insight is that claims should only
//! be believed if they are grounded in evidence - specifically, the information
//! gained from evidence (KL divergence) should account for the claim's specificity.
//!
//! ## Core Algorithm
//!
//! For each claim C with evidence E:
//!
//! 1. **p0**: Estimate P(C|context_without_E) - prior probability without evidence
//! 2. **p1**: Estimate P(C|context_with_E) - posterior probability with evidence
//! 3. **KL(p1||p0)**: Information gain in bits
//! 4. **budget_gap**: required_bits - observed_bits
//!    - If budget_gap > 0: claim exceeds epistemic budget (potential hallucination)
//!    - If budget_gap <= 0: claim is within budget (grounded)
//!
//! ## Semantic Consistency Approach
//!
//! Since Claude doesn't provide logprobs, we use semantic consistency:
//! - Sample multiple completions with evidence masked
//! - Estimate p0 from agreement rate across samples
//! - Compare to p1 from original response
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::epistemic::{SelfVerifier, VerificationConfig, ClaimExtractor};
//! use std::sync::Arc;
//!
//! // Create verifier with LLM client
//! let verifier = SelfVerifier::new(client, VerificationConfig::default());
//!
//! // Verify a response
//! let result = verifier.verify_response(
//!     "The function returns an integer. It validates user input.",
//!     "Context about the codebase...",
//! ).await?;
//!
//! // Check results
//! println!("Verdict: {}", result.verdict);
//! println!("Hallucination rate: {:.1}%", result.stats.hallucination_rate() * 100.0);
//!
//! for (claim, budget) in result.claims.iter().zip(&result.budget_results) {
//!     if budget.should_flag(0.5) {
//!         println!("FLAGGED: {} (gap={:.2})", claim.text, budget.budget_gap);
//!     }
//! }
//! ```
//!
//! ## Memory Gate Integration
//!
//! The memory gate prevents ungrounded facts from being stored in long-term memory:
//!
//! ```rust,ignore
//! use rlm_core::epistemic::{MemoryGate, MemoryGateConfig};
//!
//! let gate = MemoryGate::new(verifier, MemoryGateConfig::default());
//!
//! // Evaluate before storing
//! let decision = gate.evaluate(&node, context).await?;
//! if decision.allowed {
//!     // Safe to store, possibly with adjusted confidence
//!     node.confidence = decision.adjusted_confidence.unwrap_or(node.confidence);
//!     store.insert(node)?;
//! } else {
//!     // Rejected as potential hallucination
//!     log::warn!("Rejected: {}", decision.reason);
//! }
//! ```
//!
//! ## REPL Functions
//!
//! For interactive verification:
//!
//! - `verify_claim(claim, context)` - Verify a single claim
//! - `audit_reasoning(trace)` - Audit a reasoning trace for hallucinations
//! - `evidence_dependence(response, context)` - Measure evidence dependence

pub mod claims;
pub mod kl;
pub mod memory_gate;
pub mod scrubber;
pub mod types;
pub mod verifier;

#[cfg(test)]
mod proptest;

// Re-exports for convenience
pub use claims::{extract_doc_claims, extract_numerical_claims, ClaimExtractor};
pub use kl::{
    aggregate_evidence_bits, aggregate_evidence_bits_with_correlation, bernoulli_kl_bits,
    bernoulli_kl_nats, binary_entropy_bits, binary_entropy_nats, jensen_shannon_bits, kl_interval,
    mutual_information_bits, required_bits_for_specificity, surprise_bits, KLInterval,
};
pub use memory_gate::{
    GateDecision, GateRecommendation, GateStats, MemoryGate, MemoryGateConfig, ThresholdGate,
};
pub use scrubber::{
    create_p0_prompt, EvidenceScrubber, P0Prompt, ScrubConfig, ScrubResult, ScrubTarget,
    ScrubbedItem,
};
pub use types::{
    BudgetResult, Claim, ClaimCategory, ClaimId, Evidence, EvidenceContribution, EvidenceEffect,
    EvidenceRef, EvidenceType, GroundingStatus, Probability, VerificationConfig,
    VerificationResult, VerificationStats, VerificationVerdict,
};
pub use verifier::{BatchVerifier, EpistemicVerifier, HaikuVerifier, SelfVerifier};

/// Verify a claim and return the budget result.
///
/// This is a convenience function for single-claim verification.
///
/// # Arguments
/// * `verifier` - The verifier to use
/// * `claim_text` - The claim to verify
/// * `context` - Context in which the claim was made
///
/// # Returns
/// Budget result with grounding status
pub async fn verify_claim<V: EpistemicVerifier>(
    verifier: &V,
    claim_text: &str,
    context: &str,
) -> crate::error::Result<BudgetResult> {
    let claim = Claim::new(claim_text, ClaimCategory::Factual);
    verifier.verify_claim(&claim, context, &[]).await
}

/// Audit a reasoning trace for potential hallucinations.
///
/// Extracts claims from each step and verifies them.
///
/// # Arguments
/// * `verifier` - The verifier to use
/// * `trace` - The reasoning trace (steps separated by newlines)
/// * `context` - Original context/prompt
///
/// # Returns
/// Verification result covering all steps
pub async fn audit_reasoning<V: EpistemicVerifier>(
    verifier: &V,
    trace: &str,
    context: &str,
) -> crate::error::Result<VerificationResult> {
    verifier.verify_response(trace, context).await
}

/// Measure evidence dependence of a response.
///
/// Returns a score indicating how much the response depends on evidence
/// vs. being generated from prior knowledge.
///
/// # Arguments
/// * `verifier` - The verifier to use
/// * `response` - The response to analyze
/// * `context` - Context with evidence
///
/// # Returns
/// Tuple of (average_kl_gain, max_kl_gain) in bits
pub async fn evidence_dependence<V: EpistemicVerifier>(
    verifier: &V,
    response: &str,
    context: &str,
) -> crate::error::Result<(f64, f64)> {
    let result = verifier.verify_response(response, context).await?;

    let kl_gains: Vec<f64> = result
        .budget_results
        .iter()
        .map(|b| b.observed_bits)
        .collect();

    if kl_gains.is_empty() {
        return Ok((0.0, 0.0));
    }

    let avg = kl_gains.iter().sum::<f64>() / kl_gains.len() as f64;
    let max = kl_gains.iter().cloned().fold(0.0_f64, f64::max);

    Ok((avg, max))
}

/// Quick check if a response likely contains hallucinations.
///
/// Uses heuristics for fast screening without full LLM verification.
///
/// # Arguments
/// * `response` - The response to check
///
/// # Returns
/// Risk score (0.0 = low risk, 1.0 = high risk)
pub fn quick_hallucination_check(response: &str) -> f64 {
    let mut risk: f64 = 0.0;
    let lower = response.to_lowercase();

    // High specificity without evidence references
    let extractor = ClaimExtractor::new();
    let claims = extractor.extract(response);

    let high_specificity_count = claims.iter().filter(|c| c.specificity > 0.7).count();
    let with_evidence_count = claims.iter().filter(|c| !c.evidence_refs.is_empty()).count();

    if high_specificity_count > 0 && with_evidence_count == 0 {
        risk += 0.3;
    }

    // Universal claims without hedging
    let universal_words = ["always", "never", "all", "none", "every", "guaranteed"];
    let hedge_words = ["might", "could", "possibly", "perhaps", "likely", "probably"];

    let has_universal = universal_words.iter().any(|w| lower.contains(w));
    let has_hedge = hedge_words.iter().any(|w| lower.contains(w));

    if has_universal && !has_hedge {
        risk += 0.2;
    }

    // Specific numbers without context
    let number_re = regex::Regex::new(r"\b\d{3,}\b").unwrap();
    let number_count = number_re.find_iter(&lower).count();
    if number_count > 2 {
        risk += 0.15;
    }

    // Very long sentences (often contain unsupported claims)
    let long_sentences = response
        .split('.')
        .filter(|s| s.len() > 200)
        .count();
    if long_sentences > 1 {
        risk += 0.1;
    }

    risk.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_hallucination_check() {
        // Low risk: hedged, short
        let safe = "The function might return null in some cases.";
        assert!(quick_hallucination_check(safe) < 0.3);

        // Higher risk: universal claim without hedging
        let risky = "This function always returns exactly 42. It never fails under any circumstances.";
        assert!(quick_hallucination_check(risky) > 0.1);
    }

    #[test]
    fn test_module_exports() {
        // Verify key types are exported
        let _config = VerificationConfig::default();
        let _claim = Claim::new("test", ClaimCategory::Factual);
        let _prob = Probability::point(0.5);

        // KL functions
        let kl = bernoulli_kl_bits(0.8, 0.5);
        assert!(kl > 0.0);

        // Entropy
        let h = binary_entropy_bits(0.5);
        assert!((h - 1.0).abs() < 0.01);
    }
}
