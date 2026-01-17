//! Python bindings for epistemic verification module.
//!
//! Provides access to hallucination detection and claim verification functionality.

use pyo3::prelude::*;

use crate::epistemic::{
    self, BudgetResult, Claim, ClaimCategory, ClaimExtractor, ClaimId, EvidenceContribution,
    EvidenceEffect, EvidenceRef, EvidenceType, GroundingStatus, Probability, VerificationConfig,
    VerificationResult, VerificationStats, VerificationVerdict,
};

/// Python wrapper for ClaimCategory enum.
#[pyclass(name = "ClaimCategory", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyClaimCategory {
    Factual = 0,
    CodeBehavior = 1,
    Relational = 2,
    Numerical = 3,
    Temporal = 4,
    UserIntent = 5,
    MetaReasoning = 6,
    Unknown = 7,
}

impl From<ClaimCategory> for PyClaimCategory {
    fn from(c: ClaimCategory) -> Self {
        match c {
            ClaimCategory::Factual => Self::Factual,
            ClaimCategory::CodeBehavior => Self::CodeBehavior,
            ClaimCategory::Relational => Self::Relational,
            ClaimCategory::Numerical => Self::Numerical,
            ClaimCategory::Temporal => Self::Temporal,
            ClaimCategory::UserIntent => Self::UserIntent,
            ClaimCategory::MetaReasoning => Self::MetaReasoning,
            ClaimCategory::Unknown => Self::Unknown,
        }
    }
}

impl From<PyClaimCategory> for ClaimCategory {
    fn from(c: PyClaimCategory) -> Self {
        match c {
            PyClaimCategory::Factual => Self::Factual,
            PyClaimCategory::CodeBehavior => Self::CodeBehavior,
            PyClaimCategory::Relational => Self::Relational,
            PyClaimCategory::Numerical => Self::Numerical,
            PyClaimCategory::Temporal => Self::Temporal,
            PyClaimCategory::UserIntent => Self::UserIntent,
            PyClaimCategory::MetaReasoning => Self::MetaReasoning,
            PyClaimCategory::Unknown => Self::Unknown,
        }
    }
}

/// Python wrapper for GroundingStatus enum.
#[pyclass(name = "GroundingStatus", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyGroundingStatus {
    Grounded = 0,
    WeaklyGrounded = 1,
    Ungrounded = 2,
    Uncertain = 3,
}

impl From<GroundingStatus> for PyGroundingStatus {
    fn from(s: GroundingStatus) -> Self {
        match s {
            GroundingStatus::Grounded => Self::Grounded,
            GroundingStatus::WeaklyGrounded => Self::WeaklyGrounded,
            GroundingStatus::Ungrounded => Self::Ungrounded,
            GroundingStatus::Uncertain => Self::Uncertain,
        }
    }
}

/// Python wrapper for EvidenceType enum.
#[pyclass(name = "EvidenceType", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyEvidenceType {
    Citation = 0,
    CodeRef = 1,
    ToolOutput = 2,
    UserStatement = 3,
    Inference = 4,
    Prior = 5,
}

impl From<EvidenceType> for PyEvidenceType {
    fn from(e: EvidenceType) -> Self {
        match e {
            EvidenceType::Citation => Self::Citation,
            EvidenceType::CodeRef => Self::CodeRef,
            EvidenceType::ToolOutput => Self::ToolOutput,
            EvidenceType::UserStatement => Self::UserStatement,
            EvidenceType::Inference => Self::Inference,
            EvidenceType::Prior => Self::Prior,
        }
    }
}

impl From<PyEvidenceType> for EvidenceType {
    fn from(e: PyEvidenceType) -> Self {
        match e {
            PyEvidenceType::Citation => Self::Citation,
            PyEvidenceType::CodeRef => Self::CodeRef,
            PyEvidenceType::ToolOutput => Self::ToolOutput,
            PyEvidenceType::UserStatement => Self::UserStatement,
            PyEvidenceType::Inference => Self::Inference,
            PyEvidenceType::Prior => Self::Prior,
        }
    }
}

