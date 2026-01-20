// Package rlmcore provides Go bindings for the rlm-core Rust library.
// This file contains epistemic verification bindings.

package rlmcore

/*
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -lrlm_core
#cgo darwin LDFLAGS: -framework Security -framework CoreFoundation

#include <stdlib.h>
#include <stdint.h>

// ClaimExtractor opaque type and functions
typedef struct RlmClaimExtractor RlmClaimExtractor;
RlmClaimExtractor* rlm_claim_extractor_new(void);
void rlm_claim_extractor_free(RlmClaimExtractor* extractor);
char* rlm_claim_extractor_extract(RlmClaimExtractor* extractor, const char* response);
char* rlm_claim_extractor_extract_high_specificity(RlmClaimExtractor* extractor, const char* response, double threshold);

// EvidenceScrubber opaque type and functions
typedef struct RlmEvidenceScrubber RlmEvidenceScrubber;
RlmEvidenceScrubber* rlm_evidence_scrubber_new(void);
RlmEvidenceScrubber* rlm_evidence_scrubber_new_aggressive(void);
void rlm_evidence_scrubber_free(RlmEvidenceScrubber* scrubber);
char* rlm_evidence_scrubber_scrub(RlmEvidenceScrubber* scrubber, const char* text);

// KL Divergence functions
double rlm_kl_bernoulli_bits(double p, double q);
double rlm_binary_entropy_bits(double p);
double rlm_surprise_bits(double p);
double rlm_mutual_information_bits(double p_prior, double p_posterior);
double rlm_required_bits_for_specificity(double specificity);
double rlm_aggregate_evidence_bits(const double* kl_values, size_t len);

// ThresholdGate opaque type and functions
typedef struct RlmThresholdGate RlmThresholdGate;
RlmThresholdGate* rlm_threshold_gate_new(void);
RlmThresholdGate* rlm_threshold_gate_new_strict(void);
RlmThresholdGate* rlm_threshold_gate_new_permissive(void);
void rlm_threshold_gate_free(RlmThresholdGate* gate);
char* rlm_threshold_gate_evaluate(RlmThresholdGate* gate, const char* node_json);

// Quick hallucination check
double rlm_quick_hallucination_check(const char* response);
*/
import "C"

import (
	"encoding/json"
	"errors"
	"runtime"
	"unsafe"
)

// ============================================================================
// ClaimExtractor
// ============================================================================

// ClaimExtractor extracts verifiable claims from LLM responses.
type ClaimExtractor struct {
	ptr *C.RlmClaimExtractor
}

// Claim represents an atomic claim extracted from a response.
type Claim struct {
	ID          string  `json:"id"`
	Text        string  `json:"text"`
	Category    string  `json:"category"`
	Specificity float64 `json:"specificity"`
	SpanStart   *int    `json:"span_start,omitempty"`
	SpanEnd     *int    `json:"span_end,omitempty"`
}

// NewClaimExtractor creates a new claim extractor with default settings.
func NewClaimExtractor() *ClaimExtractor {
	e := &ClaimExtractor{ptr: C.rlm_claim_extractor_new()}
	runtime.SetFinalizer(e, (*ClaimExtractor).Free)
	return e
}

// Free releases the claim extractor resources.
func (e *ClaimExtractor) Free() {
	if e.ptr != nil {
		C.rlm_claim_extractor_free(e.ptr)
		e.ptr = nil
	}
}

// Extract extracts all claims from a response.
func (e *ClaimExtractor) Extract(response string) ([]Claim, error) {
	cresponse := cString(response)
	defer C.free(unsafe.Pointer(cresponse))

	cstr := C.rlm_claim_extractor_extract(e.ptr, cresponse)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var claims []Claim
	if err := json.Unmarshal([]byte(jsonStr), &claims); err != nil {
		return nil, err
	}
	return claims, nil
}

// ExtractHighSpecificity extracts claims above a specificity threshold.
func (e *ClaimExtractor) ExtractHighSpecificity(response string, threshold float64) ([]Claim, error) {
	cresponse := cString(response)
	defer C.free(unsafe.Pointer(cresponse))

	cstr := C.rlm_claim_extractor_extract_high_specificity(e.ptr, cresponse, C.double(threshold))
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var claims []Claim
	if err := json.Unmarshal([]byte(jsonStr), &claims); err != nil {
		return nil, err
	}
	return claims, nil
}

