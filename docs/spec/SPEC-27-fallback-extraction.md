# SPEC-27: Fallback Extraction on Max Iterations

> Graceful output extraction when execution limits reached

**Status**: Implemented in `rlm-core` runtime primitives (`orchestrator::FallbackLoop` + `signature::fallback`), with adapter-level adoption tracked separately in milestone sequencing
**Created**: 2026-01-20
**Epic**: loop-zcx (DSPy-Inspired RLM Improvements)
**Task**: loop-tua
**Depends On**: SPEC-20 (Typed Signatures)

---

## Overview

Implement DSPy-style fallback extraction that forces output extraction when max_iterations is reached without a SUBMIT call. This ensures the orchestrator always returns structured output even when the REPL execution doesn't cleanly terminate.

## Implementation Snapshot (2026-02-19)

| Section | Status | Runtime Evidence |
|---|---|---|
| SPEC-27.01 Fallback trigger checks | Implemented | `FallbackExtractor::should_trigger` in `rlm-core/src/signature/fallback.rs` |
| SPEC-27.02 Extraction context capture | Implemented (shape differs from draft structs) | `ReplHistory`, variable capture, and prompt builders in `rlm-core/src/signature/fallback.rs` |
| SPEC-27.03 Extraction prompt | Implemented | `FallbackExtractor::extraction_prompt` |
| SPEC-27.04 Execution result model | Implemented | `ExecutionResult` and confidence helpers in `rlm-core/src/signature/fallback.rs` |
| Orchestrator loop wiring | Implemented (runtime helper path) | `FallbackLoop` + fallback trigger tests in `rlm-core/src/orchestrator.rs` |

## Requirements

### SPEC-27.01: Fallback Trigger

Conditions that trigger fallback extraction.

```rust
/// Trigger conditions for fallback extraction
#[derive(Debug, Clone)]
pub struct FallbackTrigger {
    /// Maximum REPL iterations
    pub max_iterations: u32,
    /// Maximum LLM calls within REPL
    pub max_llm_calls: u32,
    /// Maximum execution time
    pub max_duration: Duration,
}

impl Default for FallbackTrigger {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            max_llm_calls: 50,
            max_duration: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl FallbackTrigger {
    /// Check if fallback should be triggered
    pub fn should_trigger(&self, state: &ExecutionState) -> Option<FallbackReason> {
        if state.submitted {
            return None;  // SUBMIT was called, no fallback needed
        }

        if state.iterations >= self.max_iterations {
            return Some(FallbackReason::MaxIterations {
                iterations: state.iterations,
                limit: self.max_iterations,
            });
        }

        if state.llm_calls >= self.max_llm_calls {
            return Some(FallbackReason::MaxLLMCalls {
                calls: state.llm_calls,
                limit: self.max_llm_calls,
            });
        }

        if state.elapsed >= self.max_duration {
            return Some(FallbackReason::Timeout {
                elapsed: state.elapsed,
                limit: self.max_duration,
            });
        }

        None
    }
}

#[derive(Debug, Clone)]
pub enum FallbackReason {
    MaxIterations { iterations: u32, limit: u32 },
    MaxLLMCalls { calls: u32, limit: u32 },
    Timeout { elapsed: Duration, limit: Duration },
}
```

**Acceptance Criteria**:
- [x] All trigger conditions checked
- [x] SUBMIT bypasses fallback
- [x] Reason captured for logging

### SPEC-27.02: Extract Signature

Auto-generated signature for extraction.

```rust
/// Inputs for fallback extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractFallbackInputs {
    /// Full REPL history
    pub history: Vec<REPLHistoryEntry>,
    /// All variable values (serialized)
    pub variables: HashMap<String, serde_json::Value>,
    /// Output field specifications
    pub output_fields: Vec<FieldSpec>,
    /// Reason for fallback
    pub fallback_reason: String,
}

/// Entry in REPL history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct REPLHistoryEntry {
    /// Iteration number
    pub iteration: u32,
    /// Reasoning/thinking before code
    pub reasoning: Option<String>,
    /// Code executed
    pub code: String,
    /// Execution output
    pub output: String,
    /// Whether execution succeeded
    pub success: bool,
}

impl<S: Signature> Predict<S> {
    /// Generate extraction inputs for fallback
    fn extraction_inputs(
        &self,
        history: &[REPLHistoryEntry],
        variables: &HashMap<String, serde_json::Value>,
        reason: FallbackReason,
    ) -> ExtractFallbackInputs {
        ExtractFallbackInputs {
            history: history.to_vec(),
            variables: variables.clone(),
            output_fields: S::output_fields(),
            fallback_reason: reason.to_string(),
        }
    }
}
```

