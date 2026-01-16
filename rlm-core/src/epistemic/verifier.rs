//! EpistemicVerifier trait and implementations.
//!
//! This module provides the core verification interface and implementations
//! for different verification backends:
//! - Self-verification (same model, different context)
//! - Haiku-assisted (fast, cheap verification)
//! - External API (for specialized verification services)

use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::error::{Error, Result};
use crate::llm::{ChatMessage, CompletionRequest, LLMClient};
use crate::trajectory::{TrajectoryEvent, TrajectoryEventType};

use super::claims::ClaimExtractor;
use super::kl::required_bits_for_specificity;
use super::scrubber::{create_p0_prompt, EvidenceScrubber, ScrubConfig};
use super::types::{
    BudgetResult, Claim, GroundingStatus, Probability, VerificationConfig, VerificationResult,
    VerificationStats, VerificationVerdict,
};

/// Trait for epistemic verification backends.
#[async_trait]
pub trait EpistemicVerifier: Send + Sync {
    /// Verify a single claim.
    async fn verify_claim(
        &self,
        claim: &Claim,
        context: &str,
        evidence: &[String],
    ) -> Result<BudgetResult>;

    /// Verify all claims in a response.
    async fn verify_response(
        &self,
        response: &str,
        context: &str,
    ) -> Result<VerificationResult>;

    /// Get the verifier's configuration.
    fn config(&self) -> &VerificationConfig;

    /// Get trajectory events emitted during verification.
    async fn get_events(&self) -> Vec<TrajectoryEvent>;
}

/// Self-verification using the same model with masked evidence.
///
/// This implementation estimates p0 by sampling completions with evidence
/// masked, then comparing to the original response (p1).
pub struct SelfVerifier {
    client: Arc<dyn LLMClient>,
    config: VerificationConfig,
    claim_extractor: ClaimExtractor,
    scrubber: EvidenceScrubber,
    events: Arc<RwLock<Vec<TrajectoryEvent>>>,
}

