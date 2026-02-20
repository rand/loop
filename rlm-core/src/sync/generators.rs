//! Code generators for Topos-Lean synchronization.
//!
//! This module provides functions to generate Lean code from Topos specs
//! and vice versa, respecting the formalization level setting.

use std::collections::HashMap;

use super::types::{
    FormalizationLevel, LeanStructure, LeanTheorem, ToposBehavior, ToposConcept, ToposInvariant,
};

/// Generator for Lean code from Topos specifications.
pub struct LeanGenerator {
    /// Current formalization level.
    level: FormalizationLevel,
    /// Type mappings from Topos to Lean.
    type_mappings: HashMap<String, String>,
    /// Indentation string.
    indent: String,
}

impl LeanGenerator {
    /// Create a new Lean generator with default settings.
    pub fn new(level: FormalizationLevel) -> Self {
        let mut type_mappings = HashMap::new();

        // Default type mappings
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

        Self {
            level,
            type_mappings,
            indent: "  ".to_string(),
        }
    }

    /// Set the formalization level.
    pub fn with_level(mut self, level: FormalizationLevel) -> Self {
        self.level = level;
        self
    }

    /// Add a custom type mapping.
    pub fn add_type_mapping(
        &mut self,
        topos_type: impl Into<String>,
        lean_type: impl Into<String>,
    ) {
        self.type_mappings
            .insert(topos_type.into(), lean_type.into());
    }

    /// Generate a Lean structure from a Topos concept.
    pub fn generate_structure(&self, concept: &ToposConcept) -> String {
        let mut lines = Vec::new();

        // Doc comment
        if let Some(ref doc) = concept.doc {
            lines.push(format!("/-- {} -/", doc));
        } else {
            lines.push(format!("/-- {} from Topos specification. -/", concept.name));
        }

        // Topos reference annotation
        lines.push(format!(
            "/-- @topos: {}#{} -/",
            concept.source_file.display(),
            concept.name
        ));

        // Structure definition
        lines.push(format!("structure {} where", concept.name));

        // Fields
        for field in &concept.fields {
            let lean_type = self.map_topos_type(&field.field_type);
            let field_line = format!("{}{} : {}", self.indent, field.name, lean_type);
            lines.push(field_line);
        }

        // If we need invariants, add them as a separate section
        if self.level.includes_invariants() && !concept.invariants.is_empty() {
            lines.push(String::new());
            lines.push(format!("namespace {}", concept.name));
            lines.push(String::new());

            for inv in &concept.invariants {
                let theorem = self.generate_invariant_theorem(concept, inv);
                lines.push(theorem);
                lines.push(String::new());
            }

            lines.push(format!("end {}", concept.name));
        }

        lines.join("\n")
    }

