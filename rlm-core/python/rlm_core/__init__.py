"""
rlm-core: Unified RLM (Recursive Language Model) orchestration library.

This package provides Python bindings for the Rust rlm-core library,
enabling use from Claude Code plugins and other Python applications.

Example:
    >>> from rlm_core import SessionContext, Message, PatternClassifier
    >>>
    >>> # Create a session context
    >>> ctx = SessionContext()
    >>> ctx.add_message(Message.user("Analyze the auth system"))
    >>>
    >>> # Check if RLM should activate
    >>> classifier = PatternClassifier()
    >>> decision = classifier.should_activate("Analyze the auth system", ctx)
    >>> if decision.should_activate:
    ...     print(f"RLM activated: {decision.reason}")
"""

from __future__ import annotations

# Import from Rust extension
try:
    from rlm_core.rlm_core import (
        # Context types
        Role,
        Message,
        ToolOutput,
        SessionContext,
        # Memory types
        NodeType,
        Tier,
        Node,
        HyperEdge,
        MemoryStore,
        MemoryStats,
        # LLM types
        Provider,
        ModelTier,
        ModelSpec,
        ChatMessage,
        TokenUsage,
        CompletionRequest,
        CompletionResponse,
        QueryType,
        RoutingContext,
        RoutingDecision,
        SmartRouter,
        CostTracker,
        # Trajectory types
        TrajectoryEventType,
        TrajectoryEvent,
        # Complexity types
        ActivationDecision,
        PatternClassifier,
        # Epistemic verification types
        ClaimCategory,
        GroundingStatus,
        EvidenceType,
        VerificationVerdict,
        Probability,
        EvidenceRef,
        Claim,
        BudgetResult,
        VerificationConfig,
        VerificationStats,
        ClaimExtractor,
        KL,
        quick_hallucination_check,
    )
except ImportError:
    # Provide stubs for IDE support when extension not built
    pass

__version__ = "0.1.0"
__all__ = [
    # Context
    "Role",
    "Message",
    "ToolOutput",
    "SessionContext",
    # Memory
    "NodeType",
    "Tier",
    "Node",
    "HyperEdge",
    "MemoryStore",
    "MemoryStats",
    # LLM
    "Provider",
    "ModelTier",
    "ModelSpec",
    "ChatMessage",
    "TokenUsage",
    "CompletionRequest",
    "CompletionResponse",
    "QueryType",
    "RoutingContext",
    "RoutingDecision",
    "SmartRouter",
    "CostTracker",
    # Trajectory
    "TrajectoryEventType",
    "TrajectoryEvent",
    # Complexity
    "ActivationDecision",
    "PatternClassifier",
    # Epistemic verification
    "ClaimCategory",
    "GroundingStatus",
    "EvidenceType",
    "VerificationVerdict",
    "Probability",
    "EvidenceRef",
    "Claim",
    "BudgetResult",
    "VerificationConfig",
    "VerificationStats",
    "ClaimExtractor",
    "KL",
    "quick_hallucination_check",
]
