//! AI-assisted proof generation using LLMs.
//!
//! This module provides AI-powered tactic suggestion and proof generation
//! using language models. It serves as Tier 3 in the proof automation pipeline.

use std::sync::Arc;

use crate::error::{Error, Result};
use crate::lean::repl::LeanRepl;
use crate::lean::types::{Goal, TacticSuggestion};
use crate::llm::{ChatMessage, CompletionRequest, LLMClient};
use crate::proof::tactics::domain_specific_tactics;
use crate::proof::types::{ProofContext, SpecDomain, TacticResult};

/// Configuration for the AI proof assistant.
#[derive(Debug, Clone)]
pub struct AIAssistantConfig {
    /// Model to use for tactic suggestions.
    pub model: Option<String>,

    /// Maximum tokens for completion.
    pub max_tokens: u32,

    /// Temperature for generation (lower = more deterministic).
    pub temperature: f64,

    /// Maximum number of tactics to suggest per request.
    pub max_suggestions: usize,

    /// Whether to include explanations with suggestions.
    pub include_explanations: bool,

    /// Whether to validate suggested tactics.
    pub validate_suggestions: bool,

    /// Timeout for AI requests in milliseconds.
    pub timeout_ms: u64,
}

impl Default for AIAssistantConfig {
    fn default() -> Self {
        Self {
            model: None, // Use client default
            max_tokens: 1024,
            temperature: 0.3, // Low temperature for consistent suggestions
            max_suggestions: 5,
            include_explanations: true,
            validate_suggestions: true,
            timeout_ms: 30_000,
        }
    }
}

impl AIAssistantConfig {
    /// Create with a specific model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set temperature.
    pub fn with_temperature(mut self, temp: f64) -> Self {
        self.temperature = temp.clamp(0.0, 1.0);
        self
    }

    /// Set max suggestions.
    pub fn with_max_suggestions(mut self, max: usize) -> Self {
        self.max_suggestions = max;
        self
    }
}

/// AI-powered proof assistant for tactic suggestion.
pub struct AIProofAssistant {
    /// LLM client for generating suggestions.
    client: Arc<dyn LLMClient>,

    /// Configuration.
    config: AIAssistantConfig,
}

impl AIProofAssistant {
    /// Create a new AI proof assistant.
    pub fn new(client: Arc<dyn LLMClient>, config: AIAssistantConfig) -> Self {
        Self { client, config }
    }

    /// Create with default configuration.
    pub fn with_defaults(client: Arc<dyn LLMClient>) -> Self {
        Self::new(client, AIAssistantConfig::default())
    }

    /// Suggest tactics for a proof goal.
    pub async fn suggest_tactics(
        &self,
        goal: &Goal,
        context: &ProofContext,
    ) -> Result<Vec<TacticSuggestion>> {
        let prompt = self.build_prompt(goal, context);
        let system = self.build_system_prompt(context.domain);

        let request = CompletionRequest::new()
            .with_system(system)
            .with_message(ChatMessage::user(prompt))
            .with_max_tokens(self.config.max_tokens)
            .with_temperature(self.config.temperature);

        let request = if let Some(ref model) = self.config.model {
            request.with_model(model.clone())
        } else {
            request
        };

        let response = self.client.complete(request).await?;
        let suggestions = self.parse_suggestions(&response.content)?;

        Ok(suggestions)
    }

    /// Suggest tactics and validate them against the REPL.
    pub async fn suggest_and_validate(
        &self,
        repl: &mut LeanRepl,
        goal: &Goal,
        context: &ProofContext,
    ) -> Result<Vec<TacticSuggestion>> {
        let suggestions = self.suggest_tactics(goal, context).await?;

        if !self.config.validate_suggestions {
            return Ok(suggestions);
        }

        // Validate each suggestion
        let mut validated = Vec::new();
        for mut suggestion in suggestions {
            let result = self.validate_tactic(repl, goal, &suggestion.tactic).await;
            if result.is_ok() && result.as_ref().unwrap().success {
                // Boost confidence for validated tactics
                suggestion.confidence = (suggestion.confidence * 1.5).min(1.0);
                validated.push(suggestion);
            } else {
                // Keep but lower confidence for unvalidated
                suggestion.confidence *= 0.5;
                validated.push(suggestion);
            }
        }

        // Sort by confidence
        validated.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        Ok(validated)
    }