    /// Generate a Lean theorem for a Topos behavior.
    pub fn generate_theorem(&self, behavior: &ToposBehavior) -> String {
        let mut lines = Vec::new();

        // Doc comment
        if let Some(ref doc) = behavior.doc {
            lines.push(format!("/-- {} -/", doc));
        } else {
            lines.push(format!(
                "/-- Specification for {} from Topos. -/",
                behavior.name
            ));
        }

        // Topos reference
        lines.push(format!(
            "/-- @topos: {}#{} -/",
            behavior.source_file.display(),
            behavior.name
        ));

        // Build theorem signature
        let theorem_name = format!("{}_spec", behavior.name);

        // Parameters
        let params: Vec<String> = behavior
            .inputs
            .iter()
            .map(|input| {
                let lean_type = self.map_topos_type(&input.field_type);
                format!("({} : {})", input.name, lean_type)
            })
            .collect();

        let params_str = params.join(" ");

        // Return type
        let return_type = behavior
            .returns
            .as_ref()
            .map(|r| self.map_topos_type(r))
            .unwrap_or_else(|| "Unit".to_string());

        // Build the theorem based on formalization level
        match self.level {
            FormalizationLevel::Types => {
                // Just a function signature
                lines.push(format!(
                    "def {} {} : {} :=",
                    theorem_name, params_str, return_type
                ));
                lines.push(format!("{}sorry", self.indent));
            }
            FormalizationLevel::Invariants | FormalizationLevel::Contracts => {
                // Add pre/post conditions as propositions
                if !behavior.preconditions.is_empty() || !behavior.postconditions.is_empty() {
                    // Generate the function
                    lines.push(format!(
                        "def {} {} : {} :=",
                        behavior.name, params_str, return_type
                    ));
                    lines.push(format!("{}sorry", self.indent));
                    lines.push(String::new());

                    // Generate precondition theorem
                    if !behavior.preconditions.is_empty() {
                        let pre_name = format!("{}_pre", behavior.name);
                        let pre_body = behavior.preconditions.join(" /\\ ");
                        lines.push(format!(
                            "-- Precondition: {}",
                            behavior.preconditions.join(", ")
                        ));
                        lines.push(format!("theorem {} {} : Prop :=", pre_name, params_str));
                        lines.push(format!("{}sorry -- {}", self.indent, pre_body));
                        lines.push(String::new());
                    }

                    // Generate postcondition theorem
                    if !behavior.postconditions.is_empty() {
                        let post_name = format!("{}_post", behavior.name);
                        let post_body = behavior.postconditions.join(" /\\ ");
                        lines.push(format!(
                            "-- Postcondition: {}",
                            behavior.postconditions.join(", ")
                        ));
                        lines.push(format!(
                            "theorem {} {} (result : {}) : Prop :=",
                            post_name, params_str, return_type
                        ));
                        lines.push(format!("{}sorry -- {}", self.indent, post_body));
                    }
                } else {
                    lines.push(format!(
                        "def {} {} : {} :=",
                        theorem_name, params_str, return_type
                    ));
                    lines.push(format!("{}sorry", self.indent));
                }
            }
            FormalizationLevel::FullProofs => {
                // Full theorem with proof sketch
                lines.push(format!("theorem {} {} :", theorem_name, params_str));

                // Build proposition from pre/post conditions
                let mut props = Vec::new();
                for pre in &behavior.preconditions {
                    props.push(format!("-- Pre: {}", pre));
                }
                for post in &behavior.postconditions {
                    props.push(format!("-- Post: {}", post));
                }

                if !props.is_empty() {
                    for prop in props {
                        lines.push(format!("{}{}", self.indent, prop));
                    }
                }

                lines.push(format!("{}True := by", self.indent)); // Placeholder
                lines.push(format!("{}{}trivial", self.indent, self.indent));
            }
        }

        lines.join("\n")
    }

    /// Generate an invariant theorem.
    fn generate_invariant_theorem(&self, concept: &ToposConcept, inv: &ToposInvariant) -> String {
        let mut lines = Vec::new();

        lines.push(format!("/-- Invariant: {} -/", inv.expression));

        let param = concept.name.to_lowercase();
        let theorem_name = &inv.name;

        lines.push(format!(
            "theorem {} ({} : {}) : Prop :=",
            theorem_name, param, concept.name
        ));
        lines.push(format!("{}sorry -- {}", self.indent, inv.expression));

        lines.join("\n")
    }

    /// Map a Topos type to a Lean type.
    pub fn map_topos_type(&self, topos_type: &str) -> String {
        let ty = topos_type.trim().trim_matches('`');

        // Handle "list of X"
        if let Some(inner) = ty.strip_prefix("list of ") {
            let inner_type = self.map_topos_type(inner.trim());
            return format!("List {}", inner_type);
        }

        // Handle "optional X"
        if let Some(inner) = ty.strip_prefix("optional ") {
            let inner_type = self.map_topos_type(inner.trim());
            return format!("Option {}", inner_type);
        }

        // Handle "set of X"
        if let Some(inner) = ty.strip_prefix("set of ") {
            let inner_type = self.map_topos_type(inner.trim());
            return format!("Set {}", inner_type);
        }

        // Handle "map from X to Y"
        if let Some(rest) = ty.strip_prefix("map from ") {
            if let Some((key, value)) = rest.split_once(" to ") {
                let key_type = self.map_topos_type(key.trim());
                let value_type = self.map_topos_type(value.trim());
                return format!("{} -> {}", key_type, value_type);
            }
        }

        // Direct mapping lookup
        if let Some(lean_type) = self.type_mappings.get(ty) {
            return lean_type.clone();
        }

        // Assume custom type - keep as-is (PascalCase)
        ty.to_string()
    }

