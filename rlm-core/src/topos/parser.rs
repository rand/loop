//! Annotation parser for @lean and @topos annotations.
//!
//! This module parses bidirectional link annotations from both Topos spec files
//! and Lean source files.

use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

use super::types::{LeanRef, LinkMetadata, LinkSource, LinkType, ToposRef};

/// Regex for parsing @lean annotations in Topos files.
/// Matches: `@lean: path/to/file.lean#ArtifactName`
///          `@lean.invariant: path/to/file.lean#TheoremName`
static LEAN_ANNOTATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@lean(?:\.(\w+))?:\s*([^\s\]]+)").expect("Invalid regex"));

/// Regex for parsing @topos annotations in Lean files.
/// Matches: `@topos: path/to/spec.tps#ElementName`
///          `@topos: path/to/spec.tps#Element.subfield`
static TOPOS_ANNOTATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@topos:\s*([^\s]+)").expect("Invalid regex"));

/// Regex for parsing @spec annotations in Lean files.
/// Matches: `@spec: SPEC-01.01`
static SPEC_ANNOTATION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@spec:\s*([A-Z]+-[\d.]+)").expect("Invalid regex"));

/// A parsed annotation from a source file.
#[derive(Debug, Clone)]
pub struct ParsedAnnotation {
    /// The annotation type.
    pub annotation_type: AnnotationType,
    /// The reference target.
    pub target: AnnotationTarget,
    /// Line number where the annotation was found (0-indexed).
    pub line: u32,
    /// Column where the annotation starts.
    pub column: u32,
    /// Optional link type modifier (e.g., "invariant" from @lean.invariant).
    pub link_type_modifier: Option<String>,
    /// Optional spec identifier.
    pub spec_id: Option<String>,
}

/// Type of annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationType {
    /// @lean annotation in a Topos file.
    Lean,
    /// @topos annotation in a Lean file.
    Topos,
    /// @spec annotation (can appear in either).
    Spec,
}

/// Target of an annotation.
#[derive(Debug, Clone)]
pub enum AnnotationTarget {
    /// A Lean reference.
    Lean(LeanRef),
    /// A Topos reference.
    Topos(ToposRef),
    /// A spec identifier (e.g., SPEC-01.01).
    SpecId(String),
}

/// Parser for annotations in source files.
pub struct AnnotationParser;

impl AnnotationParser {
    /// Parse all @lean annotations from Topos file content.
    pub fn parse_lean_annotations(content: &str) -> Vec<ParsedAnnotation> {
        let mut annotations = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            for cap in LEAN_ANNOTATION_RE.captures_iter(line) {
                let modifier = cap.get(1).map(|m| m.as_str().to_string());
                let target_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");

                if let Some(lean_ref) = LeanRef::parse(target_str) {
                    let col = cap.get(0).map(|m| m.start()).unwrap_or(0);
                    annotations.push(ParsedAnnotation {
                        annotation_type: AnnotationType::Lean,
                        target: AnnotationTarget::Lean(lean_ref),
                        line: line_num as u32,
                        column: col as u32,
                        link_type_modifier: modifier,
                        spec_id: None,
                    });
                }
            }
        }

        annotations
    }

    /// Parse all @topos annotations from Lean file content.
    pub fn parse_topos_annotations(content: &str) -> Vec<ParsedAnnotation> {
        let mut annotations = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            // Parse @topos annotations
            for cap in TOPOS_ANNOTATION_RE.captures_iter(line) {
                let target_str = cap.get(1).map(|m| m.as_str()).unwrap_or("");

                if let Some(topos_ref) = ToposRef::parse(target_str) {
                    let col = cap.get(0).map(|m| m.start()).unwrap_or(0);
                    annotations.push(ParsedAnnotation {
                        annotation_type: AnnotationType::Topos,
                        target: AnnotationTarget::Topos(topos_ref),
                        line: line_num as u32,
                        column: col as u32,
                        link_type_modifier: None,
                        spec_id: None,
                    });
                }
            }

            // Parse @spec annotations
            for cap in SPEC_ANNOTATION_RE.captures_iter(line) {
                let spec_id = cap
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                let col = cap.get(0).map(|m| m.start()).unwrap_or(0);
                annotations.push(ParsedAnnotation {
                    annotation_type: AnnotationType::Spec,
                    target: AnnotationTarget::SpecId(spec_id.clone()),
                    line: line_num as u32,
                    column: col as u32,
                    link_type_modifier: None,
                    spec_id: Some(spec_id),
                });
            }
        }

        annotations
    }

    /// Parse annotations from a file based on its extension.
    pub fn parse_file(path: &Path, content: &str) -> Vec<ParsedAnnotation> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            "tps" | "topos" => Self::parse_lean_annotations(content),
            "lean" => Self::parse_topos_annotations(content),
            _ => Vec::new(),
        }
    }

    /// Infer the link type from an annotation.
    pub fn infer_link_type(annotation: &ParsedAnnotation) -> LinkType {
        // Check for explicit modifier
        if let Some(modifier) = &annotation.link_type_modifier {
            match modifier.as_str() {
                "invariant" => return LinkType::FieldInvariant,
                "theorem" => return LinkType::Theorem,
                "property" => return LinkType::Property,
                "function" | "spec" => return LinkType::FunctionSpec,
                "structure" => return LinkType::Structure,
                _ => {}
            }
        }

        // Infer from target
        match &annotation.target {
            AnnotationTarget::Lean(lean_ref) => {
                let artifact = &lean_ref.artifact;
                if artifact.contains("_spec") || artifact.starts_with("spec_") {
                    LinkType::FunctionSpec
                } else if artifact.ends_with("_theorem")
                    || artifact.starts_with("theorem_")
                    || artifact.contains("_preserves_")
                    || artifact.contains("_invariant")
                {
                    LinkType::Theorem
                } else if artifact
                    .chars()
                    .next()
                    .map(|c| c.is_uppercase())
                    .unwrap_or(false)
                {
                    LinkType::Structure
                } else {
                    LinkType::Annotation
                }
            }
            AnnotationTarget::Topos(topos_ref) => {
                let element = &topos_ref.element;
                if element.starts_with("REQ-") {
                    LinkType::Property
                } else if element
                    .chars()
                    .next()
                    .map(|c| c.is_lowercase())
                    .unwrap_or(false)
                {
                    // Lowercase typically indicates a behavior
                    LinkType::FunctionSpec
                } else {
                    LinkType::Structure
                }
            }
            AnnotationTarget::SpecId(_) => LinkType::Property,
        }
    }

    /// Infer the link source from an annotation type.
    pub fn infer_link_source(annotation_type: AnnotationType) -> LinkSource {
        match annotation_type {
            AnnotationType::Lean => LinkSource::Topos,
            AnnotationType::Topos | AnnotationType::Spec => LinkSource::Lean,
        }
    }

    /// Create link metadata from an annotation.
    pub fn to_metadata(annotation: &ParsedAnnotation) -> LinkMetadata {
        LinkMetadata {
            line: Some(annotation.line),
            column: Some(annotation.column),
            notes: None,
            spec_id: annotation.spec_id.clone(),
        }
    }
}

