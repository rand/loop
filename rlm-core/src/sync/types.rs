//! Type definitions for the Dual-Track Sync Engine.
//!
//! This module defines the core types for bidirectional synchronization
//! between Topos specifications and Lean formalizations.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::topos::{LeanRef, ToposRef};

/// Direction of synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncDirection {
    /// Sync from Topos to Lean (generate Lean from Topos specs).
    ToposToLean,
    /// Sync from Lean to Topos (update Topos from Lean artifacts).
    LeanToTopos,
    /// Bidirectional sync (reconcile both directions).
    Bidirectional,
}

impl Default for SyncDirection {
    fn default() -> Self {
        Self::Bidirectional
    }
}

impl std::fmt::Display for SyncDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ToposToLean => write!(f, "Topos -> Lean"),
            Self::LeanToTopos => write!(f, "Lean -> Topos"),
            Self::Bidirectional => write!(f, "Bidirectional"),
        }
    }
}

/// Type of drift detected between Topos and Lean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriftType {
    /// Structural drift: types, fields, or signatures don't match.
    Structural,
    /// Semantic drift: names or meanings have diverged.
    Semantic,
    /// Missing in destination: source has element not in destination.
    Missing,
    /// Extra in destination: destination has element not in source.
    Extra,
}

impl DriftType {
    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Structural => "structural mismatch",
            Self::Semantic => "semantic divergence",
            Self::Missing => "missing in destination",
            Self::Extra => "extra in destination",
        }
    }
}

impl std::fmt::Display for DriftType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// A single drift item between Topos and Lean.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Drift {
    /// Reference to the Topos element (if applicable).
    pub topos_ref: Option<ToposRef>,
    /// Reference to the Lean artifact (if applicable).
    pub lean_ref: Option<LeanRef>,
    /// Type of drift.
    pub drift_type: DriftType,
    /// Human-readable description of the drift.
    pub description: String,
    /// Severity level (1-5, 5 being critical).
    #[serde(default = "default_severity")]
    pub severity: u8,
    /// Optional details about the drift (e.g., field differences).
    pub details: Option<DriftDetails>,
}

fn default_severity() -> u8 {
    3
}

impl Drift {
    /// Create a new drift.
    pub fn new(drift_type: DriftType, description: impl Into<String>) -> Self {
        Self {
            topos_ref: None,
            lean_ref: None,
            drift_type,
            description: description.into(),
            severity: 3,
            details: None,
        }
    }

    /// Create a drift for a missing Lean artifact.
    pub fn missing_lean(topos_ref: ToposRef, description: impl Into<String>) -> Self {
        Self {
            topos_ref: Some(topos_ref),
            lean_ref: None,
            drift_type: DriftType::Missing,
            description: description.into(),
            severity: 4,
            details: None,
        }
    }

    /// Create a drift for a missing Topos element.
    pub fn missing_topos(lean_ref: LeanRef, description: impl Into<String>) -> Self {
        Self {
            topos_ref: None,
            lean_ref: Some(lean_ref),
            drift_type: DriftType::Missing,
            description: description.into(),
            severity: 4,
            details: None,
        }
    }

    /// Create a drift for structural mismatch.
    pub fn structural(
        topos_ref: ToposRef,
        lean_ref: LeanRef,
        description: impl Into<String>,
    ) -> Self {
        Self {
            topos_ref: Some(topos_ref),
            lean_ref: Some(lean_ref),
            drift_type: DriftType::Structural,
            description: description.into(),
            severity: 3,
            details: None,
        }
    }

    /// Create a drift for semantic divergence.
    pub fn semantic(
        topos_ref: ToposRef,
        lean_ref: LeanRef,
        description: impl Into<String>,
    ) -> Self {
        Self {
            topos_ref: Some(topos_ref),
            lean_ref: Some(lean_ref),
            drift_type: DriftType::Semantic,
            description: description.into(),
            severity: 2,
            details: None,
        }
    }

    /// Set the severity.
    pub fn with_severity(mut self, severity: u8) -> Self {
        self.severity = severity.min(5);
        self
    }

    /// Add details.
    pub fn with_details(mut self, details: DriftDetails) -> Self {
        self.details = Some(details);
        self
    }
}

/// Additional details about a drift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriftDetails {
    /// Fields that differ.
    #[serde(default)]
    pub field_diffs: Vec<FieldDiff>,
    /// Type mismatches.
    #[serde(default)]
    pub type_mismatches: Vec<TypeMismatch>,
    /// Constraint differences.
    #[serde(default)]
    pub constraint_diffs: Vec<String>,
}

impl DriftDetails {
    /// Create empty details.
    pub fn new() -> Self {
        Self {
            field_diffs: Vec::new(),
            type_mismatches: Vec::new(),
            constraint_diffs: Vec::new(),
        }
    }

