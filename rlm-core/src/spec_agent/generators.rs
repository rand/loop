//! Generators for Topos and Lean specifications.
//!
//! This module generates formal specifications from extracted requirements:
//! - Topos (.tps) specifications for semantic contracts
//! - Lean (.lean) specifications for formal verification
//! - Cross-references between the two formats


use super::types::{
    CrossReference, ExtractedRequirement, FormalizationLevel, FormalizationResult,
    RequirementType, SpecContext, SpecDomain,
};

// ============================================================================
// Topos Spec Generator
// ============================================================================

/// Generator for Topos specification files.
pub struct ToposGenerator;

impl ToposGenerator {
    /// Generate a Topos specification from the context.
    pub fn generate(ctx: &SpecContext, spec_name: &str) -> GeneratedSpec {
        let mut content = String::new();
        let mut warnings = Vec::new();

        // Header
        content.push_str(&format!("# {}\n", spec_name));
        content.push_str(&format!(
            "# Generated from: {}\n\n",
            Self::truncate(&ctx.nl_input, 60)
        ));

        // Domain-specific imports/context
        if !ctx.detected_domains.is_empty() {
            content.push_str("# Domains: ");
            content.push_str(
                &ctx.detected_domains
                    .iter()
                    .map(|d| format!("{:?}", d))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            content.push_str("\n\n");
        }

        // Group requirements by type
        let data_structures: Vec<_> = ctx
            .requirements
            .iter()
            .filter(|r| r.req_type == RequirementType::DataStructure)
            .collect();
        let behaviors: Vec<_> = ctx
            .requirements
            .iter()
            .filter(|r| r.req_type == RequirementType::Behavior)
            .collect();
        let constraints: Vec<_> = ctx
            .requirements
            .iter()
            .filter(|r| r.req_type == RequirementType::Constraint)
            .collect();
        let error_cases: Vec<_> = ctx
            .requirements
            .iter()
            .filter(|r| r.req_type == RequirementType::ErrorCase)
            .collect();

        // Generate Concepts (from data structures)
        if !data_structures.is_empty() {
            content.push_str("# ============================================================\n");
            content.push_str("# Concepts\n");
            content.push_str("# ============================================================\n\n");

            for req in &data_structures {
                content.push_str(&Self::generate_concept(req, &constraints));
                content.push_str("\n");
            }
        }

        // Generate Behaviors
        if !behaviors.is_empty() {
            content.push_str("# ============================================================\n");
            content.push_str("# Behaviors\n");
            content.push_str("# ============================================================\n\n");

            for req in &behaviors {
                content.push_str(&Self::generate_behavior(req, &error_cases));
                content.push_str("\n");
            }
        }

        // Generate Requirements section
        let all_reqs: Vec<_> = ctx.requirements.iter().collect();
        if !all_reqs.is_empty() {
            content.push_str("# ============================================================\n");
            content.push_str("# Requirements Traceability\n");
            content.push_str("# ============================================================\n\n");

            for (idx, req) in all_reqs.iter().enumerate() {
                content.push_str(&format!(
                    "Requirement {}:\n  id: `{}`\n  description: \"{}\"\n  type: {:?}\n\n",
                    idx + 1,
                    req.id,
                    Self::escape_string(&req.text),
                    req.req_type
                ));
            }
        }

        // Add warnings for incomplete specs
        if data_structures.is_empty() {
            warnings.push("No data structures detected - consider adding Concept definitions".to_string());
        }
        if behaviors.is_empty() {
            warnings.push("No behaviors detected - consider adding Behavior definitions".to_string());
        }

        GeneratedSpec {
            content,
            filename: format!("{}.tps", Self::to_filename(spec_name)),
            warnings,
        }
    }

    /// Generate a Concept definition from a data structure requirement.
    fn generate_concept(req: &ExtractedRequirement, constraints: &[&ExtractedRequirement]) -> String {
        let name = req
            .formal_name
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "Entity".to_string());

        let mut concept = format!("Concept {}:\n", name);
        concept.push_str(&format!("  # Source: {}\n", Self::truncate(&req.text, 50)));

        // Generate placeholder fields based on entities
        if !req.entities.is_empty() {
            for entity in &req.entities {
                if entity != &name {
                    concept.push_str(&format!(
                        "  {}: `{}`\n",
                        Self::to_field_name(entity),
                        entity
                    ));
                }
            }
        } else {
            // Default fields
            concept.push_str("  id: `Id`\n");
            concept.push_str("  # TODO: Add fields\n");
        }

        // Add relevant constraints as invariants
        let relevant_constraints: Vec<_> = constraints
            .iter()
            .filter(|c| c.entities.iter().any(|e| e == &name))
            .collect();

        if !relevant_constraints.is_empty() {
            concept.push_str("\n  # Invariants\n");
            for constraint in relevant_constraints {
                concept.push_str(&format!(
                    "  invariant: # {}\n",
                    Self::truncate(&constraint.text, 40)
                ));
            }
        }

        // Add Lean cross-reference placeholder
        concept.push_str(&format!(
            "\n  @lean: specs/{}.lean#{}\n",
            Self::to_filename(&name),
            name
        ));

        concept
    }