/// Python wrapper for VerificationVerdict enum.
#[pyclass(name = "VerificationVerdict", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyVerificationVerdict {
    Verified = 0,
    PartiallyVerified = 1,
    Unverified = 2,
    Error = 3,
}

impl From<VerificationVerdict> for PyVerificationVerdict {
    fn from(v: VerificationVerdict) -> Self {
        match v {
            VerificationVerdict::Verified => Self::Verified,
            VerificationVerdict::PartiallyVerified => Self::PartiallyVerified,
            VerificationVerdict::Unverified => Self::Unverified,
            VerificationVerdict::Error => Self::Error,
        }
    }
}

/// Python wrapper for Probability.
#[pyclass(name = "Probability")]
#[derive(Clone)]
pub struct PyProbability {
    inner: Probability,
}

#[pymethods]
impl PyProbability {
    /// Create a probability from a point estimate.
    #[staticmethod]
    fn point(p: f64) -> Self {
        Self {
            inner: Probability::point(p),
        }
    }

    /// Create a probability from samples (agreeing/total).
    #[staticmethod]
    fn from_samples(agreeing: u32, total: u32) -> Self {
        Self {
            inner: Probability::from_samples(agreeing, total),
        }
    }

    #[getter]
    fn estimate(&self) -> f64 {
        self.inner.estimate
    }

    #[getter]
    fn lower(&self) -> f64 {
        self.inner.lower
    }

    #[getter]
    fn upper(&self) -> f64 {
        self.inner.upper
    }

    #[getter]
    fn n_samples(&self) -> u32 {
        self.inner.n_samples
    }

    /// Compute KL divergence from self to other in bits.
    fn kl_divergence(&self, other: &PyProbability) -> f64 {
        self.inner.kl_divergence(&other.inner)
    }

    /// Get the uncertainty range.
    fn uncertainty(&self) -> f64 {
        self.inner.uncertainty()
    }

    fn __repr__(&self) -> String {
        format!(
            "Probability(estimate={:.3}, lower={:.3}, upper={:.3})",
            self.inner.estimate, self.inner.lower, self.inner.upper
        )
    }
}

/// Python wrapper for EvidenceRef.
#[pyclass(name = "EvidenceRef")]
#[derive(Clone)]
pub struct PyEvidenceRef {
    inner: EvidenceRef,
}

#[pymethods]
impl PyEvidenceRef {
    #[new]
    fn new(id: String, evidence_type: PyEvidenceType, description: String) -> Self {
        Self {
            inner: EvidenceRef::new(id, evidence_type.into(), description),
        }
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn evidence_type(&self) -> PyEvidenceType {
        self.inner.evidence_type.into()
    }

    #[getter]
    fn description(&self) -> String {
        self.inner.description.clone()
    }

    #[getter]
    fn strength(&self) -> f64 {
        self.inner.strength
    }

    fn with_strength(&self, strength: f64) -> Self {
        Self {
            inner: self.inner.clone().with_strength(strength),
        }
    }
}

/// Python wrapper for Claim.
#[pyclass(name = "Claim")]
#[derive(Clone)]
pub struct PyClaim {
    inner: Claim,
}

#[pymethods]
impl PyClaim {
    #[new]
    fn new(text: String, category: PyClaimCategory) -> Self {
        Self {
            inner: Claim::new(text, category.into()),
        }
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    #[getter]
    fn text(&self) -> String {
        self.inner.text.clone()
    }

    #[getter]
    fn category(&self) -> PyClaimCategory {
        self.inner.category.into()
    }

    #[getter]
    fn specificity(&self) -> f64 {
        self.inner.specificity
    }

    /// Set the specificity (0.0-1.0).
    fn with_specificity(&self, specificity: f64) -> Self {
        Self {
            inner: self.inner.clone().with_specificity(specificity),
        }
    }

    /// Get the required bits based on specificity.
    fn required_bits(&self) -> f64 {
        self.inner.required_bits()
    }

    fn __repr__(&self) -> String {
        format!(
            "Claim(text={:?}, category={:?}, specificity={:.2})",
            self.inner.text, self.inner.category, self.inner.specificity
        )
    }
}

/// Python wrapper for BudgetResult.
#[pyclass(name = "BudgetResult")]
#[derive(Clone)]
pub struct PyBudgetResult {
    inner: BudgetResult,
}

#[pymethods]
impl PyBudgetResult {
    #[new]
    fn new(claim_id: String, p0: &PyProbability, p1: &PyProbability, required_bits: f64) -> Self {
        let id = ClaimId(uuid::Uuid::parse_str(&claim_id).unwrap_or_else(|_| uuid::Uuid::new_v4()));
        Self {
            inner: BudgetResult::new(id, p0.inner, p1.inner, required_bits),
        }
    }

