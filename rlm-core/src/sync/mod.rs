//! Dual-Track Sync Engine for Topos-Lean synchronization.
//!
//! This module provides bidirectional synchronization between Topos semantic
//! contracts and Lean formal specifications.
//!
//! ## Overview
//!
//! The sync engine maintains consistency between:
//! - **Topos** (.tps files): Human-readable semantic contracts
//! - **Lean** (.lean files): Machine-verifiable formal specifications
//!
//! ## Components
//!
//! - [`types`]: Core types for sync operations (drift, suggestions, results)
//! - [`drift`]: Drift detection between Topos and Lean
//! - [`generators`]: Code generation for both directions
//! - [`engine`]: The main `DualTrackSync` orchestrator
//!
//! ## Formalization Levels
//!
//! The sync engine supports different levels of formalization when generating
//! Lean code:
//!
//! - **Types**: Generate structure definitions only
//! - **Invariants**: Add basic invariant theorems
//! - **Contracts**: Add pre/post conditions
//! - **FullProofs**: Generate complete theorems with proof sketches
//!
//! ## Example
//!
//! ```rust,ignore
//! use rlm_core::sync::{DualTrackSync, FormalizationLevel, SyncDirection};
//!
//! // Create sync engine
//! let mut sync = DualTrackSync::new(
//!     PathBuf::from("./specs"),
//!     PathBuf::from("./lean"),
//! )
//! .with_level(FormalizationLevel::Invariants);
//!
//! // Scan project files
//! sync.scan().await?;
//!
//! // Detect drift between Topos and Lean
//! let report = sync.detect_drift().await?;
//! println!("Found {} drifts", report.drifts.len());
//!
//! // Sync Topos -> Lean (generate Lean from Topos)
//! let result = sync.sync_topos_to_lean().await?;
//! println!("Created {} files", result.files_created.len());
//!
//! // Or run bidirectional sync
//! let result = sync.sync(SyncDirection::Bidirectional).await?;
//! ```
//!
//! ## Drift Detection
//!
//! The engine detects several types of drift:
//!
//! - **Structural**: Types, fields, or signatures don't match
//! - **Semantic**: Names or meanings have diverged
//! - **Missing**: Element exists in source but not destination
//! - **Extra**: Element exists in destination but not source
//!
//! For each drift, the engine provides suggestions for resolution with
//! confidence scores.
//!
//! ## Code Generation
//!
//! ### Topos to Lean
//!
//! ```rust,ignore
//! use rlm_core::sync::{topos_to_lean_structure, FormalizationLevel};
//!
//! let lean_code = topos_to_lean_structure(&concept, FormalizationLevel::Types);
//! ```
//!
//! ### Lean to Topos
//!
//! ```rust,ignore
//! use rlm_core::sync::lean_to_topos_concept;
//!
//! let topos_code = lean_to_topos_concept(&structure);
//! ```

pub mod drift;
pub mod engine;
pub mod generators;
pub mod types;

// Re-exports for convenience
pub use drift::{
    parse_lean_structures, parse_lean_theorems, parse_topos_behaviors, parse_topos_concepts,
    DriftDetector,
};
pub use engine::DualTrackSync;
pub use generators::{
    lean_to_topos_behavior, lean_to_topos_concept, topos_to_lean_structure, topos_to_lean_theorem,
    LeanGenerator, ToposGenerator,
};
pub use types::{
    Drift, DriftDetails, DriftReport, DriftSummary, DriftType, FieldDiff, FieldDiffKind,
    FormalizationLevel, LeanField, LeanStructure, LeanTheorem, SuggestedAction, SyncConfig,
    SyncDirection, SyncResult, SyncSuggestion, ToposBehavior, ToposConcept, ToposField,
    ToposInvariant, TypeMismatch,
};