    /// Generate a complete Lean module from Topos concepts and behaviors.
    pub fn generate_module(
        &self,
        module_name: &str,
        concepts: &[ToposConcept],
        behaviors: &[ToposBehavior],
    ) -> String {
        let mut lines = Vec::new();

        // Module header
        lines.push(format!(
            "/-! # {}\n\nGenerated from Topos specification.\n-/",
            module_name
        ));
        lines.push(String::new());

        // Generate structures for concepts
        for concept in concepts {
            lines.push(self.generate_structure(concept));
            lines.push(String::new());
        }

        // Generate theorems for behaviors
        for behavior in behaviors {
            lines.push(self.generate_theorem(behavior));
            lines.push(String::new());
        }

        lines.join("\n")
    }
}

impl Default for LeanGenerator {
    fn default() -> Self {
        Self::new(FormalizationLevel::Types)
    }
}

/// Generator for Topos specs from Lean code.
pub struct ToposGenerator {
    /// Type mappings from Lean to Topos.
    type_mappings: HashMap<String, String>,
}

impl ToposGenerator {
    /// Create a new Topos generator.
    pub fn new() -> Self {
        let mut type_mappings = HashMap::new();

        // Reverse mappings from Lean to Topos
        type_mappings.insert("String".to_string(), "string".to_string());
        type_mappings.insert("Int".to_string(), "integer".to_string());
        type_mappings.insert("Nat".to_string(), "natural".to_string());
        type_mappings.insert("Bool".to_string(), "boolean".to_string());
        type_mappings.insert("Float".to_string(), "decimal".to_string());

        Self { type_mappings }
    }

    /// Add a custom type mapping.
    pub fn add_type_mapping(
        &mut self,
        lean_type: impl Into<String>,
        topos_type: impl Into<String>,
    ) {
        self.type_mappings
            .insert(lean_type.into(), topos_type.into());
    }

    /// Generate a Topos concept from a Lean structure.
    pub fn generate_concept(&self, structure: &LeanStructure) -> String {
        let mut lines = Vec::new();

        // Concept definition
        lines.push(format!("Concept {}:", structure.name));

        // Fields
        for field in &structure.fields {
            let topos_type = self.map_lean_type(&field.field_type);
            lines.push(format!("  {}: `{}`", field.name, topos_type));
        }

        // Add Lean reference
        lines.push(format!(
            "  @lean: {}#{}",
            structure.source_file.display(),
            structure.name
        ));

        lines.join("\n")
    }

    /// Generate a Topos behavior from a Lean theorem.
    pub fn generate_behavior(&self, theorem: &LeanTheorem) -> String {
        let mut lines = Vec::new();

        // Extract behavior name (remove _spec suffix if present)
        let behavior_name = theorem
            .name
            .strip_suffix("_spec")
            .unwrap_or(&theorem.name)
            .to_string();

        lines.push(format!("Behavior {}:", behavior_name));

        // Try to parse parameters from statement
        // This is a simplified extraction
        if !theorem.statement.is_empty() {
            lines.push(format!("  // Statement: {}", theorem.statement));
        }

        // Add documentation if available
        if let Some(ref doc) = theorem.doc {
            lines.push(format!("  description: {}", doc));
        }

        // Add Lean reference
        lines.push(format!(
            "  @lean: {}#{}",
            theorem.source_file.display(),
            theorem.name
        ));

        lines.join("\n")
    }