    #[getter]
    fn claim_id(&self) -> String {
        self.inner.claim_id.to_string()
    }

    #[getter]
    fn p0(&self) -> PyProbability {
        PyProbability {
            inner: self.inner.p0,
        }
    }

    #[getter]
    fn p1(&self) -> PyProbability {
        PyProbability {
            inner: self.inner.p1,
        }
    }

    #[getter]
    fn observed_bits(&self) -> f64 {
        self.inner.observed_bits
    }

    #[getter]
    fn required_bits(&self) -> f64 {
        self.inner.required_bits
    }

    #[getter]
    fn budget_gap(&self) -> f64 {
        self.inner.budget_gap
    }

    #[getter]
    fn status(&self) -> PyGroundingStatus {
        self.inner.status.into()
    }

    #[getter]
    fn confidence(&self) -> f64 {
        self.inner.confidence
    }

    /// Check if the claim is within epistemic budget.
    fn is_grounded(&self) -> bool {
        self.inner.is_grounded()
    }

    /// Check if this should trigger a hallucination flag.
    fn should_flag(&self, threshold: f64) -> bool {
        self.inner.should_flag(threshold)
    }

    fn __repr__(&self) -> String {
        format!(
            "BudgetResult(status={:?}, budget_gap={:.2}, observed={:.2}, required={:.2})",
            self.inner.status, self.inner.budget_gap, self.inner.observed_bits, self.inner.required_bits
        )
    }
}

/// Python wrapper for VerificationConfig.
#[pyclass(name = "VerificationConfig")]
#[derive(Clone)]
pub struct PyVerificationConfig {
    pub(crate) inner: VerificationConfig,
}

#[pymethods]
impl PyVerificationConfig {
    #[new]
    fn new() -> Self {
        Self {
            inner: VerificationConfig::default(),
        }
    }

    /// Create a fast configuration (low latency).
    #[staticmethod]
    fn fast() -> Self {
        Self {
            inner: VerificationConfig::fast(),
        }
    }

    /// Create a thorough configuration (high accuracy).
    #[staticmethod]
    fn thorough() -> Self {
        Self {
            inner: VerificationConfig::thorough(),
        }
    }

    #[getter]
    fn n_samples(&self) -> u32 {
        self.inner.n_samples
    }

    #[setter]
    fn set_n_samples(&mut self, value: u32) {
        self.inner.n_samples = value;
    }

    #[getter]
    fn hallucination_threshold(&self) -> f64 {
        self.inner.hallucination_threshold
    }

    #[setter]
    fn set_hallucination_threshold(&mut self, value: f64) {
        self.inner.hallucination_threshold = value;
    }

    #[getter]
    fn max_latency_ms(&self) -> u64 {
        self.inner.max_latency_ms
    }

