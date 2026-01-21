//! Dual-Track Sync Engine for Topos-Lean synchronization.
//!
//! This module provides the main `DualTrackSync` struct that orchestrates
//! bidirectional synchronization between Topos specifications and Lean
//! formalizations.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::topos::{LeanRef, Link, LinkIndex, LinkSource, LinkType};

use super::drift::{
    parse_lean_structures, parse_lean_theorems, parse_topos_behaviors, parse_topos_concepts,
    DriftDetector,
};
use super::generators::{LeanGenerator, ToposGenerator};
use super::types::{
    DriftReport, FormalizationLevel, LeanStructure, LeanTheorem, SuggestedAction, SyncConfig,
    SyncDirection, SyncResult, ToposBehavior, ToposConcept,
};

/// Dual-track synchronization engine for Topos and Lean.
pub struct DualTrackSync {
    /// Root directory for Topos specifications.
    topos_root: PathBuf,
    /// Root directory for Lean files.
    lean_root: PathBuf,
    /// Bidirectional link index.
    link_index: LinkIndex,
    /// Current formalization level.
    formalization_level: FormalizationLevel,
    /// Sync configuration.
    config: SyncConfig,
    /// Drift detector.
    drift_detector: DriftDetector,
    /// Lean code generator.
    lean_generator: LeanGenerator,
    /// Topos code generator.
    topos_generator: ToposGenerator,
    /// Cached Topos concepts.
    topos_concepts: Vec<ToposConcept>,
    /// Cached Topos behaviors.
    topos_behaviors: Vec<ToposBehavior>,
    /// Cached Lean structures.
    lean_structures: Vec<LeanStructure>,
    /// Cached Lean theorems.
    lean_theorems: Vec<LeanTheorem>,
}

impl DualTrackSync {
    /// Create a new sync engine.
    pub fn new(topos_root: PathBuf, lean_root: PathBuf) -> Self {
        Self {
            topos_root: topos_root.clone(),
            lean_root: lean_root.clone(),
            link_index: LinkIndex::with_project_root(&topos_root),
            formalization_level: FormalizationLevel::Types,
            config: SyncConfig::default(),
            drift_detector: DriftDetector::new(),
            lean_generator: LeanGenerator::new(FormalizationLevel::Types),
            topos_generator: ToposGenerator::new(),
            topos_concepts: Vec::new(),
            topos_behaviors: Vec::new(),
            lean_structures: Vec::new(),
            lean_theorems: Vec::new(),
        }
    }

    /// Create with a specific formalization level.
    pub fn with_level(mut self, level: FormalizationLevel) -> Self {
        self.formalization_level = level;
        self.lean_generator = LeanGenerator::new(level);
        self
    }

    /// Create with custom configuration.
    pub fn with_config(mut self, config: SyncConfig) -> Self {
        self.formalization_level = config.formalization_level;
        self.lean_generator = LeanGenerator::new(config.formalization_level);
        self.config = config;
        self
    }

    /// Create with an existing link index.
    pub fn with_index(mut self, index: LinkIndex) -> Self {
        self.link_index = index;
        self
    }

    /// Get the Topos root directory.
    pub fn topos_root(&self) -> &Path {
        &self.topos_root
    }

    /// Get the Lean root directory.
    pub fn lean_root(&self) -> &Path {
        &self.lean_root
    }

    /// Get the current formalization level.
    pub fn formalization_level(&self) -> FormalizationLevel {
        self.formalization_level
    }

    /// Set the formalization level.
    pub fn set_formalization_level(&mut self, level: FormalizationLevel) {
        self.formalization_level = level;
        self.lean_generator = LeanGenerator::new(level);
    }

    /// Get a reference to the link index.
    pub fn link_index(&self) -> &LinkIndex {
        &self.link_index
    }

    /// Get a mutable reference to the link index.
    pub fn link_index_mut(&mut self) -> &mut LinkIndex {
        &mut self.link_index
    }

    /// Scan and parse all Topos and Lean files.
    pub async fn scan(&mut self) -> Result<()> {
        self.scan_topos_files()?;
        self.scan_lean_files()?;
        self.build_link_index()?;
        Ok(())
    }

