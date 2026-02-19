# SPEC-20: Typed Signatures System

> DSPy-inspired typed signatures for rlm-core

**Status**: In progress (runtime protocol implemented through M2-T04)
**Created**: 2026-01-20
**Epic**: loop-zcx (DSPy-Inspired RLM Improvements)
**Tasks**: loop-d75, loop-jqo, loop-9l6, loop-bzz

---

## Overview

Implement DSPy-style typed signatures that enable composable modules, automatic output validation, and optimization. This is the foundational system upon which module composition and BootstrapFewShot optimization depend.

## Requirements

### SPEC-20.01: Signature Trait

The core trait defining typed LLM I/O contracts.

```rust
pub trait Signature: Send + Sync + 'static {
    /// Input type (must be serializable)
    type Inputs: Serialize + DeserializeOwned + Clone + Send + Sync;
    /// Output type (must be serializable)
    type Outputs: Serialize + DeserializeOwned + Clone + Send + Sync;

    /// Task instructions for the LLM
    fn instructions() -> &'static str;

    /// Input field specifications
    fn input_fields() -> Vec<FieldSpec>;

    /// Output field specifications
    fn output_fields() -> Vec<FieldSpec>;

    /// Generate prompt from inputs
    fn to_prompt(inputs: &Self::Inputs) -> String {
        // Default implementation using field specs
    }

    /// Parse outputs from LLM response
    fn from_response(response: &str) -> Result<Self::Outputs, ParseError>;
}
```

**Acceptance Criteria**:
- [ ] Trait compiles and is object-safe where possible
- [ ] Default `to_prompt` generates structured prompt
- [ ] Default `from_response` parses JSON or structured text

### SPEC-20.02: Field Specification

Metadata for input and output fields.

```rust
pub struct FieldSpec {
    /// Field name (matches struct field)
    pub name: String,
    /// Field type for validation
    pub field_type: FieldType,
    /// Human-readable description (for prompt generation)
    pub description: String,
    /// Optional display prefix/label
    pub prefix: Option<String>,
    /// Whether field is required
    pub required: bool,
    /// Default value (JSON) if not required
    pub default: Option<serde_json::Value>,
}

pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
    List(Box<FieldType>),
    Object(Vec<FieldSpec>),
    Enum(Vec<String>),  // Allowed values
    Custom(String),     // Custom type name
}
```

**Acceptance Criteria**:
- [ ] FieldType covers common Rust types
- [ ] Nested types (List, Object) work recursively
- [ ] Enum validates against allowed values

### SPEC-20.03: Signature Validation

Runtime validation of inputs and outputs.

```rust
pub trait SignatureValidator {
    /// Validate inputs before execution
    fn validate_inputs<S: Signature>(inputs: &S::Inputs) -> Result<(), ValidationError>;

    /// Validate outputs before returning
    fn validate_outputs<S: Signature>(outputs: &S::Outputs) -> Result<(), ValidationError>;
}

pub enum ValidationError {
    MissingField { field: String },
    TypeMismatch { field: String, expected: FieldType, got: String },
    EnumInvalid { field: String, value: String, allowed: Vec<String> },
    ConstraintViolated { field: String, constraint: String },
    Custom(String),
}
```

**Acceptance Criteria**:
- [ ] Inputs validated before execution
- [ ] Outputs validated before returning to caller
- [ ] Clear error messages with field context

### SPEC-20.04: Derive Macro Attributes

Proc macro for automatic Signature implementation.

```rust
#[derive(Signature)]
#[signature(instructions = "Analyze code for security vulnerabilities")]
pub struct AnalyzeCode {
    // Inputs
    #[input(desc = "Source code to analyze")]
    code: String,

    #[input(desc = "Programming language", prefix = "Language")]
    language: String,

    // Outputs
    #[output(desc = "List of vulnerabilities found")]
    vulnerabilities: Vec<String>,

    #[output(desc = "Overall severity", prefix = "Severity")]
    severity: Severity,
}
```

**Attributes**:
- `#[derive(Signature)]` - Generate Signature impl
- `#[signature(instructions = "...")]` - Set instructions text
- `#[input(desc = "...", prefix = "...")]` - Mark as input field
- `#[output(desc = "...", prefix = "...")]` - Mark as output field
- `#[field(required = false, default = "...")]` - Optional field config

**Acceptance Criteria**:
- [ ] Macro generates correct Signature impl
- [ ] All attributes parsed correctly
- [ ] Compile-time errors for invalid usage

### SPEC-20.05: Type Inference

Automatic FieldType inference from Rust types.

| Rust Type | FieldType |
|-----------|-----------|
| `String`, `&str` | `FieldType::String` |
| `i8`..`i128`, `u8`..`u128`, `isize`, `usize` | `FieldType::Integer` |
| `f32`, `f64` | `FieldType::Float` |
| `bool` | `FieldType::Boolean` |
| `Vec<T>` | `FieldType::List(T)` |
| `Option<T>` | Same as T, but `required = false` |
| Enum with `#[derive(Signature)]` | `FieldType::Enum(variants)` |
| Other | `FieldType::Custom(type_name)` |

