//! Python bindings for rlm-core using PyO3.
//!
//! This module provides Python bindings for the core rlm-core functionality,
//! enabling use from Claude Code plugins and other Python applications.

#[cfg(feature = "python")]
mod adversarial;
#[cfg(feature = "python")]
mod context;
#[cfg(feature = "python")]
mod epistemic;
#[cfg(feature = "python")]
mod llm;
#[cfg(feature = "python")]
mod memory;
#[cfg(feature = "python")]
mod trajectory;

#[cfg(feature = "python")]
use pyo3::prelude::*;

/// Initialize the rlm_core Python module.
#[cfg(feature = "python")]
#[pymodule]
fn rlm_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Context types
    m.add_class::<context::PyMessage>()?;
    m.add_class::<context::PyToolOutput>()?;
    m.add_class::<context::PySessionContext>()?;
    m.add_class::<context::PyRole>()?;

    // Memory types
    m.add_class::<memory::PyNode>()?;
    m.add_class::<memory::PyNodeType>()?;
    m.add_class::<memory::PyTier>()?;
    m.add_class::<memory::PyHyperEdge>()?;
    m.add_class::<memory::PyMemoryStore>()?;
    m.add_class::<memory::PyMemoryStats>()?;

    // LLM types
    m.add_class::<llm::PyProvider>()?;
    m.add_class::<llm::PyModelTier>()?;
    m.add_class::<llm::PyModelSpec>()?;
    m.add_class::<llm::PyChatMessage>()?;
    m.add_class::<llm::PyCompletionRequest>()?;
    m.add_class::<llm::PyCompletionResponse>()?;
    m.add_class::<llm::PyTokenUsage>()?;
    m.add_class::<llm::PyQueryType>()?;
    m.add_class::<llm::PyRoutingContext>()?;
    m.add_class::<llm::PySmartRouter>()?;
    m.add_class::<llm::PyRoutingDecision>()?;
    m.add_class::<llm::PyCostTracker>()?;

    // Trajectory types
    m.add_class::<trajectory::PyTrajectoryEvent>()?;
    m.add_class::<trajectory::PyTrajectoryEventType>()?;

    // Complexity types
    m.add_class::<PyActivationDecision>()?;
    m.add_class::<PyPatternClassifier>()?;

    // Epistemic verification types
    m.add_class::<epistemic::PyClaimCategory>()?;
    m.add_class::<epistemic::PyGroundingStatus>()?;
    m.add_class::<epistemic::PyEvidenceType>()?;
    m.add_class::<epistemic::PyVerificationVerdict>()?;
    m.add_class::<epistemic::PyProbability>()?;
    m.add_class::<epistemic::PyEvidenceRef>()?;
    m.add_class::<epistemic::PyClaim>()?;
    m.add_class::<epistemic::PyBudgetResult>()?;
    m.add_class::<epistemic::PyVerificationConfig>()?;
    m.add_class::<epistemic::PyVerificationStats>()?;
    m.add_class::<epistemic::PyClaimExtractor>()?;
    m.add_class::<epistemic::PyKL>()?;
    m.add_function(wrap_pyfunction!(epistemic::quick_hallucination_check, m)?)?;

    // Adversarial validation types
    m.add_class::<adversarial::PyIssueSeverity>()?;
    m.add_class::<adversarial::PyIssueCategory>()?;
    m.add_class::<adversarial::PyValidationVerdict>()?;
    m.add_class::<adversarial::PyAdversarialTrigger>()?;
    m.add_class::<adversarial::PyCodeFile>()?;
    m.add_class::<adversarial::PyToolOutput>()?;
    m.add_class::<adversarial::PyIssueLocation>()?;
    m.add_class::<adversarial::PyIssue>()?;
    m.add_class::<adversarial::PyValidationContext>()?;
    m.add_class::<adversarial::PyValidationStats>()?;
    m.add_class::<adversarial::PyValidationResult>()?;
    m.add_class::<adversarial::PyAdversarialConfig>()?;

    Ok(())
}

/// Python wrapper for ActivationDecision.
#[cfg(feature = "python")]
#[pyclass(name = "ActivationDecision")]
#[derive(Clone)]
pub struct PyActivationDecision {
    inner: crate::complexity::ActivationDecision,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyActivationDecision {
    #[getter]
    fn should_activate(&self) -> bool {
        self.inner.should_activate
    }

    #[getter]
    fn reason(&self) -> String {
        self.inner.reason.clone()
    }

    #[getter]
    fn score(&self) -> i32 {
        self.inner.score
    }

    fn __repr__(&self) -> String {
        format!(
            "ActivationDecision(should_activate={}, score={}, reason={:?})",
            self.inner.should_activate, self.inner.score, self.inner.reason
        )
    }
}

/// Python wrapper for PatternClassifier.
#[cfg(feature = "python")]
#[pyclass(name = "PatternClassifier")]
pub struct PyPatternClassifier {
    inner: crate::complexity::PatternClassifier,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyPatternClassifier {
    #[new]
    fn new() -> Self {
        Self {
            inner: crate::complexity::PatternClassifier::new(),
        }
    }

    /// Check if RLM should activate for a query.
    fn should_activate(
        &self,
        query: &str,
        context: &context::PySessionContext,
    ) -> PyActivationDecision {
        PyActivationDecision {
            inner: self.inner.should_activate(query, &context.inner),
        }
    }
}