    /// Validate a tactic against the REPL.
    pub async fn validate_tactic(
        &self,
        repl: &mut LeanRepl,
        goal: &Goal,
        tactic: &str,
    ) -> Result<TacticResult> {
        let start = std::time::Instant::now();

        let proof_state = match repl.active_proof_state_id() {
            Some(id) => id,
            None => {
                let elapsed_ms = start.elapsed().as_millis() as u64;
                return Ok(TacticResult::failure(
                    tactic,
                    format!(
                        "Missing proof state for goal `{}`; initialize proof state before validating tactics",
                        goal.target
                    ),
                    elapsed_ms,
                ));
            }
        };

        let response = repl.apply_tactic(tactic, proof_state);
        let elapsed_ms = start.elapsed().as_millis() as u64;

        match response {
            Ok(resp) => {
                if resp.has_errors() {
                    let error = resp.format_errors();
                    Ok(TacticResult::failure(tactic, error, elapsed_ms))
                } else {
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

    /// Build the system prompt for tactic generation.
    fn build_system_prompt(&self, domain: SpecDomain) -> String {
        let domain_tactics = domain_specific_tactics(domain);
        let domain_tactics_str = domain_tactics.join(", ");

        format!(
            r#"You are an expert Lean 4 proof assistant. Your task is to suggest tactics that will help prove the given goal.

## Domain
The current goal appears to be in the {domain} domain. Relevant tactics for this domain include: {domain_tactics}

## Response Format
Respond with a JSON array of tactic suggestions. Each suggestion should have:
- "tactic": The exact Lean 4 tactic to apply
- "confidence": A number from 0.0 to 1.0 indicating your confidence
- "explanation": (optional) Brief explanation of why this might work

Example response:
```json
[
  {{"tactic": "simp", "confidence": 0.8, "explanation": "Simplification often resolves equality goals"}},
  {{"tactic": "ring", "confidence": 0.6, "explanation": "Ring solver for algebraic expressions"}}
]
```

## Guidelines
1. Suggest tactics in order of most likely to succeed
2. Start with simple tactics (simp, rfl, decide) before complex ones
3. Consider the hypotheses available in the context
4. For arithmetic goals, prefer omega, linarith, ring
5. For equality goals, try rfl, simp, congr
6. For logical goals, try decide, tauto, constructor
7. Limit suggestions to {max_suggestions} tactics
8. Use exact Lean 4 syntax"#,
            domain = domain,
            domain_tactics = domain_tactics_str,
            max_suggestions = self.config.max_suggestions
        )
    }

    /// Build the user prompt for a specific goal.
    fn build_prompt(&self, goal: &Goal, context: &ProofContext) -> String {
        let mut prompt = String::new();

        prompt.push_str("## Goal\n");
        prompt.push_str(&format!("Target: {}\n", goal.target));

        if !goal.hypotheses.is_empty() {
            prompt.push_str("\n## Hypotheses\n");
            for hyp in &goal.hypotheses {
                if let Some(ref value) = hyp.value {
                    prompt.push_str(&format!("{} : {} := {}\n", hyp.name, hyp.ty, value));
                } else {
                    prompt.push_str(&format!("{} : {}\n", hyp.name, hyp.ty));
                }
            }
        }

        if !context.history.is_empty() {
            prompt.push_str("\n## Previously Tried Tactics\n");
            for result in &context.history {
                let status = if result.success {
                    "succeeded"
                } else {
                    "failed"
                };
                prompt.push_str(&format!("- `{}` ({})\n", result.tactic, status));
            }
            prompt.push_str("\nAvoid suggesting tactics that have already failed.\n");
        }

        if !context.available_lemmas.is_empty() {
            prompt.push_str("\n## Available Lemmas\n");
            for lemma in &context.available_lemmas {
                prompt.push_str(&format!("- {}\n", lemma));
            }
        }

        prompt.push_str("\nSuggest tactics to prove this goal.");

        prompt
    }

    /// Parse tactic suggestions from LLM response.
    fn parse_suggestions(&self, content: &str) -> Result<Vec<TacticSuggestion>> {
        // Try to extract JSON from the response
        let json_content = if let Some(start) = content.find('[') {
            if let Some(end) = content.rfind(']') {
                &content[start..=end]
            } else {
                content
            }
        } else {
            content
        };

        // Parse the JSON
        let parsed: std::result::Result<Vec<SuggestionJson>, _> =
            serde_json::from_str(json_content);

        match parsed {
            Ok(suggestions) => {
                let tactics: Vec<TacticSuggestion> = suggestions
                    .into_iter()
                    .take(self.config.max_suggestions)
                    .map(|s| TacticSuggestion {
                        tactic: s.tactic,
                        confidence: s.confidence.clamp(0.0, 1.0),
                        explanation: if self.config.include_explanations {
                            s.explanation
                        } else {
                            None
                        },
                    })
                    .collect();
                Ok(tactics)
            }
            Err(_) => {
                // Fall back to simple parsing if JSON fails
                self.parse_suggestions_fallback(content)
            }
        }
    }

    /// Fallback parser for when JSON parsing fails.
    fn parse_suggestions_fallback(&self, content: &str) -> Result<Vec<TacticSuggestion>> {
        let mut suggestions = Vec::new();

        // Look for backtick-quoted tactics
        for line in content.lines() {
            if let Some(start) = line.find('`') {
                if let Some(end) = line[start + 1..].find('`') {
                    let tactic = &line[start + 1..start + 1 + end];
                    if !tactic.is_empty() && !tactic.contains('\n') {
                        suggestions.push(TacticSuggestion {
                            tactic: tactic.to_string(),
                            confidence: 0.5, // Default confidence
                            explanation: None,
                        });
                    }
                }
            }
        }

        // If no backtick tactics found, try line-based parsing
        if suggestions.is_empty() {
            for line in content.lines() {
                let line = line.trim();
                // Skip empty lines and comments
                if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                    continue;
                }
                // Check if it looks like a tactic
                if line.starts_with("simp")
                    || line.starts_with("rfl")
                    || line.starts_with("ring")
                    || line.starts_with("omega")
                    || line.starts_with("decide")
                    || line.starts_with("intro")
                    || line.starts_with("apply")
                    || line.starts_with("exact")
                    || line.starts_with("constructor")
                    || line.starts_with("cases")
                    || line.starts_with("induction")
                {
                    // Extract tactic (remove trailing punctuation)
                    let tactic = line.trim_end_matches(|c| c == '.' || c == ',');
                    suggestions.push(TacticSuggestion {
                        tactic: tactic.to_string(),
                        confidence: 0.4,
                        explanation: None,
                    });
                }
            }
        }

        if suggestions.is_empty() {
            Err(Error::LLM("Failed to parse tactic suggestions".to_string()))
        } else {
            Ok(suggestions)
        }
    }

    /// Generate a complete proof for a goal (sequence of tactics).
    pub async fn generate_proof(&self, goal: &Goal, context: &ProofContext) -> Result<Vec<String>> {
        let prompt = self.build_proof_prompt(goal, context);
        let system = self.build_proof_system_prompt();

        let request = CompletionRequest::new()
            .with_system(system)
            .with_message(ChatMessage::user(prompt))
            .with_max_tokens(self.config.max_tokens * 2) // More tokens for full proof
            .with_temperature(self.config.temperature);

        let request = if let Some(ref model) = self.config.model {
            request.with_model(model.clone())
        } else {
            request
        };

        let response = self.client.complete(request).await?;
        self.parse_proof(&response.content)
    }

    /// Build system prompt for full proof generation.
    fn build_proof_system_prompt(&self) -> String {
        r#"You are an expert Lean 4 proof assistant. Generate a complete proof (sequence of tactics) for the given goal.

## Response Format
Respond with a JSON array of tactics in the order they should be applied:
```json
["intro x", "simp", "ring"]
```

## Guidelines
1. Generate a complete proof that will close all goals
2. Use appropriate tactics for the goal type
3. Handle all cases and subcases
4. Prefer shorter, more elegant proofs
5. Use exact Lean 4 syntax"#
            .to_string()
    }

    /// Build prompt for full proof generation.
    fn build_proof_prompt(&self, goal: &Goal, context: &ProofContext) -> String {
        let mut prompt = String::new();

        prompt.push_str("Generate a complete proof for this goal.\n\n");
        prompt.push_str("## Goal\n");
        prompt.push_str(&format!("Target: {}\n", goal.target));

        if !goal.hypotheses.is_empty() {
            prompt.push_str("\n## Hypotheses\n");
            for hyp in &goal.hypotheses {
                if let Some(ref value) = hyp.value {
                    prompt.push_str(&format!("{} : {} := {}\n", hyp.name, hyp.ty, value));
                } else {
                    prompt.push_str(&format!("{} : {}\n", hyp.name, hyp.ty));
                }
            }
        }

        if !context.history.is_empty() {
            prompt.push_str("\n## Previously Tried (failed)\n");
            for result in &context.history {
                if !result.success {
                    prompt.push_str(&format!("- `{}`\n", result.tactic));
                }
            }
        }

        prompt
    }

    /// Parse a complete proof from LLM response.
    fn parse_proof(&self, content: &str) -> Result<Vec<String>> {
        // Try to extract JSON array
        let json_content = if let Some(start) = content.find('[') {
            if let Some(end) = content.rfind(']') {
                &content[start..=end]
            } else {
                content
            }
        } else {
            content
        };

        let parsed: std::result::Result<Vec<String>, _> = serde_json::from_str(json_content);

        match parsed {
            Ok(tactics) => Ok(tactics),
            Err(_) => {
                // Fall back to line-based parsing
                let tactics: Vec<String> = content
                    .lines()
                    .filter_map(|line| {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                            None
                        } else if let Some(start) = line.find('`') {
                            line[start + 1..]
                                .find('`')
                                .map(|end| line[start + 1..start + 1 + end].to_string())
                        } else {
                            None
                        }
                    })
                    .collect();

                if tactics.is_empty() {
                    Err(Error::LLM("Failed to parse proof tactics".to_string()))
                } else {
                    Ok(tactics)
                }
            }
        }
    }
}

/// JSON structure for parsing suggestions.
#[derive(Debug, serde::Deserialize)]
struct SuggestionJson {
    tactic: String,
    confidence: f64,
    #[serde(default)]
    explanation: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{
        CompletionResponse, EmbeddingRequest, EmbeddingResponse, ModelSpec, Provider, StopReason,
        TokenUsage,
    };
    use async_trait::async_trait;
    use chrono::Utc;

    /// Mock LLM client for testing.
    struct MockLLMClient {
        response: String,
    }

    impl MockLLMClient {
        fn new(response: impl Into<String>) -> Self {
            Self {
                response: response.into(),
            }
        }
    }

    #[async_trait]
    impl LLMClient for MockLLMClient {
        async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
            Ok(CompletionResponse {
                id: "test".to_string(),
                model: "test-model".to_string(),
                content: self.response.clone(),
                stop_reason: Some(StopReason::EndTurn),
                usage: TokenUsage::default(),
                timestamp: Utc::now(),
                cost: Some(0.0),
            })
        }

        async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse> {
            Err(Error::LLM("Not implemented".to_string()))
        }

        fn provider(&self) -> Provider {
            Provider::Anthropic
        }

        fn available_models(&self) -> Vec<ModelSpec> {
            vec![]
        }
    }

    #[test]
    fn test_config_default() {
        let config = AIAssistantConfig::default();
        assert_eq!(config.max_tokens, 1024);
        assert_eq!(config.temperature, 0.3);
        assert_eq!(config.max_suggestions, 5);
    }

    #[test]
    fn test_config_builder() {
        let config = AIAssistantConfig::default()
            .with_model("claude-3-5-sonnet")
            .with_temperature(0.5)
            .with_max_suggestions(10);

        assert_eq!(config.model, Some("claude-3-5-sonnet".to_string()));
        assert_eq!(config.temperature, 0.5);
        assert_eq!(config.max_suggestions, 10);
    }

    #[tokio::test]
    async fn test_suggest_tactics_json() {
        let response = r#"[
            {"tactic": "simp", "confidence": 0.8, "explanation": "Simplify"},
            {"tactic": "ring", "confidence": 0.6, "explanation": "Ring solver"}
        ]"#;

        let client = Arc::new(MockLLMClient::new(response));
        let assistant = AIProofAssistant::with_defaults(client);

        let goal = Goal::from_string("x + 0 = x");
        let context = ProofContext::new(goal.clone());

        let suggestions = assistant.suggest_tactics(&goal, &context).await.unwrap();

        assert_eq!(suggestions.len(), 2);
        assert_eq!(suggestions[0].tactic, "simp");
        assert!((suggestions[0].confidence - 0.8).abs() < 0.01);
        assert_eq!(suggestions[1].tactic, "ring");
    }