// ============================================================================
// EvidenceScrubber
// ============================================================================

// EvidenceScrubber scrubs evidence from text for P0 hiding.
type EvidenceScrubber struct {
	ptr *C.RlmEvidenceScrubber
}

// ScrubResult contains the result of scrubbing evidence.
type ScrubResult struct {
	ScrubbedText       string `json:"scrubbed_text"`
	HasScrubbedContent bool   `json:"has_scrubbed_content"`
	ScrubbedCount      int    `json:"scrubbed_count"`
	TotalCharsScrubbed int    `json:"total_chars_scrubbed"`
}

// NewEvidenceScrubber creates a new evidence scrubber with default settings.
func NewEvidenceScrubber() *EvidenceScrubber {
	s := &EvidenceScrubber{ptr: C.rlm_evidence_scrubber_new()}
	runtime.SetFinalizer(s, (*EvidenceScrubber).Free)
	return s
}

// NewEvidenceScrubberAggressive creates an evidence scrubber with aggressive settings.
func NewEvidenceScrubberAggressive() *EvidenceScrubber {
	s := &EvidenceScrubber{ptr: C.rlm_evidence_scrubber_new_aggressive()}
	runtime.SetFinalizer(s, (*EvidenceScrubber).Free)
	return s
}

// Free releases the evidence scrubber resources.
func (s *EvidenceScrubber) Free() {
	if s.ptr != nil {
		C.rlm_evidence_scrubber_free(s.ptr)
		s.ptr = nil
	}
}

// Scrub scrubs evidence from text.
func (s *EvidenceScrubber) Scrub(text string) (*ScrubResult, error) {
	ctext := cString(text)
	defer C.free(unsafe.Pointer(ctext))

	cstr := C.rlm_evidence_scrubber_scrub(s.ptr, ctext)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var result ScrubResult
	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return nil, err
	}
	return &result, nil
}

// ============================================================================
// KL Divergence Functions
// ============================================================================

// KLBernoulliBits calculates Bernoulli KL divergence in bits.
// Returns KL(p||q) in bits, or an error if probabilities are invalid.
func KLBernoulliBits(p, q float64) (float64, error) {
	result := float64(C.rlm_kl_bernoulli_bits(C.double(p), C.double(q)))
	if result < 0 {
		return 0, lastError()
	}
	return result, nil
}

// BinaryEntropyBits calculates binary entropy in bits.
// Returns H(p) in bits, or an error if probability is invalid.
func BinaryEntropyBits(p float64) (float64, error) {
	result := float64(C.rlm_binary_entropy_bits(C.double(p)))
	if result < 0 {
		return 0, lastError()
	}
	return result, nil
}

// SurpriseBits calculates surprise in bits.
// Returns -log2(p) bits, or an error if probability is invalid.
func SurpriseBits(p float64) (float64, error) {
	result := float64(C.rlm_surprise_bits(C.double(p)))
	if result < 0 {
		return 0, lastError()
	}
	return result, nil
}

// MutualInformationBits calculates mutual information in bits.
// Returns I(prior; posterior) in bits, or an error if probabilities are invalid.
func MutualInformationBits(pPrior, pPosterior float64) (float64, error) {
	result := float64(C.rlm_mutual_information_bits(C.double(pPrior), C.double(pPosterior)))
	if result < 0 {
		return 0, lastError()
	}
	return result, nil
}

// RequiredBitsForSpecificity calculates the required bits for a given specificity level.
func RequiredBitsForSpecificity(specificity float64) (float64, error) {
	result := float64(C.rlm_required_bits_for_specificity(C.double(specificity)))
	if result < 0 {
		return 0, lastError()
	}
	return result, nil
}

// AggregateEvidenceBits aggregates evidence bits from multiple sources.
func AggregateEvidenceBits(klValues []float64) (float64, error) {
	if len(klValues) == 0 {
		return 0, nil
	}
	result := float64(C.rlm_aggregate_evidence_bits(
		(*C.double)(unsafe.Pointer(&klValues[0])),
		C.size_t(len(klValues)),
	))
	if result < 0 {
		return 0, lastError()
	}
	return result, nil
}

