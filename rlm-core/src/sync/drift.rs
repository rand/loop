//! Drift detection between Topos specifications and Lean formalizations.
//!
//! This module provides functionality to detect structural and semantic
//! differences between Topos specs and their corresponding Lean artifacts.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::topos::{LeanRef, LinkIndex, LinkType, ToposRef};

use super::types::{
    Drift, DriftDetails, DriftReport, DriftType, FieldDiff, FieldDiffKind, LeanField,
    LeanStructure, LeanTheorem, SuggestedAction, SyncSuggestion, ToposBehavior, ToposConcept,
    ToposField, ToposInvariant, TypeMismatch,
};

/// Drift detector for comparing Topos and Lean artifacts.
pub struct DriftDetector {
    /// Type mapping from Topos to Lean.
    type_mappings: HashMap<String, String>,
}

impl DriftDetector {
    /// Create a new drift detector with default type mappings.
    pub fn new() -> Self {
        let mut type_mappings = HashMap::new();

        // Common type mappings from Topos to Lean
        type_mappings.insert("string".to_string(), "String".to_string());
        type_mappings.insert("String".to_string(), "String".to_string());
        type_mappings.insert("int".to_string(), "Int".to_string());
        type_mappings.insert("integer".to_string(), "Int".to_string());
        type_mappings.insert("nat".to_string(), "Nat".to_string());
        type_mappings.insert("natural".to_string(), "Nat".to_string());
        type_mappings.insert("bool".to_string(), "Bool".to_string());
        type_mappings.insert("boolean".to_string(), "Bool".to_string());
        type_mappings.insert("float".to_string(), "Float".to_string());
        type_mappings.insert("decimal".to_string(), "Float".to_string());
        type_mappings.insert("list".to_string(), "List".to_string());
        type_mappings.insert("array".to_string(), "Array".to_string());
        type_mappings.insert("optional".to_string(), "Option".to_string());
        type_mappings.insert("maybe".to_string(), "Option".to_string());

        Self { type_mappings }
    }

    /// Create a drift detector with custom type mappings.
    pub fn with_mappings(mappings: HashMap<String, String>) -> Self {
        let mut detector = Self::new();
        detector.type_mappings.extend(mappings);
        detector
    }

    /// Add a type mapping.
    pub fn add_mapping(&mut self, topos_type: impl Into<String>, lean_type: impl Into<String>) {
        self.type_mappings
            .insert(topos_type.into(), lean_type.into());
    }

