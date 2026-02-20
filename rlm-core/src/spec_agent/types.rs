//! Type definitions for the Spec Agent.
//!
//! This module defines the core types for specification agents that transform
//! natural language requirements into formal specifications (Topos + Lean).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Level of formalization to target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormalizationLevel {
    /// Only type definitions (structures, enums).
    Types,
    /// Types plus invariants (constraints on data).
    Invariants,
    /// Types, invariants, plus function contracts (pre/post conditions).
    Contracts,
    /// Full formal proofs of correctness properties.
    FullProofs,
}

impl FormalizationLevel {
    /// Returns true if this level includes type definitions.
    pub fn includes_types(&self) -> bool {
        true // All levels include types
    }

    /// Returns true if this level includes invariants.
    pub fn includes_invariants(&self) -> bool {
        matches!(self, Self::Invariants | Self::Contracts | Self::FullProofs)
    }

    /// Returns true if this level includes contracts.
    pub fn includes_contracts(&self) -> bool {
        matches!(self, Self::Contracts | Self::FullProofs)
    }

    /// Returns true if this level includes full proofs.
    pub fn includes_proofs(&self) -> bool {
        matches!(self, Self::FullProofs)
    }
}

impl Default for FormalizationLevel {
    fn default() -> Self {
        Self::Contracts
    }
}

/// Completeness mode for generated Topos/Lean artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletenessMode {
    /// Generate executable/non-placeholder baseline stubs.
    ///
    /// This mode avoids `TODO` and `sorry` markers by emitting permissive
    /// baseline definitions such as `True` predicates and trivial proofs.
    Baseline,
    /// Emit draft-oriented placeholders (`TODO`, `sorry`) for manual completion.
    Placeholder,
}

impl Default for CompletenessMode {
    fn default() -> Self {
        Self::Baseline
    }
}

/// Domain of the specification being written.
///
/// This represents the high-level application domain for requirements,
/// distinct from [`crate::proof::SpecDomain`] which represents mathematical proof domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApplicationDomain {
    /// Algorithm specifications (sorting, searching, graph algorithms, etc.).
    Algorithms,
    /// Distributed systems (consensus, replication, messaging, etc.).
    DistributedSystems,
    /// API specifications (REST, GraphQL, RPC contracts).
    APIs,
    /// Security specifications (auth, crypto, access control).
    Security,
    /// Application flow (workflows, state machines, business logic).
    ApplicationFlow,
    /// Data models and schemas.
    DataModels,
    /// Concurrent/parallel programming.
    Concurrency,
}

/// Legacy alias retained for backward compatibility.
pub type SpecDomain = ApplicationDomain;

impl ApplicationDomain {
    /// Get suggested Lean libraries for this domain.
    pub fn suggested_lean_imports(&self) -> Vec<&'static str> {
        match self {
            Self::Algorithms => vec!["Mathlib.Data.List.Sort", "Mathlib.Data.Finset.Basic"],
            Self::DistributedSystems => {
                vec!["Mathlib.Data.Set.Basic", "Mathlib.Order.PartialOrder"]
            }
            Self::APIs => vec!["Mathlib.Data.String.Basic", "Mathlib.Data.Option.Basic"],
            Self::Security => vec!["Mathlib.Data.ByteArray", "Mathlib.Algebra.Group.Basic"],
            Self::ApplicationFlow => vec!["Mathlib.Data.Finset.Basic", "Mathlib.Logic.Basic"],
            Self::DataModels => vec!["Mathlib.Data.List.Basic", "Mathlib.Data.Option.Basic"],
            Self::Concurrency => vec!["Mathlib.Data.Set.Basic", "Mathlib.Order.Lattice"],
        }
    }

    /// Get common patterns for this domain.
    pub fn common_patterns(&self) -> Vec<&'static str> {
        match self {
            Self::Algorithms => vec!["termination", "correctness", "complexity"],
            Self::DistributedSystems => {
                vec![
                    "safety",
                    "liveness",
                    "eventual_consistency",
                    "linearizability",
                ]
            }
            Self::APIs => vec!["request_response", "idempotency", "validation"],
            Self::Security => vec![
                "authentication",
                "authorization",
                "confidentiality",
                "integrity",
            ],
            Self::ApplicationFlow => vec!["state_transition", "workflow", "saga"],
            Self::DataModels => vec!["schema", "relationship", "constraint"],
            Self::Concurrency => vec![
                "mutual_exclusion",
                "deadlock_freedom",
                "progress",
                "atomicity",
            ],
        }
    }
}

