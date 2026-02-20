//! FFI type definitions and conversions.
//!
//! Defines opaque handle types and conversion utilities for the C API.

use std::os::raw::c_char;

/// Opaque handle for SessionContext.
pub struct RlmSessionContext(pub(crate) crate::context::SessionContext);

/// Opaque handle for Message.
pub struct RlmMessage(pub(crate) crate::context::Message);

/// Opaque handle for ToolOutput.
pub struct RlmToolOutput(pub(crate) crate::context::ToolOutput);

/// Opaque handle for SqliteMemoryStore.
pub struct RlmMemoryStore(pub(crate) crate::memory::SqliteMemoryStore);

/// Opaque handle for Node.
pub struct RlmNode(pub(crate) crate::memory::Node);

/// Opaque handle for HyperEdge.
pub struct RlmHyperEdge(pub(crate) crate::memory::HyperEdge);

/// Opaque handle for TrajectoryEvent.
pub struct RlmTrajectoryEvent(pub(crate) crate::trajectory::TrajectoryEvent);

/// Opaque handle for PatternClassifier.
pub struct RlmPatternClassifier(pub(crate) crate::complexity::PatternClassifier);

/// Opaque handle for ActivationDecision.
pub struct RlmActivationDecision(pub(crate) crate::complexity::ActivationDecision);

/// Opaque handle for ReplHandle.
pub struct RlmReplHandle(pub(crate) crate::repl::ReplHandle);

/// Opaque handle for ReplPool.
pub struct RlmReplPool(pub(crate) crate::repl::ReplPool);

// ============================================================================
// Enum representations for FFI
// ============================================================================

/// Role enum for messages.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlmRole {
    System = 0,
    User = 1,
    Assistant = 2,
    Tool = 3,
}

impl From<crate::context::Role> for RlmRole {
    fn from(r: crate::context::Role) -> Self {
        match r {
            crate::context::Role::System => RlmRole::System,
            crate::context::Role::User => RlmRole::User,
            crate::context::Role::Assistant => RlmRole::Assistant,
            crate::context::Role::Tool => RlmRole::Tool,
        }
    }
}

impl From<RlmRole> for crate::context::Role {
    fn from(r: RlmRole) -> Self {
        match r {
            RlmRole::System => crate::context::Role::System,
            RlmRole::User => crate::context::Role::User,
            RlmRole::Assistant => crate::context::Role::Assistant,
            RlmRole::Tool => crate::context::Role::Tool,
        }
    }
}

/// NodeType enum for memory nodes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlmNodeType {
    Entity = 0,
    Fact = 1,
    Experience = 2,
    Decision = 3,
    Snippet = 4,
}

impl From<crate::memory::NodeType> for RlmNodeType {
    fn from(t: crate::memory::NodeType) -> Self {
        match t {
            crate::memory::NodeType::Entity => RlmNodeType::Entity,
            crate::memory::NodeType::Fact => RlmNodeType::Fact,
            crate::memory::NodeType::Experience => RlmNodeType::Experience,
            crate::memory::NodeType::Decision => RlmNodeType::Decision,
            crate::memory::NodeType::Snippet => RlmNodeType::Snippet,
        }
    }
}

impl From<RlmNodeType> for crate::memory::NodeType {
    fn from(t: RlmNodeType) -> Self {
        match t {
            RlmNodeType::Entity => crate::memory::NodeType::Entity,
            RlmNodeType::Fact => crate::memory::NodeType::Fact,
            RlmNodeType::Experience => crate::memory::NodeType::Experience,
            RlmNodeType::Decision => crate::memory::NodeType::Decision,
            RlmNodeType::Snippet => crate::memory::NodeType::Snippet,
        }
    }
}

/// Tier enum for memory tiers.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlmTier {
    Task = 0,
    Session = 1,
    LongTerm = 2,
    Archive = 3,
}

impl From<crate::memory::Tier> for RlmTier {
    fn from(t: crate::memory::Tier) -> Self {
        match t {
            crate::memory::Tier::Task => RlmTier::Task,
            crate::memory::Tier::Session => RlmTier::Session,
            crate::memory::Tier::LongTerm => RlmTier::LongTerm,
            crate::memory::Tier::Archive => RlmTier::Archive,
        }
    }
}

