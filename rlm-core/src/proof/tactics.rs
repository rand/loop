//! Tactic libraries for proof automation.
//!
//! This module provides tactic constants and selection functions for
//! different proof automation tiers and specification domains.

use crate::lean::types::Goal;
use crate::proof::types::{AutomationTier, SpecDomain};

// ============================================================================
// Tier 1: Decidable Tactics
// ============================================================================

/// Tactics that are guaranteed to terminate for decidable problems.
/// These are always safe to try first.
pub const DECIDABLE_TACTICS: &[&str] = &[
    "decide",        // Decidable propositions
    "native_decide", // Native decision procedures
    "omega",         // Linear arithmetic over integers/naturals
    "simp",          // Simplification
    "rfl",           // Reflexivity
    "trivial",       // Trivial goals
    "assumption",    // Use a hypothesis
];

/// Extended decidable tactics (may take longer but still terminate).
pub const DECIDABLE_EXTENDED: &[&str] = &[
    "simp_all",      // Simplify everywhere
    "decide!",       // Decidable with computation
    "omega",         // Linear arithmetic (repeated for emphasis)
    "rfl",           // Reflexivity
    "exact?",        // Try to find exact match (library_search lite)
];

// ============================================================================
// Tier 2: Automation Tactics
// ============================================================================

/// Automation tactics that use search-based proof.
/// These may not terminate on all inputs.
pub const AUTOMATION_TACTICS: &[&str] = &[
    "aesop",        // Proof search automation
    "linarith",     // Linear arithmetic reasoning
    "ring",         // Ring identities
    "norm_num",     // Numeric normalization
    "positivity",   // Positivity of expressions
    "nlinarith",    // Nonlinear arithmetic (can be slow)
    "polyrith",     // Polynomial arithmetic (requires Mathlib)
    "field_simp",   // Field simplification
];

/// Structural automation tactics.
pub const STRUCTURAL_TACTICS: &[&str] = &[
    "constructor",   // Apply constructor
    "cases",         // Case analysis
    "induction",     // Induction
    "rcases",        // Recursive case analysis
    "obtain",        // Destructure exists
    "ext",           // Extensionality
    "funext",        // Function extensionality
    "congr",         // Congruence
];

/// Introduction and elimination tactics.
pub const INTRO_ELIM_TACTICS: &[&str] = &[
    "intro",         // Introduce hypothesis
    "intros",        // Introduce multiple
    "apply",         // Apply a lemma
    "exact",         // Exact term
    "refine",        // Refined term with holes
    "use",           // Provide witness for exists
    "exists",        // Alias for use
    "left",          // Choose left disjunct
    "right",         // Choose right disjunct
    "exfalso",       // Prove by contradiction
    "contradiction", // Find contradiction
    "absurd",        // Absurdity reasoning
];

// ============================================================================
// Domain-Specific Tactics
// ============================================================================

/// Tactics for arithmetic/number theory proofs.
pub const ARITHMETIC_TACTICS: &[&str] = &[
    "omega",
    "linarith",
    "ring",
    "norm_num",
    "nlinarith",
    "positivity",
    "simp only [Nat.add_comm, Nat.add_assoc, Nat.mul_comm, Nat.mul_assoc]",
    "decide",
];

/// Tactics for set theory proofs.
pub const SET_THEORY_TACTICS: &[&str] = &[
    "ext",
    "simp only [Set.mem_union, Set.mem_inter_iff, Set.mem_diff]",
    "aesop",
    "intro x",
    "constructor",
    "cases",
    "exact?",
];

/// Tactics for order relation proofs.
pub const ORDER_TACTICS: &[&str] = &[
    "linarith",
    "omega",
    "positivity",
    "nlinarith",
    "simp",
    "exact le_refl _",
    "exact lt_of_le_of_lt",
];

/// Tactics for algebraic proofs.
pub const ALGEBRA_TACTICS: &[&str] = &[
    "ring",
    "ring_nf",
    "field_simp",
    "norm_num",
    "group",
    "abel",
    "simp only [mul_comm, mul_assoc, add_comm, add_assoc]",
];

/// Tactics for logic/propositional proofs.
pub const LOGIC_TACTICS: &[&str] = &[
    "decide",
    "tauto",
    "trivial",
    "simp only [and_true, true_and, or_false, false_or]",
    "constructor",
    "cases",
    "by_contra",
    "push_neg",
    "contrapose",
];

/// Tactics for type theory/equality proofs.
pub const TYPE_THEORY_TACTICS: &[&str] = &[
    "rfl",
    "congr",
    "subst",
    "simp",
    "rw",
    "conv",
    "calc",
    "trans",
    "symm",
];

/// Tactics for data structure proofs.
pub const DATA_STRUCTURE_TACTICS: &[&str] = &[
    "simp only [List.length, List.map, List.filter]",
    "induction",
    "cases",
    "simp",
    "rfl",
    "ext",
    "decide",
];

