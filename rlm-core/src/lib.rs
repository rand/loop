//! # rlm-core
//!
//! A unified RLM (Recursive Language Model) orchestration library supporting both
//! Claude Code plugins and agentic TUIs.
//!
//! ## Core Components
//!
//! - **Context**: Session context, messages, and tool outputs
//! - **Trajectory**: Observable execution events for streaming
//! - **Complexity**: Task complexity signals and activation decisions
//! - **Orchestrator**: Core RLM orchestration loop
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::{Orchestrator, SessionContext, PatternClassifier};
//!
//! let classifier = PatternClassifier::default();
//! let ctx = SessionContext::new();
//!
//! let decision = classifier.should_activate("Analyze the auth system", &ctx);
//! if decision.should_activate {
//!     println!("RLM activated: {}", decision.reason);
//! }
//! ```

// Self-alias for derive macro support within the crate
extern crate self as rlm_core;

pub mod adapters;
#[cfg(feature = "adversarial")]
pub mod adversarial;
pub mod complexity;
pub mod context;
pub mod dp_integration;
pub mod epistemic;
pub mod error;
pub mod ffi;
pub mod lean;
pub mod llm;
pub mod memory;
pub mod module;
pub mod orchestrator;
pub mod proof;
#[cfg(feature = "python")]
pub mod pybind;
pub mod reasoning;
pub mod repl;
pub mod signature;
pub mod spec_agent;
pub mod sync;
pub mod topos;
pub mod trajectory;

// Re-exports for convenience
pub use complexity::{ActivationDecision, PatternClassifier, TaskComplexitySignals};
pub use context::{
    ContextSizeTracker, ContextVarType, ContextVariable, ExternalizedContext,
    ExternalizationConfig, Message, Role, SessionContext, SizeConfig, SizeWarning, ToolOutput,
    VariableAccessHelper,
};
pub use error::{Error, Result};
pub use llm::{
    AnthropicClient, BatchConfig, BatchExecutor, BatchQueryResult, BatchedLLMQuery,
    BatchedQueryResults, ClientConfig, CompletionRequest, CompletionResponse, CostTracker,
    DualModelConfig, LLMClient, ModelCallTier, ModelSpec, ModelTier, Provider, QueryType,
    RoutingContext, SmartRouter, SwitchStrategy, TierBreakdown,
};
pub use memory::{Node, NodeId, NodeType, SqliteMemoryStore, Tier};
pub use module::{
    chain_direct, BootstrapFewShot, Chain, Demonstration, Example, Module, ModuleConfig,
    Metric, NamedMetric, OptimizationStats, OptimizedModule, Optimizer, ParallelVec, Predict,
    PredictConfig, Predictor,
};
pub use orchestrator::{FallbackLoop, FallbackLoopStep, OrchestrationRoutingRuntime, Orchestrator};
pub use repl::{ExecuteResult, ReplConfig, ReplHandle, ReplPool};
pub use topos::{
    IndexBuilder, LeanRef, Link, LinkIndex, LinkType, ToposClient, ToposClientConfig, ToposRef,
};
pub use proof::{
    AIAssistantConfig, AIProofAssistant, AutomationTier, HelperLemma, HelperProofStatus,
    LimitReason, ProofAttempt, ProofAutomation, ProofAutomationBuilder, ProofContext,
    ProofSession, ProofSessionStatus, ProofStats, ProofStrategy, ProtocolConfig, ProtocolEnforcer,
    SorryLocation, SpecDomain, TacticResult,
};
pub use sync::{
    DriftReport, DriftType, DualTrackSync, FormalizationLevel, SyncDirection, SyncResult,
};
pub use trajectory::{TrajectoryEvent, TrajectoryEventType};
pub use reasoning::{
    DecisionNode, DecisionNodeId, DecisionNodeType, DecisionPath, DecisionPoint, DecisionTree,
    DotConfig, HtmlConfig, HtmlTheme, NetworkXGraph, NetworkXGraphAttrs, NetworkXLink, NetworkXNode,
    OptionStatus, ReasoningTrace, ReasoningTraceStore, TraceAnalyzer, TraceComparison,
    TraceEdge, TraceEdgeLabel, TraceId, TraceQuery, TraceStats, TraceStoreStats,
};
pub use dp_integration::{
    CoverageReport, CoverageSummary, DPCommand, DPCommandHandler, DPCommandResult,
    FormalizationReview, LeanProofScanner, ProofEvidence, ProofStatus, ReviewCheck,
    ReviewResult, SpecCoverage, SpecCoverageTracker, SpecId, TheoremInfo,
};
pub use epistemic::{
    audit_reasoning, evidence_dependence, quick_hallucination_check, verify_claim,
    BatchVerifier, BudgetResult, Claim, ClaimCategory, ClaimExtractor, EpistemicVerifier,
    EvidenceScrubber, GateDecision, GroundingStatus, HaikuVerifier, MemoryGate,
    MemoryGateConfig, Probability, SelfVerifier, ThresholdGate, VerificationConfig,
    VerificationResult, VerificationStats, VerificationVerdict,
};
pub use adapters::{
    suggested_output_path, trace_visualize, trace_visualize_from_json, AdapterConfig,
    AdapterSessionContext, AdapterStatus, ClaudeCodeAdapter, CompactData, HookContext,
    HookHandler, HookResult, HookTrigger, HtmlPreset, McpTool, McpToolRegistry,
    PromptEnhancement, RlmRequest, RlmResponse, RlmSkill, TraceVisualizeFormat,
    TraceVisualizeOptions, TraceVisualizeResult,
};
pub use signature::{
    apply_defaults, validate_fields, validate_value, ExecutionLimits, ExecutionResult,
    FallbackConfig, FallbackExtractor, FallbackTrigger, FieldSpec, FieldType, HistoryEntry,
    HistoryEntryType, ParseError, ReplHistory, Signature, ValidationError, ValidationResult,
};
#[cfg(feature = "adversarial")]
pub use adversarial::{
    AdversarialConfig, AdversarialTrigger, AdversarialValidator, CodeFile, CriticStrategy,
    EdgeCaseStrategy, FreshContextInvoker, FreshInvokerBuilder, GeminiFreshInvoker,
    GeminiValidator, InvocationStats, Issue, IssueCategory, IssueLocation, IssueSeverity,
    PerformanceStrategy, PooledFreshInvoker, SecurityStrategy, StrategyFactory, TestingStrategy,
    ToolOutput as AdversarialToolOutput, TraceabilityStrategy, ValidationContext, ValidationId,
    ValidationIteration, ValidationResult as AdversarialValidationResult,
    ValidationStats as AdversarialValidationStats, ValidationStrategy, ValidationVerdict,
};