    /// Detect drift between Topos concepts and Lean structures.
    pub fn detect_concept_drift(
        &self,
        concepts: &[ToposConcept],
        structures: &[LeanStructure],
        index: &LinkIndex,
    ) -> DriftReport {
        let mut report = DriftReport::new();

        // Build lookup maps
        let struct_by_name: HashMap<&str, &LeanStructure> =
            structures.iter().map(|s| (s.name.as_str(), s)).collect();

        let linked_lean_names: HashSet<String> = index
            .links_by_type(LinkType::Structure)
            .iter()
            .map(|link| link.lean.artifact.clone())
            .collect();

        // Check each concept for a corresponding structure
        for concept in concepts {
            let topos_ref = ToposRef::new(&concept.source_file, &concept.name);
            let links = index.get_lean_refs(&topos_ref);

            if links.is_empty() {
                // No link - check if there's a structure with matching name
                if let Some(structure) = struct_by_name.get(concept.name.as_str()) {
                    // Structure exists but no link
                    let lean_ref = LeanRef::new(&structure.source_file, &structure.name);
                    let drift = Drift::new(
                        DriftType::Missing,
                        format!(
                            "Concept '{}' has matching Lean structure but no @lean annotation",
                            concept.name
                        ),
                    )
                    .with_severity(2);

                    let drift_idx = report.drifts.len();
                    report.add_drift(
                        Drift {
                            topos_ref: Some(topos_ref.clone()),
                            lean_ref: Some(lean_ref.clone()),
                            ..drift
                        }
                    );

                    report.add_suggestion(SyncSuggestion::new(
                        drift_idx,
                        SuggestedAction::AddLink,
                        format!(
                            "Add @lean: {}#{} annotation to Concept {}",
                            structure.source_file.display(),
                            structure.name,
                            concept.name
                        ),
                        0.95,
                    ));
                } else {
                    // No structure at all
                    let drift = Drift::missing_lean(
                        topos_ref.clone(),
                        format!(
                            "Concept '{}' has no corresponding Lean structure",
                            concept.name
                        ),
                    );

                    let drift_idx = report.drifts.len();
                    report.add_drift(drift);

                    report.add_suggestion(SyncSuggestion::new(
                        drift_idx,
                        SuggestedAction::GenerateLean,
                        format!("Generate Lean structure for Concept '{}'", concept.name),
                        0.9,
                    ));
                }
            } else {
                // Has links - check each linked structure
                for link in links {
                    if let Some(structure) = struct_by_name.get(link.lean.artifact.as_str()) {
                        // Compare fields
                        let field_drifts =
                            self.compare_fields(&concept.fields, &structure.fields, concept, structure);

                        for (drift, suggestion) in field_drifts {
                            let drift_idx = report.drifts.len();
                            report.add_drift(drift);
                            if let Some(mut sugg) = suggestion {
                                sugg.drift_index = drift_idx;
                                report.add_suggestion(sugg);
                            }
                        }
                    } else {
                        // Link points to non-existent structure
                        let drift = Drift {
                            topos_ref: Some(topos_ref.clone()),
                            lean_ref: Some(link.lean.clone()),
                            drift_type: DriftType::Missing,
                            description: format!(
                                "Link points to non-existent Lean structure '{}'",
                                link.lean.artifact
                            ),
                            severity: 5,
                            details: None,
                        };

                        let drift_idx = report.drifts.len();
                        report.add_drift(drift);

                        report.add_suggestion(SyncSuggestion::new(
                            drift_idx,
                            SuggestedAction::GenerateLean,
                            format!(
                                "Generate Lean structure '{}' or update link",
                                link.lean.artifact
                            ),
                            0.8,
                        ));
                    }
                }
            }
        }

        // Check for extra structures (in Lean but not linked from Topos)
        for structure in structures {
            if !linked_lean_names.contains(&structure.name) {
                let lean_ref = LeanRef::new(&structure.source_file, &structure.name);

                // Check if there's a concept with matching name
                let matching_concept = concepts.iter().find(|c| c.name == structure.name);

                if matching_concept.is_some() {
                    // Already handled above (missing link case)
                    continue;
                }

                let drift = Drift::missing_topos(
                    lean_ref,
                    format!(
                        "Lean structure '{}' has no corresponding Topos concept",
                        structure.name
                    ),
                )
                .with_severity(3);

                let drift_idx = report.drifts.len();
                report.add_drift(drift);

                report.add_suggestion(SyncSuggestion::new(
                    drift_idx,
                    SuggestedAction::UpdateTopos,
                    format!(
                        "Create Topos concept for Lean structure '{}'",
                        structure.name
                    ),
                    0.7,
                ));
            }
        }

        report
    }

