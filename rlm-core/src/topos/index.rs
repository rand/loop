//! Bidirectional link index for Topos-Lean references.
//!
//! This module provides the `LinkIndex` for maintaining and querying
//! bidirectional links between Topos specifications and Lean formalizations.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::parser::{AnnotationParser, AnnotationTarget};
use super::types::{LeanRef, Link, LinkSource, LinkType, ToposRef};
use crate::error::{Error, Result};

/// Bidirectional index of Topos-Lean links.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LinkIndex {
    /// Topos element -> Lean artifact(s) mapping.
    topos_to_lean: HashMap<String, Vec<Link>>,
    /// Lean artifact -> Topos element(s) mapping.
    lean_to_topos: HashMap<String, Vec<Link>>,
    /// All links in insertion order.
    links: Vec<Link>,
    /// Index metadata.
    metadata: IndexMetadata,
}

/// Metadata about the index.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// When the index was last updated.
    pub last_updated: Option<String>,
    /// Number of Topos files indexed.
    pub topos_file_count: usize,
    /// Number of Lean files indexed.
    pub lean_file_count: usize,
    /// Project root path (for resolving relative paths).
    pub project_root: Option<PathBuf>,
}

impl LinkIndex {
    /// Create a new empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an index with a project root.
    pub fn with_project_root(root: impl Into<PathBuf>) -> Self {
        Self {
            metadata: IndexMetadata {
                project_root: Some(root.into()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// Add a link to the index.
    pub fn add_link(&mut self, link: Link) {
        let topos_key = link.topos.to_string_canonical();
        let lean_key = link.lean.to_string_canonical();

        self.topos_to_lean
            .entry(topos_key)
            .or_default()
            .push(link.clone());
        self.lean_to_topos
            .entry(lean_key)
            .or_default()
            .push(link.clone());
        self.links.push(link);
    }

    /// Get all Lean references for a Topos element.
    pub fn get_lean_refs(&self, topos_ref: &ToposRef) -> Vec<&Link> {
        let key = topos_ref.to_string_canonical();
        self.topos_to_lean
            .get(&key)
            .map(|links| links.iter().collect())
            .unwrap_or_default()
    }

    /// Get all Topos references for a Lean artifact.
    pub fn get_topos_refs(&self, lean_ref: &LeanRef) -> Vec<&Link> {
        let key = lean_ref.to_string_canonical();
        self.lean_to_topos
            .get(&key)
            .map(|links| links.iter().collect())
            .unwrap_or_default()
    }

    /// Find links by Topos file path.
    pub fn links_for_topos_file(&self, path: &Path) -> Vec<&Link> {
        self.links
            .iter()
            .filter(|link| link.topos.file == path)
            .collect()
    }

    /// Find links by Lean file path.
    pub fn links_for_lean_file(&self, path: &Path) -> Vec<&Link> {
        self.links
            .iter()
            .filter(|link| link.lean.file == path)
            .collect()
    }

    /// Get all links.
    pub fn all_links(&self) -> &[Link] {
        &self.links
    }

    /// Get the number of links.
    pub fn len(&self) -> usize {
        self.links.len()
    }

    /// Check if the index is empty.
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }

    /// Get links by type.
    pub fn links_by_type(&self, link_type: LinkType) -> Vec<&Link> {
        self.links
            .iter()
            .filter(|link| link.link_type == link_type)
            .collect()
    }

    /// Get links by source.
    pub fn links_by_source(&self, source: LinkSource) -> Vec<&Link> {
        self.links
            .iter()
            .filter(|link| link.source == source)
            .collect()
    }

    /// Clear all links.
    pub fn clear(&mut self) {
        self.topos_to_lean.clear();
        self.lean_to_topos.clear();
        self.links.clear();
    }

    /// Get unique Topos elements.
    pub fn unique_topos_elements(&self) -> Vec<&str> {
        self.topos_to_lean.keys().map(|s| s.as_str()).collect()
    }

    /// Get unique Lean artifacts.
    pub fn unique_lean_artifacts(&self) -> Vec<&str> {
        self.lean_to_topos.keys().map(|s| s.as_str()).collect()
    }

    /// Index a Topos file by parsing its @lean annotations.
    pub fn index_topos_file(&mut self, path: &Path, content: &str) -> Result<usize> {
        let annotations = AnnotationParser::parse_lean_annotations(content);
        let mut count = 0;

        // We need context about what element each annotation belongs to
        // For now, use a simple heuristic: find the closest preceding element definition
        let lines: Vec<&str> = content.lines().collect();

        for annotation in annotations {
            if let AnnotationTarget::Lean(ref lean_ref) = annotation.target {
                // Find the element this annotation belongs to
                let element_name =
                    Self::find_element_context(&lines, annotation.line as usize);

                if let Some(name) = element_name {
                    let topos_ref = ToposRef::new(path, name);
                    let link_type = AnnotationParser::infer_link_type(&annotation);
                    let metadata = AnnotationParser::to_metadata(&annotation);

                    let link = Link::new(topos_ref, lean_ref.clone(), link_type, LinkSource::Topos)
                        .with_metadata(metadata);

                    self.add_link(link);
                    count += 1;
                }
            }
        }

        self.metadata.topos_file_count += 1;
        Ok(count)
    }

    /// Index a Lean file by parsing its @topos annotations.
    pub fn index_lean_file(&mut self, path: &Path, content: &str) -> Result<usize> {
        let annotations = AnnotationParser::parse_topos_annotations(content);
        let mut count = 0;

        let lines: Vec<&str> = content.lines().collect();

        for annotation in annotations {
            match &annotation.target {
                AnnotationTarget::Topos(topos_ref) => {
                    // Find the Lean artifact this annotation belongs to
                    let artifact_name =
                        Self::find_lean_artifact_context(&lines, annotation.line as usize);

                    if let Some(name) = artifact_name {
                        let lean_ref = LeanRef::new(path, name);
                        let link_type = AnnotationParser::infer_link_type(&annotation);
                        let metadata = AnnotationParser::to_metadata(&annotation);

                        let link = Link::new(topos_ref.clone(), lean_ref, link_type, LinkSource::Lean)
                            .with_metadata(metadata);

                        self.add_link(link);
                        count += 1;
                    }
                }
                AnnotationTarget::SpecId(_) => {
                    // Spec IDs are handled separately
                }
                AnnotationTarget::Lean(_) => {
                    // Lean targets in Lean files are not expected
                }
            }
        }

        self.metadata.lean_file_count += 1;
        Ok(count)
    }

    /// Find the Topos element context for a line.
    fn find_element_context(lines: &[&str], target_line: usize) -> Option<String> {
        // Look backwards for element definitions
        // Patterns: "Concept Name:", "Behavior name:", "## REQ-X:", "## TASK-X:"
        for i in (0..=target_line).rev() {
            let line = lines.get(i)?.trim();

            // Concept definition
            if line.starts_with("Concept ") && line.ends_with(':') {
                let name = line
                    .strip_prefix("Concept ")?
                    .strip_suffix(':')?
                    .trim();
                return Some(name.to_string());
            }

            // Behavior definition
            if line.starts_with("Behavior ") && line.ends_with(':') {
                let name = line
                    .strip_prefix("Behavior ")?
                    .strip_suffix(':')?
                    .trim();
                return Some(name.to_string());
            }

            // Requirement definition
            if line.starts_with("## REQ-") {
                if let Some(colon_pos) = line.find(':') {
                    let id = line[3..colon_pos].trim();
                    return Some(id.to_string());
                }
            }

            // Invariant definition
            if line.starts_with("Invariant ") && line.ends_with(':') {
                let name = line
                    .strip_prefix("Invariant ")?
                    .strip_suffix(':')?
                    .trim();
                return Some(name.to_string());
            }
        }

        None
    }

    /// Find the Lean artifact context for a line.
    fn find_lean_artifact_context(lines: &[&str], target_line: usize) -> Option<String> {
        // Look forward for Lean definitions after doc comments
        // Patterns: "structure Name", "theorem name", "def name", "lemma name"
        for i in target_line..lines.len() {
            let line = lines.get(i)?.trim();

            // Skip empty lines, comments, and doc comment content
            if line.is_empty()
                || line.starts_with("--")
                || line.starts_with("-/")
                || line.starts_with("/--")
                || line.starts_with("@")
                || line.contains("@topos:")
                || line.contains("@spec:")
            {
                continue;
            }

            // Structure definition
            if line.starts_with("structure ") {
                let rest = line.strip_prefix("structure ")?;
                let name = rest.split_whitespace().next()?;
                return Some(name.to_string());
            }

            // Theorem definition
            if line.starts_with("theorem ") {
                let rest = line.strip_prefix("theorem ")?;
                let name = rest.split_whitespace().next()?;
                return Some(name.to_string());
            }

            // Def definition
            if line.starts_with("def ") {
                let rest = line.strip_prefix("def ")?;
                let name = rest.split_whitespace().next()?;
                return Some(name.to_string());
            }

            // Lemma definition
            if line.starts_with("lemma ") {
                let rest = line.strip_prefix("lemma ")?;
                let name = rest.split_whitespace().next()?;
                return Some(name.to_string());
            }

            // Skip lines that look like prose in doc comments (no Lean keywords)
            // Continue looking for the definition
        }

        None
    }

    /// Serialize the index to JSON.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(Error::Serialization)
    }

    /// Deserialize the index from JSON.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(Error::Serialization)
    }

    /// Save the index to a file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = self.to_json()?;
        fs::write(path, json).map_err(|e| Error::Internal(format!("Failed to save index: {}", e)))
    }