/// Tactics for category theory proofs.
pub const CATEGORY_THEORY_TACTICS: &[&str] = &[
    "simp only [Category.comp_id, Category.id_comp, Category.assoc]",
    "aesop_cat",
    "ext",
    "rfl",
    "simp",
    "constructor",
];

// ============================================================================
// Tactic Selection Functions
// ============================================================================

/// Get tactics for a specific automation tier.
pub fn tactics_for_tier(tier: AutomationTier) -> Vec<&'static str> {
    match tier {
        AutomationTier::Decidable => DECIDABLE_TACTICS.to_vec(),
        AutomationTier::Automation => {
            let mut tactics = AUTOMATION_TACTICS.to_vec();
            tactics.extend(STRUCTURAL_TACTICS);
            tactics.extend(INTRO_ELIM_TACTICS);
            tactics
        }
        AutomationTier::AIAssisted | AutomationTier::HumanLoop => Vec::new(),
    }
}

/// Get domain-specific tactics for a goal.
pub fn domain_specific_tactics(domain: SpecDomain) -> Vec<&'static str> {
    match domain {
        SpecDomain::Arithmetic => ARITHMETIC_TACTICS.to_vec(),
        SpecDomain::SetTheory => SET_THEORY_TACTICS.to_vec(),
        SpecDomain::Order => ORDER_TACTICS.to_vec(),
        SpecDomain::Algebra => ALGEBRA_TACTICS.to_vec(),
        SpecDomain::Logic => LOGIC_TACTICS.to_vec(),
        SpecDomain::TypeTheory => TYPE_THEORY_TACTICS.to_vec(),
        SpecDomain::DataStructures => DATA_STRUCTURE_TACTICS.to_vec(),
        SpecDomain::CategoryTheory => CATEGORY_THEORY_TACTICS.to_vec(),
        SpecDomain::General => {
            // Mix of common tactics
            vec![
                "simp", "rfl", "trivial", "decide", "aesop", "constructor", "cases", "exact?",
            ]
        }
    }
}

/// Get tactics appropriate for a specific goal.
///
/// This function analyzes the goal structure and returns a prioritized
/// list of tactics likely to work.
pub fn tactics_for_goal(goal: &Goal) -> Vec<&'static str> {
    let mut tactics = Vec::new();
    let target = goal.target.to_lowercase();

    // Check for equality goals
    if target.contains(" = ") || target.contains("eq ") {
        tactics.push("rfl");
        tactics.push("simp");
        tactics.push("ring");
        tactics.push("congr");
    }

    // Check for inequality goals
    if target.contains(" < ") || target.contains(" > ")
        || target.contains("<=") || target.contains(">=")
        || target.contains("le ") || target.contains("lt ")
    {
        tactics.push("linarith");
        tactics.push("omega");
        tactics.push("positivity");
        tactics.push("nlinarith");
    }

    // Check for logical connectives
    if target.contains(" and ") || target.contains("And ") || target.contains(" /\\ ") {
        tactics.push("constructor");
        tactics.push("simp");
    }

    if target.contains(" or ") || target.contains("Or ") || target.contains(" \\/ ") {
        tactics.push("left");
        tactics.push("right");
        tactics.push("cases");
    }

    // Check for quantifiers
    if target.contains("forall") || target.contains("->") {
        tactics.push("intro");
        tactics.push("intros");
    }

    if target.contains("exists") || target.contains("Exists") {
        tactics.push("use");
        tactics.push("exists");
        tactics.push("constructor");
    }

    // Check for negation
    if target.contains("not ") || target.contains("Not ") || target.contains("False") {
        tactics.push("contradiction");
        tactics.push("by_contra");
        tactics.push("exfalso");
    }

    // Check for decidable propositions
    if target.contains("decide") || target.contains("Decidable") {
        tactics.push("decide");
        tactics.push("native_decide");
    }

    // Check for membership
    if target.contains("mem ") || target.contains(" in ") {
        tactics.push("simp");
        tactics.push("exact?");
    }

    // Add aesop as a catch-all automation
    if !tactics.contains(&"aesop") {
        tactics.push("aesop");
    }

    // Infer domain and add domain-specific tactics
    let domain = SpecDomain::infer_from_goal(&goal.target);
    for tactic in domain_specific_tactics(domain) {
        if !tactics.contains(&tactic) {
            tactics.push(tactic);
        }
    }

    tactics
}

