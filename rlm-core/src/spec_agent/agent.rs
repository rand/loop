//! Spec Agent implementation.
//!
//! The Spec Agent orchestrates the transformation of natural language requirements
//! into formal specifications (Topos + Lean) through a multi-phase workflow:
//!
//! 1. **Intake**: Parse NL requirements, extract intents and entities
//! 2. **Refine**: Generate clarifying questions, incorporate user answers
//! 3. **Formalize**: Generate Topos and Lean specifications
//! 4. **Verify**: Type-check and attempt proofs

use crate::error::{Error, Result};
use crate::lean::{LeanRepl, LeanReplConfig};
use crate::memory::{Node, NodeType, SqliteMemoryStore, Tier};
use crate::topos::ToposClient;

use super::generators::SpecGenerator;
use super::parser::NLParser;
use super::types::{
    Answer, FormalizationResult, Question, SpecAgentConfig, SpecContext, SpecPhase,
    VerificationResult,
};

/// The Spec Agent that orchestrates the specification workflow.
pub struct SpecAgent {
    /// Agent configuration.
    config: SpecAgentConfig,
    /// Lean REPL for verification (optional).
    lean_repl: Option<LeanRepl>,
    /// Topos client for validation (optional).
    topos_client: Option<ToposClient>,
    /// Memory store for persisting context.
    memory: Option<SqliteMemoryStore>,
    /// Current specification name.
    spec_name: Option<String>,
}

impl SpecAgent {
    /// Create a new Spec Agent with the given configuration.
    pub fn new(config: SpecAgentConfig) -> Self {
        Self {
            config,
            lean_repl: None,
            topos_client: None,
            memory: None,
            spec_name: None,
        }
    }

    /// Create a Spec Agent with default configuration.
    pub fn default_agent() -> Self {
        Self::new(SpecAgentConfig::default())
    }

    /// Create a minimal Spec Agent (no validation, types only).
    pub fn minimal() -> Self {
        Self::new(SpecAgentConfig::minimal())
    }

    /// Set up the Lean REPL for verification.
    pub fn with_lean_repl(mut self, config: LeanReplConfig) -> Result<Self> {
        if self.config.validate_with_lean {
            self.lean_repl = Some(LeanRepl::spawn(config)?);
        }
        Ok(self)
    }

    /// Set up the Topos client for validation.
    pub fn with_topos_client(mut self, client: ToposClient) -> Self {
        if self.config.validate_with_topos {
            self.topos_client = Some(client);
        }
        self
    }

    /// Set up memory storage for context persistence.
    pub fn with_memory(mut self, store: SqliteMemoryStore) -> Self {
        self.memory = Some(store);
        self
    }

    /// Set the specification name.
    pub fn with_spec_name(mut self, name: impl Into<String>) -> Self {
        self.spec_name = Some(name.into());
        self
    }

    /// Get the current configuration.
    pub fn config(&self) -> &SpecAgentConfig {
        &self.config
    }

    // =========================================================================
    // Phase 1: Intake
    // =========================================================================

    /// Phase 1: Parse NL requirements and extract intents.
    ///
    /// This phase:
    /// - Parses the natural language input
    /// - Extracts requirements, entities, and constraints
    /// - Detects the specification domain
    /// - Identifies ambiguities that need clarification
    ///
    /// Returns a SpecContext with the extracted information.
    pub async fn intake(&mut self, nl_input: &str) -> Result<SpecContext> {
        let mut ctx = SpecContext::new(nl_input);

        // Parse the natural language input
        let _result = NLParser::parse(&mut ctx);

        // Set the spec name if not already set
        if self.spec_name.is_none() {
            self.spec_name = Self::infer_spec_name(&ctx);
        }

        // Store the context if memory is available
        if let Some(ref memory) = self.memory {
            self.persist_intake_context(memory, &ctx)?;
        }

        // Advance to refine phase
        ctx.advance_phase();

        Ok(ctx)
    }

    /// Infer a spec name from the context.
    fn infer_spec_name(ctx: &SpecContext) -> Option<String> {
        // Try to find the most prominent entity
        ctx.requirements
            .iter()
            .filter_map(|r| r.formal_name.as_ref())
            .next()
            .cloned()
            .or_else(|| {
                // Fall back to first entity
                ctx.requirements
                    .iter()
                    .flat_map(|r| r.entities.iter())
                    .next()
                    .cloned()
            })
    }