    /// Generate a Behavior definition from a behavioral requirement.
    fn generate_behavior(req: &ExtractedRequirement, error_cases: &[&ExtractedRequirement]) -> String {
        let name = req
            .formal_name
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "operation".to_string());

        let mut behavior = format!("Behavior {}:\n", name);
        behavior.push_str(&format!("  # Source: {}\n", Self::truncate(&req.text, 50)));

        // Placeholder pre/post conditions
        behavior.push_str("  pre: # TODO: Define preconditions\n");
        behavior.push_str("  post: # TODO: Define postconditions\n");

        // Add relevant error cases
        let relevant_errors: Vec<_> = error_cases
            .iter()
            .filter(|e| {
                e.entities.iter().any(|ent| req.entities.contains(ent))
                    || e.text.to_lowercase().contains(&name.to_lowercase())
            })
            .collect();

        if !relevant_errors.is_empty() {
            behavior.push_str("\n  # Error Cases\n");
            for error in relevant_errors {
                behavior.push_str(&format!("  error: # {}\n", Self::truncate(&error.text, 40)));
            }
        }

        // Add Lean cross-reference placeholder
        behavior.push_str(&format!(
            "\n  @lean.spec: specs/{}.lean#{}_spec\n",
            Self::to_filename(&name),
            name
        ));

        behavior
    }

    /// Convert name to a safe filename.
    fn to_filename(name: &str) -> String {
        name.chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect::<String>()
            .to_lowercase()
    }

    /// Convert name to a field name (snake_case).
    fn to_field_name(name: &str) -> String {
        let mut result = String::new();
        for (i, c) in name.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
        }
        result
    }

    /// Truncate and add ellipsis.
    fn truncate(s: &str, max_len: usize) -> String {
        let s = s.replace('\n', " ");
        if s.len() <= max_len {
            s
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }

    /// Escape string for Topos.
    fn escape_string(s: &str) -> String {
        s.replace('"', "\\\"").replace('\n', " ")
    }
}

// ============================================================================
// Lean Spec Generator
// ============================================================================

/// Generator for Lean specification files.
pub struct LeanGenerator;

impl LeanGenerator {
    /// Generate a Lean specification from the context.
    pub fn generate(
        ctx: &SpecContext,
        spec_name: &str,
        level: FormalizationLevel,
    ) -> GeneratedSpec {
        let mut content = String::new();
        let mut warnings = Vec::new();

        // File header
        content.push_str(&format!("/-!\n# {}\n\n", spec_name));
        content.push_str(&format!(
            "Generated from: {}\n",
            Self::truncate(&ctx.nl_input, 60)
        ));
        content.push_str(&format!("Formalization level: {:?}\n-/\n\n", level));

        // Imports based on domains
        content.push_str(&Self::generate_imports(&ctx.detected_domains));
        content.push_str("\n");

        // Namespace
        let namespace = Self::to_namespace(spec_name);
        content.push_str(&format!("namespace {}\n\n", namespace));

        // Group requirements by type
        let data_structures: Vec<_> = ctx
            .requirements
            .iter()
            .filter(|r| r.req_type == RequirementType::DataStructure)
            .collect();
        let behaviors: Vec<_> = ctx
            .requirements
            .iter()
            .filter(|r| r.req_type == RequirementType::Behavior)
            .collect();
        let constraints: Vec<_> = ctx
            .requirements
            .iter()
            .filter(|r| r.req_type == RequirementType::Constraint)
            .collect();

        // Generate structures (always included)
        if !data_structures.is_empty() {
            content.push_str("-- ============================================================\n");
            content.push_str("-- Type Definitions\n");
            content.push_str("-- ============================================================\n\n");

            for req in &data_structures {
                content.push_str(&Self::generate_structure(req, spec_name));
                content.push_str("\n");
            }
        }

        // Generate invariants (if level >= Invariants)
        if level.includes_invariants() && !constraints.is_empty() {
            content.push_str("-- ============================================================\n");
            content.push_str("-- Invariants\n");
            content.push_str("-- ============================================================\n\n");

            for (idx, req) in constraints.iter().enumerate() {
                content.push_str(&Self::generate_invariant(req, idx, &data_structures));
                content.push_str("\n");
            }
        }

        // Generate function specs (if level >= Contracts)
        if level.includes_contracts() && !behaviors.is_empty() {
            content.push_str("-- ============================================================\n");
            content.push_str("-- Function Specifications\n");
            content.push_str("-- ============================================================\n\n");

            for req in &behaviors {
                content.push_str(&Self::generate_function_spec(req, &data_structures, level));
                content.push_str("\n");
            }
        }

        // Generate proofs (if level == FullProofs)
        if level.includes_proofs() {
            content.push_str("-- ============================================================\n");
            content.push_str("-- Proofs\n");
            content.push_str("-- ============================================================\n\n");

            for (idx, req) in constraints.iter().enumerate() {
                content.push_str(&Self::generate_proof_stub(req, idx));
                content.push_str("\n");
            }
        }

        // Close namespace
        content.push_str(&format!("end {}\n", namespace));

        // Add warnings
        if data_structures.is_empty() {
            warnings.push("No structures generated - add data structure requirements".to_string());
        }

        GeneratedSpec {
            content,
            filename: format!("{}.lean", Self::to_filename(spec_name)),
            warnings,
        }
    }