    /// Detect drift between Topos behaviors and Lean function specs/theorems.
    pub fn detect_behavior_drift(
        &self,
        behaviors: &[ToposBehavior],
        theorems: &[LeanTheorem],
        index: &LinkIndex,
    ) -> DriftReport {
        let mut report = DriftReport::new();

        let theorem_by_name: HashMap<&str, &LeanTheorem> =
            theorems.iter().map(|t| (t.name.as_str(), t)).collect();

        let linked_theorem_names: HashSet<String> = index
            .links_by_type(LinkType::FunctionSpec)
            .iter()
            .chain(index.links_by_type(LinkType::Theorem).iter())
            .map(|link| link.lean.artifact.clone())
            .collect();

        for behavior in behaviors {
            let topos_ref = ToposRef::new(&behavior.source_file, &behavior.name);
            let links = index.get_lean_refs(&topos_ref);

            if links.is_empty() {
                // Check for matching theorem by name convention
                let expected_name = format!("{}_spec", behavior.name);
                let alternate_name = behavior.name.clone();

                let matching_theorem = theorem_by_name
                    .get(expected_name.as_str())
                    .or_else(|| theorem_by_name.get(alternate_name.as_str()));

                if let Some(theorem) = matching_theorem {
                    let lean_ref = LeanRef::new(&theorem.source_file, &theorem.name);
                    let drift = Drift {
                        topos_ref: Some(topos_ref.clone()),
                        lean_ref: Some(lean_ref.clone()),
                        drift_type: DriftType::Missing,
                        description: format!(
                            "Behavior '{}' has matching Lean theorem but no @lean annotation",
                            behavior.name
                        ),
                        severity: 2,
                        details: None,
                    };

                    let drift_idx = report.drifts.len();
                    report.add_drift(drift);

                    report.add_suggestion(SyncSuggestion::new(
                        drift_idx,
                        SuggestedAction::AddLink,
                        format!(
                            "Add @lean: {}#{} annotation to Behavior {}",
                            theorem.source_file.display(),
                            theorem.name,
                            behavior.name
                        ),
                        0.9,
                    ));
                } else {
                    let drift = Drift::missing_lean(
                        topos_ref.clone(),
                        format!(
                            "Behavior '{}' has no corresponding Lean theorem/spec",
                            behavior.name
                        ),
                    );

                    let drift_idx = report.drifts.len();
                    report.add_drift(drift);

                    report.add_suggestion(SyncSuggestion::new(
                        drift_idx,
                        SuggestedAction::GenerateLean,
                        format!("Generate Lean theorem for Behavior '{}'", behavior.name),
                        0.85,
                    ));
                }
            } else {
                // Has links - verify they exist
                for link in links {
                    if !theorem_by_name.contains_key(link.lean.artifact.as_str()) {
                        let drift = Drift {
                            topos_ref: Some(topos_ref.clone()),
                            lean_ref: Some(link.lean.clone()),
                            drift_type: DriftType::Missing,
                            description: format!(
                                "Link points to non-existent Lean theorem '{}'",
                                link.lean.artifact
                            ),
                            severity: 5,
                            details: None,
                        };

                        let drift_idx = report.drifts.len();
                        report.add_drift(drift);

                        report.add_suggestion(SyncSuggestion::new(
                            drift_idx,
                            SuggestedAction::GenerateLean,
                            format!(
                                "Generate Lean theorem '{}' or update link",
                                link.lean.artifact
                            ),
                            0.8,
                        ));
                    }
                }
            }
        }

        // Check for extra theorems
        for theorem in theorems {
            if !linked_theorem_names.contains(&theorem.name) {
                // Skip theorems that look like proofs of invariants (not behavior specs)
                if theorem.name.ends_with("_inv") || theorem.name.contains("invariant") {
                    continue;
                }

                let lean_ref = LeanRef::new(&theorem.source_file, &theorem.name);

                // Check for matching behavior
                let base_name = theorem.name.strip_suffix("_spec").unwrap_or(&theorem.name);
                let has_behavior = behaviors.iter().any(|b| b.name == base_name);

                if has_behavior {
                    continue; // Handled in missing link case
                }

                let drift = Drift::missing_topos(
                    lean_ref,
                    format!(
                        "Lean theorem '{}' has no corresponding Topos behavior",
                        theorem.name
                    ),
                )
                .with_severity(2);

                let drift_idx = report.drifts.len();
                report.add_drift(drift);

                report.add_suggestion(SyncSuggestion::new(
                    drift_idx,
                    SuggestedAction::UpdateTopos,
                    format!("Consider adding Topos behavior for '{}'", theorem.name),
                    0.6,
                ));
            }
        }

        report
    }