impl From<RlmTier> for crate::memory::Tier {
    fn from(t: RlmTier) -> Self {
        match t {
            RlmTier::Task => crate::memory::Tier::Task,
            RlmTier::Session => crate::memory::Tier::Session,
            RlmTier::LongTerm => crate::memory::Tier::LongTerm,
            RlmTier::Archive => crate::memory::Tier::Archive,
        }
    }
}

/// TrajectoryEventType enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlmTrajectoryEventType {
    RlmStart = 0,
    Analyze = 1,
    ReplExec = 2,
    ReplResult = 3,
    Reason = 4,
    RecurseStart = 5,
    RecurseEnd = 6,
    Final = 7,
    Error = 8,
    ToolUse = 9,
    CostReport = 10,
    VerifyStart = 11,
    ClaimExtracted = 12,
    EvidenceChecked = 13,
    BudgetComputed = 14,
    HallucinationFlag = 15,
    VerifyComplete = 16,
    Memory = 17,
    Externalize = 18,
    Decompose = 19,
    Synthesize = 20,
    AdversarialStart = 21,
    CriticInvoked = 22,
    IssueFound = 23,
    AdversarialComplete = 24,
}

impl From<crate::trajectory::TrajectoryEventType> for RlmTrajectoryEventType {
    fn from(t: crate::trajectory::TrajectoryEventType) -> Self {
        match t {
            crate::trajectory::TrajectoryEventType::RlmStart => RlmTrajectoryEventType::RlmStart,
            crate::trajectory::TrajectoryEventType::Analyze => RlmTrajectoryEventType::Analyze,
            crate::trajectory::TrajectoryEventType::ReplExec => RlmTrajectoryEventType::ReplExec,
            crate::trajectory::TrajectoryEventType::ReplResult => {
                RlmTrajectoryEventType::ReplResult
            }
            crate::trajectory::TrajectoryEventType::Reason => RlmTrajectoryEventType::Reason,
            crate::trajectory::TrajectoryEventType::RecurseStart => {
                RlmTrajectoryEventType::RecurseStart
            }
            crate::trajectory::TrajectoryEventType::RecurseEnd => {
                RlmTrajectoryEventType::RecurseEnd
            }
            crate::trajectory::TrajectoryEventType::Final => RlmTrajectoryEventType::Final,
            crate::trajectory::TrajectoryEventType::Error => RlmTrajectoryEventType::Error,
            crate::trajectory::TrajectoryEventType::ToolUse => RlmTrajectoryEventType::ToolUse,
            crate::trajectory::TrajectoryEventType::CostReport => {
                RlmTrajectoryEventType::CostReport
            }
            crate::trajectory::TrajectoryEventType::VerifyStart => {
                RlmTrajectoryEventType::VerifyStart
            }
            crate::trajectory::TrajectoryEventType::ClaimExtracted => {
                RlmTrajectoryEventType::ClaimExtracted
            }
            crate::trajectory::TrajectoryEventType::EvidenceChecked => {
                RlmTrajectoryEventType::EvidenceChecked
            }
            crate::trajectory::TrajectoryEventType::BudgetComputed => {
                RlmTrajectoryEventType::BudgetComputed
            }
            crate::trajectory::TrajectoryEventType::HallucinationFlag => {
                RlmTrajectoryEventType::HallucinationFlag
            }
            crate::trajectory::TrajectoryEventType::VerifyComplete => {
                RlmTrajectoryEventType::VerifyComplete
            }
            crate::trajectory::TrajectoryEventType::Memory => RlmTrajectoryEventType::Memory,
            crate::trajectory::TrajectoryEventType::Externalize => {
                RlmTrajectoryEventType::Externalize
            }
            crate::trajectory::TrajectoryEventType::Decompose => RlmTrajectoryEventType::Decompose,
            crate::trajectory::TrajectoryEventType::Synthesize => {
                RlmTrajectoryEventType::Synthesize
            }
            crate::trajectory::TrajectoryEventType::AdversarialStart => {
                RlmTrajectoryEventType::AdversarialStart
            }
            crate::trajectory::TrajectoryEventType::CriticInvoked => {
                RlmTrajectoryEventType::CriticInvoked
            }
            crate::trajectory::TrajectoryEventType::IssueFound => {
                RlmTrajectoryEventType::IssueFound
            }
            crate::trajectory::TrajectoryEventType::AdversarialComplete => {
                RlmTrajectoryEventType::AdversarialComplete
            }
        }
    }
}

