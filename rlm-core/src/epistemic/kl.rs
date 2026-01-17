//! KL divergence computation with interval arithmetic.
//!
//! This module provides mathematically rigorous KL divergence calculations
//! for epistemic verification. The key insight is that uncertainty in
//! probability estimates (from sampling) propagates through the computation.

use super::types::Probability;

/// Bernoulli KL divergence D_KL(P || Q) in bits.
///
/// Measures the information gained by updating from Q (prior) to P (posterior).
/// When P and Q are Bernoulli distributions:
///
/// D_KL(P || Q) = p * log(p/q) + (1-p) * log((1-p)/(1-q))
///
/// # Arguments
/// * `p` - Posterior probability (what we believe after seeing evidence)
/// * `q` - Prior probability (what we would believe without evidence)
///
/// # Returns
/// KL divergence in bits (log base 2)
pub fn bernoulli_kl_bits(p: f64, q: f64) -> f64 {
    // Clamp to avoid infinities
    let p = p.clamp(1e-10, 1.0 - 1e-10);
    let q = q.clamp(1e-10, 1.0 - 1e-10);

    let kl_nats = p * (p / q).ln() + (1.0 - p) * ((1.0 - p) / (1.0 - q)).ln();
    kl_nats / std::f64::consts::LN_2
}

/// Bernoulli KL divergence in nats (natural log).
pub fn bernoulli_kl_nats(p: f64, q: f64) -> f64 {
    let p = p.clamp(1e-10, 1.0 - 1e-10);
    let q = q.clamp(1e-10, 1.0 - 1e-10);

    p * (p / q).ln() + (1.0 - p) * ((1.0 - p) / (1.0 - q)).ln()
}

/// Compute KL divergence with interval arithmetic.
///
/// When probabilities have uncertainty bounds (from sampling), we need to
/// propagate that uncertainty through the KL computation.
///
/// # Arguments
/// * `p_posterior` - Posterior probability with bounds
/// * `q_prior` - Prior probability with bounds
///
/// # Returns
/// (lower_bound, upper_bound) for the KL divergence in bits
pub fn kl_interval(p_posterior: &Probability, q_prior: &Probability) -> KLInterval {
    // For KL(P||Q) = sum_x P(x) log(P(x)/Q(x))
    // The minimum occurs when P is small and Q is large (closer distributions)
    // The maximum occurs when P is large and Q is small (more divergent)

    let lower = bernoulli_kl_bits(p_posterior.lower, q_prior.upper);
    let upper = bernoulli_kl_bits(p_posterior.upper, q_prior.lower);

    // Point estimate using central values
    let estimate = bernoulli_kl_bits(p_posterior.estimate, q_prior.estimate);

    KLInterval {
        estimate,
        lower: lower.max(0.0),
        upper: upper.max(0.0),
    }
}

/// KL divergence result with uncertainty bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KLInterval {
    /// Point estimate
    pub estimate: f64,
    /// Lower bound (conservative)
    pub lower: f64,
    /// Upper bound (aggressive)
    pub upper: f64,
}

impl KLInterval {
    /// Create a point estimate with no uncertainty.
    pub fn point(value: f64) -> Self {
        Self {
            estimate: value,
            lower: value,
            upper: value,
        }
    }

    /// Get the uncertainty range.
    pub fn uncertainty(&self) -> f64 {
        self.upper - self.lower
    }

    /// Check if this interval contains zero.
    pub fn contains_zero(&self) -> bool {
        self.lower <= 0.0
    }

    /// Get conservative estimate (lower bound).
    pub fn conservative(&self) -> f64 {
        self.lower
    }

    /// Get aggressive estimate (upper bound).
    pub fn aggressive(&self) -> f64 {
        self.upper
    }
}

/// Information-theoretic surprise for an observation.
///
/// Surprise(x; P) = -log P(x)
///
/// High surprise means the observation was unlikely under P.
pub fn surprise_bits(probability: f64) -> f64 {
    let p = probability.clamp(1e-10, 1.0);
    -p.log2()
}