    /// Map a Lean type to a Topos type.
    pub fn map_lean_type(&self, lean_type: &str) -> String {
        let ty = lean_type.trim();

        // Handle "List X"
        if let Some(inner) = ty.strip_prefix("List ") {
            let inner_type = self.map_lean_type(inner.trim());
            return format!("list of `{}`", inner_type);
        }

        // Handle "Option X"
        if let Some(inner) = ty.strip_prefix("Option ") {
            let inner_type = self.map_lean_type(inner.trim());
            return format!("optional `{}`", inner_type);
        }

        // Handle "Array X"
        if let Some(inner) = ty.strip_prefix("Array ") {
            let inner_type = self.map_lean_type(inner.trim());
            return format!("list of `{}`", inner_type);
        }

        // Handle "Set X"
        if let Some(inner) = ty.strip_prefix("Set ") {
            let inner_type = self.map_lean_type(inner.trim());
            return format!("set of `{}`", inner_type);
        }

        // Direct mapping lookup
        if let Some(topos_type) = self.type_mappings.get(ty) {
            return topos_type.clone();
        }

        // Assume custom type - keep as-is
        ty.to_string()
    }

    /// Generate a complete Topos spec from Lean structures and theorems.
    pub fn generate_spec(
        &self,
        spec_name: &str,
        structures: &[LeanStructure],
        theorems: &[LeanTheorem],
    ) -> String {
        let mut lines = Vec::new();

        // Spec header
        lines.push(format!("# {}", spec_name));
        lines.push(String::new());
        lines.push("## Concepts".to_string());
        lines.push(String::new());

        // Generate concepts from structures
        for structure in structures {
            lines.push(self.generate_concept(structure));
            lines.push(String::new());
        }

        // Generate behaviors from theorems
        if !theorems.is_empty() {
            lines.push("## Behaviors".to_string());
            lines.push(String::new());

            for theorem in theorems {
                // Skip invariant theorems
                if theorem.name.ends_with("_inv") || theorem.name.contains("invariant") {
                    continue;
                }
                lines.push(self.generate_behavior(theorem));
                lines.push(String::new());
            }
        }

        lines.join("\n")
    }
}

impl Default for ToposGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a Topos concept to a Lean structure (convenience function).
pub fn topos_to_lean_structure(concept: &ToposConcept, level: FormalizationLevel) -> String {
    LeanGenerator::new(level).generate_structure(concept)
}

/// Convert a Topos behavior to a Lean theorem (convenience function).
pub fn topos_to_lean_theorem(behavior: &ToposBehavior, level: FormalizationLevel) -> String {
    LeanGenerator::new(level).generate_theorem(behavior)
}

/// Convert a Lean structure to a Topos concept (convenience function).
pub fn lean_to_topos_concept(structure: &LeanStructure) -> String {
    ToposGenerator::new().generate_concept(structure)
}