impl From<RlmTrajectoryEventType> for crate::trajectory::TrajectoryEventType {
    fn from(t: RlmTrajectoryEventType) -> Self {
        match t {
            RlmTrajectoryEventType::RlmStart => crate::trajectory::TrajectoryEventType::RlmStart,
            RlmTrajectoryEventType::Analyze => crate::trajectory::TrajectoryEventType::Analyze,
            RlmTrajectoryEventType::ReplExec => crate::trajectory::TrajectoryEventType::ReplExec,
            RlmTrajectoryEventType::ReplResult => {
                crate::trajectory::TrajectoryEventType::ReplResult
            }
            RlmTrajectoryEventType::Reason => crate::trajectory::TrajectoryEventType::Reason,
            RlmTrajectoryEventType::RecurseStart => {
                crate::trajectory::TrajectoryEventType::RecurseStart
            }
            RlmTrajectoryEventType::RecurseEnd => {
                crate::trajectory::TrajectoryEventType::RecurseEnd
            }
            RlmTrajectoryEventType::Final => crate::trajectory::TrajectoryEventType::Final,
            RlmTrajectoryEventType::Error => crate::trajectory::TrajectoryEventType::Error,
            RlmTrajectoryEventType::ToolUse => crate::trajectory::TrajectoryEventType::ToolUse,
            RlmTrajectoryEventType::CostReport => {
                crate::trajectory::TrajectoryEventType::CostReport
            }
            RlmTrajectoryEventType::VerifyStart => {
                crate::trajectory::TrajectoryEventType::VerifyStart
            }
            RlmTrajectoryEventType::ClaimExtracted => {
                crate::trajectory::TrajectoryEventType::ClaimExtracted
            }
            RlmTrajectoryEventType::EvidenceChecked => {
                crate::trajectory::TrajectoryEventType::EvidenceChecked
            }
            RlmTrajectoryEventType::BudgetComputed => {
                crate::trajectory::TrajectoryEventType::BudgetComputed
            }
            RlmTrajectoryEventType::HallucinationFlag => {
                crate::trajectory::TrajectoryEventType::HallucinationFlag
            }
            RlmTrajectoryEventType::VerifyComplete => {
                crate::trajectory::TrajectoryEventType::VerifyComplete
            }
            RlmTrajectoryEventType::Memory => crate::trajectory::TrajectoryEventType::Memory,
            RlmTrajectoryEventType::Externalize => {
                crate::trajectory::TrajectoryEventType::Externalize
            }
            RlmTrajectoryEventType::Decompose => crate::trajectory::TrajectoryEventType::Decompose,
            RlmTrajectoryEventType::Synthesize => {
                crate::trajectory::TrajectoryEventType::Synthesize
            }
            RlmTrajectoryEventType::AdversarialStart => {
                crate::trajectory::TrajectoryEventType::AdversarialStart
            }
            RlmTrajectoryEventType::CriticInvoked => {
                crate::trajectory::TrajectoryEventType::CriticInvoked
            }
            RlmTrajectoryEventType::IssueFound => {
                crate::trajectory::TrajectoryEventType::IssueFound
            }
            RlmTrajectoryEventType::AdversarialComplete => {
                crate::trajectory::TrajectoryEventType::AdversarialComplete
            }
        }
    }
}

// ============================================================================
// Callback types for streaming
// ============================================================================

/// Callback for receiving trajectory events.
///
/// The callback receives ownership of the event pointer and must free it
/// with `rlm_trajectory_event_free()` when done.
///
/// The `user_data` pointer is passed through unchanged.
pub type RlmTrajectoryCallback =
    extern "C" fn(event: *mut RlmTrajectoryEvent, user_data: *mut std::ffi::c_void);

/// Callback for receiving error messages.
///
/// The error string is valid only for the duration of the callback.
pub type RlmErrorCallback = extern "C" fn(error: *const c_char, user_data: *mut std::ffi::c_void);