**Acceptance Criteria**:
- [x] History captured for extraction prompt
- [x] Variables serialized for prompt context
- [x] Output fields derived from signature metadata

### SPEC-27.03: Extraction Prompt

Template for extraction LLM call.

```rust
const EXTRACTION_PROMPT_TEMPLATE: &str = r#"
The REPL execution reached its limit ({fallback_reason}).
Extract the final outputs from the execution history and variables.

## REPL History

{history}

## Current Variables

{variables}

## Required Outputs

You must extract values for these fields:
{output_fields}

## Instructions

1. Review the REPL history to understand what was computed
2. Look at the current variable values
3. Extract or infer the required output values
4. Return ONLY valid JSON matching the output schema

If a value cannot be determined, use null and explain in _extraction_notes.

## Output Schema

```json
{{
{output_schema}
  "_extraction_notes": "string (optional explanation)"
}}
```

Return the JSON now:
"#;

impl<S: Signature> Predict<S> {
    fn generate_extraction_prompt(&self, inputs: &ExtractFallbackInputs) -> String {
        EXTRACTION_PROMPT_TEMPLATE
            .replace("{fallback_reason}", &inputs.fallback_reason)
            .replace("{history}", &self.format_history(&inputs.history))
            .replace("{variables}", &self.format_variables(&inputs.variables))
            .replace("{output_fields}", &self.format_output_fields(&inputs.output_fields))
            .replace("{output_schema}", &self.generate_schema::<S::Outputs>())
    }

    fn format_history(&self, history: &[REPLHistoryEntry]) -> String {
        history
            .iter()
            .map(|e| format!(
                "### Iteration {}\n{}\n```python\n{}\n```\nOutput: {}\n",
                e.iteration,
                e.reasoning.as_deref().unwrap_or(""),
                e.code,
                e.output
            ))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
```

**Acceptance Criteria**:
- [x] Prompt includes history + variable context
- [x] Schema generated from signature field metadata
- [x] Notes/confidence extraction fields are supported

### SPEC-27.04: Fallback Result

Result type distinguishing clean vs extracted outputs.

```rust
/// Result of module execution
#[derive(Debug, Clone)]
pub enum ExecutionResult<S: Signature> {
    /// Clean termination via SUBMIT
    Submitted {
        outputs: S::Outputs,
        iterations: u32,
        llm_calls: u32,
    },

    /// Extracted via fallback
    Extracted {
        outputs: S::Outputs,
        confidence: f64,
        reason: FallbackReason,
        extraction_notes: Option<String>,
    },

    /// Failed to extract
    Failed {
        reason: String,
        partial_outputs: Option<serde_json::Value>,
        history: Vec<REPLHistoryEntry>,
    },
}

impl<S: Signature> ExecutionResult<S> {
    /// Get outputs regardless of how obtained
    pub fn outputs(&self) -> Option<&S::Outputs> {
        match self {
            Self::Submitted { outputs, .. } => Some(outputs),
            Self::Extracted { outputs, .. } => Some(outputs),
            Self::Failed { .. } => None,
        }
    }

    /// Check if outputs were extracted (vs submitted)
    pub fn was_extracted(&self) -> bool {
        matches!(self, Self::Extracted { .. })
    }

    /// Get confidence (1.0 for submitted, calculated for extracted)
    pub fn confidence(&self) -> f64 {
        match self {
            Self::Submitted { .. } => 1.0,
            Self::Extracted { confidence, .. } => *confidence,
            Self::Failed { .. } => 0.0,
        }
    }
}
```