    /// Scan Topos files and parse concepts/behaviors.
    fn scan_topos_files(&mut self) -> Result<()> {
        self.topos_concepts.clear();
        self.topos_behaviors.clear();

        let pattern = self.topos_root.join("**/*.tps");
        let pattern_str = pattern.to_str().unwrap_or("");

        if let Ok(entries) = glob::glob(pattern_str) {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(&entry) {
                    let rel_path = entry
                        .strip_prefix(&self.topos_root)
                        .unwrap_or(&entry);

                    let concepts = parse_topos_concepts(&content, rel_path);
                    let behaviors = parse_topos_behaviors(&content, rel_path);

                    self.topos_concepts.extend(concepts);
                    self.topos_behaviors.extend(behaviors);
                }
            }
        }

        // Also check .topos extension
        let pattern = self.topos_root.join("**/*.topos");
        let pattern_str = pattern.to_str().unwrap_or("");

        if let Ok(entries) = glob::glob(pattern_str) {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(&entry) {
                    let rel_path = entry
                        .strip_prefix(&self.topos_root)
                        .unwrap_or(&entry);

                    let concepts = parse_topos_concepts(&content, rel_path);
                    let behaviors = parse_topos_behaviors(&content, rel_path);

                    self.topos_concepts.extend(concepts);
                    self.topos_behaviors.extend(behaviors);
                }
            }
        }

        Ok(())
    }

    /// Scan Lean files and parse structures/theorems.
    fn scan_lean_files(&mut self) -> Result<()> {
        self.lean_structures.clear();
        self.lean_theorems.clear();

        let pattern = self.lean_root.join("**/*.lean");
        let pattern_str = pattern.to_str().unwrap_or("");

        if let Ok(entries) = glob::glob(pattern_str) {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(&entry) {
                    let rel_path = entry
                        .strip_prefix(&self.lean_root)
                        .unwrap_or(&entry);

                    let structures = parse_lean_structures(&content, rel_path);
                    let theorems = parse_lean_theorems(&content, rel_path);

                    self.lean_structures.extend(structures);
                    self.lean_theorems.extend(theorems);
                }
            }
        }

        Ok(())
    }

    /// Build the link index from scanned files.
    fn build_link_index(&mut self) -> Result<()> {
        self.link_index.clear();

        // Index Topos files
        let pattern = self.topos_root.join("**/*.tps");
        let pattern_str = pattern.to_str().unwrap_or("");

        if let Ok(entries) = glob::glob(pattern_str) {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(&entry) {
                    let rel_path = entry
                        .strip_prefix(&self.topos_root)
                        .unwrap_or(&entry);
                    let _ = self.link_index.index_topos_file(rel_path, &content);
                }
            }
        }

        // Index Lean files
        let pattern = self.lean_root.join("**/*.lean");
        let pattern_str = pattern.to_str().unwrap_or("");

        if let Ok(entries) = glob::glob(pattern_str) {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(&entry) {
                    let rel_path = entry
                        .strip_prefix(&self.lean_root)
                        .unwrap_or(&entry);
                    let _ = self.link_index.index_lean_file(rel_path, &content);
                }
            }
        }

        self.link_index.touch();
        Ok(())
    }

    /// Detect drift between Topos and Lean specifications.
    pub async fn detect_drift(&self) -> Result<DriftReport> {
        let report = self.drift_detector.detect_all(
            &self.topos_concepts,
            &self.topos_behaviors,
            &self.lean_structures,
            &self.lean_theorems,
            &self.link_index,
        );

        Ok(report)
    }

    /// Sync Topos specifications to Lean (generate Lean from Topos).
    pub async fn sync_topos_to_lean(&mut self) -> Result<SyncResult> {
        let drift_report = self.detect_drift().await?;
        let mut result = SyncResult::success(SyncDirection::ToposToLean);

        // Process each drift that suggests generating Lean
        for suggestion in drift_report.suggestions.iter() {
            if suggestion.action != SuggestedAction::GenerateLean {
                continue;
            }

            // Skip if severity exceeds auto-resolve threshold
            let drift = &drift_report.drifts[suggestion.drift_index];
            if drift.severity > self.config.auto_resolve_max_severity && self.config.require_confirmation {
                result = result.with_remaining_drift(drift.clone());
                continue;
            }

            // Generate Lean code based on drift type
            if let Some(ref topos_ref) = drift.topos_ref {
                // Find the corresponding concept or behavior
                if let Some(concept) = self.find_concept(&topos_ref.element) {
                    let lean_code = self.lean_generator.generate_structure(concept);
                    let lean_file = self.lean_root.join(format!("{}.lean", concept.name));

                    if self.config.auto_generate_lean {
                        // Check if file already exists
                        if lean_file.exists() {
                            // Append to existing file
                            let existing = fs::read_to_string(&lean_file)
                                .map_err(|e| Error::Internal(format!("Failed to read {}: {}", lean_file.display(), e)))?;
                            let new_content = format!("{}\n\n{}", existing, lean_code);
                            fs::write(&lean_file, new_content)
                                .map_err(|e| Error::Internal(format!("Failed to write {}: {}", lean_file.display(), e)))?;
                            result = result.with_modified(lean_file.clone());
                        } else {
                            // Create new file
                            fs::write(&lean_file, &lean_code)
                                .map_err(|e| Error::Internal(format!("Failed to write {}: {}", lean_file.display(), e)))?;
                            result = result.with_created(lean_file.clone());
                        }

                        // Add link to index
                        let lean_ref = LeanRef::new(
                            lean_file.strip_prefix(&self.lean_root).unwrap_or(&lean_file),
                            &concept.name,
                        );
                        let link = Link::new(
                            topos_ref.clone(),
                            lean_ref,
                            LinkType::Structure,
                            LinkSource::Synthesized,
                        );
                        self.link_index.add_link(link);
                        result.links_added += 1;
                        result.drifts_resolved += 1;
                    }
                } else if let Some(behavior) = self.find_behavior(&topos_ref.element) {
                    let lean_code = self.lean_generator.generate_theorem(behavior);
                    let lean_file = self
                        .lean_root
                        .join(format!("{}_spec.lean", behavior.name));

                    if self.config.auto_generate_lean {
                        if lean_file.exists() {
                            let existing = fs::read_to_string(&lean_file)
                                .map_err(|e| Error::Internal(format!("Failed to read {}: {}", lean_file.display(), e)))?;
                            let new_content = format!("{}\n\n{}", existing, lean_code);
                            fs::write(&lean_file, new_content)
                                .map_err(|e| Error::Internal(format!("Failed to write {}: {}", lean_file.display(), e)))?;
                            result = result.with_modified(lean_file.clone());
                        } else {
                            fs::write(&lean_file, &lean_code)
                                .map_err(|e| Error::Internal(format!("Failed to write {}: {}", lean_file.display(), e)))?;
                            result = result.with_created(lean_file.clone());
                        }

                        let lean_ref = LeanRef::new(
                            lean_file.strip_prefix(&self.lean_root).unwrap_or(&lean_file),
                            format!("{}_spec", behavior.name),
                        );
                        let link = Link::new(
                            topos_ref.clone(),
                            lean_ref,
                            LinkType::FunctionSpec,
                            LinkSource::Synthesized,
                        );
                        self.link_index.add_link(link);
                        result.links_added += 1;
                        result.drifts_resolved += 1;
                    }
                }
            }
        }

        // Handle suggestions for adding links
        for suggestion in &drift_report.suggestions {
            if suggestion.action != SuggestedAction::AddLink {
                continue;
            }

            let drift = &drift_report.drifts[suggestion.drift_index];
            if let (Some(topos_ref), Some(lean_ref)) = (&drift.topos_ref, &drift.lean_ref) {
                if self.config.auto_link {
                    let link = Link::new(
                        topos_ref.clone(),
                        lean_ref.clone(),
                        LinkType::Structure, // Default, should be inferred
                        LinkSource::Synthesized,
                    );
                    self.link_index.add_link(link);
                    result.links_added += 1;
                    result.drifts_resolved += 1;
                }
            }
        }

        Ok(result)
    }

    /// Sync Lean artifacts to Topos (update Topos from Lean).
    pub async fn sync_lean_to_topos(&mut self) -> Result<SyncResult> {
        let drift_report = self.detect_drift().await?;
        let mut result = SyncResult::success(SyncDirection::LeanToTopos);

        // Process each drift that suggests updating Topos
        for suggestion in &drift_report.suggestions {
            if suggestion.action != SuggestedAction::UpdateTopos {
                continue;
            }

            let drift = &drift_report.drifts[suggestion.drift_index];
            if drift.severity > self.config.auto_resolve_max_severity && self.config.require_confirmation {
                result = result.with_remaining_drift(drift.clone());
                continue;
            }

            if let Some(ref lean_ref) = drift.lean_ref {
                // Find the corresponding structure or theorem
                if let Some(structure) = self.find_structure(&lean_ref.artifact) {
                    let topos_code = self.topos_generator.generate_concept(structure);

                    if self.config.auto_update_topos {
                        // Determine target file
                        let topos_file = self.topos_root.join(format!("{}.tps", structure.name.to_lowercase()));

                        if topos_file.exists() {
                            let existing = fs::read_to_string(&topos_file)
                                .map_err(|e| Error::Internal(format!("Failed to read {}: {}", topos_file.display(), e)))?;
                            let new_content = format!("{}\n\n{}", existing, topos_code);
                            fs::write(&topos_file, new_content)
                                .map_err(|e| Error::Internal(format!("Failed to write {}: {}", topos_file.display(), e)))?;
                            result = result.with_modified(topos_file.clone());
                        } else {
                            fs::write(&topos_file, &topos_code)
                                .map_err(|e| Error::Internal(format!("Failed to write {}: {}", topos_file.display(), e)))?;
                            result = result.with_created(topos_file.clone());
                        }

                        result.drifts_resolved += 1;
                    }
                }
            }
        }

        Ok(result)
    }

    /// Full bidirectional sync.
    pub async fn sync(&mut self, direction: SyncDirection) -> Result<SyncResult> {
        // Re-scan first
        self.scan().await?;

        match direction {
            SyncDirection::ToposToLean => self.sync_topos_to_lean().await,
            SyncDirection::LeanToTopos => self.sync_lean_to_topos().await,
            SyncDirection::Bidirectional => {
                // Run both syncs and merge results
                let topos_result = self.sync_topos_to_lean().await?;
                let lean_result = self.sync_lean_to_topos().await?;

                let mut merged = SyncResult::success(SyncDirection::Bidirectional);
                merged.files_created.extend(topos_result.files_created);
                merged.files_created.extend(lean_result.files_created);
                merged.files_modified.extend(topos_result.files_modified);
                merged.files_modified.extend(lean_result.files_modified);
                merged.links_added = topos_result.links_added + lean_result.links_added;
                merged.drifts_resolved = topos_result.drifts_resolved + lean_result.drifts_resolved;
                merged.remaining_drifts.extend(topos_result.remaining_drifts);
                merged.remaining_drifts.extend(lean_result.remaining_drifts);
                merged.errors.extend(topos_result.errors);
                merged.errors.extend(lean_result.errors);
                merged.warnings.extend(topos_result.warnings);
                merged.warnings.extend(lean_result.warnings);
                merged.success = topos_result.success && lean_result.success;

                Ok(merged)
            }
        }
    }

    /// Get cached Topos concepts.
    pub fn concepts(&self) -> &[ToposConcept] {
        &self.topos_concepts
    }

    /// Get cached Topos behaviors.
    pub fn behaviors(&self) -> &[ToposBehavior] {
        &self.topos_behaviors
    }

    /// Get cached Lean structures.
    pub fn structures(&self) -> &[LeanStructure] {
        &self.lean_structures
    }

    /// Get cached Lean theorems.
    pub fn theorems(&self) -> &[LeanTheorem] {
        &self.lean_theorems
    }

    /// Find a concept by name.
    fn find_concept(&self, name: &str) -> Option<&ToposConcept> {
        self.topos_concepts.iter().find(|c| c.name == name)
    }

    /// Find a behavior by name.
    fn find_behavior(&self, name: &str) -> Option<&ToposBehavior> {
        self.topos_behaviors.iter().find(|b| b.name == name)
    }

    /// Find a structure by name.
    fn find_structure(&self, name: &str) -> Option<&LeanStructure> {
        self.lean_structures.iter().find(|s| s.name == name)
    }

    /// Find a theorem by name.
    #[allow(dead_code)] // Public API for external consumers
    fn find_theorem(&self, name: &str) -> Option<&LeanTheorem> {
        self.lean_theorems.iter().find(|t| t.name == name)
    }

    /// Save the link index to a file.
    pub fn save_index(&self, path: &Path) -> Result<()> {
        self.link_index.save(path)
    }

    /// Load the link index from a file.
    pub fn load_index(&mut self, path: &Path) -> Result<()> {
        self.link_index = LinkIndex::load(path)?;
        Ok(())
    }

    /// Generate a sync report as a string.
    pub fn generate_report(&self, drift_report: &DriftReport) -> String {
        let mut lines = Vec::new();

        lines.push("# Dual-Track Sync Report".to_string());
        lines.push(String::new());

        // Summary
        lines.push("## Summary".to_string());
        lines.push(format!(
            "- Total drifts: {}",
            drift_report.summary.total
        ));
        lines.push(format!(
            "- Structural: {}",
            drift_report.summary.structural
        ));
        lines.push(format!(
            "- Semantic: {}",
            drift_report.summary.semantic
        ));
        lines.push(format!(
            "- Missing: {}",
            drift_report.summary.missing
        ));
        lines.push(format!(
            "- Extra: {}",
            drift_report.summary.extra
        ));
        lines.push(String::new());

        // Statistics
        lines.push("## Project Statistics".to_string());
        lines.push(format!(
            "- Topos concepts: {}",
            self.topos_concepts.len()
        ));
        lines.push(format!(
            "- Topos behaviors: {}",
            self.topos_behaviors.len()
        ));
        lines.push(format!(
            "- Lean structures: {}",
            self.lean_structures.len()
        ));
        lines.push(format!(
            "- Lean theorems: {}",
            self.lean_theorems.len()
        ));
        lines.push(format!(
            "- Links in index: {}",
            self.link_index.len()
        ));
        lines.push(String::new());

        // Drifts
        if !drift_report.drifts.is_empty() {
            lines.push("## Drifts".to_string());
            lines.push(String::new());

            for (i, drift) in drift_report.drifts.iter().enumerate() {
                lines.push(format!(
                    "### {}. {} (severity: {})",
                    i + 1,
                    drift.drift_type,
                    drift.severity
                ));
                lines.push(format!("- {}", drift.description));
                if let Some(ref topos_ref) = drift.topos_ref {
                    lines.push(format!("- Topos: {}", topos_ref));
                }
                if let Some(ref lean_ref) = drift.lean_ref {
                    lines.push(format!("- Lean: {}", lean_ref));
                }
                lines.push(String::new());
            }
        }

        // Suggestions
        if !drift_report.suggestions.is_empty() {
            lines.push("## Suggestions".to_string());
            lines.push(String::new());

            for suggestion in &drift_report.suggestions {
                lines.push(format!(
                    "- **{}**: {} (confidence: {:.0}%)",
                    suggestion.action.description(),
                    suggestion.description,
                    suggestion.confidence * 100.0
                ));
            }
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::types::Drift;
    use crate::topos::{Link, LinkType, LinkSource, LeanRef, ToposRef};
    use tempfile::TempDir;

    fn setup_test_dirs() -> (TempDir, PathBuf, PathBuf) {
        let temp = TempDir::new().unwrap();
        let topos_dir = temp.path().join("topos");
        let lean_dir = temp.path().join("lean");
        fs::create_dir_all(&topos_dir).unwrap();
        fs::create_dir_all(&lean_dir).unwrap();
        (temp, topos_dir, lean_dir)
    }

    #[test]
    fn test_dual_track_sync_creation() {
        let (_temp, topos_dir, lean_dir) = setup_test_dirs();
        let sync = DualTrackSync::new(topos_dir.clone(), lean_dir.clone());

        assert_eq!(sync.topos_root(), &topos_dir);
        assert_eq!(sync.lean_root(), &lean_dir);
        assert_eq!(sync.formalization_level(), FormalizationLevel::Types);
    }

    #[test]
    fn test_set_formalization_level() {
        let (_temp, topos_dir, lean_dir) = setup_test_dirs();
        let mut sync = DualTrackSync::new(topos_dir, lean_dir);

        sync.set_formalization_level(FormalizationLevel::Contracts);
        assert_eq!(sync.formalization_level(), FormalizationLevel::Contracts);
    }

    #[tokio::test]
    async fn test_scan_empty_dirs() {
        let (_temp, topos_dir, lean_dir) = setup_test_dirs();
        let mut sync = DualTrackSync::new(topos_dir, lean_dir);

        sync.scan().await.unwrap();

        assert!(sync.concepts().is_empty());
        assert!(sync.behaviors().is_empty());
        assert!(sync.structures().is_empty());
        assert!(sync.theorems().is_empty());
    }

    #[tokio::test]
    async fn test_scan_with_files() {
        let (_temp, topos_dir, lean_dir) = setup_test_dirs();

        // Create a Topos file
        let topos_content = r#"
Concept Order:
  id: `nat`
  items: list of `OrderItem`

Behavior create_order:
  given: request (`OrderRequest`)
  returns: `Order`
"#;
        fs::write(topos_dir.join("order.tps"), topos_content).unwrap();

        // Create a Lean file
        let lean_content = r#"
structure Order where
  id : Nat
  items : List OrderItem

theorem create_order_spec (req : OrderRequest) : Order :=
  sorry
"#;
        fs::write(lean_dir.join("Order.lean"), lean_content).unwrap();

        let mut sync = DualTrackSync::new(topos_dir, lean_dir);
        sync.scan().await.unwrap();

        assert_eq!(sync.concepts().len(), 1);
        assert_eq!(sync.behaviors().len(), 1);
        assert_eq!(sync.structures().len(), 1);
        assert_eq!(sync.theorems().len(), 1);
    }

    #[tokio::test]
    async fn test_detect_drift_missing_link() {
        let (_temp, topos_dir, lean_dir) = setup_test_dirs();

        // Create Topos and Lean files with matching names but no links
        let topos_content = r#"
Concept Order:
  id: `nat`
"#;
        fs::write(topos_dir.join("order.tps"), topos_content).unwrap();

        let lean_content = r#"
structure Order where
  id : Nat
"#;
        fs::write(lean_dir.join("Order.lean"), lean_content).unwrap();

        let mut sync = DualTrackSync::new(topos_dir, lean_dir);
        sync.scan().await.unwrap();

        let report = sync.detect_drift().await.unwrap();

        // Should detect missing link between Order concept and Order structure
        assert!(report.has_drifts());
    }

    #[tokio::test]
    async fn test_detect_drift_missing_structure() {
        let (_temp, topos_dir, lean_dir) = setup_test_dirs();

        // Create only Topos file
        let topos_content = r#"
Concept Order:
  id: `nat`
"#;
        fs::write(topos_dir.join("order.tps"), topos_content).unwrap();

        let mut sync = DualTrackSync::new(topos_dir, lean_dir);
        sync.scan().await.unwrap();

        let report = sync.detect_drift().await.unwrap();

        assert!(report.has_drifts());
        assert_eq!(report.summary.missing, 1);
    }

    #[tokio::test]
    async fn test_sync_topos_to_lean() {
        let (_temp, topos_dir, lean_dir) = setup_test_dirs();

        // Create Topos file
        let topos_content = r#"
Concept Order:
  id: `nat`
  status: `OrderStatus`
"#;
        fs::write(topos_dir.join("order.tps"), topos_content).unwrap();

        let config = SyncConfig {
            auto_generate_lean: true,
            auto_resolve_max_severity: 5,
            require_confirmation: false,
            ..Default::default()
        };

        let mut sync = DualTrackSync::new(topos_dir, lean_dir.clone())
            .with_config(config);
        sync.scan().await.unwrap();

        let result = sync.sync_topos_to_lean().await.unwrap();

        assert!(result.success);
        assert!(!result.files_created.is_empty() || result.links_added > 0);
    }

    #[test]
    fn test_generate_report() {
        let (_temp, topos_dir, lean_dir) = setup_test_dirs();
        let sync = DualTrackSync::new(topos_dir, lean_dir);

        let mut report = DriftReport::new();
        report.add_drift(Drift::missing_lean(
            ToposRef::new("order.tps", "Order"),
            "No Lean structure for Order",
        ));

        let output = sync.generate_report(&report);

        assert!(output.contains("Dual-Track Sync Report"));
        assert!(output.contains("Total drifts: 1"));
        assert!(output.contains("Missing: 1"));
    }
}