**Acceptance Criteria**:
- [ ] All primitive types inferred correctly
- [ ] Generic types (Vec, Option) handled
- [ ] Custom types fall back to Custom variant

### SPEC-20.06: Compile-Time Validation

Errors at compile time for invalid signatures.

| Condition | Error Message |
|-----------|---------------|
| No `#[input]` fields | "Signature must have at least one input field" |
| No `#[output]` fields | "Signature must have at least one output field" |
| Field without attribute | "Field 'X' must be marked with #[input] or #[output]" |
| Invalid attribute syntax | "Invalid attribute: expected #[input(desc = \"...\")]" |

**Acceptance Criteria**:
- [ ] All invalid usages produce compile errors
- [ ] Error messages are helpful and actionable

### SPEC-20.07: SUBMIT Function

Python REPL function for structured output termination.

```python
# In REPL sandbox (rlm_repl.sandbox.Sandbox._submit)
def SUBMIT(outputs: dict) -> NoReturn:
    """
    Terminate execution and return validated outputs.

    Args:
        outputs: Dictionary matching signature output fields

    Behavior:
        - Validates against active signature registration
        - Stores structured submit_result payload
        - Terminates current execution via internal control-flow signal
    """
```

**Behavior**:
1. SUBMIT() immediately terminates current execution
2. Validates all required output fields present
3. Validates field types match signature
4. Includes `submit_result` in execute response to Rust orchestrator

**Acceptance Criteria**:
- [ ] SUBMIT() terminates execution immediately
- [ ] Validation against registered signature
- [ ] Clear errors for missing/invalid fields

### SPEC-20.08: SUBMIT Behavior

Detailed SUBMIT semantics.

```rust
pub enum SubmitResult {
    /// Successful submission with validated outputs
    Success {
        outputs: serde_json::Value,
        metrics: Option<SubmitMetrics>,
    },
    /// Validation failed
    ValidationError {
        errors: Vec<SubmitError>,
        original_outputs: Option<serde_json::Value>,
    },
    /// Execution completed without SUBMIT
    NotSubmitted {
        reason: String,
    },
}
```

**Rules**:
- SUBMIT() MUST terminate current execution immediately
- SUBMIT() MUST validate all required output fields present
- SUBMIT() MUST validate field types match signature
- SUBMIT() MUST return SubmitResult to Rust side
- Multiple SUBMIT() calls: only first is processed

**Acceptance Criteria**:
- [ ] Immediate termination on SUBMIT()
- [ ] All validation rules enforced
- [ ] Multiple calls handled gracefully

### SPEC-20.09: Validation Errors

Error types for SUBMIT validation.

```rust
pub enum SubmitError {
    MissingField {
        field: String,
        expected_type: FieldType,
    },
    TypeMismatch {
        field: String,
        expected: FieldType,
        got: String,
        value_preview: String,  // First 100 chars
    },
    EnumInvalid {
        field: String,
        value: String,
        allowed: Vec<String>,
    },
    ValidationFailed {
        field: String,
        reason: String,
    },
}

impl SubmitError {
    pub fn to_user_message(&self) -> String {
        // Human-readable error message
    }
}
```

**Acceptance Criteria**:
- [ ] All error variants have clear messages
- [ ] Value preview helps debugging
- [ ] Errors are actionable

### SPEC-20.10: REPL Protocol Extension

JSON-RPC protocol for signature registration and execute-response submit payload.

```json
// Register signature before execution
{
    "jsonrpc": "2.0",
    "method": "register_signature",
    "params": {
        "output_fields": [
            {"name": "vulnerabilities", "type": "list", "required": true},
            {"name": "severity", "type": "enum", "values": ["low", "medium", "high", "critical"]}
        ]
    },
    "id": 1
}

// Optional signature cleanup
{
    "jsonrpc": "2.0",
    "method": "clear_signature",
    "params": null,
    "id": 2
}

// Execute response carrying SUBMIT result
{
    "jsonrpc": "2.0",
    "result": {
        "success": true,
        "result": null,
        "stdout": "",
        "stderr": "",
        "error": null,
        "error_type": null,
        "execution_time_ms": 12.3,
        "pending_operations": [],
        "submit_result": {
            "status": "success",
            "outputs": {
                "vulnerabilities": ["SQL injection"],
                "severity": "high"
            }
        }
    },
    "id": 3
}
```

**Acceptance Criteria**:
- [ ] `register_signature` method implemented
- [ ] `clear_signature` method implemented
- [ ] Execute responses include optional `submit_result` payload for SUBMIT outcomes

### SPEC-20.11: Module Trait

Composable module abstraction.