**Confidence Calculation**:
```rust
impl<S: Signature> Predict<S> {
    fn calculate_extraction_confidence(
        &self,
        extracted: &S::Outputs,
        history: &[REPLHistoryEntry],
        variables: &HashMap<String, serde_json::Value>,
    ) -> f64 {
        let mut confidence = 0.5;  // Base confidence

        // Boost if outputs appear in variables
        if self.outputs_in_variables(extracted, variables) {
            confidence += 0.3;
        }

        // Boost if recent history mentions output values
        if self.outputs_in_history(extracted, history) {
            confidence += 0.2;
        }

        // Reduce if null fields present
        let null_ratio = self.null_field_ratio(extracted);
        confidence -= null_ratio * 0.3;

        confidence.clamp(0.1, 0.99)
    }
}
```

**Acceptance Criteria**:
- [x] Three result variants
- [x] Confidence calculated
- [x] Partial outputs preserved on failure

---

## Integration

### With Orchestrator

Status: Implemented as reusable runtime wiring in `orchestrator::FallbackLoop`. The pseudocode below remains target-shape architecture guidance.

```rust
impl Orchestrator {
    async fn run_with_fallback<S: Signature>(
        &self,
        module: &Predict<S>,
        inputs: S::Inputs,
    ) -> ExecutionResult<S> {
        let trigger = FallbackTrigger::default();
        let mut state = ExecutionState::new();

        loop {
            // Check for fallback trigger
            if let Some(reason) = trigger.should_trigger(&state) {
                return self.extract_fallback(module, &state, reason).await;
            }

            // Execute one iteration
            match self.execute_iteration(module, &mut state).await {
                IterationResult::Continue => continue,
                IterationResult::Submitted(outputs) => {
                    return ExecutionResult::Submitted {
                        outputs,
                        iterations: state.iterations,
                        llm_calls: state.llm_calls,
                    };
                }
                IterationResult::Error(e) => {
                    state.record_error(e);
                }
            }
        }
    }

    async fn extract_fallback<S: Signature>(
        &self,
        module: &Predict<S>,
        state: &ExecutionState,
        reason: FallbackReason,
    ) -> ExecutionResult<S> {
        let inputs = module.extraction_inputs(&state.history, &state.variables, reason.clone());
        let prompt = module.generate_extraction_prompt(&inputs);

        match self.llm.complete(&prompt).await {
            Ok(response) => {
                match serde_json::from_str::<S::Outputs>(&response) {
                    Ok(outputs) => {
                        let confidence = module.calculate_extraction_confidence(
                            &outputs,
                            &state.history,
                            &state.variables,
                        );
                        ExecutionResult::Extracted {
                            outputs,
                            confidence,
                            reason,
                            extraction_notes: extract_notes(&response),
                        }
                    }
                    Err(e) => ExecutionResult::Failed {
                        reason: format!("Failed to parse extraction: {}", e),
                        partial_outputs: serde_json::from_str(&response).ok(),
                        history: state.history.clone(),
                    },
                }
            }
            Err(e) => ExecutionResult::Failed {
                reason: format!("Extraction LLM call failed: {}", e),
                partial_outputs: None,
                history: state.history.clone(),
            },
        }
    }
}
```

---

## Test Plan

| Test | Description | Spec |
|------|-------------|------|
| `signature::fallback::tests::test_should_trigger` | Trigger behavior for limits/submitted state | SPEC-27.01 |
| `signature::fallback::tests::test_extraction_prompt` | Prompt generation includes required context | SPEC-27.03 |
| `signature::fallback::tests::test_parse_extraction_response` | Successful JSON extraction path | SPEC-27.04 |
| `signature::fallback::tests::test_parse_extraction_response_markdown` | Markdown-wrapped JSON extraction path | SPEC-27.04 |
| `signature::fallback::tests::test_parse_extraction_response_failure` | Extraction failure handling | SPEC-27.04 |
| `signature::fallback::tests::test_execution_result_submitted` | Submitted result semantics | SPEC-27.04 |
| `signature::fallback::tests::test_execution_result_extracted` | Extracted result semantics | SPEC-27.04 |
| `signature::fallback::tests::test_execution_result_failed` | Failed result semantics | SPEC-27.04 |
| `orchestrator::tests::fallback::*` | Orchestrator runtime-loop trigger + submit-bypass wiring | Integration section |

---

## References

- [DSPy RLM extract_fallback](https://github.com/stanfordnlp/dspy/blob/main/dspy/predict/rlm.py)
- SPEC-20: Typed Signatures (prerequisite)
- Existing fallback runtime: `rlm-core/src/signature/fallback.rs`