    /// Compare fields between a Topos concept and Lean structure.
    fn compare_fields(
        &self,
        topos_fields: &[ToposField],
        lean_fields: &[LeanField],
        concept: &ToposConcept,
        structure: &LeanStructure,
    ) -> Vec<(Drift, Option<SyncSuggestion>)> {
        let mut results = Vec::new();

        let topos_by_name: HashMap<&str, &ToposField> =
            topos_fields.iter().map(|f| (f.name.as_str(), f)).collect();
        let lean_by_name: HashMap<&str, &LeanField> =
            lean_fields.iter().map(|f| (f.name.as_str(), f)).collect();

        let topos_ref = ToposRef::new(&concept.source_file, &concept.name);
        let lean_ref = LeanRef::new(&structure.source_file, &structure.name);

        // Check Topos fields
        for topos_field in topos_fields {
            if let Some(lean_field) = lean_by_name.get(topos_field.name.as_str()) {
                // Both exist - check types
                if !self.types_compatible(&topos_field.field_type, &lean_field.field_type) {
                    let details = DriftDetails::new().with_type_mismatch(TypeMismatch {
                        context: format!("field '{}'", topos_field.name),
                        topos_type: topos_field.field_type.clone(),
                        lean_type: lean_field.field_type.clone(),
                    });

                    let drift = Drift::structural(
                        topos_ref.clone(),
                        lean_ref.clone(),
                        format!(
                            "Field '{}' type mismatch: Topos '{}' vs Lean '{}'",
                            topos_field.name, topos_field.field_type, lean_field.field_type
                        ),
                    )
                    .with_details(details);

                    let suggestion = SyncSuggestion::new(
                        0, // Will be updated
                        SuggestedAction::UpdateType,
                        format!(
                            "Update type of field '{}' to match",
                            topos_field.name
                        ),
                        0.7,
                    );

                    results.push((drift, Some(suggestion)));
                }
            } else {
                // Field only in Topos
                let details = DriftDetails::new().with_field_diff(FieldDiff {
                    topos_name: Some(topos_field.name.clone()),
                    lean_name: None,
                    topos_type: Some(topos_field.field_type.clone()),
                    lean_type: None,
                    kind: FieldDiffKind::OnlyInTopos,
                });

                let drift = Drift::structural(
                    topos_ref.clone(),
                    lean_ref.clone(),
                    format!(
                        "Field '{}' exists in Topos but not in Lean structure",
                        topos_field.name
                    ),
                )
                .with_details(details)
                .with_severity(3);

                let suggestion = SyncSuggestion::new(
                    0,
                    SuggestedAction::AddField,
                    format!("Add field '{}' to Lean structure", topos_field.name),
                    0.85,
                );

                results.push((drift, Some(suggestion)));
            }
        }

        // Check Lean fields not in Topos
        for lean_field in lean_fields {
            if !topos_by_name.contains_key(lean_field.name.as_str()) {
                let details = DriftDetails::new().with_field_diff(FieldDiff {
                    topos_name: None,
                    lean_name: Some(lean_field.name.clone()),
                    topos_type: None,
                    lean_type: Some(lean_field.field_type.clone()),
                    kind: FieldDiffKind::OnlyInLean,
                });

                let drift = Drift {
                    topos_ref: Some(topos_ref.clone()),
                    lean_ref: Some(lean_ref.clone()),
                    drift_type: DriftType::Extra,
                    description: format!(
                        "Field '{}' exists in Lean but not in Topos concept",
                        lean_field.name
                    ),
                    severity: 2,
                    details: Some(details),
                };

                let suggestion = SyncSuggestion::new(
                    0,
                    SuggestedAction::UpdateTopos,
                    format!("Add field '{}' to Topos concept", lean_field.name),
                    0.7,
                );

                results.push((drift, Some(suggestion)));
            }
        }

        results
    }

    /// Check if Topos and Lean types are compatible.
    fn types_compatible(&self, topos_type: &str, lean_type: &str) -> bool {
        let normalized_topos = self.normalize_topos_type(topos_type);
        let normalized_lean = self.normalize_lean_type(lean_type);

        normalized_topos == normalized_lean
    }

    /// Normalize a Topos type for comparison.
    fn normalize_topos_type(&self, ty: &str) -> String {
        let ty = ty.trim().trim_matches('`');

        // Handle "list of X" -> "List X"
        if let Some(inner) = ty.strip_prefix("list of ") {
            let inner_normalized = self.normalize_topos_type(inner);
            return format!("List {}", inner_normalized);
        }

        // Handle "optional X" -> "Option X"
        if let Some(inner) = ty.strip_prefix("optional ") {
            let inner_normalized = self.normalize_topos_type(inner);
            return format!("Option {}", inner_normalized);
        }

        // Check direct mappings
        if let Some(lean_type) = self.type_mappings.get(ty) {
            return lean_type.clone();
        }

        // PascalCase assumption for custom types
        ty.to_string()
    }

    /// Normalize a Lean type for comparison.
    fn normalize_lean_type(&self, ty: &str) -> String {
        let ty = ty.trim();

        // Handle List X -> List X (already normalized)
        // Handle Option X -> Option X (already normalized)

        ty.to_string()
    }

