//! FFI bindings for epistemic verification (pure functions).
//!
//! This module exposes the synchronous, LLM-independent parts of the epistemic
//! module for use via CGO. LLM-dependent verification remains in the host language.

use std::os::raw::c_char;

use super::error::{cstr_to_str, ffi_try, set_last_error, str_to_cstring};
use crate::epistemic::{
    self, ClaimExtractor, EvidenceScrubber, GateRecommendation, MemoryGateConfig, ScrubConfig,
    ThresholdGate,
};
use crate::memory::{Node, NodeType, Tier};

// ============================================================================
// ClaimExtractor
// ============================================================================

/// Opaque handle for ClaimExtractor.
pub struct RlmClaimExtractor(ClaimExtractor);

/// Create a new claim extractor with default settings.
///
/// # Safety
/// The returned pointer must be freed with `rlm_claim_extractor_free()`.
#[no_mangle]
pub extern "C" fn rlm_claim_extractor_new() -> *mut RlmClaimExtractor {
    Box::into_raw(Box::new(RlmClaimExtractor(ClaimExtractor::new())))
}

/// Free a claim extractor.
///
/// # Safety
/// - `extractor` must be a valid pointer or NULL.
/// - After calling this function, `extractor` must not be used.
#[no_mangle]
pub unsafe extern "C" fn rlm_claim_extractor_free(extractor: *mut RlmClaimExtractor) {
    if !extractor.is_null() {
        drop(Box::from_raw(extractor));
    }
}

/// Extract claims from a response.
///
/// Returns a JSON array of claims.
///
/// # Safety
/// - `extractor` must be a valid pointer.
/// - `response` must be a valid null-terminated string.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_claim_extractor_extract(
    extractor: *mut RlmClaimExtractor,
    response: *const c_char,
) -> *mut c_char {
    if extractor.is_null() {
        set_last_error("null extractor pointer");
        return std::ptr::null_mut();
    }
    let response = ffi_try!(cstr_to_str(response));
    let claims = (*extractor).0.extract(response);

    // Convert claims to JSON
    let json_claims: Vec<serde_json::Value> = claims
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id.0.to_string(),
                "text": c.text,
                "category": format!("{:?}", c.category),
                "specificity": c.specificity,
                "span_start": c.source_span.map(|(s, _)| s),
                "span_end": c.source_span.map(|(_, e)| e),
            })
        })
        .collect();

    let json = ffi_try!(serde_json::to_string(&json_claims));
    str_to_cstring(&json)
}

/// Extract high-specificity claims above a threshold.
///
/// Returns a JSON array of claims.
///
/// # Safety
/// - `extractor` must be a valid pointer.
/// - `response` must be a valid null-terminated string.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_claim_extractor_extract_high_specificity(
    extractor: *mut RlmClaimExtractor,
    response: *const c_char,
    threshold: f64,
) -> *mut c_char {
    if extractor.is_null() {
        set_last_error("null extractor pointer");
        return std::ptr::null_mut();
    }
    let response = ffi_try!(cstr_to_str(response));
    let claims = (*extractor).0.extract_high_specificity(response, threshold);

    let json_claims: Vec<serde_json::Value> = claims
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id.0.to_string(),
                "text": c.text,
                "category": format!("{:?}", c.category),
                "specificity": c.specificity,
            })
        })
        .collect();

    let json = ffi_try!(serde_json::to_string(&json_claims));
    str_to_cstring(&json)
}

// ============================================================================
// EvidenceScrubber
// ============================================================================

/// Opaque handle for EvidenceScrubber.
pub struct RlmEvidenceScrubber(EvidenceScrubber);

/// Create a new evidence scrubber with default settings.
///
/// # Safety
/// The returned pointer must be freed with `rlm_evidence_scrubber_free()`.
#[no_mangle]
pub extern "C" fn rlm_evidence_scrubber_new() -> *mut RlmEvidenceScrubber {
    Box::into_raw(Box::new(RlmEvidenceScrubber(
        EvidenceScrubber::default_scrubber(),
    )))
}

/// Create a new evidence scrubber with aggressive settings.
///
/// # Safety
/// The returned pointer must be freed with `rlm_evidence_scrubber_free()`.
#[no_mangle]
pub extern "C" fn rlm_evidence_scrubber_new_aggressive() -> *mut RlmEvidenceScrubber {
    Box::into_raw(Box::new(RlmEvidenceScrubber(EvidenceScrubber::new(
        ScrubConfig::aggressive(),
    ))))
}