    /// Add a field diff.
    pub fn with_field_diff(mut self, diff: FieldDiff) -> Self {
        self.field_diffs.push(diff);
        self
    }

    /// Add a type mismatch.
    pub fn with_type_mismatch(mut self, mismatch: TypeMismatch) -> Self {
        self.type_mismatches.push(mismatch);
        self
    }

    /// Add a constraint diff.
    pub fn with_constraint_diff(mut self, diff: impl Into<String>) -> Self {
        self.constraint_diffs.push(diff.into());
        self
    }
}

impl Default for DriftDetails {
    fn default() -> Self {
        Self::new()
    }
}

/// A difference in a field between Topos and Lean.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDiff {
    /// Field name in Topos.
    pub topos_name: Option<String>,
    /// Field name in Lean.
    pub lean_name: Option<String>,
    /// Type in Topos.
    pub topos_type: Option<String>,
    /// Type in Lean.
    pub lean_type: Option<String>,
    /// Difference kind.
    pub kind: FieldDiffKind,
}

/// Kind of field difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldDiffKind {
    /// Field exists only in Topos.
    OnlyInTopos,
    /// Field exists only in Lean.
    OnlyInLean,
    /// Type mismatch between Topos and Lean.
    TypeMismatch,
    /// Name differs (but semantically same).
    NameDiffers,
}

/// A type mismatch between Topos and Lean.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeMismatch {
    /// Context (e.g., "field 'items'").
    pub context: String,
    /// Type in Topos.
    pub topos_type: String,
    /// Type in Lean.
    pub lean_type: String,
}

/// A suggestion for resolving a drift.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncSuggestion {
    /// The drift this suggestion addresses.
    pub drift_index: usize,
    /// Type of action suggested.
    pub action: SuggestedAction,
    /// Human-readable description of the suggestion.
    pub description: String,
    /// Confidence in the suggestion (0.0-1.0).
    pub confidence: f64,
    /// Generated code snippet (if applicable).
    pub code_snippet: Option<String>,
}

impl SyncSuggestion {
    /// Create a new suggestion.
    pub fn new(
        drift_index: usize,
        action: SuggestedAction,
        description: impl Into<String>,
        confidence: f64,
    ) -> Self {
        Self {
            drift_index,
            action,
            description: description.into(),
            confidence,
            code_snippet: None,
        }
    }

    /// Add a code snippet.
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code_snippet = Some(code.into());
        self
    }
}

/// Type of action suggested to resolve a drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuggestedAction {
    /// Generate Lean code from Topos.
    GenerateLean,
    /// Update Topos from Lean.
    UpdateTopos,
    /// Add a link annotation.
    AddLink,
    /// Remove an orphaned element.
    RemoveOrphan,
    /// Rename for consistency.
    Rename,
    /// Add missing field/member.
    AddField,
    /// Update type to match.
    UpdateType,
    /// Manual review required.
    ManualReview,
}

impl SuggestedAction {
    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::GenerateLean => "generate Lean code",
            Self::UpdateTopos => "update Topos specification",
            Self::AddLink => "add link annotation",
            Self::RemoveOrphan => "remove orphaned element",
            Self::Rename => "rename for consistency",
            Self::AddField => "add missing field",
            Self::UpdateType => "update type",
            Self::ManualReview => "requires manual review",
        }
    }
}

/// Report of all drifts detected between Topos and Lean.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    /// All detected drifts.
    pub drifts: Vec<Drift>,
    /// Suggestions for resolving drifts.
    pub suggestions: Vec<SyncSuggestion>,
    /// Summary statistics.
    pub summary: DriftSummary,
    /// When the report was generated.
    pub generated_at: String,
}