    #[tokio::test]
    async fn test_suggest_tactics_fallback() {
        let response = "Try these tactics:\n- `simp` for simplification\n- `ring` for algebra";

        let client = Arc::new(MockLLMClient::new(response));
        let assistant = AIProofAssistant::with_defaults(client);

        let goal = Goal::from_string("x * 1 = x");
        let context = ProofContext::new(goal.clone());

        let suggestions = assistant.suggest_tactics(&goal, &context).await.unwrap();

        assert!(suggestions.len() >= 2);
        assert!(suggestions.iter().any(|s| s.tactic == "simp"));
        assert!(suggestions.iter().any(|s| s.tactic == "ring"));
    }

    #[test]
    fn test_build_prompt() {
        let client = Arc::new(MockLLMClient::new(""));
        let assistant = AIProofAssistant::with_defaults(client);

        let goal = Goal::from_string("x + y = y + x")
            .with_hypothesis("x", "Nat")
            .with_hypothesis("y", "Nat");

        let context = ProofContext::new(goal.clone());
        let prompt = assistant.build_prompt(&goal, &context);

        assert!(prompt.contains("x + y = y + x"));
        assert!(prompt.contains("x : Nat"));
        assert!(prompt.contains("y : Nat"));
    }

    #[test]
    fn test_build_system_prompt() {
        let client = Arc::new(MockLLMClient::new(""));
        let assistant = AIProofAssistant::with_defaults(client);

        let system = assistant.build_system_prompt(SpecDomain::Arithmetic);

        assert!(system.contains("arithmetic"));
        assert!(system.contains("omega"));
        assert!(system.contains("JSON"));
    }