    /// Detect all drifts from index and parsed elements.
    pub fn detect_all(
        &self,
        concepts: &[ToposConcept],
        behaviors: &[ToposBehavior],
        structures: &[LeanStructure],
        theorems: &[LeanTheorem],
        index: &LinkIndex,
    ) -> DriftReport {
        let concept_report = self.detect_concept_drift(concepts, structures, index);
        let behavior_report = self.detect_behavior_drift(behaviors, theorems, index);

        // Merge reports
        let mut merged = DriftReport::new();

        for drift in concept_report.drifts {
            merged.add_drift(drift);
        }
        for suggestion in concept_report.suggestions {
            // Adjust indices (they're relative to concept_report)
            merged.add_suggestion(suggestion);
        }

        let concept_drift_count = merged.drifts.len();

        for drift in behavior_report.drifts {
            merged.add_drift(drift);
        }
        for mut suggestion in behavior_report.suggestions {
            // Adjust indices to account for concept drifts
            suggestion.drift_index += concept_drift_count;
            merged.add_suggestion(suggestion);
        }

        merged
    }
}

impl Default for DriftDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse Topos concepts from content (simplified parser for sync purposes).
pub fn parse_topos_concepts(content: &str, file: &Path) -> Vec<ToposConcept> {
    let mut concepts = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // Look for "Concept Name:"
        if line.starts_with("Concept ") && line.ends_with(':') {
            if let Some(name) = line.strip_prefix("Concept ").and_then(|s| s.strip_suffix(':')) {
                let name = name.trim().to_string();
                let start_line = i as u32 + 1;

                // Parse fields until next block or end
                let mut fields = Vec::new();
                let mut invariants = Vec::new();
                let mut doc = None;

                i += 1;
                while i < lines.len() {
                    let field_line = lines[i].trim();

                    // Stop at next top-level definition
                    if field_line.starts_with("Concept ")
                        || field_line.starts_with("Behavior ")
                        || field_line.starts_with("Invariant ")
                        || field_line.starts_with("##")
                    {
                        break;
                    }

                    // Parse field: "name: type" or "name: `type`"
                    if let Some((field_name, field_type)) = parse_field_line(field_line) {
                        fields.push(ToposField {
                            name: field_name,
                            field_type,
                            description: None,
                            constraints: Vec::new(),
                        });
                    }

                    // Parse inline invariant
                    if field_line.starts_with("invariant:") {
                        if let Some(inv) = field_line.strip_prefix("invariant:") {
                            invariants.push(ToposInvariant {
                                name: format!("{}_inv_{}", name, invariants.len()),
                                expression: inv.trim().to_string(),
                                field: None,
                            });
                        }
                    }

                    // Look for @doc annotation
                    if field_line.starts_with("@doc:") || field_line.starts_with("description:") {
                        let prefix = if field_line.starts_with("@doc:") {
                            "@doc:"
                        } else {
                            "description:"
                        };
                        doc = field_line.strip_prefix(prefix).map(|s| s.trim().to_string());
                    }

                    i += 1;
                }

                concepts.push(ToposConcept {
                    name,
                    fields,
                    invariants,
                    doc,
                    source_file: file.to_path_buf(),
                    line: start_line,
                });

                continue; // Don't increment i again
            }
        }

        i += 1;
    }

    concepts
}

/// Parse Topos behaviors from content.
pub fn parse_topos_behaviors(content: &str, file: &Path) -> Vec<ToposBehavior> {
    let mut behaviors = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // Look for "Behavior name:"
        if line.starts_with("Behavior ") && line.ends_with(':') {
            if let Some(name) = line.strip_prefix("Behavior ").and_then(|s| s.strip_suffix(':')) {
                let name = name.trim().to_string();
                let start_line = i as u32 + 1;

                let mut inputs = Vec::new();
                let mut returns = None;
                let mut preconditions = Vec::new();
                let mut postconditions = Vec::new();
                let doc = None;

                i += 1;
                while i < lines.len() {
                    let field_line = lines[i].trim();

                    // Stop at next top-level definition
                    if field_line.starts_with("Concept ")
                        || field_line.starts_with("Behavior ")
                        || field_line.starts_with("##")
                    {
                        break;
                    }

                    // Parse "given:" or "input:"
                    if field_line.starts_with("given:") || field_line.starts_with("input:") {
                        let prefix = if field_line.starts_with("given:") {
                            "given:"
                        } else {
                            "input:"
                        };
                        if let Some(rest) = field_line.strip_prefix(prefix) {
                            // Parse "name (Type)" or "name: Type"
                            if let Some((name, ty)) = parse_param(rest.trim()) {
                                inputs.push(ToposField {
                                    name,
                                    field_type: ty,
                                    description: None,
                                    constraints: Vec::new(),
                                });
                            }
                        }
                    }

                    // Parse "returns:"
                    if field_line.starts_with("returns:") {
                        if let Some(rest) = field_line.strip_prefix("returns:") {
                            returns = Some(rest.trim().trim_matches('`').to_string());
                        }
                    }

                    // Parse preconditions
                    if field_line.starts_with("pre:") || field_line.starts_with("requires:") {
                        let prefix = if field_line.starts_with("pre:") {
                            "pre:"
                        } else {
                            "requires:"
                        };
                        if let Some(rest) = field_line.strip_prefix(prefix) {
                            preconditions.push(rest.trim().to_string());
                        }
                    }

                    // Parse postconditions
                    if field_line.starts_with("post:") || field_line.starts_with("ensures:") {
                        let prefix = if field_line.starts_with("post:") {
                            "post:"
                        } else {
                            "ensures:"
                        };
                        if let Some(rest) = field_line.strip_prefix(prefix) {
                            postconditions.push(rest.trim().to_string());
                        }
                    }

                    i += 1;
                }

                behaviors.push(ToposBehavior {
                    name,
                    inputs,
                    returns,
                    preconditions,
                    postconditions,
                    doc,
                    source_file: file.to_path_buf(),
                    line: start_line,
                });

                continue;
            }
        }

        i += 1;
    }

    behaviors
}