/// Free an evidence scrubber.
///
/// # Safety
/// - `scrubber` must be a valid pointer or NULL.
/// - After calling this function, `scrubber` must not be used.
#[no_mangle]
pub unsafe extern "C" fn rlm_evidence_scrubber_free(scrubber: *mut RlmEvidenceScrubber) {
    if !scrubber.is_null() {
        drop(Box::from_raw(scrubber));
    }
}

/// Scrub evidence from text.
///
/// Returns a JSON object with scrubbed text and metadata.
///
/// # Safety
/// - `scrubber` must be a valid pointer.
/// - `text` must be a valid null-terminated string.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_evidence_scrubber_scrub(
    scrubber: *mut RlmEvidenceScrubber,
    text: *const c_char,
) -> *mut c_char {
    if scrubber.is_null() {
        set_last_error("null scrubber pointer");
        return std::ptr::null_mut();
    }
    let text = ffi_try!(cstr_to_str(text));
    let result = (*scrubber).0.scrub(text);

    let json = serde_json::json!({
        "scrubbed_text": result.scrubbed_text,
        "has_scrubbed_content": result.has_scrubbed_content(),
        "scrubbed_count": result.scrubbed_count(),
        "total_chars_scrubbed": result.total_chars_scrubbed(),
    });

    let json_str = ffi_try!(serde_json::to_string(&json));
    str_to_cstring(&json_str)
}

// ============================================================================
// KL Divergence Functions
// ============================================================================

/// Calculate Bernoulli KL divergence in bits.
///
/// Returns KL(p||q) in bits, or -1.0 on error.
#[no_mangle]
pub extern "C" fn rlm_kl_bernoulli_bits(p: f64, q: f64) -> f64 {
    if p < 0.0 || p > 1.0 || q < 0.0 || q > 1.0 {
        set_last_error("probabilities must be in [0, 1]");
        return -1.0;
    }
    epistemic::bernoulli_kl_bits(p, q)
}

/// Calculate binary entropy in bits.
///
/// Returns H(p) in bits, or -1.0 on error.
#[no_mangle]
pub extern "C" fn rlm_binary_entropy_bits(p: f64) -> f64 {
    if p < 0.0 || p > 1.0 {
        set_last_error("probability must be in [0, 1]");
        return -1.0;
    }
    epistemic::binary_entropy_bits(p)
}

/// Calculate surprise in bits.
///
/// Returns -log2(p) bits, or -1.0 on error.
#[no_mangle]
pub extern "C" fn rlm_surprise_bits(p: f64) -> f64 {
    if p <= 0.0 || p > 1.0 {
        set_last_error("probability must be in (0, 1]");
        return -1.0;
    }
    epistemic::surprise_bits(p)
}

/// Calculate mutual information in bits.
///
/// Returns I(prior; posterior) in bits, or -1.0 on error.
#[no_mangle]
pub extern "C" fn rlm_mutual_information_bits(p_prior: f64, p_posterior: f64) -> f64 {
    if p_prior < 0.0 || p_prior > 1.0 || p_posterior < 0.0 || p_posterior > 1.0 {
        set_last_error("probabilities must be in [0, 1]");
        return -1.0;
    }
    epistemic::mutual_information_bits(p_prior, p_posterior)
}

/// Calculate required bits for a given specificity level.
///
/// Returns the information required to justify a claim with the given specificity.
#[no_mangle]
pub extern "C" fn rlm_required_bits_for_specificity(specificity: f64) -> f64 {
    if specificity < 0.0 || specificity > 1.0 {
        set_last_error("specificity must be in [0, 1]");
        return -1.0;
    }
    epistemic::required_bits_for_specificity(specificity)
}

/// Aggregate evidence bits from multiple sources.
///
/// # Safety
/// - `kl_values` must be a valid pointer to an array of f64.
/// - `len` must be the number of elements in the array.
#[no_mangle]
pub unsafe extern "C" fn rlm_aggregate_evidence_bits(kl_values: *const f64, len: usize) -> f64 {
    if kl_values.is_null() && len > 0 {
        set_last_error("null kl_values pointer");
        return -1.0;
    }
    let values = if len > 0 {
        std::slice::from_raw_parts(kl_values, len)
    } else {
        &[]
    };
    epistemic::aggregate_evidence_bits(values)
}

// ============================================================================
// ThresholdGate
// ============================================================================

/// Opaque handle for ThresholdGate.
pub struct RlmThresholdGate(ThresholdGate);