/// Convert a Lean theorem to a Topos behavior (convenience function).
pub fn lean_to_topos_behavior(theorem: &LeanTheorem) -> String {
    ToposGenerator::new().generate_behavior(theorem)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::types::{LeanField, ToposField};
    use std::path::PathBuf;

    fn sample_concept() -> ToposConcept {
        ToposConcept {
            name: "Order".to_string(),
            fields: vec![
                ToposField {
                    name: "id".to_string(),
                    field_type: "nat".to_string(),
                    description: None,
                    constraints: vec![],
                },
                ToposField {
                    name: "items".to_string(),
                    field_type: "list of `OrderItem`".to_string(),
                    description: None,
                    constraints: vec![],
                },
                ToposField {
                    name: "status".to_string(),
                    field_type: "OrderStatus".to_string(),
                    description: None,
                    constraints: vec![],
                },
            ],
            invariants: vec![ToposInvariant {
                name: "Order_items_nonempty".to_string(),
                expression: "items is not empty".to_string(),
                field: Some("items".to_string()),
            }],
            doc: Some("Represents a customer order.".to_string()),
            source_file: PathBuf::from("order.tps"),
            line: 1,
        }
    }

    fn sample_behavior() -> ToposBehavior {
        ToposBehavior {
            name: "create_order".to_string(),
            inputs: vec![ToposField {
                name: "request".to_string(),
                field_type: "OrderRequest".to_string(),
                description: None,
                constraints: vec![],
            }],
            returns: Some("Order".to_string()),
            preconditions: vec!["request.items is not empty".to_string()],
            postconditions: vec!["result.status == pending".to_string()],
            doc: Some("Creates a new order from a request.".to_string()),
            source_file: PathBuf::from("order.tps"),
            line: 10,
        }
    }

    fn sample_lean_structure() -> LeanStructure {
        LeanStructure {
            name: "Order".to_string(),
            fields: vec![
                LeanField {
                    name: "id".to_string(),
                    field_type: "Nat".to_string(),
                    default_value: None,
                },
                LeanField {
                    name: "items".to_string(),
                    field_type: "List OrderItem".to_string(),
                    default_value: None,
                },
            ],
            namespace: None,
            doc: Some("Customer order".to_string()),
            source_file: PathBuf::from("Order.lean"),
            line: 1,
        }
    }

    #[test]
    fn test_lean_generator_structure() {
        let generator = LeanGenerator::new(FormalizationLevel::Types);
        let concept = sample_concept();
        let output = generator.generate_structure(&concept);

        assert!(output.contains("structure Order where"));
        assert!(output.contains("id : Nat"));
        assert!(output.contains("items : List OrderItem"));
        assert!(output.contains("@topos: order.tps#Order"));
    }

    #[test]
    fn test_lean_generator_structure_with_invariants() {
        let generator = LeanGenerator::new(FormalizationLevel::Invariants);
        let concept = sample_concept();
        let output = generator.generate_structure(&concept);

        assert!(output.contains("structure Order where"));
        assert!(output.contains("namespace Order"));
        assert!(output.contains("theorem Order_items_nonempty"));
    }

    #[test]
    fn test_lean_generator_theorem() {
        let generator = LeanGenerator::new(FormalizationLevel::Types);
        let behavior = sample_behavior();
        let output = generator.generate_theorem(&behavior);

        assert!(output.contains("def create_order_spec"));
        assert!(output.contains("(request : OrderRequest)"));
        assert!(output.contains(": Order"));
    }

    #[test]
    fn test_lean_generator_theorem_with_contracts() {
        let generator = LeanGenerator::new(FormalizationLevel::Contracts);
        let behavior = sample_behavior();
        let output = generator.generate_theorem(&behavior);

        assert!(output.contains("def create_order"));
        assert!(output.contains("create_order_pre") || output.contains("Precondition"));
        assert!(output.contains("create_order_post") || output.contains("Postcondition"));
    }

    #[test]
    fn test_type_mapping() {
        let generator = LeanGenerator::default();

        assert_eq!(generator.map_topos_type("string"), "String");
        assert_eq!(generator.map_topos_type("nat"), "Nat");
        assert_eq!(generator.map_topos_type("list of `Item`"), "List Item");
        assert_eq!(generator.map_topos_type("optional `Order`"), "Option Order");
        assert_eq!(generator.map_topos_type("CustomType"), "CustomType");
    }

    #[test]
    fn test_topos_generator_concept() {
        let generator = ToposGenerator::new();
        let structure = sample_lean_structure();
        let output = generator.generate_concept(&structure);

        assert!(output.contains("Concept Order:"));
        assert!(output.contains("id: `natural`") || output.contains("id: `Nat`"));
        assert!(output.contains("@lean: Order.lean#Order"));
    }

    #[test]
    fn test_lean_type_mapping() {
        let generator = ToposGenerator::new();

        assert_eq!(generator.map_lean_type("String"), "string");
        assert_eq!(generator.map_lean_type("Nat"), "natural");
        assert_eq!(generator.map_lean_type("List Item"), "list of `Item`");
        assert_eq!(generator.map_lean_type("Option Order"), "optional `Order`");
    }

    #[test]
    fn test_convenience_functions() {
        let concept = sample_concept();
        let structure = sample_lean_structure();

        let lean_output = topos_to_lean_structure(&concept, FormalizationLevel::Types);
        assert!(lean_output.contains("structure Order where"));

        let topos_output = lean_to_topos_concept(&structure);
        assert!(topos_output.contains("Concept Order:"));
    }

    #[test]
    fn test_module_generation() {
        let generator = LeanGenerator::new(FormalizationLevel::Types);
        let concepts = vec![sample_concept()];
        let behaviors = vec![sample_behavior()];

        let output = generator.generate_module("OrderModule", &concepts, &behaviors);

        assert!(output.contains("# OrderModule"));
        assert!(output.contains("structure Order where"));
        assert!(output.contains("def create_order_spec"));
    }
}