// ============================================================================
// ThresholdGate
// ============================================================================

// ThresholdGate gates memory writes based on confidence thresholds.
type ThresholdGate struct {
	ptr *C.RlmThresholdGate
}

// GateDecision contains the result of evaluating a node against the gate.
type GateDecision struct {
	Allowed            bool     `json:"allowed"`
	Recommendation     string   `json:"recommendation"`
	Reason             string   `json:"reason"`
	AdjustedConfidence *float64 `json:"adjusted_confidence,omitempty"`
}

// NewThresholdGate creates a new threshold gate with default settings.
func NewThresholdGate() *ThresholdGate {
	g := &ThresholdGate{ptr: C.rlm_threshold_gate_new()}
	runtime.SetFinalizer(g, (*ThresholdGate).Free)
	return g
}

// NewThresholdGateStrict creates a threshold gate with strict settings.
func NewThresholdGateStrict() *ThresholdGate {
	g := &ThresholdGate{ptr: C.rlm_threshold_gate_new_strict()}
	runtime.SetFinalizer(g, (*ThresholdGate).Free)
	return g
}

// NewThresholdGatePermissive creates a threshold gate with permissive settings.
func NewThresholdGatePermissive() *ThresholdGate {
	g := &ThresholdGate{ptr: C.rlm_threshold_gate_new_permissive()}
	runtime.SetFinalizer(g, (*ThresholdGate).Free)
	return g
}

// Free releases the threshold gate resources.
func (g *ThresholdGate) Free() {
	if g.ptr != nil {
		C.rlm_threshold_gate_free(g.ptr)
		g.ptr = nil
	}
}

// NodeInput represents input for gate evaluation.
type NodeInput struct {
	Type       string  `json:"type"`
	Content    string  `json:"content"`
	Tier       string  `json:"tier,omitempty"`
	Confidence float64 `json:"confidence,omitempty"`
}

// Evaluate evaluates a node against the threshold gate.
func (g *ThresholdGate) Evaluate(input *NodeInput) (*GateDecision, error) {
	nodeJSON, err := json.Marshal(input)
	if err != nil {
		return nil, err
	}

	cnodeJSON := cString(string(nodeJSON))
	defer C.free(unsafe.Pointer(cnodeJSON))

	cstr := C.rlm_threshold_gate_evaluate(g.ptr, cnodeJSON)
	if cstr == nil {
		return nil, lastError()
	}
	jsonStr := goString(cstr)

	var decision GateDecision
	if err := json.Unmarshal([]byte(jsonStr), &decision); err != nil {
		return nil, err
	}
	return &decision, nil
}

// EvaluateNode evaluates a memory node against the threshold gate.
// This is a convenience method that converts a Node to NodeInput.
func (g *ThresholdGate) EvaluateNode(node *Node) (*GateDecision, error) {
	input := &NodeInput{
		Type:       node.Type().String(),
		Content:    node.Content(),
		Tier:       node.Tier().String(),
		Confidence: node.Confidence(),
	}
	return g.Evaluate(input)
}

// ============================================================================
// Quick Hallucination Check
// ============================================================================

// QuickHallucinationCheck performs a quick heuristic check for potential hallucinations.
// Returns a risk score from 0.0 (low risk) to 1.0 (high risk).
func QuickHallucinationCheck(response string) (float64, error) {
	cresponse := cString(response)
	defer C.free(unsafe.Pointer(cresponse))

	result := float64(C.rlm_quick_hallucination_check(cresponse))
	if result < 0 {
		return 0, lastError()
	}
	return result, nil
}

// ============================================================================
// Convenience Functions
// ============================================================================

// ErrInvalidProbability is returned when a probability is out of range.
var ErrInvalidProbability = errors.New("probability must be in [0, 1]")

// ValidateProbability checks if a probability is valid.
func ValidateProbability(p float64) error {
	if p < 0 || p > 1 {
		return ErrInvalidProbability
	}
	return nil
}