/// Parse Lean structures from content.
pub fn parse_lean_structures(content: &str, file: &Path) -> Vec<LeanStructure> {
    let mut structures = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut current_namespace: Option<String> = None;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // Track namespace
        if line.starts_with("namespace ") {
            current_namespace = line.strip_prefix("namespace ").map(|s| s.trim().to_string());
        } else if line == "end" || line.starts_with("end ") {
            current_namespace = None;
        }

        // Look for "structure Name"
        if line.starts_with("structure ") {
            // Extract name (handle "structure Name where" or "structure Name extends ...")
            let rest = line.strip_prefix("structure ").unwrap();
            let name = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();

            if name.is_empty() {
                i += 1;
                continue;
            }

            let start_line = i as u32 + 1;
            let mut fields = Vec::new();
            let mut doc = None;

            // Look for doc comment above
            if i > 0 {
                let prev_line = lines[i - 1].trim();
                if prev_line.ends_with("-/") {
                    // Multi-line doc comment - find start
                    let mut j = i - 1;
                    while j > 0 && !lines[j].trim().starts_with("/--") {
                        j -= 1;
                    }
                    if lines[j].trim().starts_with("/--") {
                        let doc_lines: Vec<&str> = lines[j..i]
                            .iter()
                            .map(|l| {
                                l.trim()
                                    .trim_start_matches("/--")
                                    .trim_end_matches("-/")
                                    .trim()
                            })
                            .filter(|l| !l.is_empty() && !l.starts_with("@"))
                            .collect();
                        doc = Some(doc_lines.join(" "));
                    }
                }
            }

            // Parse fields
            i += 1;
            while i < lines.len() {
                let field_line = lines[i].trim();

                // Stop at empty line or next definition
                if field_line.is_empty()
                    || field_line.starts_with("structure ")
                    || field_line.starts_with("def ")
                    || field_line.starts_with("theorem ")
                    || field_line.starts_with("lemma ")
                    || field_line.starts_with("namespace ")
                    || field_line == "end"
                    || field_line.starts_with("end ")
                {
                    break;
                }

                // Parse "name : Type" or "name : Type := default"
                if let Some((field_name, field_type, default)) = parse_lean_field(field_line) {
                    fields.push(LeanField {
                        name: field_name,
                        field_type,
                        default_value: default,
                    });
                }

                i += 1;
            }

            structures.push(LeanStructure {
                name,
                fields,
                namespace: current_namespace.clone(),
                doc,
                source_file: file.to_path_buf(),
                line: start_line,
            });

            continue;
        }

        i += 1;
    }

    structures
}