    #[setter]
    fn set_max_latency_ms(&mut self, value: u64) {
        self.inner.max_latency_ms = value;
    }
}

/// Python wrapper for VerificationStats.
#[pyclass(name = "VerificationStats")]
#[derive(Clone)]
pub struct PyVerificationStats {
    inner: VerificationStats,
}

#[pymethods]
impl PyVerificationStats {
    #[getter]
    fn total_claims(&self) -> u32 {
        self.inner.total_claims
    }

    #[getter]
    fn grounded_claims(&self) -> u32 {
        self.inner.grounded_claims
    }

    #[getter]
    fn weakly_grounded_claims(&self) -> u32 {
        self.inner.weakly_grounded_claims
    }

    #[getter]
    fn ungrounded_claims(&self) -> u32 {
        self.inner.ungrounded_claims
    }

    #[getter]
    fn uncertain_claims(&self) -> u32 {
        self.inner.uncertain_claims
    }

    #[getter]
    fn avg_budget_gap(&self) -> f64 {
        self.inner.avg_budget_gap
    }

    #[getter]
    fn max_budget_gap(&self) -> f64 {
        self.inner.max_budget_gap
    }

    /// Calculate hallucination rate (ungrounded / total).
    fn hallucination_rate(&self) -> f64 {
        self.inner.hallucination_rate()
    }

    /// Calculate grounding rate (grounded / total).
    fn grounding_rate(&self) -> f64 {
        self.inner.grounding_rate()
    }

    fn __repr__(&self) -> String {
        format!(
            "VerificationStats(total={}, grounded={}, ungrounded={}, hallucination_rate={:.1}%)",
            self.inner.total_claims,
            self.inner.grounded_claims,
            self.inner.ungrounded_claims,
            self.inner.hallucination_rate() * 100.0
        )
    }
}

/// Python wrapper for ClaimExtractor.
#[pyclass(name = "ClaimExtractor")]
pub struct PyClaimExtractor {
    inner: ClaimExtractor,
}

#[pymethods]
impl PyClaimExtractor {
    #[new]
    fn new() -> Self {
        Self {
            inner: ClaimExtractor::new(),
        }
    }

    /// Extract claims from text.
    fn extract(&self, text: &str) -> Vec<PyClaim> {
        self.inner
            .extract(text)
            .into_iter()
            .map(|c| PyClaim { inner: c })
            .collect()
    }
}

/// KL divergence functions module.
#[pyclass(name = "KL")]
pub struct PyKL;

#[pymethods]
impl PyKL {
    /// Compute Bernoulli KL divergence in bits.
    #[staticmethod]
    fn bernoulli_kl_bits(p: f64, q: f64) -> f64 {
        epistemic::bernoulli_kl_bits(p, q)
    }

    /// Compute binary entropy in bits.
    #[staticmethod]
    fn binary_entropy_bits(p: f64) -> f64 {
        epistemic::binary_entropy_bits(p)
    }

    /// Compute required bits for a given specificity.
    #[staticmethod]
    fn required_bits_for_specificity(specificity: f64) -> f64 {
        epistemic::required_bits_for_specificity(specificity)
    }

    /// Compute Jensen-Shannon divergence in bits.
    #[staticmethod]
    fn jensen_shannon_bits(p: f64, q: f64) -> f64 {
        epistemic::jensen_shannon_bits(p, q)
    }

    /// Compute surprise in bits.
    #[staticmethod]
    fn surprise_bits(p: f64) -> f64 {
        epistemic::surprise_bits(p)
    }

    /// Aggregate evidence bits from multiple sources.
    #[staticmethod]
    fn aggregate_evidence_bits(bits_list: Vec<f64>) -> f64 {
        epistemic::aggregate_evidence_bits(&bits_list)
    }
}

/// Quick hallucination check without full LLM verification.
#[pyfunction]
pub fn quick_hallucination_check(response: &str) -> f64 {
    epistemic::quick_hallucination_check(response)
}