```rust
pub trait Module: Send + Sync {
    type Signature: Signature;

    /// Execute the module with inputs
    async fn forward(
        &self,
        inputs: <Self::Signature as Signature>::Inputs
    ) -> Result<<Self::Signature as Signature>::Outputs, ModuleError>;

    /// Get all predictors in this module (for optimization)
    fn predictors(&self) -> Vec<&dyn Predictor>;

    /// Set LLM client for all predictors
    fn set_lm(&mut self, lm: Arc<dyn LLMClient>);

    /// Get current LLM client
    fn get_lm(&self) -> Option<Arc<dyn LLMClient>>;
}
```

**Acceptance Criteria**:
- [ ] Module trait is object-safe where needed
- [ ] Predictors discoverable for optimization
- [ ] LM propagation works

### SPEC-20.12: Predict Wrapper

Basic predictor that executes a signature.

```rust
pub struct Predict<S: Signature> {
    signature: PhantomData<S>,
    lm: Option<Arc<dyn LLMClient>>,
    demonstrations: Vec<Demonstration<S>>,
    config: PredictConfig,
}

pub struct PredictConfig {
    pub temperature: f64,
    pub max_tokens: Option<u32>,
    pub stop_sequences: Vec<String>,
}

pub struct Demonstration<S: Signature> {
    pub inputs: S::Inputs,
    pub outputs: S::Outputs,
}

impl<S: Signature> Module for Predict<S> {
    type Signature = S;

    async fn forward(&self, inputs: S::Inputs) -> Result<S::Outputs, ModuleError> {
        // 1. Generate prompt from signature + inputs + demonstrations
        // 2. Call LLM
        // 3. Parse response into S::Outputs
        // 4. Validate outputs
    }
}
```

**Acceptance Criteria**:
- [ ] Predict generates correct prompts
- [ ] Demonstrations injected as few-shot examples
- [ ] Config options respected

### SPEC-20.13: Composition Validation

Compile-time and runtime composition checks.

```rust
/// Compose two modules where output of A matches input of B
pub fn compose<A, B>(a: A, b: B) -> Composed<A, B>
where
    A: Module,
    B: Module,
    // Compile-time: A's output type must match B's input type
    <A::Signature as Signature>::Outputs: Into<<B::Signature as Signature>::Inputs>,
{
    Composed { a, b }
}
```

**Runtime Validation**:
- Verify output field names match input field names
- Verify output types are compatible with input types
- Propagate LM to all sub-modules

**Acceptance Criteria**:
- [ ] Type-safe composition at compile time
- [ ] Runtime validation for dynamic cases
- [ ] LM propagation through composition

---

## File Locations

| Component | Location |
|-----------|----------|
| Signature trait | `rlm-core/src/signature/mod.rs` |
| FieldSpec, FieldType | `rlm-core/src/signature/types.rs` |
| Validation | `rlm-core/src/signature/validation.rs` |
| SUBMIT result/error types | `rlm-core/src/signature/submit.rs` |
| Derive macro | `rlm-core-derive/src/lib.rs` |
| Module trait | `rlm-core/src/module/mod.rs` |
| Predict wrapper | `rlm-core/src/module/predict.rs` |
| REPL SUBMIT runtime | `rlm-core/python/rlm_repl/sandbox.py` |
| REPL protocol handlers | `rlm-core/python/rlm_repl/main.py` |
| REPL protocol schema | `rlm-core/python/rlm_repl/protocol.py` |
| Rust REPL client bridge | `rlm-core/src/repl.rs` |

---

## Test Plan

| Test | Description | Spec |
|------|-------------|------|
| `signature::tests::derive_tests::*` | Derive macro and signature behavior | SPEC-20.04, SPEC-20.05 |
| `signature::validation::tests::*` | Validation behavior and error paths | SPEC-20.03, SPEC-20.09 |
| `signature::submit::tests::*` | SubmitResult/SubmitError serialization and semantics | SPEC-20.08, SPEC-20.09 |
| `tests/test_repl.py::TestReplServer::test_submit_*` | Python SUBMIT scenarios (success + validation failures) | SPEC-20.07, SPEC-20.09, SPEC-20.10 |
| `repl::tests::test_submit_result_roundtrip_*` (ignored) | Rust/Python end-to-end submit_result roundtrip scenarios | SPEC-20.08, SPEC-20.10 |
| `module::compose::tests::*` | Module composition/name generation scaffolding | SPEC-20.13 |
| `module::predict::tests::*` | Predict wrapper behavior and prompt/input formatting | SPEC-20.12 |

---

## References

- [DSPy Signature](https://github.com/stanfordnlp/dspy/blob/main/dspy/signatures/signature.py)
- [DSPy Module](https://github.com/stanfordnlp/dspy/blob/main/dspy/primitives/module.py)
- [DSPy RLM](https://github.com/stanfordnlp/dspy/blob/main/dspy/predict/rlm.py)
