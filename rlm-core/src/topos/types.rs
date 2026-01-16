//! Type definitions for Topos-Lean linking.
//!
//! This module defines the core types for bidirectional references between
//! Topos specifications and Lean formalizations.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A reference to a Topos specification element.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToposRef {
    /// Path to the Topos spec file (relative to project root).
    pub file: PathBuf,
    /// Element name within the spec (Concept, Behavior, Requirement, etc.).
    pub element: String,
    /// Optional field or sub-element within the element.
    pub sub_element: Option<String>,
}

impl ToposRef {
    /// Create a new Topos reference.
    pub fn new(file: impl Into<PathBuf>, element: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            element: element.into(),
            sub_element: None,
        }
    }

    /// Create a Topos reference with a sub-element.
    pub fn with_sub_element(
        file: impl Into<PathBuf>,
        element: impl Into<String>,
        sub: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            element: element.into(),
            sub_element: Some(sub.into()),
        }
    }

    /// Parse a Topos reference from a string like "path/to/spec.tps#ConceptName"
    /// or "path/to/spec.tps#ConceptName.field".
    pub fn parse(s: &str) -> Option<Self> {
        let (path, fragment) = s.split_once('#')?;
        let file = PathBuf::from(path.trim());

        // Check for sub-element (e.g., "Order.status" or "Order.invariant")
        if let Some((element, sub)) = fragment.split_once('.') {
            Some(Self {
                file,
                element: element.trim().to_string(),
                sub_element: Some(sub.trim().to_string()),
            })
        } else {
            Some(Self {
                file,
                element: fragment.trim().to_string(),
                sub_element: None,
            })
        }
    }

    /// Format as a canonical string representation.
    pub fn to_string_canonical(&self) -> String {
        let base = format!("{}#{}", self.file.display(), self.element);
        match &self.sub_element {
            Some(sub) => format!("{}.{}", base, sub),
            None => base,
        }
    }
}

impl std::fmt::Display for ToposRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_canonical())
    }
}

/// A reference to a Lean artifact (structure, theorem, lemma, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LeanRef {
    /// Path to the Lean file (relative to project root).
    pub file: PathBuf,
    /// Artifact name (structure, theorem, lemma, etc.).
    pub artifact: String,
    /// Optional namespace prefix.
    pub namespace: Option<String>,
}

impl LeanRef {
    /// Create a new Lean reference.
    pub fn new(file: impl Into<PathBuf>, artifact: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            artifact: artifact.into(),
            namespace: None,
        }
    }

    /// Create a Lean reference with a namespace.
    pub fn with_namespace(
        file: impl Into<PathBuf>,
        namespace: impl Into<String>,
        artifact: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            artifact: artifact.into(),
            namespace: Some(namespace.into()),
        }
    }

    /// Parse a Lean reference from a string like "path/to/file.lean#ArtifactName"
    /// or "path/to/file.lean#Namespace.ArtifactName".
    pub fn parse(s: &str) -> Option<Self> {
        let (path, fragment) = s.split_once('#')?;
        let file = PathBuf::from(path.trim());

        // Check for namespace (e.g., "Order.create_order_spec")
        if let Some((ns, artifact)) = fragment.rsplit_once('.') {
            // Could be namespace.artifact or just artifact with dots
            // Simple heuristic: if ns starts with uppercase, treat as namespace
            if ns.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                Some(Self {
                    file,
                    artifact: artifact.trim().to_string(),
                    namespace: Some(ns.trim().to_string()),
                })
            } else {
                Some(Self {
                    file,
                    artifact: fragment.trim().to_string(),
                    namespace: None,
                })
            }
        } else {
            Some(Self {
                file,
                artifact: fragment.trim().to_string(),
                namespace: None,
            })
        }
    }

    /// Format as a canonical string representation.
    pub fn to_string_canonical(&self) -> String {
        let artifact = match &self.namespace {
            Some(ns) => format!("{}.{}", ns, self.artifact),
            None => self.artifact.clone(),
        };
        format!("{}#{}", self.file.display(), artifact)
    }
}

impl std::fmt::Display for LeanRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string_canonical())
    }
}