impl SelfVerifier {
    /// Create a new self-verifier.
    pub fn new(client: Arc<dyn LLMClient>, config: VerificationConfig) -> Self {
        Self {
            client,
            config,
            claim_extractor: ClaimExtractor::new(),
            scrubber: EvidenceScrubber::new(ScrubConfig::default()),
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create with custom claim extractor.
    pub fn with_extractor(mut self, extractor: ClaimExtractor) -> Self {
        self.claim_extractor = extractor;
        self
    }

    /// Create with custom scrubber.
    pub fn with_scrubber(mut self, scrubber: EvidenceScrubber) -> Self {
        self.scrubber = scrubber;
        self
    }

    async fn emit_event(&self, event: TrajectoryEvent) {
        self.events.write().await.push(event);
    }

    /// Estimate p0 by sampling with masked evidence.
    async fn estimate_p0(
        &self,
        claim: &Claim,
        context: &str,
        _evidence: &[String],
    ) -> Result<Probability> {
        let p0_prompt = create_p0_prompt(context, &claim.text, &self.scrubber);

        let mut agreeing = 0u32;
        let total = self.config.n_samples;

        // Sample multiple completions
        for _ in 0..total {
            let request = CompletionRequest::new()
                .with_message(ChatMessage::user(&p0_prompt.prompt))
                .with_temperature(self.config.sample_temperature)
                .with_max_tokens(100);

            let response = self.client.complete(request).await?;

            // Parse probability from response
            if let Some(p) = self.parse_probability(&response.content) {
                // Consider it "agreeing" if the model gives >0.5 probability
                if p > 0.5 {
                    agreeing += 1;
                }
            }
        }

        Ok(Probability::from_samples(agreeing, total))
    }

    /// Estimate p1 (posterior with evidence).
    /// For self-verification, p1 is derived from the original response confidence.
    fn estimate_p1(&self, claim: &Claim) -> Probability {
        // The claim was made in the original response, so we assume high confidence
        // unless there are hedge words (handled by claim specificity)
        let base_p = 0.85;
        let adjusted = base_p * claim.specificity + (1.0 - claim.specificity) * 0.5;
        Probability::point(adjusted)
    }

    /// Parse a probability value from model output.
    fn parse_probability(&self, text: &str) -> Option<f64> {
        // Look for patterns like "0.7", "0.85", "70%"
        let text = text.trim().to_lowercase();

        // Try parsing as decimal
        if let Ok(p) = text.lines().next().unwrap_or("").trim().parse::<f64>() {
            if (0.0..=1.0).contains(&p) {
                return Some(p);
            }
        }

        // Try parsing percentage
        if let Some(stripped) = text.strip_suffix('%') {
            if let Ok(p) = stripped.trim().parse::<f64>() {
                return Some(p / 100.0);
            }
        }

        // Look for probability in the text
        let re = regex::Regex::new(r"(\d+\.?\d*)\s*%?").ok()?;
        if let Some(cap) = re.captures(&text) {
            if let Ok(p) = cap[1].parse::<f64>() {
                let p = if p > 1.0 { p / 100.0 } else { p };
                if (0.0..=1.0).contains(&p) {
                    return Some(p);
                }
            }
        }

        None
    }
}

#[async_trait]
impl EpistemicVerifier for SelfVerifier {
    async fn verify_claim(
        &self,
        claim: &Claim,
        context: &str,
        evidence: &[String],
    ) -> Result<BudgetResult> {
        let start = Instant::now();

        // Emit start event
        self.emit_event(TrajectoryEvent::new(
            TrajectoryEventType::VerifyStart,
            0,
            format!("Verifying claim: {}", &claim.text[..claim.text.len().min(50)]),
        ))
        .await;

        // Estimate p0 (prior without evidence)
        let p0 = self.estimate_p0(claim, context, evidence).await?;

        // Estimate p1 (posterior with evidence)
        let p1 = self.estimate_p1(claim);

        // Calculate required bits based on specificity
        let required_bits = required_bits_for_specificity(claim.specificity);

        // Create budget result
        let result = BudgetResult::new(claim.id.clone(), p0, p1, required_bits);

        // Emit result event
        let event = if result.should_flag(self.config.hallucination_threshold) {
            TrajectoryEvent::hallucination_flag(
                0,
                claim.text.clone(),
                result.budget_gap,
                result.status.to_string(),
            )
        } else {
            TrajectoryEvent::new(
                TrajectoryEventType::BudgetComputed,
                0,
                format!(
                    "Claim verified: gap={:.2}, status={}",
                    result.budget_gap, result.status
                ),
            )
            .with_metadata("budget_gap", result.budget_gap)
            .with_metadata("status", result.status.to_string())
        };
        self.emit_event(event).await;

        let _elapsed = start.elapsed().as_millis() as u64;
        Ok(result)
    }

    async fn verify_response(
        &self,
        response: &str,
        context: &str,
    ) -> Result<VerificationResult> {
        let start = Instant::now();
        let session_id = uuid::Uuid::new_v4().to_string();

        self.emit_event(TrajectoryEvent::new(
            TrajectoryEventType::VerifyStart,
            0,
            "Starting response verification",
        ))
        .await;

        // Extract claims
        let mut claims = self.claim_extractor.extract(response);

        // Emit claim extraction events
        for claim in &claims {
            self.emit_event(TrajectoryEvent::new(
                TrajectoryEventType::ClaimExtracted,
                0,
                format!("[{}] {}", claim.category, &claim.text[..claim.text.len().min(60)]),
            ))
            .await;
        }

        // Limit claims if configured
        if !self.config.verify_all_claims {
            if let Some(max) = self.config.max_claims {
                // Sort by specificity (verify most specific first)
                claims.sort_by(|a, b| {
                    b.specificity
                        .partial_cmp(&a.specificity)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                claims.truncate(max as usize);
            }
        }

        // Verify each claim
        let mut budget_results = Vec::new();
        for claim in &claims {
            // Collect evidence from claim refs
            let evidence: Vec<String> = claim
                .evidence_refs
                .iter()
                .map(|e| e.description.clone())
                .collect();

            match self.verify_claim(claim, context, &evidence).await {
                Ok(result) => budget_results.push(result),
                Err(e) => {
                    self.emit_event(TrajectoryEvent::error(
                        0,
                        format!("Verification error: {}", e),
                    ))
                    .await;
                }
            }
        }

        // Calculate statistics
        let stats = self.calculate_stats(&budget_results);

        // Determine verdict
        let verdict = if stats.ungrounded_claims > 0 {
            VerificationVerdict::Unverified
        } else if stats.weakly_grounded_claims > 0 {
            VerificationVerdict::PartiallyVerified
        } else if stats.total_claims > 0 {
            VerificationVerdict::Verified
        } else {
            VerificationVerdict::Error
        };

        let latency_ms = start.elapsed().as_millis() as u64;

        self.emit_event(TrajectoryEvent::new(
            TrajectoryEventType::VerifyComplete,
            0,
            format!(
                "Verification complete: {} claims, {} ungrounded, latency {}ms",
                stats.total_claims, stats.ungrounded_claims, latency_ms
            ),
        ))
        .await;

        Ok(VerificationResult {
            session_id,
            claims,
            budget_results,
            verdict,
            stats,
            completed_at: Utc::now(),
            latency_ms,
        })
    }

    fn config(&self) -> &VerificationConfig {
        &self.config
    }

    async fn get_events(&self) -> Vec<TrajectoryEvent> {
        self.events.read().await.clone()
    }
}

impl SelfVerifier {
    fn calculate_stats(&self, results: &[BudgetResult]) -> VerificationStats {
        let mut stats = VerificationStats::default();
        stats.total_claims = results.len() as u32;

        let mut total_gap = 0.0;
        let mut max_gap = f64::NEG_INFINITY;

        for result in results {
            match result.status {
                GroundingStatus::Grounded => stats.grounded_claims += 1,
                GroundingStatus::WeaklyGrounded => stats.weakly_grounded_claims += 1,
                GroundingStatus::Ungrounded => stats.ungrounded_claims += 1,
                GroundingStatus::Uncertain => stats.uncertain_claims += 1,
            }

            total_gap += result.budget_gap;
            if result.budget_gap > max_gap {
                max_gap = result.budget_gap;
            }
        }

        if !results.is_empty() {
            stats.avg_budget_gap = total_gap / results.len() as f64;
            stats.max_budget_gap = max_gap;
        }

        stats.total_samples = self.config.n_samples * stats.total_claims;

        stats
    }
}

/// Haiku-assisted verification (fast and cheap).
///
/// Uses Claude Haiku for p0 estimation, which is much faster and cheaper
/// while still providing reasonable accuracy.
pub struct HaikuVerifier {
    inner: SelfVerifier,
}

impl HaikuVerifier {
    /// Create a new Haiku verifier.
    pub fn new(client: Arc<dyn LLMClient>) -> Self {
        let mut config = VerificationConfig::fast();
        config.verification_model = Some("claude-3-5-haiku-20241022".to_string());

        Self {
            inner: SelfVerifier::new(client, config),
        }
    }
}

#[async_trait]
impl EpistemicVerifier for HaikuVerifier {
    async fn verify_claim(
        &self,
        claim: &Claim,
        context: &str,
        evidence: &[String],
    ) -> Result<BudgetResult> {
        self.inner.verify_claim(claim, context, evidence).await
    }

    async fn verify_response(
        &self,
        response: &str,
        context: &str,
    ) -> Result<VerificationResult> {
        self.inner.verify_response(response, context).await
    }

    fn config(&self) -> &VerificationConfig {
        self.inner.config()
    }

    async fn get_events(&self) -> Vec<TrajectoryEvent> {
        self.inner.get_events().await
    }
}

/// Batch verifier for efficient verification of multiple claims.
///
/// Sends all p0 estimation requests in parallel for lower latency.
pub struct BatchVerifier {
    client: Arc<dyn LLMClient>,
    config: VerificationConfig,
    claim_extractor: ClaimExtractor,
    scrubber: EvidenceScrubber,
    events: Arc<RwLock<Vec<TrajectoryEvent>>>,
}

impl BatchVerifier {
    /// Create a new batch verifier.
    pub fn new(client: Arc<dyn LLMClient>, config: VerificationConfig) -> Self {
        Self {
            client,
            config,
            claim_extractor: ClaimExtractor::new(),
            scrubber: EvidenceScrubber::new(ScrubConfig::default()),
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    async fn emit_event(&self, event: TrajectoryEvent) {
        self.events.write().await.push(event);
    }

    /// Verify multiple claims in parallel.
    async fn verify_claims_batch(
        &self,
        claims: &[Claim],
        context: &str,
    ) -> Vec<Result<BudgetResult>> {
        let futures: Vec<_> = claims
            .iter()
            .map(|claim| {
                let client = self.client.clone();
                let config = self.config.clone();
                let scrubber = EvidenceScrubber::new(ScrubConfig::default());
                let claim = claim.clone();
                let context = context.to_string();

                async move {
                    let p0_prompt = create_p0_prompt(&context, &claim.text, &scrubber);

                    // Single sample for batch mode (faster)
                    let request = CompletionRequest::new()
                        .with_message(ChatMessage::user(&p0_prompt.prompt))
                        .with_temperature(config.sample_temperature)
                        .with_max_tokens(100);

                    let response = client.complete(request).await?;

                    // Parse p0
                    let p0 = if let Some(p) = parse_probability_from_text(&response.content) {
                        Probability::point(p)
                    } else {
                        Probability::point(0.5) // Default to uncertain
                    };

                    // p1 from original response
                    let p1 = Probability::point(0.85 * claim.specificity + 0.15);

                    let required_bits = required_bits_for_specificity(claim.specificity);

                    Ok(BudgetResult::new(claim.id, p0, p1, required_bits))
                }
            })
            .collect();

        futures::future::join_all(futures).await
    }
}

#[async_trait]
impl EpistemicVerifier for BatchVerifier {
    async fn verify_claim(
        &self,
        claim: &Claim,
        context: &str,
        _evidence: &[String],
    ) -> Result<BudgetResult> {
        let results = self.verify_claims_batch(&[claim.clone()], context).await;
        results.into_iter().next().unwrap_or_else(|| {
            Err(Error::Internal("No verification result".to_string()))
        })
    }

    async fn verify_response(
        &self,
        response: &str,
        context: &str,
    ) -> Result<VerificationResult> {
        let start = Instant::now();
        let session_id = uuid::Uuid::new_v4().to_string();

        self.emit_event(TrajectoryEvent::new(
            TrajectoryEventType::VerifyStart,
            0,
            "Starting batch verification",
        ))
        .await;

        // Extract claims
        let mut claims = self.claim_extractor.extract(response);

        // Limit claims if configured
        if !self.config.verify_all_claims {
            if let Some(max) = self.config.max_claims {
                claims.sort_by(|a, b| {
                    b.specificity
                        .partial_cmp(&a.specificity)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                claims.truncate(max as usize);
            }
        }

        // Verify all claims in parallel
        let results = self.verify_claims_batch(&claims, context).await;

        let mut budget_results = Vec::new();
        for result in results {
            match result {
                Ok(r) => {
                    if r.should_flag(self.config.hallucination_threshold) {
                        self.emit_event(TrajectoryEvent::hallucination_flag(
                            0,
                            "Claim flagged".to_string(),
                            r.budget_gap,
                            r.status.to_string(),
                        ))
                        .await;
                    }
                    budget_results.push(r);
                }
                Err(e) => {
                    self.emit_event(TrajectoryEvent::error(
                        0,
                        format!("Batch verification error: {}", e),
                    ))
                    .await;
                }
            }
        }

        // Calculate statistics
        let stats = calculate_verification_stats(&budget_results, self.config.n_samples);

        let verdict = if stats.ungrounded_claims > 0 {
            VerificationVerdict::Unverified
        } else if stats.weakly_grounded_claims > 0 {
            VerificationVerdict::PartiallyVerified
        } else if stats.total_claims > 0 {
            VerificationVerdict::Verified
        } else {
            VerificationVerdict::Error
        };

        let latency_ms = start.elapsed().as_millis() as u64;

        self.emit_event(TrajectoryEvent::new(
            TrajectoryEventType::VerifyComplete,
            0,
            format!(
                "Batch verification complete: {} claims, latency {}ms",
                stats.total_claims, latency_ms
            ),
        ))
        .await;

        Ok(VerificationResult {
            session_id,
            claims,
            budget_results,
            verdict,
            stats,
            completed_at: Utc::now(),
            latency_ms,
        })
    }

    fn config(&self) -> &VerificationConfig {
        &self.config
    }

    async fn get_events(&self) -> Vec<TrajectoryEvent> {
        self.events.read().await.clone()
    }
}

/// Parse probability from text response.
fn parse_probability_from_text(text: &str) -> Option<f64> {
    let text = text.trim().to_lowercase();

    // Try first line
    if let Some(first_line) = text.lines().next() {
        let cleaned = first_line.trim().trim_matches(|c| c == '"' || c == '\'');

        // Try decimal
        if let Ok(p) = cleaned.parse::<f64>() {
            if (0.0..=1.0).contains(&p) {
                return Some(p);
            }
        }

        // Try percentage
        if let Some(stripped) = cleaned.strip_suffix('%') {
            if let Ok(p) = stripped.trim().parse::<f64>() {
                return Some(p / 100.0);
            }
        }
    }

    // Regex fallback
    let re = regex::Regex::new(r"(\d+\.?\d*)\s*%?").ok()?;
    if let Some(cap) = re.captures(&text) {
        if let Ok(p) = cap[1].parse::<f64>() {
            let p = if p > 1.0 { p / 100.0 } else { p };
            if (0.0..=1.0).contains(&p) {
                return Some(p);
            }
        }
    }

    None
}

/// Calculate verification statistics.
fn calculate_verification_stats(results: &[BudgetResult], n_samples: u32) -> VerificationStats {
    let mut stats = VerificationStats::default();
    stats.total_claims = results.len() as u32;

    let mut total_gap = 0.0;
    let mut max_gap = f64::NEG_INFINITY;

    for result in results {
        match result.status {
            GroundingStatus::Grounded => stats.grounded_claims += 1,
            GroundingStatus::WeaklyGrounded => stats.weakly_grounded_claims += 1,
            GroundingStatus::Ungrounded => stats.ungrounded_claims += 1,
            GroundingStatus::Uncertain => stats.uncertain_claims += 1,
        }

        total_gap += result.budget_gap;
        if result.budget_gap > max_gap {
            max_gap = result.budget_gap;
        }
    }

    if !results.is_empty() {
        stats.avg_budget_gap = total_gap / results.len() as f64;
        stats.max_budget_gap = max_gap;
    }

    stats.total_samples = n_samples * stats.total_claims;

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_probability() {
        assert_eq!(parse_probability_from_text("0.7"), Some(0.7));
        assert_eq!(parse_probability_from_text("0.85"), Some(0.85));
        assert_eq!(parse_probability_from_text("70%"), Some(0.7));
        assert_eq!(parse_probability_from_text("\"0.6\""), Some(0.6));
        assert_eq!(parse_probability_from_text("0.9\n\nExplanation..."), Some(0.9));
    }

    #[test]
    fn test_calculate_stats() {
        let results = vec![
            BudgetResult::new(
                super::super::types::ClaimId::new(),
                Probability::point(0.5),
                Probability::point(0.9),
                1.0,
            ),
            BudgetResult::new(
                super::super::types::ClaimId::new(),
                Probability::point(0.3),
                Probability::point(0.8),
                0.5,
            ),
        ];

        let stats = calculate_verification_stats(&results, 5);
        assert_eq!(stats.total_claims, 2);
        assert_eq!(stats.total_samples, 10);
    }

    #[test]
    fn test_verification_config_presets() {
        let fast = VerificationConfig::fast();
        assert!(fast.n_samples <= 5);
        assert!(fast.max_latency_ms <= 300);

        let thorough = VerificationConfig::thorough();
        assert!(thorough.n_samples >= 8);
        assert!(thorough.verify_all_claims);
    }
}