/// Strategy for attempting proofs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofStrategy {
    /// Don't attempt proofs, leave as sorry.
    Skip,
    /// Try basic automation (simp, decide, trivial).
    BasicAuto,
    /// Use hammer tactics (aesop, omega).
    Hammer,
    /// Interactive proof with AI assistance.
    Interactive,
}

impl Default for ProofStrategy {
    fn default() -> Self {
        Self::BasicAuto
    }
}

/// Configuration for the Spec Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecAgentConfig {
    /// Target formalization level.
    pub formalization_level: FormalizationLevel,
    /// Domains this agent specializes in.
    pub domains: Vec<ApplicationDomain>,
    /// Strategy for proof attempts.
    pub proof_strategy: ProofStrategy,
    /// Completeness mode used by Topos/Lean generators.
    pub completeness_mode: CompletenessMode,
    /// Maximum number of clarification rounds.
    pub max_clarification_rounds: u32,
    /// Whether to generate cross-references between Topos and Lean.
    pub generate_cross_refs: bool,
    /// Whether to validate generated specs with the Lean REPL.
    pub validate_with_lean: bool,
    /// Whether to validate generated specs with the Topos client.
    pub validate_with_topos: bool,
    /// Timeout for LLM calls in milliseconds.
    pub llm_timeout_ms: u64,
}

impl Default for SpecAgentConfig {
    fn default() -> Self {
        Self {
            formalization_level: FormalizationLevel::Contracts,
            domains: vec![ApplicationDomain::ApplicationFlow],
            proof_strategy: ProofStrategy::BasicAuto,
            completeness_mode: CompletenessMode::Baseline,
            max_clarification_rounds: 3,
            generate_cross_refs: true,
            validate_with_lean: true,
            validate_with_topos: true,
            llm_timeout_ms: 30_000,
        }
    }
}

impl SpecAgentConfig {
    /// Create a minimal configuration (types only, no validation).
    pub fn minimal() -> Self {
        Self {
            formalization_level: FormalizationLevel::Types,
            domains: Vec::new(),
            proof_strategy: ProofStrategy::Skip,
            completeness_mode: CompletenessMode::Baseline,
            max_clarification_rounds: 1,
            generate_cross_refs: false,
            validate_with_lean: false,
            validate_with_topos: false,
            llm_timeout_ms: 30_000,
        }
    }

    /// Create a full configuration (proofs with validation).
    pub fn full() -> Self {
        Self {
            formalization_level: FormalizationLevel::FullProofs,
            proof_strategy: ProofStrategy::Hammer,
            max_clarification_rounds: 5,
            ..Default::default()
        }
    }

    /// Set the formalization level.
    pub fn with_level(mut self, level: FormalizationLevel) -> Self {
        self.formalization_level = level;
        self
    }

    /// Add a domain.
    pub fn with_domain(mut self, domain: ApplicationDomain) -> Self {
        if !self.domains.contains(&domain) {
            self.domains.push(domain);
        }
        self
    }

    /// Set the proof strategy.
    pub fn with_proof_strategy(mut self, strategy: ProofStrategy) -> Self {
        self.proof_strategy = strategy;
        self
    }

    /// Set the completeness mode used by spec generators.
    pub fn with_completeness_mode(mut self, mode: CompletenessMode) -> Self {
        self.completeness_mode = mode;
        self
    }
}

/// Phase of the specification workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecPhase {
    /// Initial intake of natural language requirements.
    Intake,
    /// Refining requirements through clarifying questions.
    Refine,
    /// Generating formal specifications.
    Formalize,
    /// Verifying specifications (type checking, proof attempts).
    Verify,
}

impl SpecPhase {
    /// Get the next phase in the workflow.
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Intake => Some(Self::Refine),
            Self::Refine => Some(Self::Formalize),
            Self::Formalize => Some(Self::Verify),
            Self::Verify => None,
        }
    }

    /// Get the previous phase in the workflow.
    pub fn prev(&self) -> Option<Self> {
        match self {
            Self::Intake => None,
            Self::Refine => Some(Self::Intake),
            Self::Formalize => Some(Self::Refine),
            Self::Verify => Some(Self::Formalize),
        }
    }
}

/// A clarifying question generated during the Refine phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    /// Unique identifier for this question.
    pub id: String,
    /// The question text.
    pub text: String,
    /// Category of the question.
    pub category: QuestionCategory,
    /// Why this question is being asked.
    pub rationale: String,
    /// Suggested answers (if applicable).
    pub suggestions: Vec<String>,
    /// Whether an answer is required to proceed.
    pub required: bool,
}