/// Create a new threshold gate with default settings.
///
/// # Safety
/// The returned pointer must be freed with `rlm_threshold_gate_free()`.
#[no_mangle]
pub extern "C" fn rlm_threshold_gate_new() -> *mut RlmThresholdGate {
    Box::into_raw(Box::new(RlmThresholdGate(ThresholdGate::new(
        MemoryGateConfig::default(),
    ))))
}

/// Create a new threshold gate with strict settings.
///
/// # Safety
/// The returned pointer must be freed with `rlm_threshold_gate_free()`.
#[no_mangle]
pub extern "C" fn rlm_threshold_gate_new_strict() -> *mut RlmThresholdGate {
    Box::into_raw(Box::new(RlmThresholdGate(ThresholdGate::new(
        MemoryGateConfig::strict(),
    ))))
}

/// Create a new threshold gate with permissive settings.
///
/// # Safety
/// The returned pointer must be freed with `rlm_threshold_gate_free()`.
#[no_mangle]
pub extern "C" fn rlm_threshold_gate_new_permissive() -> *mut RlmThresholdGate {
    Box::into_raw(Box::new(RlmThresholdGate(ThresholdGate::new(
        MemoryGateConfig::permissive(),
    ))))
}

/// Free a threshold gate.
///
/// # Safety
/// - `gate` must be a valid pointer or NULL.
/// - After calling this function, `gate` must not be used.
#[no_mangle]
pub unsafe extern "C" fn rlm_threshold_gate_free(gate: *mut RlmThresholdGate) {
    if !gate.is_null() {
        drop(Box::from_raw(gate));
    }
}

/// Evaluate a node against the threshold gate.
///
/// Returns a JSON object with the gate decision.
///
/// # Safety
/// - `gate` must be a valid pointer.
/// - `node_json` must be a valid JSON string describing the node.
/// - The returned string must be freed with `rlm_string_free()`.
#[no_mangle]
pub unsafe extern "C" fn rlm_threshold_gate_evaluate(
    gate: *mut RlmThresholdGate,
    node_json: *const c_char,
) -> *mut c_char {
    if gate.is_null() {
        set_last_error("null gate pointer");
        return std::ptr::null_mut();
    }

    let json_str = ffi_try!(cstr_to_str(node_json));
    let json: serde_json::Value = ffi_try!(serde_json::from_str(json_str));

    // Parse node from JSON
    let node_type = match json.get("type").and_then(|v| v.as_str()) {
        Some("entity") => NodeType::Entity,
        Some("fact") => NodeType::Fact,
        Some("experience") => NodeType::Experience,
        Some("decision") => NodeType::Decision,
        Some("snippet") => NodeType::Snippet,
        _ => NodeType::Fact,
    };

    let content = json.get("content").and_then(|v| v.as_str()).unwrap_or("");
    let confidence = json
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    let tier = match json.get("tier").and_then(|v| v.as_str()) {
        Some("task") => Tier::Task,
        Some("session") => Tier::Session,
        Some("long_term") => Tier::LongTerm,
        Some("archive") => Tier::Archive,
        _ => Tier::Task,
    };

    let node = Node::new(node_type, content.to_string())
        .with_tier(tier)
        .with_confidence(confidence);
    let decision = (*gate).0.evaluate(&node);

    let recommendation_str = match decision.recommendation {
        GateRecommendation::Allow => "allow",
        GateRecommendation::AllowWithPenalty => "allow_with_penalty",
        GateRecommendation::Reject => "reject",
        GateRecommendation::Defer => "defer",
        GateRecommendation::QueueForVerification => "queue_for_verification",
    };

    let result = serde_json::json!({
        "allowed": decision.allowed,
        "recommendation": recommendation_str,
        "reason": decision.reason,
        "adjusted_confidence": decision.adjusted_confidence,
    });

    let json_str = ffi_try!(serde_json::to_string(&result));
    str_to_cstring(&json_str)
}

// ============================================================================
// Quick Hallucination Check
// ============================================================================

/// Perform a quick heuristic check for potential hallucinations.
///
/// Returns a risk score from 0.0 (low risk) to 1.0 (high risk).
/// Returns -1.0 on error.
///
/// # Safety
/// - `response` must be a valid null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn rlm_quick_hallucination_check(response: *const c_char) -> f64 {
    let response = match cstr_to_str(response) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&e.to_string());
            return -1.0;
        }
    };
    epistemic::quick_hallucination_check(response)
}