    /// Generate import statements based on domains.
    fn generate_imports(domains: &[SpecDomain]) -> String {
        let mut imports = vec!["import Mathlib.Tactic"];

        for domain in domains {
            for imp in domain.suggested_lean_imports() {
                if !imports.contains(&imp) {
                    imports.push(imp);
                }
            }
        }

        imports
            .iter()
            .map(|i| format!("import {}", i))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Generate a structure definition from a data structure requirement.
    fn generate_structure(req: &ExtractedRequirement, spec_name: &str) -> String {
        let name = req
            .formal_name
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "Entity".to_string());

        let mut structure = format!(
            "/--\n@topos: {}.tps#{}\n{}\n-/\n",
            Self::to_filename(spec_name),
            name,
            Self::truncate(&req.text, 60)
        );

        structure.push_str(&format!("structure {} where\n", name));

        // Generate fields based on entities
        if !req.entities.is_empty() {
            for entity in &req.entities {
                if entity != &name {
                    structure.push_str(&format!(
                        "  {} : {} -- TODO: specify type\n",
                        Self::to_field_name(entity),
                        Self::infer_type(entity)
                    ));
                }
            }
        } else {
            // Default fields
            structure.push_str("  id : Nat\n");
            structure.push_str("  -- TODO: Add fields\n");
        }

        structure.push_str(&format!("  deriving Repr, DecidableEq\n"));

        structure
    }

    /// Generate an invariant theorem from a constraint requirement.
    fn generate_invariant(
        req: &ExtractedRequirement,
        idx: usize,
        data_structures: &[&ExtractedRequirement],
    ) -> String {
        // Try to find the related structure
        let related_struct = data_structures
            .iter()
            .find(|ds| req.entities.iter().any(|e| ds.entities.contains(e)))
            .and_then(|ds| ds.formal_name.as_ref());

        let struct_name = related_struct.cloned().unwrap_or_else(|| "Entity".to_string());
        let inv_name = format!("{}_invariant_{}", Self::to_field_name(&struct_name), idx);

        let mut invariant = format!(
            "/--\nInvariant: {}\n-/\n",
            Self::truncate(&req.text, 60)
        );

        invariant.push_str(&format!(
            "def {} (x : {}) : Prop :=\n",
            inv_name, struct_name
        ));
        invariant.push_str("  sorry -- TODO: formalize invariant\n");

        invariant
    }

    /// Generate a function specification from a behavioral requirement.
    fn generate_function_spec(
        req: &ExtractedRequirement,
        data_structures: &[&ExtractedRequirement],
        level: FormalizationLevel,
    ) -> String {
        let name = req
            .formal_name
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "operation".to_string());

        // Try to infer input/output types from entities
        let input_type = data_structures
            .iter()
            .find(|ds| req.entities.iter().any(|e| ds.entities.contains(e)))
            .and_then(|ds| ds.formal_name.as_ref())
            .cloned()
            .unwrap_or_else(|| "Unit".to_string());

        let mut spec = format!(
            "/--\nBehavior: {}\n-/\n",
            Self::truncate(&req.text, 60)
        );

        // Function signature
        spec.push_str(&format!(
            "def {} (input : {}) : Option {} :=\n",
            name, input_type, input_type
        ));
        spec.push_str("  sorry -- TODO: implement\n\n");