/// Mutual information between claim and evidence.
///
/// I(C; E) = H(C) - H(C|E)
///
/// This measures how much knowing the evidence reduces uncertainty about the claim.
///
/// # Arguments
/// * `p_prior` - P(C) without evidence
/// * `p_posterior` - P(C|E) with evidence
///
/// # Returns
/// Mutual information in bits
pub fn mutual_information_bits(p_prior: f64, p_posterior: f64) -> f64 {
    let p = p_prior.clamp(1e-10, 1.0 - 1e-10);
    let q = p_posterior.clamp(1e-10, 1.0 - 1e-10);

    // H(C) - H(C|E) using Bernoulli entropy
    let h_prior = -p * p.log2() - (1.0 - p) * (1.0 - p).log2();
    let h_posterior = -q * q.log2() - (1.0 - q) * (1.0 - q).log2();

    (h_prior - h_posterior).max(0.0)
}

/// Binary entropy H(p) in nats (natural log).
pub fn binary_entropy_nats(p: f64) -> f64 {
    let p = p.clamp(1e-10, 1.0 - 1e-10);
    -p * p.ln() - (1.0 - p) * (1.0 - p).ln()
}

/// Binary entropy H(p) in bits.
pub fn binary_entropy_bits(p: f64) -> f64 {
    binary_entropy_nats(p) / std::f64::consts::LN_2
}

/// Cross entropy H(P, Q) = -sum_x P(x) log Q(x) in bits.
pub fn cross_entropy_bits(p: f64, q: f64) -> f64 {
    let p = p.clamp(1e-10, 1.0 - 1e-10);
    let q = q.clamp(1e-10, 1.0 - 1e-10);

    -p * q.log2() - (1.0 - p) * (1.0 - q).log2()
}

/// Compute required bits for a claim based on specificity.
///
/// More specific claims require more evidence. The intuition is that
/// a claim like "x = 42" is more specific than "x > 0" and thus
/// needs more bits of evidence to justify.
///
/// Using -log2(1 - specificity) as the base formula:
/// - specificity 0.5 -> ~1 bit required
/// - specificity 0.9 -> ~3.3 bits required
/// - specificity 0.99 -> ~6.6 bits required
pub fn required_bits_for_specificity(specificity: f64) -> f64 {
    let s = specificity.clamp(0.01, 0.999);
    // -log2(1-s) increases as s increases
    -(1.0 - s).log2()
}

/// Aggregate KL from multiple evidence sources.
///
/// When multiple independent pieces of evidence support a claim,
/// we can (approximately) sum their information contributions.
///
/// # Arguments
/// * `kl_values` - Individual KL contributions from each evidence
///
/// # Returns
/// Aggregated information gain in bits
pub fn aggregate_evidence_bits(kl_values: &[f64]) -> f64 {
    // For independent evidence, information is approximately additive
    // This is a simplification; in practice, evidence may be correlated
    kl_values.iter().sum()
}

/// Aggregate KL with correlation adjustment.
///
/// When evidence sources are correlated, simply summing overstates
/// the information gain. This applies a discount factor.
///
/// # Arguments
/// * `kl_values` - Individual KL contributions
/// * `correlation` - Estimated correlation between evidence (0-1)
///
/// # Returns
/// Adjusted information gain
pub fn aggregate_evidence_bits_with_correlation(kl_values: &[f64], correlation: f64) -> f64 {
    let c = correlation.clamp(0.0, 1.0);
    let n = kl_values.len() as f64;
    let raw_sum: f64 = kl_values.iter().sum();

    // Apply discount: more correlation = less additional information
    // At correlation=0, we get the full sum
    // At correlation=1, we get the max
    if n <= 1.0 {
        raw_sum
    } else {
        let max = kl_values.iter().cloned().fold(0.0_f64, f64::max);
        (1.0 - c) * raw_sum + c * max
    }
}

/// Jeffrey's divergence (symmetric KL).
///
/// D_J(P, Q) = D_KL(P||Q) + D_KL(Q||P)
///
/// This is a symmetric measure of distribution difference.
pub fn jeffreys_divergence_bits(p: f64, q: f64) -> f64 {
    bernoulli_kl_bits(p, q) + bernoulli_kl_bits(q, p)
}