/// Parse Lean theorems from content.
pub fn parse_lean_theorems(content: &str, file: &Path) -> Vec<LeanTheorem> {
    let mut theorems = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    let mut current_namespace: Option<String> = None;

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // Track namespace
        if line.starts_with("namespace ") {
            current_namespace = line.strip_prefix("namespace ").map(|s| s.trim().to_string());
        } else if line == "end" || line.starts_with("end ") {
            current_namespace = None;
        }

        // Look for "theorem name" or "lemma name"
        let is_theorem = line.starts_with("theorem ");
        let is_lemma = line.starts_with("lemma ");

        if is_theorem || is_lemma {
            let prefix = if is_theorem { "theorem " } else { "lemma " };
            let rest = line.strip_prefix(prefix).unwrap();

            // Extract name (before : or parameters)
            let name = rest
                .split(|c: char| c == ':' || c == '(' || c.is_whitespace())
                .next()
                .unwrap_or("")
                .trim()
                .to_string();

            if name.is_empty() {
                i += 1;
                continue;
            }

            let start_line = i as u32 + 1;

            // Extract type/statement (everything after :)
            let statement = if let Some(colon_pos) = rest.find(':') {
                let after_colon = &rest[colon_pos + 1..];
                // Take until := or end of line
                if let Some(assign_pos) = after_colon.find(":=") {
                    after_colon[..assign_pos].trim().to_string()
                } else {
                    after_colon.trim().to_string()
                }
            } else {
                String::new()
            };

            // Check if it has a proof or uses sorry
            let has_proof = !content[i..].contains("sorry");

            let mut doc = None;
            // Look for doc comment
            if i > 0 {
                let prev_line = lines[i - 1].trim();
                if prev_line.ends_with("-/") || prev_line.starts_with("--") {
                    // Simple doc extraction
                    doc = Some(
                        prev_line
                            .trim_start_matches("--")
                            .trim_end_matches("-/")
                            .trim()
                            .to_string(),
                    );
                }
            }

            theorems.push(LeanTheorem {
                name,
                statement,
                namespace: current_namespace.clone(),
                doc,
                has_proof,
                source_file: file.to_path_buf(),
                line: start_line,
            });
        }

        i += 1;
    }

    theorems
}

/// Parse a field line like "name: `Type`" or "name: Type".
fn parse_field_line(line: &str) -> Option<(String, String)> {
    // Skip empty lines, comments, annotations
    if line.is_empty()
        || line.starts_with("//")
        || line.starts_with("@")
        || line.starts_with("invariant:")
        || line.starts_with("description:")
    {
        return None;
    }

    // Parse "name: type" pattern
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() == 2 {
        let name = parts[0].trim().to_string();
        let ty = parts[1].trim().trim_matches('`').to_string();
        if !name.is_empty() && !ty.is_empty() {
            return Some((name, ty));
        }
    }

    None
}

/// Parse a parameter like "name (Type)" or "name: Type".
fn parse_param(s: &str) -> Option<(String, String)> {
    let s = s.trim();

    // Handle "name (Type)"
    if let Some(paren_start) = s.find('(') {
        let name = s[..paren_start].trim().to_string();
        let ty = s[paren_start..]
            .trim_matches(|c| c == '(' || c == ')' || c == '`')
            .trim()
            .to_string();
        if !name.is_empty() && !ty.is_empty() {
            return Some((name, ty));
        }
    }

    // Handle "name: Type"
    parse_field_line(s)
}