impl DriftReport {
    /// Create a new empty report.
    pub fn new() -> Self {
        Self {
            drifts: Vec::new(),
            suggestions: Vec::new(),
            summary: DriftSummary::default(),
            generated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Add a drift.
    pub fn add_drift(&mut self, drift: Drift) {
        self.drifts.push(drift);
        self.update_summary();
    }

    /// Add a suggestion.
    pub fn add_suggestion(&mut self, suggestion: SyncSuggestion) {
        self.suggestions.push(suggestion);
    }

    /// Check if there are any drifts.
    pub fn has_drifts(&self) -> bool {
        !self.drifts.is_empty()
    }

    /// Get drifts by type.
    pub fn drifts_by_type(&self, drift_type: DriftType) -> Vec<&Drift> {
        self.drifts
            .iter()
            .filter(|d| d.drift_type == drift_type)
            .collect()
    }

    /// Get high-severity drifts (severity >= 4).
    pub fn high_severity_drifts(&self) -> Vec<&Drift> {
        self.drifts.iter().filter(|d| d.severity >= 4).collect()
    }

    /// Update the summary statistics.
    fn update_summary(&mut self) {
        self.summary = DriftSummary {
            total: self.drifts.len(),
            structural: self
                .drifts
                .iter()
                .filter(|d| d.drift_type == DriftType::Structural)
                .count(),
            semantic: self
                .drifts
                .iter()
                .filter(|d| d.drift_type == DriftType::Semantic)
                .count(),
            missing: self
                .drifts
                .iter()
                .filter(|d| d.drift_type == DriftType::Missing)
                .count(),
            extra: self
                .drifts
                .iter()
                .filter(|d| d.drift_type == DriftType::Extra)
                .count(),
        };
    }
}

impl Default for DriftReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary statistics for a drift report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriftSummary {
    /// Total number of drifts.
    pub total: usize,
    /// Structural drifts.
    pub structural: usize,
    /// Semantic drifts.
    pub semantic: usize,
    /// Missing elements.
    pub missing: usize,
    /// Extra elements.
    pub extra: usize,
}

/// Level of formalization for generated Lean code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormalizationLevel {
    /// Types only: generate structure definitions.
    Types,
    /// Types + invariants: add basic invariant theorems.
    Invariants,
    /// Types + invariants + contracts: add pre/post conditions.
    Contracts,
    /// Full proofs: generate complete theorems with proof sketches.
    FullProofs,
}

impl FormalizationLevel {
    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Types => "type definitions only",
            Self::Invariants => "types with invariant theorems",
            Self::Contracts => "types with contracts (pre/post conditions)",
            Self::FullProofs => "full theorems with proof sketches",
        }
    }

    /// Check if invariants should be generated.
    pub fn includes_invariants(&self) -> bool {
        matches!(
            self,
            Self::Invariants | Self::Contracts | Self::FullProofs
        )
    }

    /// Check if contracts should be generated.
    pub fn includes_contracts(&self) -> bool {
        matches!(self, Self::Contracts | Self::FullProofs)
    }

    /// Check if proof sketches should be generated.
    pub fn includes_proofs(&self) -> bool {
        matches!(self, Self::FullProofs)
    }
}

impl Default for FormalizationLevel {
    fn default() -> Self {
        Self::Types
    }
}

impl std::fmt::Display for FormalizationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Result of a sync operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Direction of sync.
    pub direction: SyncDirection,
    /// Whether the sync was successful.
    pub success: bool,
    /// Files created.
    pub files_created: Vec<PathBuf>,
    /// Files modified.
    pub files_modified: Vec<PathBuf>,
    /// Links added to index.
    pub links_added: usize,
    /// Drifts resolved.
    pub drifts_resolved: usize,
    /// Remaining drifts (could not be auto-resolved).
    pub remaining_drifts: Vec<Drift>,
    /// Errors encountered.
    pub errors: Vec<String>,
    /// Warnings.
    pub warnings: Vec<String>,
}

impl SyncResult {
    /// Create a new successful result.
    pub fn success(direction: SyncDirection) -> Self {
        Self {
            direction,
            success: true,
            files_created: Vec::new(),
            files_modified: Vec::new(),
            links_added: 0,
            drifts_resolved: 0,
            remaining_drifts: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed result.
    pub fn failure(direction: SyncDirection, error: impl Into<String>) -> Self {
        Self {
            direction,
            success: false,
            files_created: Vec::new(),
            files_modified: Vec::new(),
            links_added: 0,
            drifts_resolved: 0,
            remaining_drifts: Vec::new(),
            errors: vec![error.into()],
            warnings: Vec::new(),
        }
    }

    /// Add a created file.
    pub fn with_created(mut self, path: PathBuf) -> Self {
        self.files_created.push(path);
        self
    }

    /// Add a modified file.
    pub fn with_modified(mut self, path: PathBuf) -> Self {
        self.files_modified.push(path);
        self
    }

    /// Set links added count.
    pub fn with_links_added(mut self, count: usize) -> Self {
        self.links_added = count;
        self
    }

    /// Set drifts resolved count.
    pub fn with_drifts_resolved(mut self, count: usize) -> Self {
        self.drifts_resolved = count;
        self
    }

    /// Add a remaining drift.
    pub fn with_remaining_drift(mut self, drift: Drift) -> Self {
        self.remaining_drifts.push(drift);
        self
    }

    /// Add an error.
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.errors.push(error.into());
        self.success = false;
        self
    }

    /// Add a warning.
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

/// Configuration for sync operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Default formalization level.
    pub formalization_level: FormalizationLevel,
    /// Whether to auto-generate missing Lean files.
    pub auto_generate_lean: bool,
    /// Whether to auto-update Topos specs.
    pub auto_update_topos: bool,
    /// Whether to add link annotations automatically.
    pub auto_link: bool,
    /// Whether to require manual confirmation for changes.
    pub require_confirmation: bool,
    /// Maximum severity level to auto-resolve (1-5).
    pub auto_resolve_max_severity: u8,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            formalization_level: FormalizationLevel::Types,
            auto_generate_lean: true,
            auto_update_topos: false,
            auto_link: true,
            require_confirmation: true,
            auto_resolve_max_severity: 2,
        }
    }
}