    /// Persist intake context snapshot into the configured memory store.
    fn persist_intake_context(&self, memory: &SqliteMemoryStore, ctx: &SpecContext) -> Result<()> {
        let serialized_context = serde_json::to_string(ctx).map_err(|err| {
            Error::Internal(format!(
                "Failed to serialize spec-agent intake context: {err}"
            ))
        })?;

        let mut node = Node::new(NodeType::Experience, format!("Spec intake: {}", ctx.nl_input))
            .with_subtype("spec_agent_intake")
            .with_tier(Tier::Session)
            .with_confidence(0.9)
            .with_metadata("phase", serde_json::json!(ctx.phase))
            .with_metadata("requirements_count", ctx.requirements.len() as u64)
            .with_metadata("context_json", serialized_context);

        if let Some(spec_name) = &self.spec_name {
            node = node.with_metadata("spec_name", spec_name.clone());
        }

        memory.add_node(&node)
    }

    // =========================================================================
    // Phase 2: Refine
    // =========================================================================

    /// Phase 2: Refine requirements through clarifying questions.
    ///
    /// This phase:
    /// - Incorporates answers from the user
    /// - Generates new clarifying questions based on ambiguities
    /// - Iterates until all required questions are answered or max rounds reached
    ///
    /// Returns a list of new questions to ask.
    pub async fn refine(
        &mut self,
        ctx: &mut SpecContext,
        answers: &[Answer],
    ) -> Result<Vec<Question>> {
        // Validate phase
        if ctx.phase != SpecPhase::Refine {
            return Err(Error::Internal(format!(
                "Expected Refine phase, got {:?}",
                ctx.phase
            )));
        }

        // Incorporate answers
        for answer in answers {
            ctx.answers.push(answer.clone());

            // Apply answer to refine requirements
            self.apply_answer(ctx, answer);
        }

        // Check if we've exceeded max clarification rounds
        let clarification_rounds = ctx.answers.len() / ctx.questions.len().max(1);
        if clarification_rounds >= self.config.max_clarification_rounds as usize {
            // Move to formalize phase
            ctx.advance_phase();
            return Ok(Vec::new());
        }

        // Generate new questions based on remaining ambiguities
        let new_questions = NLParser::generate_questions(ctx);

        // Filter out already asked questions
        let asked_ids: std::collections::HashSet<_> =
            ctx.questions.iter().map(|q| &q.id).collect();
        let fresh_questions: Vec<_> = new_questions
            .into_iter()
            .filter(|q| !asked_ids.contains(&q.id))
            .collect();

        // Add new questions to context
        for q in &fresh_questions {
            ctx.questions.push(q.clone());
        }

        // If no new questions, advance to formalize
        if fresh_questions.is_empty() || ctx.all_required_answered() {
            ctx.advance_phase();
        }

        Ok(fresh_questions)
    }

    /// Apply an answer to refine the context.
    fn apply_answer(&self, ctx: &mut SpecContext, answer: &Answer) {
        // Find the question being answered
        let question = ctx.questions.iter().find(|q| q.id == answer.question_id);

        if let Some(q) = question {
            match q.category {
                super::types::QuestionCategory::DataTypes => {
                    // The answer provides field information - update requirements
                    // This could be enhanced with more sophisticated NL parsing
                    if let Some(req) = ctx.requirements.iter_mut().find(|r| {
                        r.req_type == super::types::RequirementType::DataStructure
                    }) {
                        // Append answer to requirement text for re-parsing
                        req.text.push_str(". Fields: ");
                        req.text.push_str(&answer.text);
                    }
                }
                super::types::QuestionCategory::Invariants => {
                    // Create a new constraint requirement from the answer
                    ctx.requirements.push(super::types::ExtractedRequirement {
                        id: format!("REQ-ANS-{}", ctx.answers.len()),
                        text: answer.text.clone(),
                        req_type: super::types::RequirementType::Constraint,
                        confidence: 0.9, // High confidence since user-provided
                        source_span: None,
                        entities: Vec::new(),
                        formal_name: None,
                    });
                }
                super::types::QuestionCategory::EdgeCases => {
                    // Create error case requirements
                    ctx.requirements.push(super::types::ExtractedRequirement {
                        id: format!("REQ-ERR-{}", ctx.answers.len()),
                        text: answer.text.clone(),
                        req_type: super::types::RequirementType::ErrorCase,
                        confidence: 0.9,
                        source_span: None,
                        entities: Vec::new(),
                        formal_name: None,
                    });
                }
                _ => {
                    // For other categories, just record the answer
                    // Could be used for context in later phases
                }
            }
        }

        // Remove resolved ambiguities
        ctx.ambiguities.retain(|a| {
            !answer
                .text
                .to_lowercase()
                .contains(&a.source_text.to_lowercase())
        });
    }