/// Parse a Lean field line like "name : Type" or "name : Type := default".
fn parse_lean_field(line: &str) -> Option<(String, String, Option<String>)> {
    let line = line.trim();

    // Skip non-field lines
    if line.starts_with("--")
        || line.starts_with("/")
        || line.starts_with("@")
        || line == "where"
        || line.is_empty()
    {
        return None;
    }

    // Parse "name : Type" or "name : Type := default"
    let parts: Vec<&str> = line.splitn(2, ':').collect();
    if parts.len() == 2 {
        let name = parts[0].trim().to_string();
        let rest = parts[1].trim();

        // Check for default value
        if let Some(assign_pos) = rest.find(":=") {
            let ty = rest[..assign_pos].trim().to_string();
            let default = rest[assign_pos + 2..].trim().to_string();
            return Some((name, ty, Some(default)));
        } else {
            return Some((name, rest.to_string(), None));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use crate::topos::{Link, LinkType, LinkSource, LeanRef, ToposRef};

    #[test]
    fn test_parse_topos_concepts() {
        let content = r#"
Concept Order:
  id: `OrderId`
  items: list of `OrderItem`
  status: `OrderStatus`
  @lean: specs/Order.lean#Order
"#;

        let concepts = parse_topos_concepts(content, Path::new("order.tps"));
        assert_eq!(concepts.len(), 1);
        assert_eq!(concepts[0].name, "Order");
        assert_eq!(concepts[0].fields.len(), 3);
        assert_eq!(concepts[0].fields[0].name, "id");
        assert_eq!(concepts[0].fields[0].field_type, "OrderId");
    }

    #[test]
    fn test_parse_topos_behaviors() {
        let content = r#"
Behavior create_order:
  given: request (`OrderRequest`)
  returns: `Order`
  pre: request.items is not empty
  post: order.status == pending
"#;

        let behaviors = parse_topos_behaviors(content, Path::new("order.tps"));
        assert_eq!(behaviors.len(), 1);
        assert_eq!(behaviors[0].name, "create_order");
        assert_eq!(behaviors[0].inputs.len(), 1);
        assert_eq!(behaviors[0].returns, Some("Order".to_string()));
        assert_eq!(behaviors[0].preconditions.len(), 1);
        assert_eq!(behaviors[0].postconditions.len(), 1);
    }

    #[test]
    fn test_parse_lean_structures() {
        let content = r#"
/-- Order represents a customer order. -/
structure Order where
  id : Nat
  items : List OrderItem
  status : OrderStatus
"#;

        let structures = parse_lean_structures(content, Path::new("Order.lean"));
        assert_eq!(structures.len(), 1);
        assert_eq!(structures[0].name, "Order");
        assert_eq!(structures[0].fields.len(), 3);
        assert_eq!(structures[0].fields[0].name, "id");
        assert_eq!(structures[0].fields[0].field_type, "Nat");
    }

    #[test]
    fn test_parse_lean_theorems() {
        let content = r#"
theorem create_order_spec (req : OrderRequest) : Order :=
  sorry
"#;

        let theorems = parse_lean_theorems(content, Path::new("Order.lean"));
        assert_eq!(theorems.len(), 1);
        assert_eq!(theorems[0].name, "create_order_spec");
    }

    #[test]
    fn test_drift_detector_type_compatibility() {
        let detector = DriftDetector::new();

        assert!(detector.types_compatible("string", "String"));
        assert!(detector.types_compatible("int", "Int"));
        assert!(detector.types_compatible("list of `Item`", "List Item"));
        assert!(!detector.types_compatible("string", "Int"));
    }

    #[test]
    fn test_detect_concept_drift_missing_structure() {
        let detector = DriftDetector::new();
        let index = LinkIndex::new();

        let concepts = vec![ToposConcept {
            name: "Order".to_string(),
            fields: vec![],
            invariants: vec![],
            doc: None,
            source_file: PathBuf::from("order.tps"),
            line: 1,
        }];

        let structures: Vec<LeanStructure> = vec![];

        let report = detector.detect_concept_drift(&concepts, &structures, &index);

        assert!(report.has_drifts());
        assert_eq!(report.drifts.len(), 1);
        assert_eq!(report.drifts[0].drift_type, DriftType::Missing);
    }

    #[test]
    fn test_field_diff_detection() {
        let detector = DriftDetector::new();
        let mut index = LinkIndex::new();

        // Add a link
        let topos_ref = ToposRef::new("order.tps", "Order");
        let lean_ref = LeanRef::new("Order.lean", "Order");
        let link = Link::new(
            topos_ref.clone(),
            lean_ref.clone(),
            LinkType::Structure,
            crate::topos::LinkSource::Topos,
        );
        index.add_link(link);

        let concepts = vec![ToposConcept {
            name: "Order".to_string(),
            fields: vec![
                ToposField {
                    name: "id".to_string(),
                    field_type: "nat".to_string(),
                    description: None,
                    constraints: vec![],
                },
                ToposField {
                    name: "name".to_string(),
                    field_type: "string".to_string(),
                    description: None,
                    constraints: vec![],
                },
            ],
            invariants: vec![],
            doc: None,
            source_file: PathBuf::from("order.tps"),
            line: 1,
        }];

        let structures = vec![LeanStructure {
            name: "Order".to_string(),
            fields: vec![
                LeanField {
                    name: "id".to_string(),
                    field_type: "Nat".to_string(),
                    default_value: None,
                },
                LeanField {
                    name: "status".to_string(),
                    field_type: "Status".to_string(),
                    default_value: None,
                },
            ],
            namespace: None,
            doc: None,
            source_file: PathBuf::from("Order.lean"),
            line: 1,
        }];

        let report = detector.detect_concept_drift(&concepts, &structures, &index);

        // Should detect: name only in Topos, status only in Lean
        assert!(report.has_drifts());
        assert!(report.drifts.len() >= 2);
    }
}