    /// Load the index from a file.
    pub fn load(path: &Path) -> Result<Self> {
        let json = fs::read_to_string(path)
            .map_err(|e| Error::Internal(format!("Failed to load index: {}", e)))?;
        Self::from_json(&json)
    }

    /// Get index metadata.
    pub fn metadata(&self) -> &IndexMetadata {
        &self.metadata
    }

    /// Update the last_updated timestamp.
    pub fn touch(&mut self) {
        self.metadata.last_updated = Some(chrono::Utc::now().to_rfc3339());
    }
}

/// Builder for constructing a LinkIndex from a project directory.
pub struct IndexBuilder {
    project_root: PathBuf,
    topos_patterns: Vec<String>,
    lean_patterns: Vec<String>,
}

impl IndexBuilder {
    /// Create a new builder with the project root.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            topos_patterns: vec!["**/*.tps".to_string(), "**/*.topos".to_string()],
            lean_patterns: vec!["**/*.lean".to_string()],
        }
    }

    /// Set custom Topos file patterns.
    pub fn topos_patterns(mut self, patterns: Vec<String>) -> Self {
        self.topos_patterns = patterns;
        self
    }

    /// Set custom Lean file patterns.
    pub fn lean_patterns(mut self, patterns: Vec<String>) -> Self {
        self.lean_patterns = patterns;
        self
    }

    /// Build the index by scanning all files.
    pub fn build(self) -> Result<LinkIndex> {
        let mut index = LinkIndex::with_project_root(&self.project_root);

        // Find and index Topos files
        for pattern in &self.topos_patterns {
            let full_pattern = self.project_root.join(pattern);
            if let Ok(entries) = glob::glob(full_pattern.to_str().unwrap_or("")) {
                for entry in entries.flatten() {
                    if let Ok(content) = fs::read_to_string(&entry) {
                        // Use relative path
                        let rel_path = entry
                            .strip_prefix(&self.project_root)
                            .unwrap_or(&entry);
                        let _ = index.index_topos_file(rel_path, &content);
                    }
                }
            }
        }

        // Find and index Lean files
        for pattern in &self.lean_patterns {
            let full_pattern = self.project_root.join(pattern);
            if let Ok(entries) = glob::glob(full_pattern.to_str().unwrap_or("")) {
                for entry in entries.flatten() {
                    if let Ok(content) = fs::read_to_string(&entry) {
                        // Use relative path
                        let rel_path = entry
                            .strip_prefix(&self.project_root)
                            .unwrap_or(&entry);
                        let _ = index.index_lean_file(rel_path, &content);
                    }
                }
            }
        }

        index.touch();
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_links() {
        let mut index = LinkIndex::new();

        let topos_ref = ToposRef::new("spec.tps", "Order");
        let lean_ref = LeanRef::new("Order.lean", "Order");
        let link = Link::new(
            topos_ref.clone(),
            lean_ref.clone(),
            LinkType::Structure,
            LinkSource::Topos,
        );

        index.add_link(link);

        assert_eq!(index.len(), 1);
        assert_eq!(index.get_lean_refs(&topos_ref).len(), 1);
        assert_eq!(index.get_topos_refs(&lean_ref).len(), 1);
    }

    #[test]
    fn test_index_topos_file() {
        let mut index = LinkIndex::new();

        let content = r#"
Concept Order:
  id: `OrderId`
  items: list of `OrderItem`
  @lean: specs/Order.lean#Order

Behavior create_order:
  given: request (`OrderRequest`)
  returns: `Order`
  @lean: specs/Order.lean#create_order_spec
"#;

        let count = index
            .index_topos_file(Path::new("orders.tps"), content)
            .unwrap();
        assert_eq!(count, 2);
        assert_eq!(index.len(), 2);
    }

    #[test]
    fn test_index_lean_file() {
        let mut index = LinkIndex::new();

        let content = r#"/--
@topos: OrderManagement.tps#Order
Order represents a customer order.
-/
structure Order where
  id : Nat
  items : List OrderItem
"#;

        let count = index
            .index_lean_file(Path::new("Order.lean"), content)
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_serialization() {
        let mut index = LinkIndex::new();
        let link = Link::new(
            ToposRef::new("spec.tps", "Order"),
            LeanRef::new("Order.lean", "Order"),
            LinkType::Structure,
            LinkSource::Topos,
        );
        index.add_link(link);

        let json = index.to_json().unwrap();
        let loaded = LinkIndex::from_json(&json).unwrap();

        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_find_element_context() {
        let content = r#"
Concept Order:
  id: `OrderId`
  items: list of `OrderItem`
  @lean: specs/Order.lean#Order
"#;
        let lines: Vec<&str> = content.lines().collect();

        // Line 4 (0-indexed) has the @lean annotation
        let element = LinkIndex::find_element_context(&lines, 4);
        assert_eq!(element, Some("Order".to_string()));
    }

    #[test]
    fn test_find_lean_artifact_context() {
        let content = r#"/--
@topos: OrderManagement.tps#Order
-/
structure Order where
  id : Nat
"#;
        let lines: Vec<&str> = content.lines().collect();

        // Line 1 has the @topos annotation
        let artifact = LinkIndex::find_lean_artifact_context(&lines, 1);
        assert_eq!(artifact, Some("Order".to_string()));
    }
}