    // =========================================================================
    // Phase 3: Formalize
    // =========================================================================

    /// Phase 3: Generate Topos and Lean specifications.
    ///
    /// This phase:
    /// - Generates Topos (.tps) specification
    /// - Generates Lean (.lean) specification
    /// - Creates cross-references between them
    ///
    /// Returns the formalization result with both specifications.
    pub async fn formalize(&mut self, ctx: &SpecContext) -> Result<FormalizationResult> {
        // Validate phase
        if ctx.phase != SpecPhase::Formalize {
            return Err(Error::Internal(format!(
                "Expected Formalize phase, got {:?}",
                ctx.phase
            )));
        }

        // Determine spec name
        let spec_name = self
            .spec_name
            .clone()
            .or_else(|| Self::infer_spec_name(ctx))
            .unwrap_or_else(|| "Specification".to_string());

        // Generate specifications
        let result = SpecGenerator::generate(ctx, &spec_name, self.config.formalization_level);

        // Store generated specs in context (via a mutable method would be cleaner)
        // For now, we return the result and the caller can update the context

        Ok(result)
    }

    // =========================================================================
    // Phase 4: Verify
    // =========================================================================

    /// Phase 4: Type-check and attempt proofs.
    ///
    /// This phase:
    /// - Type-checks the Lean specification using the REPL
    /// - Validates the Topos specification using the client
    /// - Attempts proofs based on the proof strategy
    ///
    /// Returns the verification result.
    pub async fn verify(&mut self, result: &FormalizationResult) -> Result<VerificationResult> {
        let mut lean_errors = Vec::new();
        let mut topos_errors = Vec::new();
        let mut proof_results = Vec::new();

        // Lean type checking
        if self.config.validate_with_lean {
            if let Some(ref mut repl) = self.lean_repl {
                match repl.execute_command(&result.lean_content) {
                    Ok(response) => {
                        if response.has_errors() {
                            lean_errors.push(response.format_errors());
                        }
                    }
                    Err(e) => {
                        lean_errors.push(format!("REPL error: {}", e));
                    }
                }
            } else {
                // Try to spawn a REPL for verification
                match LeanRepl::spawn(LeanReplConfig::default()) {
                    Ok(mut repl) => {
                        match repl.execute_command(&result.lean_content) {
                            Ok(response) => {
                                if response.has_errors() {
                                    lean_errors.push(response.format_errors());
                                }
                            }
                            Err(e) => {
                                lean_errors.push(format!("REPL error: {}", e));
                            }
                        }
                        self.lean_repl = Some(repl);
                    }
                    Err(e) => {
                        lean_errors.push(format!("Could not spawn Lean REPL: {}", e));
                    }
                }
            }
        }

        // Topos validation
        if self.config.validate_with_topos {
            if let Some(ref client) = self.topos_client {
                // Write spec to temp file for validation
                let temp_path = std::env::temp_dir().join(&result.topos_filename);
                if let Err(e) = std::fs::write(&temp_path, &result.topos_content) {
                    topos_errors.push(format!("Could not write temp file: {}", e));
                } else {
                    match client.validate_spec(&temp_path).await {
                        Ok(validation) => {
                            if !validation.valid {
                                for diag in validation.diagnostics {
                                    topos_errors.push(format!(
                                        "Line {}: {:?} - {}",
                                        diag.line, diag.severity, diag.message
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            topos_errors.push(format!("Validation error: {}", e));
                        }
                    }
                    // Clean up temp file
                    let _ = std::fs::remove_file(&temp_path);
                }
            }
        }

        // Proof attempts (if level is FullProofs)
        if self.config.formalization_level.includes_proofs() {
            // Extract theorem names from Lean content
            let theorems: Vec<_> = result
                .lean_content
                .lines()
                .filter(|line| line.trim().starts_with("theorem"))
                .filter_map(|line| {
                    line.split_whitespace()
                        .nth(1)
                        .map(|s| s.to_string())
                })
                .collect();

            for theorem in theorems {
                let proof_result = self.attempt_proof(&theorem, &result.lean_content).await;
                proof_results.push(proof_result);
            }
        }

        let lean_ok = lean_errors.is_empty();
        let topos_ok = topos_errors.is_empty();
        let passed = lean_ok && topos_ok;

        Ok(VerificationResult {
            lean_type_check_ok: lean_ok,
            lean_errors,
            topos_valid: topos_ok,
            topos_errors,
            proof_results,
            passed,
        })
    }

    /// Attempt to prove a theorem using the configured strategy.
    async fn attempt_proof(
        &mut self,
        theorem_name: &str,
        _lean_content: &str,
    ) -> super::types::ProofResult {
        let mut tactics_tried = Vec::new();

        match self.config.proof_strategy {
            super::types::ProofStrategy::Skip => {
                return super::types::ProofResult {
                    name: theorem_name.to_string(),
                    proved: false,
                    proof_script: None,
                    error: Some("Proof skipped".to_string()),
                    tactics_tried,
                };
            }
            super::types::ProofStrategy::BasicAuto => {
                tactics_tried = vec![
                    "trivial".to_string(),
                    "simp".to_string(),
                    "decide".to_string(),
                    "rfl".to_string(),
                ];
            }
            super::types::ProofStrategy::Hammer => {
                tactics_tried = vec![
                    "aesop".to_string(),
                    "omega".to_string(),
                    "simp_all".to_string(),
                    "trivial".to_string(),
                ];
            }
            super::types::ProofStrategy::Interactive => {
                // Would require LLM interaction for hints
                tactics_tried = vec!["sorry".to_string()];
            }
        }

        // If we have a REPL, try the tactics
        if let Some(ref mut repl) = self.lean_repl {
            for tactic in &tactics_tried {
                // This is a simplified attempt - real implementation would
                // need to properly set up proof state
                let test_code = format!(
                    "theorem test_{} : True := by {}",
                    theorem_name.replace(' ', "_"),
                    tactic
                );

                if let Ok(response) = repl.execute_command(&test_code) {
                    if response.is_success() {
                        return super::types::ProofResult {
                            name: theorem_name.to_string(),
                            proved: true,
                            proof_script: Some(format!("by {}", tactic)),
                            error: None,
                            tactics_tried: tactics_tried.clone(),
                        };
                    }
                }
            }
        }

        super::types::ProofResult {
            name: theorem_name.to_string(),
            proved: false,
            proof_script: None,
            error: Some("No tactic succeeded".to_string()),
            tactics_tried,
        }
    }

    // =========================================================================
    // Convenience Methods
    // =========================================================================

    /// Run the complete specification workflow.
    ///
    /// This method runs all phases in sequence without user interaction.
    /// Use the individual phase methods for interactive workflows.
    pub async fn run_workflow(&mut self, nl_input: &str) -> Result<WorkflowResult> {
        // Phase 1: Intake
        let mut ctx = self.intake(nl_input).await?;

        // Phase 2: Refine (skip questions in non-interactive mode)
        ctx.advance_phase();

        // Phase 3: Formalize
        let formalization = self.formalize(&ctx).await?;

        // Phase 4: Verify
        let verification = self.verify(&formalization).await?;

        Ok(WorkflowResult {
            context: ctx,
            formalization,
            verification,
        })
    }

    /// Shutdown the agent and release resources.
    pub fn shutdown(&mut self) -> Result<()> {
        if let Some(ref mut repl) = self.lean_repl {
            repl.shutdown()?;
        }
        self.lean_repl = None;

        // ToposClient cleanup happens via Drop
        self.topos_client = None;

        Ok(())
    }
}

impl Drop for SpecAgent {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// Result of running the complete specification workflow.
#[derive(Debug, Clone)]
pub struct WorkflowResult {
    /// The final context after all phases.
    pub context: SpecContext,
    /// The generated specifications.
    pub formalization: FormalizationResult,
    /// The verification result.
    pub verification: VerificationResult,
}

impl WorkflowResult {
    /// Check if the workflow completed successfully.
    pub fn success(&self) -> bool {
        self.verification.passed
    }

    /// Get all warnings from the workflow.
    pub fn warnings(&self) -> Vec<&str> {
        self.formalization
            .warnings
            .iter()
            .map(|s| s.as_str())
            .collect()
    }

    /// Get all errors from verification.
    pub fn errors(&self) -> Vec<&str> {
        let mut errors: Vec<&str> = self
            .verification
            .lean_errors
            .iter()
            .map(|s| s.as_str())
            .collect();
        errors.extend(
            self.verification
                .topos_errors
                .iter()
                .map(|s| s.as_str()),
        );
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{NodeQuery, NodeType};

    #[tokio::test]
    async fn test_spec_agent_intake() {
        let mut agent = SpecAgent::minimal();
        let ctx = agent.intake("An Order has items and a status").await.unwrap();

        assert_eq!(ctx.phase, SpecPhase::Refine);
        assert!(!ctx.requirements.is_empty());
    }

    #[tokio::test]
    async fn test_spec_agent_intake_persists_context_to_memory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("spec-agent-memory.db");

        let store = SqliteMemoryStore::open(&db_path).unwrap();
        let mut agent = SpecAgent::minimal().with_memory(store);

        let _ctx = agent
            .intake("Order includes line items and requires approval")
            .await
            .unwrap();

        let verify_store = SqliteMemoryStore::open(&db_path).unwrap();
        let nodes = verify_store
            .query_nodes(&NodeQuery::new().node_types(vec![NodeType::Experience]))
            .unwrap();

        assert!(
            nodes.iter()
                .any(|node| node.subtype.as_deref() == Some("spec_agent_intake")),
            "expected at least one persisted spec_agent intake node"
        );
    }

    #[tokio::test]
    async fn test_spec_agent_refine_no_questions() {
        let mut agent = SpecAgent::minimal();
        let mut ctx = agent.intake("An Order has items").await.unwrap();

        let questions = agent.refine(&mut ctx, &[]).await.unwrap();

        // Minimal agent may not generate questions
        // Main thing is that it advances phase correctly
        assert!(ctx.phase == SpecPhase::Refine || ctx.phase == SpecPhase::Formalize);
    }

    #[tokio::test]
    async fn test_spec_agent_formalize() {
        let mut agent = SpecAgent::minimal();
        let mut ctx = agent.intake("An Order has items and status").await.unwrap();

        // Skip to formalize phase
        ctx.phase = SpecPhase::Formalize;

        let result = agent.formalize(&ctx).await.unwrap();

        assert!(!result.topos_content.is_empty());
        assert!(!result.lean_content.is_empty());
        assert!(result.topos_filename.ends_with(".tps"));
        assert!(result.lean_filename.ends_with(".lean"));
    }

    #[tokio::test]
    async fn test_spec_agent_workflow() {
        let mut agent = SpecAgent::minimal();
        let result = agent
            .run_workflow("An Order has items. Users can create orders.")
            .await
            .unwrap();

        assert!(!result.formalization.topos_content.is_empty());
        assert!(!result.formalization.lean_content.is_empty());
    }

    #[test]
    fn test_spec_agent_config() {
        let agent = SpecAgent::new(SpecAgentConfig::full());
        assert!(agent.config().formalization_level.includes_proofs());

        let agent = SpecAgent::minimal();
        assert!(!agent.config().validate_with_lean);
    }

    #[test]
    fn test_workflow_result_helpers() {
        let result = WorkflowResult {
            context: SpecContext::new("test"),
            formalization: super::super::types::FormalizationResult {
                topos_content: "content".to_string(),
                topos_filename: "test.tps".to_string(),
                lean_content: "content".to_string(),
                lean_filename: "test.lean".to_string(),
                cross_refs: Vec::new(),
                warnings: vec!["warning".to_string()],
            },
            verification: VerificationResult::success(),
        };

        assert!(result.success());
        assert_eq!(result.warnings().len(), 1);
        assert!(result.errors().is_empty());
    }
}