/// Category of clarifying question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestionCategory {
    /// Clarifying the domain or scope.
    Scope,
    /// Clarifying data types or structures.
    DataTypes,
    /// Clarifying invariants or constraints.
    Invariants,
    /// Clarifying behavior or function contracts.
    Behavior,
    /// Clarifying edge cases or error handling.
    EdgeCases,
    /// Clarifying performance or resource constraints.
    Performance,
    /// Clarifying security requirements.
    Security,
}

/// An answer to a clarifying question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    /// ID of the question being answered.
    pub question_id: String,
    /// The answer text.
    pub text: String,
    /// Additional context or notes.
    pub notes: Option<String>,
}

/// Context maintained throughout the specification workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecContext {
    /// Original natural language input.
    pub nl_input: String,
    /// Current phase of the workflow.
    pub phase: SpecPhase,
    /// Extracted requirements from NL input.
    pub requirements: Vec<ExtractedRequirement>,
    /// Questions asked during refinement.
    pub questions: Vec<Question>,
    /// Answers received.
    pub answers: Vec<Answer>,
    /// Generated Topos specification.
    pub topos_spec: Option<String>,
    /// Generated Lean specification.
    pub lean_spec: Option<String>,
    /// Detected domains.
    pub detected_domains: Vec<ApplicationDomain>,
    /// Identified ambiguities that need clarification.
    pub ambiguities: Vec<Ambiguity>,
    /// Metadata for the context.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SpecContext {
    /// Create a new context from natural language input.
    pub fn new(nl_input: impl Into<String>) -> Self {
        Self {
            nl_input: nl_input.into(),
            phase: SpecPhase::Intake,
            requirements: Vec::new(),
            questions: Vec::new(),
            answers: Vec::new(),
            topos_spec: None,
            lean_spec: None,
            detected_domains: Vec::new(),
            ambiguities: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Advance to the next phase.
    pub fn advance_phase(&mut self) -> Option<SpecPhase> {
        if let Some(next) = self.phase.next() {
            self.phase = next;
            Some(next)
        } else {
            None
        }
    }

    /// Check if all required questions have been answered.
    pub fn all_required_answered(&self) -> bool {
        self.questions
            .iter()
            .filter(|q| q.required)
            .all(|q| self.answers.iter().any(|a| a.question_id == q.id))
    }

    /// Get unanswered questions.
    pub fn unanswered_questions(&self) -> Vec<&Question> {
        self.questions
            .iter()
            .filter(|q| !self.answers.iter().any(|a| a.question_id == q.id))
            .collect()
    }

    /// Add a requirement.
    pub fn add_requirement(&mut self, req: ExtractedRequirement) {
        self.requirements.push(req);
    }

    /// Add an ambiguity.
    pub fn add_ambiguity(&mut self, ambiguity: Ambiguity) {
        self.ambiguities.push(ambiguity);
    }
}

/// A requirement extracted from natural language.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRequirement {
    /// Unique identifier.
    pub id: String,
    /// The requirement text (normalized).
    pub text: String,
    /// Type of requirement.
    pub req_type: RequirementType,
    /// Confidence in the extraction (0.0 - 1.0).
    pub confidence: f64,
    /// Source span in the original input.
    pub source_span: Option<(usize, usize)>,
    /// Related entities mentioned.
    pub entities: Vec<String>,
    /// Suggested formal name.
    pub formal_name: Option<String>,
}

/// Type of extracted requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequirementType {
    /// A data structure or type definition.
    DataStructure,
    /// A behavioral requirement (function, operation).
    Behavior,
    /// A constraint or invariant.
    Constraint,
    /// A property that should hold.
    Property,
    /// An error case or edge condition.
    ErrorCase,
    /// A non-functional requirement (performance, etc.).
    NonFunctional,
}

/// An identified ambiguity in the input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ambiguity {
    /// Description of the ambiguity.
    pub description: String,
    /// The ambiguous text from the input.
    pub source_text: String,
    /// Possible interpretations.
    pub interpretations: Vec<String>,
    /// Severity of the ambiguity.
    pub severity: AmbiguitySeverity,
}

/// Severity of an ambiguity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AmbiguitySeverity {
    /// Minor ambiguity, can proceed with assumptions.
    Low,
    /// Moderate ambiguity, should clarify but not blocking.
    Medium,
    /// High ambiguity, must clarify before proceeding.
    High,
}

/// Result of the formalization phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormalizationResult {
    /// Generated Topos specification content.
    pub topos_content: String,
    /// Suggested filename for Topos spec.
    pub topos_filename: String,
    /// Generated Lean specification content.
    pub lean_content: String,
    /// Suggested filename for Lean spec.
    pub lean_filename: String,
    /// Cross-references between Topos and Lean.
    pub cross_refs: Vec<CrossReference>,
    /// Warnings generated during formalization.
    pub warnings: Vec<String>,
}