        // Pre/post conditions (if contracts level)
        if level.includes_contracts() {
            spec.push_str(&format!(
                "/--\nPrecondition for {}\n-/\n",
                name
            ));
            spec.push_str(&format!(
                "def {}_pre (input : {}) : Prop :=\n",
                name, input_type
            ));
            spec.push_str("  sorry -- TODO: define precondition\n\n");

            spec.push_str(&format!(
                "/--\nPostcondition for {}\n-/\n",
                name
            ));
            spec.push_str(&format!(
                "def {}_post (input : {}) (result : Option {}) : Prop :=\n",
                name, input_type, input_type
            ));
            spec.push_str("  sorry -- TODO: define postcondition\n\n");

            // Contract theorem
            spec.push_str(&format!(
                "/--\nContract theorem: {} satisfies its specification\n-/\n",
                name
            ));
            spec.push_str(&format!(
                "theorem {}_spec (input : {}) :\n",
                name, input_type
            ));
            spec.push_str(&format!(
                "    {}_pre input → {}_post input ({} input) :=\n",
                name, name, name
            ));
            spec.push_str("  sorry -- TODO: prove contract\n");
        }

        spec
    }

    /// Generate a proof stub for a constraint.
    fn generate_proof_stub(req: &ExtractedRequirement, idx: usize) -> String {
        let theorem_name = format!("proof_{}", idx);

        let mut proof = format!(
            "/--\nProof that: {}\n-/\n",
            Self::truncate(&req.text, 60)
        );

        proof.push_str(&format!(
            "theorem {} : True := by\n",
            theorem_name
        ));
        proof.push_str("  trivial -- TODO: replace with actual proof\n");

        proof
    }

    /// Infer a Lean type from an entity name.
    fn infer_type(entity: &str) -> &'static str {
        let lower = entity.to_lowercase();
        if lower.contains("id") {
            "Nat"
        } else if lower.contains("name") || lower.contains("title") || lower.contains("description") {
            "String"
        } else if lower.contains("count") || lower.contains("quantity") || lower.contains("amount") {
            "Nat"
        } else if lower.contains("price") || lower.contains("cost") || lower.contains("rate") {
            "Float"
        } else if lower.contains("date") || lower.contains("time") || lower.contains("at") {
            "Nat" // Unix timestamp
        } else if lower.contains("list") || lower.contains("items") || lower.contains("array") {
            "List α"
        } else if lower.contains("status") || lower.contains("state") || lower.contains("type") {
            "Nat" // Enum represented as Nat
        } else if lower.contains("flag") || lower.starts_with("is") || lower.contains("is_") || lower.contains("has_") {
            "Bool"
        } else {
            "α" // Generic type
        }
    }

    /// Convert to namespace format (PascalCase).
    fn to_namespace(name: &str) -> String {
        name.split(|c: char| !c.is_alphanumeric())
            .filter(|part| !part.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_uppercase().chain(chars).collect(),
                }
            })
            .collect()
    }

    /// Convert to filename (lowercase).
    fn to_filename(name: &str) -> String {
        name.chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect::<String>()
            .to_lowercase()
    }

    /// Convert to field name (snake_case).
    fn to_field_name(name: &str) -> String {
        let mut result = String::new();
        for (i, c) in name.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
        }
        result
    }

    /// Truncate string with ellipsis.
    fn truncate(s: &str, max_len: usize) -> String {
        let s = s.replace('\n', " ");
        if s.len() <= max_len {
            s
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }
}

// ============================================================================
// Cross-Reference Generator
// ============================================================================

/// Generator for cross-references between Topos and Lean.
pub struct CrossRefGenerator;

impl CrossRefGenerator {
    /// Generate cross-references from a context.
    pub fn generate(ctx: &SpecContext, topos_filename: &str, lean_filename: &str) -> Vec<CrossReference> {
        let mut refs = Vec::new();

        // Generate cross-refs for data structures
        for req in ctx.requirements.iter().filter(|r| r.req_type == RequirementType::DataStructure) {
            if let Some(ref name) = req.formal_name {
                refs.push(CrossReference {
                    topos_element: format!("{}#{}", topos_filename, name),
                    lean_artifact: format!("{}#{}", lean_filename, name),
                    ref_type: "structure".to_string(),
                });
            }
        }

        // Generate cross-refs for behaviors
        for req in ctx.requirements.iter().filter(|r| r.req_type == RequirementType::Behavior) {
            if let Some(ref name) = req.formal_name {
                refs.push(CrossReference {
                    topos_element: format!("{}#{}", topos_filename, name),
                    lean_artifact: format!("{}#{}", lean_filename, name),
                    ref_type: "behavior".to_string(),
                });

                // Also add spec cross-ref
                refs.push(CrossReference {
                    topos_element: format!("{}#{}", topos_filename, name),
                    lean_artifact: format!("{}#{}_spec", lean_filename, name),
                    ref_type: "spec".to_string(),
                });
            }
        }

        refs
    }
}