/// Generate tactic variations for a base tactic.
///
/// Many tactics have modifiers or can be combined with arguments.
pub fn tactic_variations(base: &str, goal: &Goal) -> Vec<String> {
    let mut variations = vec![base.to_string()];

    match base {
        "simp" => {
            variations.push("simp only []".to_string());
            variations.push("simp_all".to_string());
            variations.push("simp [*]".to_string());

            // Add hypothesis-based simp
            for hyp in &goal.hypotheses {
                variations.push(format!("simp only [{hyp}]", hyp = hyp.name));
            }
        }
        "intro" => {
            variations.push("intros".to_string());
            // Generate common variable names
            for name in ["x", "y", "h", "hx", "hy", "n", "m", "a", "b"] {
                variations.push(format!("intro {}", name));
            }
        }
        "induction" => {
            // Generate induction on available nat/list hypotheses
            for hyp in &goal.hypotheses {
                if hyp.ty.contains("Nat") || hyp.ty.contains("List") {
                    variations.push(format!("induction {}", hyp.name));
                }
            }
        }
        "cases" => {
            // Generate cases on sum types and conditionals
            for hyp in &goal.hypotheses {
                if hyp.ty.contains("Or")
                    || hyp.ty.contains("Sum")
                    || hyp.ty.contains("Option")
                    || hyp.ty.contains("Bool")
                {
                    variations.push(format!("cases {}", hyp.name));
                }
            }
        }
        "apply" | "exact" => {
            // Suggest applying hypotheses
            for hyp in &goal.hypotheses {
                if hyp.ty.contains("->") || hyp.ty.contains("Implies") {
                    variations.push(format!("{} {}", base, hyp.name));
                }
            }
        }
        "rw" => {
            // Generate rewrites using hypotheses
            for hyp in &goal.hypotheses {
                if hyp.ty.contains("=") || hyp.ty.contains("Eq") {
                    variations.push(format!("rw [{hyp}]", hyp = hyp.name));
                    variations.push(format!("rw [<-{hyp}]", hyp = hyp.name));
                }
            }
        }
        _ => {}
    }

    variations
}

/// Combine multiple tactics into a sequence.
pub fn tactic_sequence(tactics: &[&str]) -> String {
    tactics.join("; ")
}

/// Format a tactic with modifiers.
pub fn with_modifiers(tactic: &str, modifiers: &[&str]) -> String {
    if modifiers.is_empty() {
        tactic.to_string()
    } else {
        format!("{} {}", tactic, modifiers.join(" "))
    }
}

/// Generate a "sorry" placeholder with a TODO comment.
pub fn sorry_placeholder(goal: &Goal) -> String {
    format!(
        "-- TODO: Prove goal: {}\nsorry",
        goal.target.lines().next().unwrap_or(&goal.target)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tactics_for_tier() {
        let decidable = tactics_for_tier(AutomationTier::Decidable);
        assert!(decidable.contains(&"decide"));
        assert!(decidable.contains(&"omega"));
        assert!(decidable.contains(&"simp"));

        let automation = tactics_for_tier(AutomationTier::Automation);
        assert!(automation.contains(&"aesop"));
        assert!(automation.contains(&"linarith"));
        assert!(automation.contains(&"constructor"));

        let ai = tactics_for_tier(AutomationTier::AIAssisted);
        assert!(ai.is_empty());
    }

    #[test]
    fn test_domain_specific_tactics() {
        let arith = domain_specific_tactics(SpecDomain::Arithmetic);
        assert!(arith.contains(&"omega"));
        assert!(arith.contains(&"linarith"));

        let logic = domain_specific_tactics(SpecDomain::Logic);
        assert!(logic.contains(&"decide"));
        assert!(logic.contains(&"tauto"));
    }

    #[test]
    fn test_tactics_for_goal() {
        let eq_goal = Goal::from_string("x + 0 = x");
        let tactics = tactics_for_goal(&eq_goal);
        assert!(tactics.contains(&"rfl"));
        assert!(tactics.contains(&"simp"));

        let ineq_goal = Goal::from_string("x < y + 1");
        let tactics = tactics_for_goal(&ineq_goal);
        assert!(tactics.contains(&"linarith"));
        assert!(tactics.contains(&"omega"));

        let and_goal = Goal::from_string("P and Q");
        let tactics = tactics_for_goal(&and_goal);
        assert!(tactics.contains(&"constructor"));

        let forall_goal = Goal::from_string("forall x, P x");
        let tactics = tactics_for_goal(&forall_goal);
        assert!(tactics.contains(&"intro"));
    }

    #[test]
    fn test_tactic_variations() {
        let goal = Goal::from_string("x = y")
            .with_hypothesis("h", "x = y");

        let simp_vars = tactic_variations("simp", &goal);
        assert!(simp_vars.contains(&"simp".to_string()));
        assert!(simp_vars.contains(&"simp only [h]".to_string()));

        let intro_vars = tactic_variations("intro", &Goal::from_string("P -> Q"));
        assert!(intro_vars.contains(&"intro".to_string()));
        assert!(intro_vars.contains(&"intro x".to_string()));
    }

    #[test]
    fn test_tactic_sequence() {
        let seq = tactic_sequence(&["intro x", "simp", "ring"]);
        assert_eq!(seq, "intro x; simp; ring");
    }

    #[test]
    fn test_sorry_placeholder() {
        let goal = Goal::from_string("P -> Q");
        let sorry = sorry_placeholder(&goal);
        assert!(sorry.contains("TODO"));
        assert!(sorry.contains("P -> Q"));
        assert!(sorry.contains("sorry"));
    }
}