/// A parsed Topos concept for sync purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToposConcept {
    /// Concept name.
    pub name: String,
    /// Fields in the concept.
    pub fields: Vec<ToposField>,
    /// Invariants on the concept.
    pub invariants: Vec<ToposInvariant>,
    /// Documentation/description.
    pub doc: Option<String>,
    /// Source file.
    pub source_file: PathBuf,
    /// Line number.
    pub line: u32,
}

/// A field in a Topos concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToposField {
    /// Field name.
    pub name: String,
    /// Field type (as written in Topos).
    pub field_type: String,
    /// Optional description.
    pub description: Option<String>,
    /// Constraints on the field.
    pub constraints: Vec<String>,
}

/// An invariant in a Topos spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToposInvariant {
    /// Invariant name.
    pub name: String,
    /// Invariant expression/description.
    pub expression: String,
    /// Whether this is on a specific field.
    pub field: Option<String>,
}

/// A parsed Topos behavior for sync purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToposBehavior {
    /// Behavior name.
    pub name: String,
    /// Input parameters.
    pub inputs: Vec<ToposField>,
    /// Return type.
    pub returns: Option<String>,
    /// Pre-conditions.
    pub preconditions: Vec<String>,
    /// Post-conditions.
    pub postconditions: Vec<String>,
    /// Documentation.
    pub doc: Option<String>,
    /// Source file.
    pub source_file: PathBuf,
    /// Line number.
    pub line: u32,
}

/// A parsed Lean structure for sync purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanStructure {
    /// Structure name.
    pub name: String,
    /// Fields in the structure.
    pub fields: Vec<LeanField>,
    /// Namespace (if any).
    pub namespace: Option<String>,
    /// Documentation.
    pub doc: Option<String>,
    /// Source file.
    pub source_file: PathBuf,
    /// Line number.
    pub line: u32,
}

/// A field in a Lean structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanField {
    /// Field name.
    pub name: String,
    /// Field type.
    pub field_type: String,
    /// Default value (if any).
    pub default_value: Option<String>,
}

/// A parsed Lean theorem for sync purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanTheorem {
    /// Theorem name.
    pub name: String,
    /// Theorem type/statement.
    pub statement: String,
    /// Namespace (if any).
    pub namespace: Option<String>,
    /// Documentation.
    pub doc: Option<String>,
    /// Whether it has a proof or uses sorry.
    pub has_proof: bool,
    /// Source file.
    pub source_file: PathBuf,
    /// Line number.
    pub line: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drift_creation() {
        let topos_ref = ToposRef::new("spec.tps", "Order");
        let drift = Drift::missing_lean(topos_ref.clone(), "No Lean structure for Order");

        assert_eq!(drift.drift_type, DriftType::Missing);
        assert!(drift.topos_ref.is_some());
        assert!(drift.lean_ref.is_none());
        assert_eq!(drift.severity, 4);
    }

    #[test]
    fn test_drift_report() {
        let mut report = DriftReport::new();
        let topos_ref = ToposRef::new("spec.tps", "Order");

        report.add_drift(Drift::missing_lean(
            topos_ref,
            "No Lean structure for Order",
        ));

        assert!(report.has_drifts());
        assert_eq!(report.summary.total, 1);
        assert_eq!(report.summary.missing, 1);
    }

    #[test]
    fn test_formalization_level() {
        let level = FormalizationLevel::Contracts;
        assert!(level.includes_invariants());
        assert!(level.includes_contracts());
        assert!(!level.includes_proofs());

        let level = FormalizationLevel::Types;
        assert!(!level.includes_invariants());
    }

    #[test]
    fn test_sync_result() {
        let result = SyncResult::success(SyncDirection::ToposToLean)
            .with_created(PathBuf::from("Order.lean"))
            .with_links_added(1)
            .with_drifts_resolved(1);

        assert!(result.success);
        assert_eq!(result.files_created.len(), 1);
        assert_eq!(result.links_added, 1);
    }

    #[test]
    fn test_sync_suggestion() {
        let suggestion = SyncSuggestion::new(
            0,
            SuggestedAction::GenerateLean,
            "Generate Lean structure for Order",
            0.9,
        )
        .with_code("structure Order where\n  id : Nat");

        assert_eq!(suggestion.action, SuggestedAction::GenerateLean);
        assert!(suggestion.code_snippet.is_some());
    }
}