/// Context for parsing Topos files with element tracking.
pub struct ToposParseContext {
    /// Current element being parsed.
    pub current_element: Option<CurrentElement>,
}

/// Currently parsed element context.
#[derive(Debug, Clone)]
pub struct CurrentElement {
    /// Element type (Concept, Behavior, etc.).
    pub element_type: String,
    /// Element name.
    pub name: String,
    /// Start line of the element.
    pub start_line: u32,
}

impl ToposParseContext {
    /// Create a new parse context.
    pub fn new() -> Self {
        Self {
            current_element: None,
        }
    }

    /// Update context when entering an element.
    pub fn enter_element(&mut self, element_type: &str, name: &str, line: u32) {
        self.current_element = Some(CurrentElement {
            element_type: element_type.to_string(),
            name: name.to_string(),
            start_line: line,
        });
    }

    /// Clear current element.
    pub fn exit_element(&mut self) {
        self.current_element = None;
    }
}

impl Default for ToposParseContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lean_annotations_simple() {
        let content = r#"
Concept Order:
  id: `OrderId`
  @lean: specs/Order.lean#Order
"#;
        let annotations = AnnotationParser::parse_lean_annotations(content);
        assert_eq!(annotations.len(), 1);
        assert_eq!(annotations[0].line, 3);
        match &annotations[0].target {
            AnnotationTarget::Lean(r) => {
                assert_eq!(r.file.to_str().unwrap(), "specs/Order.lean");
                assert_eq!(r.artifact, "Order");
            }
            _ => panic!("Expected Lean target"),
        }
    }

    #[test]
    fn test_parse_lean_annotations_with_modifier() {
        let content = r#"
  invariant: items.all(i => i.quantity > 0)
  @lean.invariant: specs/Order.lean#Order.items_nonempty
"#;
        let annotations = AnnotationParser::parse_lean_annotations(content);
        assert_eq!(annotations.len(), 1);
        assert_eq!(
            annotations[0].link_type_modifier,
            Some("invariant".to_string())
        );
    }

    #[test]
    fn test_parse_topos_annotations() {
        let content = r#"/--
@topos: OrderManagement.tps#Order
Order represents a customer order with line items.
-/
structure Order where
  id : Nat
  items : List OrderItem
  status : OrderStatus
  items_pos : forall item in items, item.quantity > 0  -- @topos: OrderManagement.tps#Order.invariant
"#;
        let annotations = AnnotationParser::parse_topos_annotations(content);
        assert_eq!(annotations.len(), 2);
    }

    #[test]
    fn test_parse_spec_annotations() {
        let content = r#"/--
@spec: SPEC-01.01
@topos: OrderManagement.tps#REQ-1
-/
theorem create_order_reserves_inventory
"#;
        let annotations = AnnotationParser::parse_topos_annotations(content);
        assert_eq!(annotations.len(), 2);

        let spec_ann = annotations
            .iter()
            .find(|a| a.annotation_type == AnnotationType::Spec);
        assert!(spec_ann.is_some());
        assert_eq!(spec_ann.unwrap().spec_id, Some("SPEC-01.01".to_string()));
    }

    #[test]
    fn test_infer_link_type() {
        let ann = ParsedAnnotation {
            annotation_type: AnnotationType::Lean,
            target: AnnotationTarget::Lean(LeanRef::new("Order.lean", "Order")),
            line: 0,
            column: 0,
            link_type_modifier: None,
            spec_id: None,
        };
        assert_eq!(AnnotationParser::infer_link_type(&ann), LinkType::Structure);

        let ann = ParsedAnnotation {
            annotation_type: AnnotationType::Lean,
            target: AnnotationTarget::Lean(LeanRef::new("Order.lean", "create_order_spec")),
            line: 0,
            column: 0,
            link_type_modifier: None,
            spec_id: None,
        };
        assert_eq!(
            AnnotationParser::infer_link_type(&ann),
            LinkType::FunctionSpec
        );
    }
}