/// A cross-reference between Topos and Lean artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossReference {
    /// Topos element name.
    pub topos_element: String,
    /// Lean artifact name.
    pub lean_artifact: String,
    /// Type of reference.
    pub ref_type: String,
}

/// Result of the verification phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether Lean type checking succeeded.
    pub lean_type_check_ok: bool,
    /// Lean type checking errors.
    pub lean_errors: Vec<String>,
    /// Whether Topos validation succeeded.
    pub topos_valid: bool,
    /// Topos validation errors.
    pub topos_errors: Vec<String>,
    /// Proof results (for FullProofs level).
    pub proof_results: Vec<ProofResult>,
    /// Overall verification passed.
    pub passed: bool,
}

impl VerificationResult {
    /// Create a successful verification result.
    pub fn success() -> Self {
        Self {
            lean_type_check_ok: true,
            lean_errors: Vec::new(),
            topos_valid: true,
            topos_errors: Vec::new(),
            proof_results: Vec::new(),
            passed: true,
        }
    }

    /// Create a failed verification result.
    pub fn failure(lean_errors: Vec<String>, topos_errors: Vec<String>) -> Self {
        Self {
            lean_type_check_ok: lean_errors.is_empty(),
            lean_errors,
            topos_valid: topos_errors.is_empty(),
            topos_errors,
            proof_results: Vec::new(),
            passed: false,
        }
    }
}

/// Result of attempting a proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofResult {
    /// Name of the theorem/lemma.
    pub name: String,
    /// Whether the proof succeeded.
    pub proved: bool,
    /// Proof script if successful.
    pub proof_script: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
    /// Tactics tried.
    pub tactics_tried: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formalization_level_includes() {
        assert!(FormalizationLevel::Types.includes_types());
        assert!(!FormalizationLevel::Types.includes_invariants());

        assert!(FormalizationLevel::Contracts.includes_types());
        assert!(FormalizationLevel::Contracts.includes_invariants());
        assert!(FormalizationLevel::Contracts.includes_contracts());
        assert!(!FormalizationLevel::Contracts.includes_proofs());

        assert!(FormalizationLevel::FullProofs.includes_proofs());
    }

    #[test]
    fn test_spec_phase_navigation() {
        let phase = SpecPhase::Intake;
        assert_eq!(phase.next(), Some(SpecPhase::Refine));
        assert_eq!(phase.prev(), None);

        let phase = SpecPhase::Verify;
        assert_eq!(phase.next(), None);
        assert_eq!(phase.prev(), Some(SpecPhase::Formalize));
    }

    #[test]
    fn test_spec_context_creation() {
        let ctx = SpecContext::new("Users can create orders with multiple items");
        assert_eq!(ctx.phase, SpecPhase::Intake);
        assert!(ctx.requirements.is_empty());
        assert!(ctx.topos_spec.is_none());
    }

    #[test]
    fn test_spec_context_advance_phase() {
        let mut ctx = SpecContext::new("test");
        assert_eq!(ctx.phase, SpecPhase::Intake);

        ctx.advance_phase();
        assert_eq!(ctx.phase, SpecPhase::Refine);

        ctx.advance_phase();
        assert_eq!(ctx.phase, SpecPhase::Formalize);
    }

    #[test]
    fn test_config_builders() {
        let config = SpecAgentConfig::minimal();
        assert_eq!(config.formalization_level, FormalizationLevel::Types);
        assert!(!config.validate_with_lean);
        assert_eq!(config.completeness_mode, CompletenessMode::Baseline);

        let config = SpecAgentConfig::full();
        assert_eq!(config.formalization_level, FormalizationLevel::FullProofs);
        assert_eq!(config.proof_strategy, ProofStrategy::Hammer);
        assert_eq!(config.completeness_mode, CompletenessMode::Baseline);
    }

    #[test]
    fn test_completeness_mode_builder() {
        let config =
            SpecAgentConfig::default().with_completeness_mode(CompletenessMode::Placeholder);
        assert_eq!(config.completeness_mode, CompletenessMode::Placeholder);
    }

    #[test]
    fn test_spec_domain_patterns() {
        let domain = SpecDomain::DistributedSystems;
        let patterns = domain.common_patterns();
        assert!(patterns.contains(&"eventual_consistency"));
        assert!(patterns.contains(&"linearizability"));
    }
}