/// Jensen-Shannon divergence.
///
/// D_JS(P, Q) = 0.5 * D_KL(P||M) + 0.5 * D_KL(Q||M)
/// where M = 0.5 * (P + Q)
///
/// This is symmetric and bounded by [0, 1] bit.
pub fn jensen_shannon_bits(p: f64, q: f64) -> f64 {
    let p = p.clamp(1e-10, 1.0 - 1e-10);
    let q = q.clamp(1e-10, 1.0 - 1e-10);
    let m = (p + q) / 2.0;

    0.5 * bernoulli_kl_bits(p, m) + 0.5 * bernoulli_kl_bits(q, m)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-6;

    #[test]
    fn test_bernoulli_kl_same_distribution() {
        // KL divergence of identical distributions is 0
        let kl = bernoulli_kl_bits(0.5, 0.5);
        assert!(kl.abs() < EPSILON);

        let kl = bernoulli_kl_bits(0.8, 0.8);
        assert!(kl.abs() < EPSILON);
    }

    #[test]
    fn test_bernoulli_kl_positive() {
        // KL is always non-negative
        let kl = bernoulli_kl_bits(0.3, 0.7);
        assert!(kl >= 0.0);

        let kl = bernoulli_kl_bits(0.9, 0.1);
        assert!(kl >= 0.0);
    }

    #[test]
    fn test_bernoulli_kl_asymmetric() {
        // KL is asymmetric: D_KL(P||Q) != D_KL(Q||P) in general
        let kl_pq = bernoulli_kl_bits(0.9, 0.5);
        let kl_qp = bernoulli_kl_bits(0.5, 0.9);
        assert!((kl_pq - kl_qp).abs() > EPSILON);
    }

    #[test]
    fn test_bernoulli_kl_extreme() {
        // KL divergence increases with distribution difference
        let kl_close = bernoulli_kl_bits(0.6, 0.5);
        let kl_far = bernoulli_kl_bits(0.9, 0.5);
        assert!(kl_far > kl_close);
    }

    #[test]
    fn test_binary_entropy() {
        // Maximum entropy at p=0.5
        let h_half = binary_entropy_bits(0.5);
        assert!((h_half - 1.0).abs() < EPSILON);

        // Lower entropy at extremes
        let h_extreme = binary_entropy_bits(0.9);
        assert!(h_extreme < h_half);
    }

    #[test]
    fn test_surprise() {
        // Certain events have 0 surprise
        let s = surprise_bits(1.0);
        assert!(s.abs() < EPSILON);

        // Unlikely events have high surprise
        let s_unlikely = surprise_bits(0.1);
        let s_likely = surprise_bits(0.9);
        assert!(s_unlikely > s_likely);
    }

    #[test]
    fn test_kl_interval() {
        let p = Probability::from_samples(8, 10);
        let q = Probability::from_samples(5, 10);

        let interval = kl_interval(&p, &q);

        // Lower should be less than or equal to estimate
        assert!(interval.lower <= interval.estimate + EPSILON);
        // Upper should be greater than or equal to estimate
        assert!(interval.upper >= interval.estimate - EPSILON);
    }

    #[test]
    fn test_required_bits() {
        // More specific claims need more bits
        let bits_low = required_bits_for_specificity(0.5);
        let bits_high = required_bits_for_specificity(0.9);
        assert!(bits_high > bits_low);
    }

    #[test]
    fn test_aggregate_bits() {
        let contributions = vec![0.5, 0.3, 0.2];
        let total = aggregate_evidence_bits(&contributions);
        assert!((total - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_aggregate_with_correlation() {
        let contributions = vec![0.5, 0.5];

        // No correlation: full sum
        let total_independent = aggregate_evidence_bits_with_correlation(&contributions, 0.0);
        assert!((total_independent - 1.0).abs() < EPSILON);

        // Full correlation: max only
        let total_correlated = aggregate_evidence_bits_with_correlation(&contributions, 1.0);
        assert!((total_correlated - 0.5).abs() < EPSILON);
    }

    #[test]
    fn test_jeffreys_symmetric() {
        let j1 = jeffreys_divergence_bits(0.3, 0.7);
        let j2 = jeffreys_divergence_bits(0.7, 0.3);
        assert!((j1 - j2).abs() < EPSILON);
    }

    #[test]
    fn test_jensen_shannon_bounded() {
        // JS divergence is bounded by [0, 1] bit
        let js = jensen_shannon_bits(0.1, 0.9);
        assert!(js >= 0.0);
        assert!(js <= 1.0);
    }

    #[test]
    fn test_jensen_shannon_symmetric() {
        let js1 = jensen_shannon_bits(0.3, 0.8);
        let js2 = jensen_shannon_bits(0.8, 0.3);
        assert!((js1 - js2).abs() < EPSILON);
    }

    #[test]
    fn test_mutual_information() {
        // Information gain when going from uncertain to certain
        let mi = mutual_information_bits(0.5, 0.95);
        assert!(mi > 0.0);

        // No gain when distributions are the same
        let mi_same = mutual_information_bits(0.5, 0.5);
        assert!(mi_same.abs() < EPSILON);
    }
}