// ============================================================================
// Combined Generator
// ============================================================================

/// Combined generator that produces both Topos and Lean specs.
pub struct SpecGenerator;

impl SpecGenerator {
    /// Generate both Topos and Lean specifications.
    pub fn generate(
        ctx: &SpecContext,
        spec_name: &str,
        level: FormalizationLevel,
    ) -> FormalizationResult {
        let topos = ToposGenerator::generate(ctx, spec_name);
        let lean = LeanGenerator::generate(ctx, spec_name, level);

        let cross_refs = CrossRefGenerator::generate(ctx, &topos.filename, &lean.filename);

        let mut warnings = topos.warnings;
        warnings.extend(lean.warnings);

        FormalizationResult {
            topos_content: topos.content,
            topos_filename: topos.filename,
            lean_content: lean.content,
            lean_filename: lean.filename,
            cross_refs,
            warnings,
        }
    }
}

/// A generated specification file.
#[derive(Debug, Clone)]
pub struct GeneratedSpec {
    /// Content of the specification.
    pub content: String,
    /// Suggested filename.
    pub filename: String,
    /// Warnings during generation.
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_agent::parser::NLParser;

    #[test]
    fn test_topos_generator_basic() {
        let mut ctx = SpecContext::new("An Order has multiple items and a status");
        NLParser::parse(&mut ctx);

        let spec = ToposGenerator::generate(&ctx, "OrderManagement");
        assert!(spec.content.contains("Concept"));
        assert!(spec.content.contains("Order"));
        assert!(spec.filename.ends_with(".tps"));
    }

    #[test]
    fn test_lean_generator_basic() {
        let mut ctx = SpecContext::new("An Order has multiple items and a status");
        NLParser::parse(&mut ctx);

        let spec = LeanGenerator::generate(&ctx, "OrderManagement", FormalizationLevel::Types);
        assert!(spec.content.contains("structure"));
        assert!(spec.content.contains("namespace"));
        assert!(spec.filename.ends_with(".lean"));
    }

    #[test]
    fn test_lean_generator_with_contracts() {
        let mut ctx = SpecContext::new("Users can create orders. Each order must have at least one item.");
        NLParser::parse(&mut ctx);

        let spec = LeanGenerator::generate(&ctx, "OrderManagement", FormalizationLevel::Contracts);
        assert!(spec.content.contains("_pre"));
        assert!(spec.content.contains("_post"));
        assert!(spec.content.contains("_spec"));
    }

    #[test]
    fn test_cross_ref_generator() {
        let mut ctx = SpecContext::new("An Order has items. Users can create orders.");
        NLParser::parse(&mut ctx);

        let refs = CrossRefGenerator::generate(&ctx, "order.tps", "order.lean");
        assert!(!refs.is_empty());
    }

    #[test]
    fn test_spec_generator_combined() {
        let mut ctx = SpecContext::new("An Order has items and a status. Users can create and cancel orders.");
        NLParser::parse(&mut ctx);

        let result = SpecGenerator::generate(&ctx, "OrderManagement", FormalizationLevel::Contracts);
        assert!(!result.topos_content.is_empty());
        assert!(!result.lean_content.is_empty());
        assert!(!result.cross_refs.is_empty());
    }

    #[test]
    fn test_infer_type() {
        assert_eq!(LeanGenerator::infer_type("userId"), "Nat");
        assert_eq!(LeanGenerator::infer_type("name"), "String");
        assert_eq!(LeanGenerator::infer_type("items"), "List α");
        assert_eq!(LeanGenerator::infer_type("isActive"), "Bool");
    }

    #[test]
    fn test_to_field_name() {
        assert_eq!(LeanGenerator::to_field_name("OrderItem"), "order_item");
        assert_eq!(LeanGenerator::to_field_name("userId"), "user_id");
        assert_eq!(LeanGenerator::to_field_name("status"), "status");
    }

    #[test]
    fn test_to_namespace() {
        assert_eq!(LeanGenerator::to_namespace("order-management"), "OrderManagement");
        assert_eq!(LeanGenerator::to_namespace("user_auth"), "UserAuth");
    }
}