    #[tokio::test]
    async fn test_generate_proof() {
        let response = r#"["intro x", "intro y", "ring"]"#;

        let client = Arc::new(MockLLMClient::new(response));
        let assistant = AIProofAssistant::with_defaults(client);

        let goal = Goal::from_string("forall x y : Nat, x + y = y + x");
        let context = ProofContext::new(goal.clone());

        let proof = assistant.generate_proof(&goal, &context).await.unwrap();

        assert_eq!(proof, vec!["intro x", "intro y", "ring"]);
    }

    #[test]
    fn test_parse_suggestions_json() {
        let client = Arc::new(MockLLMClient::new(""));
        let assistant = AIProofAssistant::with_defaults(client);

        let json = r#"[{"tactic": "simp", "confidence": 0.9}]"#;
        let suggestions = assistant.parse_suggestions(json).unwrap();

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].tactic, "simp");
    }

    #[test]
    fn test_parse_suggestions_with_preamble() {
        let client = Arc::new(MockLLMClient::new(""));
        let assistant = AIProofAssistant::with_defaults(client);

        let content = r#"Here are my suggestions:

```json
[{"tactic": "omega", "confidence": 0.8}]
```"#;

        let suggestions = assistant.parse_suggestions(content).unwrap();
        assert_eq!(suggestions[0].tactic, "omega");
    }
}
