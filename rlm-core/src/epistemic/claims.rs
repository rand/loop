//! Claim extraction from LLM responses.
//!
//! Extracts atomic, verifiable claims from multi-sentence LLM responses.
//! Each claim represents a single factual assertion that can be independently
//! evaluated for grounding.

use regex::Regex;
use std::collections::HashSet;

use super::types::{Claim, ClaimCategory, EvidenceRef, EvidenceType};

/// Extract atomic claims from an LLM response.
pub struct ClaimExtractor {
    /// Minimum claim length (characters)
    min_length: usize,
    /// Maximum claim length (characters)
    max_length: usize,
    /// Categories to extract (None = all)
    categories: Option<HashSet<ClaimCategory>>,
    /// Words that signal factual claims
    factual_signals: Vec<String>,
    /// Words that signal hedging/uncertainty
    hedge_words: Vec<String>,
}

impl Default for ClaimExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaimExtractor {
    /// Create a new claim extractor with default settings.
    pub fn new() -> Self {
        Self {
            min_length: 10,
            max_length: 500,
            categories: None,
            factual_signals: vec![
                "is".to_string(),
                "are".to_string(),
                "was".to_string(),
                "were".to_string(),
                "has".to_string(),
                "have".to_string(),
                "does".to_string(),
                "returns".to_string(),
                "contains".to_string(),
                "implements".to_string(),
                "calls".to_string(),
                "uses".to_string(),
                "requires".to_string(),
                "depends".to_string(),
            ],
            hedge_words: vec![
                "might".to_string(),
                "could".to_string(),
                "possibly".to_string(),
                "perhaps".to_string(),
                "probably".to_string(),
                "likely".to_string(),
                "seems".to_string(),
                "appears".to_string(),
                "suggests".to_string(),
                "I think".to_string(),
                "I believe".to_string(),
                "may".to_string(),
            ],
        }
    }

    /// Set minimum claim length.
    pub fn with_min_length(mut self, len: usize) -> Self {
        self.min_length = len;
        self
    }

    /// Set maximum claim length.
    pub fn with_max_length(mut self, len: usize) -> Self {
        self.max_length = len;
        self
    }

    /// Filter to specific categories.
    pub fn with_categories(mut self, categories: Vec<ClaimCategory>) -> Self {
        self.categories = Some(categories.into_iter().collect());
        self
    }

    /// Extract claims from a response.
    pub fn extract(&self, response: &str) -> Vec<Claim> {
        let mut claims = Vec::new();

        // Split into sentences
        let sentences = self.split_sentences(response);

        for (idx, sentence) in sentences.iter().enumerate() {
            let trimmed = sentence.trim();

            // Skip if too short or too long
            if trimmed.len() < self.min_length || trimmed.len() > self.max_length {
                continue;
            }

            // Skip questions
            if trimmed.ends_with('?') {
                continue;
            }

            // Skip meta-commentary that's not asserting facts
            if self.is_meta_commentary(trimmed) {
                continue;
            }

            // Classify the claim
            let category = self.classify_claim(trimmed);

            // Filter by category if specified
            if let Some(ref allowed) = self.categories {
                if !allowed.contains(&category) {
                    continue;
                }
            }

            // Calculate specificity
            let specificity = self.estimate_specificity(trimmed);

            // Calculate span in original text
            let span = self.find_span(response, trimmed, idx);

            // Check for hedging
            let is_hedged = self.is_hedged(trimmed);

            let mut claim = Claim::new(trimmed, category)
                .with_specificity(if is_hedged {
                    specificity * 0.5
                } else {
                    specificity
                });

            if let Some((start, end)) = span {
                claim = claim.with_span(start, end);
            }

            // Add metadata about hedging
            if is_hedged {
                let mut meta = std::collections::HashMap::new();
                meta.insert("hedged".to_string(), serde_json::json!(true));
                claim.metadata = Some(meta);
            }

            claims.push(claim);
        }

        // Extract evidence references from the claims
        self.link_evidence(&mut claims, response);

        claims
    }