/// A bidirectional link between a Topos element and a Lean artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// Reference to the Topos element.
    pub topos: ToposRef,
    /// Reference to the Lean artifact.
    pub lean: LeanRef,
    /// Type of link (structure, theorem, invariant, etc.).
    pub link_type: LinkType,
    /// Source of the link (where it was declared).
    pub source: LinkSource,
    /// Optional metadata.
    pub metadata: Option<LinkMetadata>,
}

impl Link {
    /// Create a new link.
    pub fn new(topos: ToposRef, lean: LeanRef, link_type: LinkType, source: LinkSource) -> Self {
        Self {
            topos,
            lean,
            link_type,
            source,
            metadata: None,
        }
    }

    /// Add metadata to the link.
    pub fn with_metadata(mut self, metadata: LinkMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Type of link between Topos and Lean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkType {
    /// Concept maps to a Lean structure.
    Structure,
    /// Behavior maps to a Lean function specification.
    FunctionSpec,
    /// Invariant maps to a Lean theorem.
    Theorem,
    /// Invariant on a concept field.
    FieldInvariant,
    /// Requirement maps to a Lean property.
    Property,
    /// Generic annotation link.
    Annotation,
}

impl LinkType {
    /// Get a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Structure => "structure definition",
            Self::FunctionSpec => "function specification",
            Self::Theorem => "theorem",
            Self::FieldInvariant => "field invariant",
            Self::Property => "property",
            Self::Annotation => "annotation",
        }
    }
}

/// Source of a link declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkSource {
    /// Link declared in Topos via @lean annotation.
    Topos,
    /// Link declared in Lean via @topos comment.
    Lean,
    /// Link synthesized by the sync engine.
    Synthesized,
}

/// Additional metadata for a link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkMetadata {
    /// Line number in the source file.
    pub line: Option<u32>,
    /// Column number in the source file.
    pub column: Option<u32>,
    /// Additional notes or comments.
    pub notes: Option<String>,
    /// Spec identifier (e.g., SPEC-01.01).
    pub spec_id: Option<String>,
}

impl LinkMetadata {
    /// Create metadata with position.
    pub fn at_position(line: u32, column: u32) -> Self {
        Self {
            line: Some(line),
            column: Some(column),
            notes: None,
            spec_id: None,
        }
    }
}

/// Topos element types that can be linked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToposElementType {
    /// A Concept definition.
    Concept,
    /// A Behavior definition.
    Behavior,
    /// A Requirement definition.
    Requirement,
    /// An Invariant definition.
    Invariant,
    /// A Task definition.
    Task,
    /// A field within a Concept.
    Field,
}

impl ToposElementType {
    /// Parse from a string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "concept" => Some(Self::Concept),
            "behavior" => Some(Self::Behavior),
            "requirement" | "req" => Some(Self::Requirement),
            "invariant" => Some(Self::Invariant),
            "task" => Some(Self::Task),
            "field" => Some(Self::Field),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topos_ref_parse() {
        let r = ToposRef::parse("OrderManagement.tps#Order").unwrap();
        assert_eq!(r.file, PathBuf::from("OrderManagement.tps"));
        assert_eq!(r.element, "Order");
        assert_eq!(r.sub_element, None);

        let r = ToposRef::parse("specs/auth.tps#User.email").unwrap();
        assert_eq!(r.file, PathBuf::from("specs/auth.tps"));
        assert_eq!(r.element, "User");
        assert_eq!(r.sub_element, Some("email".to_string()));
    }

    #[test]
    fn test_lean_ref_parse() {
        let r = LeanRef::parse("Order.lean#Order").unwrap();
        assert_eq!(r.file, PathBuf::from("Order.lean"));
        assert_eq!(r.artifact, "Order");
        assert_eq!(r.namespace, None);

        let r = LeanRef::parse("specs/Order.lean#Order.create_order_spec").unwrap();
        assert_eq!(r.file, PathBuf::from("specs/Order.lean"));
        assert_eq!(r.artifact, "create_order_spec");
        assert_eq!(r.namespace, Some("Order".to_string()));
    }

    #[test]
    fn test_topos_ref_canonical() {
        let r = ToposRef::with_sub_element("spec.tps", "Order", "status");
        assert_eq!(r.to_string_canonical(), "spec.tps#Order.status");
    }

    #[test]
    fn test_lean_ref_canonical() {
        let r = LeanRef::with_namespace("Order.lean", "Order", "items_nonempty");
        assert_eq!(r.to_string_canonical(), "Order.lean#Order.items_nonempty");
    }
}