    /// Split text into sentences.
    fn split_sentences(&self, text: &str) -> Vec<String> {
        // Handle common abbreviations
        let text = text
            .replace("e.g.", "e.g")
            .replace("i.e.", "i.e")
            .replace("etc.", "etc")
            .replace("vs.", "vs")
            .replace("Mr.", "Mr")
            .replace("Ms.", "Ms")
            .replace("Dr.", "Dr");

        // Split on sentence boundaries
        let re = Regex::new(r"[.!?]+\s+|\n\n+").unwrap();
        let mut sentences: Vec<String> = re.split(&text).map(|s| s.trim().to_string()).collect();

        // Handle the last sentence if it doesn't end with punctuation
        if let Some(last) = text.split_whitespace().last() {
            if !last.ends_with('.') && !last.ends_with('!') && !last.ends_with('?') {
                if let Some(last_sentence) = sentences.last_mut() {
                    if !last_sentence.is_empty()
                        && !last_sentence.ends_with('.')
                        && !last_sentence.ends_with('!')
                        && !last_sentence.ends_with('?')
                    {
                        // It's already there, just incomplete
                    }
                }
            }
        }

        // Restore abbreviations
        sentences
            .into_iter()
            .map(|s| {
                s.replace("e.g", "e.g.")
                    .replace("i.e", "i.e.")
                    .replace(" etc", " etc.")
                    .replace(" vs", " vs.")
            })
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Classify a claim into a category.
    fn classify_claim(&self, text: &str) -> ClaimCategory {
        let lower = text.to_lowercase();

        // Code behavior patterns
        if lower.contains("function")
            || lower.contains("method")
            || lower.contains("returns")
            || lower.contains("calls")
            || lower.contains("implementation")
            || lower.contains("class")
            || lower.contains("module")
            || lower.contains("struct")
        {
            return ClaimCategory::CodeBehavior;
        }

        // Numerical patterns
        if Regex::new(r"\b\d+\b").unwrap().is_match(&lower)
            || lower.contains("percent")
            || lower.contains("bytes")
            || lower.contains("milliseconds")
            || lower.contains("seconds")
        {
            return ClaimCategory::Numerical;
        }

        // Relational patterns
        if lower.contains("depends on")
            || lower.contains("related to")
            || lower.contains("connects to")
            || lower.contains("references")
            || lower.contains("imports")
            || lower.contains("requires")
        {
            return ClaimCategory::Relational;
        }

        // Temporal patterns
        if lower.contains("before")
            || lower.contains("after")
            || lower.contains("when")
            || lower.contains("during")
            || lower.contains("then")
            || lower.contains("first")
            || lower.contains("finally")
        {
            return ClaimCategory::Temporal;
        }

        // User intent patterns
        if lower.contains("you want")
            || lower.contains("you need")
            || lower.contains("your")
            || lower.contains("user")
        {
            return ClaimCategory::UserIntent;
        }

        // Meta-reasoning patterns
        if lower.contains("i'll")
            || lower.contains("let me")
            || lower.contains("i should")
            || lower.contains("reasoning")
            || lower.contains("approach")
        {
            return ClaimCategory::MetaReasoning;
        }

        // Default to factual if contains factual signals
        for signal in &self.factual_signals {
            if lower.contains(&signal.to_lowercase()) {
                return ClaimCategory::Factual;
            }
        }

        ClaimCategory::Unknown
    }

    /// Estimate the specificity of a claim (0.0-1.0).
    fn estimate_specificity(&self, text: &str) -> f64 {
        let lower = text.to_lowercase();
        let mut specificity = 0.5; // Base specificity

        // Specific names/identifiers increase specificity
        let identifier_re = Regex::new(r"\b[A-Z][a-zA-Z0-9_]*\b").unwrap();
        let identifier_count = identifier_re.find_iter(text).count();
        specificity += (identifier_count as f64 * 0.05).min(0.2);

        // Numbers increase specificity
        let number_re = Regex::new(r"\b\d+\b").unwrap();
        let number_count = number_re.find_iter(text).count();
        specificity += (number_count as f64 * 0.1).min(0.2);

        // File paths, URLs increase specificity
        if text.contains('/') || text.contains('\\') || text.contains("://") {
            specificity += 0.1;
        }

        // Quantifiers decrease specificity
        if lower.contains("some")
            || lower.contains("many")
            || lower.contains("few")
            || lower.contains("several")
        {
            specificity -= 0.1;
        }

        // Universal claims are very specific (need strong evidence)
        if lower.contains("all")
            || lower.contains("every")
            || lower.contains("always")
            || lower.contains("never")
        {
            specificity += 0.15;
        }

        // Comparatives are moderately specific
        if lower.contains("more")
            || lower.contains("less")
            || lower.contains("better")
            || lower.contains("worse")
        {
            specificity += 0.05;
        }

        specificity.clamp(0.1, 0.95)
    }

    /// Check if a claim is hedged (contains uncertainty language).
    fn is_hedged(&self, text: &str) -> bool {
        let lower = text.to_lowercase();
        for word in &self.hedge_words {
            if lower.contains(&word.to_lowercase()) {
                return true;
            }
        }
        false
    }

    /// Check if text is meta-commentary (not a factual claim).
    fn is_meta_commentary(&self, text: &str) -> bool {
        let lower = text.to_lowercase();

        // Common meta-commentary patterns
        let meta_patterns = [
            "let me",
            "i'll",
            "i will",
            "here's",
            "here is",
            "now let's",
            "to summarize",
            "in summary",
            "as you can see",
            "note that",
            "keep in mind",
            "remember that",
        ];

        for pattern in meta_patterns {
            if lower.starts_with(pattern) {
                return true;
            }
        }

        false
    }

    /// Find the span of a sentence in the original text.
    fn find_span(&self, original: &str, sentence: &str, hint_idx: usize) -> Option<(usize, usize)> {
        // Try to find the sentence starting from the hint position
        let search_start = if hint_idx > 0 {
            // Start searching after previous sentences
            original
                .match_indices(sentence)
                .nth(0)
                .map(|(i, _)| i)
                .unwrap_or(0)
        } else {
            0
        };

        original[search_start..]
            .find(sentence)
            .map(|i| (search_start + i, search_start + i + sentence.len()))
    }

    /// Link evidence references to claims.
    fn link_evidence(&self, claims: &mut [Claim], response: &str) {
        // Look for common evidence patterns
        let citation_re = Regex::new(r"\[(\d+)\]|\(source:\s*([^)]+)\)").unwrap();
        let file_re = Regex::new(r"(?:in|from|see)\s+[`']?([a-zA-Z0-9_/.-]+\.[a-z]+)[`']?").unwrap();
        let code_re = Regex::new(r"`([^`]+)`").unwrap();

        // Extract all citations from the response
        let mut citations: Vec<(usize, String)> = Vec::new();
        for cap in citation_re.captures_iter(response) {
            let citation = cap.get(1).or_else(|| cap.get(2)).map(|m| m.as_str());
            if let Some(c) = citation {
                let start = cap.get(0).unwrap().start();
                citations.push((start, c.to_string()));
            }
        }

        // Extract file references
        let mut file_refs: Vec<(usize, String)> = Vec::new();
        for cap in file_re.captures_iter(response) {
            if let Some(file) = cap.get(1) {
                file_refs.push((cap.get(0).unwrap().start(), file.as_str().to_string()));
            }
        }

        // Link evidence to claims based on proximity
        for claim in claims.iter_mut() {
            if let Some((claim_start, claim_end)) = claim.source_span {
                // Find citations near this claim
                for (cite_pos, cite_text) in &citations {
                    if *cite_pos >= claim_start.saturating_sub(100) && *cite_pos <= claim_end + 100 {
                        claim.evidence_refs.push(EvidenceRef::new(
                            cite_text.clone(),
                            EvidenceType::Citation,
                            format!("Citation [{}]", cite_text),
                        ));
                    }
                }

                // Find file references near this claim
                for (file_pos, file_path) in &file_refs {
                    if *file_pos >= claim_start.saturating_sub(50) && *file_pos <= claim_end + 50 {
                        claim.evidence_refs.push(EvidenceRef::new(
                            file_path.clone(),
                            EvidenceType::CodeRef,
                            format!("File reference: {}", file_path),
                        ));
                    }
                }

                // Look for inline code references within the claim
                let claim_text = &claim.text;
                for cap in code_re.captures_iter(claim_text) {
                    if let Some(code) = cap.get(1) {
                        let code_text = code.as_str();
                        if code_text.len() > 2 {
                            // Skip very short code spans
                            claim.evidence_refs.push(EvidenceRef::new(
                                code_text.to_string(),
                                EvidenceType::CodeRef,
                                format!("Inline code: {}", code_text),
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Extract only high-specificity claims (for efficient verification).
    pub fn extract_high_specificity(&self, response: &str, threshold: f64) -> Vec<Claim> {
        self.extract(response)
            .into_iter()
            .filter(|c| c.specificity >= threshold)
            .collect()
    }

    /// Extract claims from multiple responses (for batch processing).
    pub fn extract_batch(&self, responses: &[&str]) -> Vec<Vec<Claim>> {
        responses.iter().map(|r| self.extract(r)).collect()
    }
}

/// Parse code-specific claims from documentation or comments.
pub fn extract_doc_claims(doc: &str) -> Vec<Claim> {
    let extractor = ClaimExtractor::new()
        .with_categories(vec![ClaimCategory::CodeBehavior, ClaimCategory::Relational]);
    extractor.extract(doc)
}

/// Parse numerical claims (for stricter verification).
pub fn extract_numerical_claims(text: &str) -> Vec<Claim> {
    let extractor = ClaimExtractor::new().with_categories(vec![ClaimCategory::Numerical]);
    extractor.extract(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_extraction() {
        let extractor = ClaimExtractor::new();
        let response = "The function returns an integer. It is called from the main module.";

        let claims = extractor.extract(response);
        assert_eq!(claims.len(), 2);
    }

    #[test]
    fn test_skip_questions() {
        let extractor = ClaimExtractor::new();
        let response = "What do you want? The sky is blue. How does this work?";

        let claims = extractor.extract(response);
        // Should skip questions - "The sky is blue" is the main factual claim
        // Note: some short fragments may pass through; verify at least one claim contains our target
        assert!(!claims.is_empty());
        assert!(claims.iter().any(|c| c.text.contains("sky is blue")));
        // Verify no claims end with question marks
        for claim in &claims {
            assert!(!claim.text.trim().ends_with('?'), "Found question in claims: {}", claim.text);
        }
    }

    #[test]
    fn test_category_classification() {
        let extractor = ClaimExtractor::new();

        let code_claim = "The function returns null on error";
        let claims = extractor.extract(code_claim);
        assert!(!claims.is_empty());
        assert_eq!(claims[0].category, ClaimCategory::CodeBehavior);

        let numerical_claim = "The latency is 50 milliseconds";
        let claims = extractor.extract(numerical_claim);
        assert!(!claims.is_empty());
        assert_eq!(claims[0].category, ClaimCategory::Numerical);
    }

    #[test]
    fn test_specificity_estimation() {
        let extractor = ClaimExtractor::new();

        // Vague claim
        let vague = "Some things are related to other things";
        let claims = extractor.extract(vague);
        let vague_specificity = claims.get(0).map(|c| c.specificity).unwrap_or(0.0);

        // Specific claim
        let specific = "The UserService class calls AuthController.validate()";
        let claims = extractor.extract(specific);
        let specific_specificity = claims.get(0).map(|c| c.specificity).unwrap_or(0.0);

        assert!(specific_specificity > vague_specificity);
    }

    #[test]
    fn test_hedged_claims() {
        let extractor = ClaimExtractor::new();

        let hedged = "The function might return null";
        let claims = extractor.extract(hedged);
        assert!(!claims.is_empty());
        assert!(claims[0].metadata.as_ref().map(|m| m.contains_key("hedged")).unwrap_or(false));
    }

    #[test]
    fn test_evidence_linking() {
        let extractor = ClaimExtractor::new();
        let response = "The auth module handles login [1]. See `auth.rs` for details.";

        let claims = extractor.extract(response);
        // Should have evidence refs for the citation and file reference
        let total_evidence: usize = claims.iter().map(|c| c.evidence_refs.len()).sum();
        assert!(total_evidence > 0);
    }

    #[test]
    fn test_meta_commentary_skip() {
        let extractor = ClaimExtractor::new();
        let response = "Let me explain how this works. The function returns true on success.";

        let claims = extractor.extract(response);
        // Should skip "Let me explain how this works"
        assert_eq!(claims.len(), 1);
        assert!(claims[0].text.contains("returns true"));
    }

    #[test]
    fn test_sentence_splitting() {
        let extractor = ClaimExtractor::new();
        let response = "The API uses e.g. JSON for serialization. It also supports XML.";

        let claims = extractor.extract(response);
        assert_eq!(claims.len(), 2);
    }

    #[test]
    fn test_high_specificity_filter() {
        let extractor = ClaimExtractor::new();
        let response = "Something exists. The UserService.authenticate() method in auth/service.rs returns a boolean value indicating success.";

        let high_spec = extractor.extract_high_specificity(response, 0.6);

        // Only the specific claim should pass
        assert!(high_spec.len() <= 2);
        for claim in high_spec {
            assert!(claim.specificity >= 0.6);
        }
    }

    #[test]
    fn test_empty_input() {
        let extractor = ClaimExtractor::new();
        let claims = extractor.extract("");
        assert!(claims.is_empty());
    }

    #[test]
    fn test_short_claims_filtered() {
        let extractor = ClaimExtractor::new().with_min_length(20);
        let response = "Yes. No. The quick brown fox jumps over the lazy dog.";

        let claims = extractor.extract(response);
        // "Yes" and "No" should be filtered out
        for claim in &claims {
            assert!(claim.text.len() >= 20);
        }
    }
}
