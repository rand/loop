//! Evidence scrubbing for p0 estimation.
//!
//! The key insight of the Strawberry/Pythea methodology is that we need to
//! estimate what the model would say *without* seeing the evidence. This
//! module provides utilities to mask or remove evidence from context before
//! re-prompting for p0 estimation.

use regex::Regex;
use std::collections::HashSet;

/// Types of evidence that can be scrubbed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrubTarget {
    /// Code snippets and file contents
    Code,
    /// Tool outputs (REPL results, search results)
    ToolOutput,
    /// Citations and references
    Citations,
    /// File paths and URLs
    Paths,
    /// Specific quoted text
    Quotes,
    /// Numerical data
    Numbers,
    /// All evidence types
    All,
}

/// Configuration for evidence scrubbing.
#[derive(Debug, Clone)]
pub struct ScrubConfig {
    /// Types of evidence to scrub
    pub targets: HashSet<ScrubTarget>,
    /// Placeholder text for scrubbed content
    pub placeholder: String,
    /// Whether to preserve structure (e.g., keep code block markers)
    pub preserve_structure: bool,
    /// Minimum length for content to be scrubbed
    pub min_length: usize,
    /// Custom patterns to scrub (regex)
    pub custom_patterns: Vec<String>,
}

impl Default for ScrubConfig {
    fn default() -> Self {
        Self {
            targets: [
                ScrubTarget::Code,
                ScrubTarget::ToolOutput,
                ScrubTarget::Citations,
            ]
            .into_iter()
            .collect(),
            placeholder: "[EVIDENCE REDACTED]".to_string(),
            preserve_structure: true,
            min_length: 10,
            custom_patterns: Vec::new(),
        }
    }
}

impl ScrubConfig {
    /// Create a config that scrubs all evidence.
    pub fn aggressive() -> Self {
        Self {
            targets: [ScrubTarget::All].into_iter().collect(),
            placeholder: "[REDACTED]".to_string(),
            preserve_structure: false,
            min_length: 5,
            custom_patterns: Vec::new(),
        }
    }

    /// Create a config that only scrubs code.
    pub fn code_only() -> Self {
        Self {
            targets: [ScrubTarget::Code].into_iter().collect(),
            placeholder: "[CODE REDACTED]".to_string(),
            preserve_structure: true,
            min_length: 10,
            custom_patterns: Vec::new(),
        }
    }

    /// Add a custom pattern to scrub.
    pub fn with_custom_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.custom_patterns.push(pattern.into());
        self
    }

    /// Set the placeholder text.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }
}

/// Scrub evidence from text based on configuration.
pub struct EvidenceScrubber {
    config: ScrubConfig,
    code_block_re: Regex,
    inline_code_re: Regex,
    tool_output_re: Regex,
    citation_re: Regex,
    path_re: Regex,
    url_re: Regex,
    quote_re: Regex,
    number_re: Regex,
}

impl EvidenceScrubber {
    /// Create a new scrubber with the given configuration.
    pub fn new(config: ScrubConfig) -> Self {
        Self {
            config,
            // Matches fenced code blocks: ```lang\n...\n```
            code_block_re: Regex::new(r"(?s)```[a-zA-Z]*\n.*?\n```").unwrap(),
            // Matches inline code: `...`
            inline_code_re: Regex::new(r"`[^`]+`").unwrap(),
            // Matches tool output sections (common patterns)
            tool_output_re: Regex::new(
                r"(?s)(?:Output|Result|Response|REPL):\s*\n?```.*?```|\{[^}]{20,}\}",
            )
            .unwrap(),
            // Matches citations: [1], [source: ...], (ref: ...)
            citation_re: Regex::new(r"\[(?:\d+|source:[^\]]+)\]|\(ref:[^)]+\)").unwrap(),
            // Matches file paths
            path_re: Regex::new(r"(?:/[a-zA-Z0-9_.-]+)+(?:\.[a-zA-Z]+)?|[a-zA-Z]:\\[^\s]+")
                .unwrap(),
            // Matches URLs
            url_re: Regex::new(r"https?://[^\s]+").unwrap(),
            // Matches quoted text
            quote_re: Regex::new(r#""[^"]{10,}"|'[^']{10,}'"#).unwrap(),
            // Matches significant numbers (more than just single digits)
            number_re: Regex::new(r"\b\d{2,}\b|\b\d+\.\d+\b|\b0x[0-9a-fA-F]+\b").unwrap(),
        }
    }

    /// Create a scrubber with default configuration.
    pub fn default_scrubber() -> Self {
        Self::new(ScrubConfig::default())
    }

    /// Scrub evidence from the given text.
    pub fn scrub(&self, text: &str) -> ScrubResult {
        let mut result = text.to_string();
        let mut scrubbed_items = Vec::new();

        let should_scrub = |target: ScrubTarget| {
            self.config.targets.contains(&ScrubTarget::All) || self.config.targets.contains(&target)
        };

        // Scrub code blocks first (they're the largest)
        if should_scrub(ScrubTarget::Code) {
            let items = self.scrub_code_blocks(&mut result);
            scrubbed_items.extend(items);
        }

        // Scrub tool outputs
        if should_scrub(ScrubTarget::ToolOutput) {
            let items = self.scrub_tool_outputs(&mut result);
            scrubbed_items.extend(items);
        }

        // Scrub inline code (after blocks, to avoid double-scrubbing)
        if should_scrub(ScrubTarget::Code) {
            let items = self.scrub_inline_code(&mut result);
            scrubbed_items.extend(items);
        }

        // Scrub citations
        if should_scrub(ScrubTarget::Citations) {
            let items = self.scrub_citations(&mut result);
            scrubbed_items.extend(items);
        }

        // Scrub paths
        if should_scrub(ScrubTarget::Paths) {
            let items = self.scrub_paths(&mut result);
            scrubbed_items.extend(items);
        }

        // Scrub quotes
        if should_scrub(ScrubTarget::Quotes) {
            let items = self.scrub_quotes(&mut result);
            scrubbed_items.extend(items);
        }

        // Scrub numbers
        if should_scrub(ScrubTarget::Numbers) {
            let items = self.scrub_numbers(&mut result);
            scrubbed_items.extend(items);
        }

        // Apply custom patterns
        for pattern in &self.config.custom_patterns {
            if let Ok(re) = Regex::new(pattern) {
                let items = self.scrub_pattern(&mut result, &re, "custom");
                scrubbed_items.extend(items);
            }
        }

        ScrubResult {
            scrubbed_text: result,
            original_text: text.to_string(),
            scrubbed_items,
        }
    }

    fn scrub_code_blocks(&self, text: &mut String) -> Vec<ScrubbedItem> {
        let mut items = Vec::new();

        let placeholder = if self.config.preserve_structure {
            format!("```\n{}\n```", self.config.placeholder)
        } else {
            self.config.placeholder.clone()
        };

        while let Some(cap) = self.code_block_re.find(text) {
            let content = cap.as_str().to_string();
            if content.len() >= self.config.min_length {
                items.push(ScrubbedItem {
                    content,
                    item_type: ScrubTarget::Code,
                    start: cap.start(),
                    end: cap.end(),
                });
                *text = format!(
                    "{}{}{}",
                    &text[..cap.start()],
                    placeholder,
                    &text[cap.end()..]
                );
            } else {
                break; // No more matches
            }
        }

        items
    }

    fn scrub_inline_code(&self, text: &mut String) -> Vec<ScrubbedItem> {
        self.scrub_pattern(text, &self.inline_code_re, "code")
    }

    fn scrub_tool_outputs(&self, text: &mut String) -> Vec<ScrubbedItem> {
        self.scrub_pattern(text, &self.tool_output_re, "tool_output")
    }

    fn scrub_citations(&self, text: &mut String) -> Vec<ScrubbedItem> {
        self.scrub_pattern(text, &self.citation_re, "citation")
    }

    fn scrub_paths(&self, text: &mut String) -> Vec<ScrubbedItem> {
        let mut items = Vec::new();

        // First URLs, then file paths
        items.extend(self.scrub_pattern(text, &self.url_re, "url"));
        items.extend(self.scrub_pattern(text, &self.path_re, "path"));

        items
    }

    fn scrub_quotes(&self, text: &mut String) -> Vec<ScrubbedItem> {
        self.scrub_pattern(text, &self.quote_re, "quote")
    }

    fn scrub_numbers(&self, text: &mut String) -> Vec<ScrubbedItem> {
        self.scrub_pattern(text, &self.number_re, "number")
    }

    fn scrub_pattern(&self, text: &mut String, re: &Regex, type_name: &str) -> Vec<ScrubbedItem> {
        let mut items = Vec::new();
        let target = match type_name {
            "code" => ScrubTarget::Code,
            "tool_output" => ScrubTarget::ToolOutput,
            "citation" => ScrubTarget::Citations,
            "url" | "path" => ScrubTarget::Paths,
            "quote" => ScrubTarget::Quotes,
            "number" => ScrubTarget::Numbers,
            _ => ScrubTarget::All,
        };

        // Find all matches first - collect positions and content
        let matches: Vec<(usize, usize, String)> = re
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str().to_string()))
            .collect();

        // Replace in reverse order to preserve indices
        for (start, end, content) in matches.into_iter().rev() {
            if content.len() >= self.config.min_length || target == ScrubTarget::Citations {
                items.push(ScrubbedItem {
                    content,
                    item_type: target,
                    start,
                    end,
                });
                *text = format!(
                    "{}{}{}",
                    &text[..start],
                    self.config.placeholder,
                    &text[end..]
                );
            }
        }

        items.reverse(); // Put back in original order
        items
    }

    /// Scrub specific evidence items by their references.
    pub fn scrub_specific(&self, text: &str, evidence_refs: &[&str]) -> ScrubResult {
        let mut result = text.to_string();
        let mut scrubbed_items = Vec::new();

        for evidence in evidence_refs {
            if let Some(pos) = result.find(evidence) {
                scrubbed_items.push(ScrubbedItem {
                    content: evidence.to_string(),
                    item_type: ScrubTarget::All,
                    start: pos,
                    end: pos + evidence.len(),
                });
                result = result.replace(evidence, &self.config.placeholder);
            }
        }

        ScrubResult {
            scrubbed_text: result,
            original_text: text.to_string(),
            scrubbed_items,
        }
    }
}

/// Result of a scrubbing operation.
#[derive(Debug, Clone)]
pub struct ScrubResult {
    /// Text after scrubbing
    pub scrubbed_text: String,
    /// Original text
    pub original_text: String,
    /// Items that were scrubbed
    pub scrubbed_items: Vec<ScrubbedItem>,
}

impl ScrubResult {
    /// Check if any evidence was scrubbed.
    pub fn has_scrubbed_content(&self) -> bool {
        !self.scrubbed_items.is_empty()
    }

    /// Get the number of items scrubbed.
    pub fn scrubbed_count(&self) -> usize {
        self.scrubbed_items.len()
    }

    /// Get total characters scrubbed.
    pub fn total_chars_scrubbed(&self) -> usize {
        self.scrubbed_items.iter().map(|i| i.content.len()).sum()
    }

    /// Get scrubbed items by type.
    pub fn items_by_type(&self, target: ScrubTarget) -> Vec<&ScrubbedItem> {
        self.scrubbed_items
            .iter()
            .filter(|i| i.item_type == target)
            .collect()
    }

    /// Restore a specific scrubbed item.
    pub fn restore_item(&self, index: usize) -> Option<String> {
        self.scrubbed_items.get(index).map(|_item| {
            // This is a simplified restoration; full implementation would track positions
            self.scrubbed_text.clone()
        })
    }
}

/// An item that was scrubbed from the text.
#[derive(Debug, Clone)]
pub struct ScrubbedItem {
    /// Original content that was scrubbed
    pub content: String,
    /// Type of evidence
    pub item_type: ScrubTarget,
    /// Start position in original text
    pub start: usize,
    /// End position in original text
    pub end: usize,
}

impl ScrubbedItem {
    /// Get the length of the scrubbed content.
    pub fn len(&self) -> usize {
        self.content.len()
    }

    /// Check if the item is empty.
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

/// Create a prompt for p0 estimation (prior without evidence).
///
/// This creates a modified prompt where evidence has been scrubbed,
/// allowing us to estimate what the model would say without seeing
/// the specific evidence.
pub fn create_p0_prompt(
    original_context: &str,
    claim: &str,
    scrubber: &EvidenceScrubber,
) -> P0Prompt {
    let scrub_result = scrubber.scrub(original_context);

    let prompt = format!(
        r#"Given this context (with some details omitted):

{}

Would the following claim be true? Answer with a probability estimate (0.0-1.0):

Claim: "{}"

Respond with just the probability (e.g., "0.7") and a brief explanation."#,
        scrub_result.scrubbed_text, claim
    );

    P0Prompt {
        prompt,
        scrub_result,
        claim: claim.to_string(),
    }
}

/// Prompt for p0 estimation.
#[derive(Debug, Clone)]
pub struct P0Prompt {
    /// The actual prompt to send
    pub prompt: String,
    /// Details of what was scrubbed
    pub scrub_result: ScrubResult,
    /// The claim being evaluated
    pub claim: String,
}

impl P0Prompt {
    /// Get the evidence that was hidden.
    pub fn hidden_evidence(&self) -> Vec<String> {
        self.scrub_result
            .scrubbed_items
            .iter()
            .map(|i| i.content.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrub_code_blocks() {
        let scrubber = EvidenceScrubber::default_scrubber();
        let text = r#"The function works like this:
```python
def foo():
    return 42
```
It returns 42."#;

        let result = scrubber.scrub(text);
        assert!(result.has_scrubbed_content());
        assert!(!result.scrubbed_text.contains("def foo"));
        assert!(result.scrubbed_text.contains("[EVIDENCE REDACTED]"));
    }

    #[test]
    fn test_scrub_inline_code() {
        let scrubber = EvidenceScrubber::default_scrubber();
        let text = "The `authenticate()` function validates credentials.";

        let result = scrubber.scrub(text);
        assert!(result.has_scrubbed_content());
    }

    #[test]
    fn test_scrub_citations() {
        let scrubber = EvidenceScrubber::default_scrubber();
        let text = "According to the documentation [1], this is correct.";

        let result = scrubber.scrub(text);
        assert!(result.has_scrubbed_content());
        assert!(!result.scrubbed_text.contains("[1]"));
    }

    #[test]
    fn test_scrub_paths() {
        let mut config = ScrubConfig::default();
        config.targets.insert(ScrubTarget::Paths);
        let scrubber = EvidenceScrubber::new(config);

        let text = "The file is at /usr/local/bin/myapp and https://example.com/api";

        let result = scrubber.scrub(text);
        assert!(result.has_scrubbed_content());
    }

    #[test]
    fn test_preserve_structure() {
        let config = ScrubConfig {
            preserve_structure: true,
            ..Default::default()
        };
        let scrubber = EvidenceScrubber::new(config);

        let text = "```python\ndef foo(): pass\n```";
        let result = scrubber.scrub(text);

        // Should still have code block markers
        assert!(result.scrubbed_text.contains("```"));
    }

    #[test]
    fn test_aggressive_scrub() {
        let scrubber = EvidenceScrubber::new(ScrubConfig::aggressive());

        let text = r#"The value is 42. See /path/to/file.txt for details.
```
code here
```
As stated in "this long quoted text here"."#;

        let result = scrubber.scrub(text);
        assert!(result.scrubbed_count() >= 3);
    }

    #[test]
    fn test_scrub_specific() {
        let scrubber = EvidenceScrubber::default_scrubber();
        let text = "The function foo() returns bar() result.";

        let result = scrubber.scrub_specific(text, &["foo()", "bar()"]);
        assert!(!result.scrubbed_text.contains("foo()"));
        assert!(!result.scrubbed_text.contains("bar()"));
    }

    #[test]
    fn test_custom_pattern() {
        let config = ScrubConfig::default().with_custom_pattern(r"\bAPI_KEY_\w+\b");
        let scrubber = EvidenceScrubber::new(config);

        let text = "Use API_KEY_12345 to authenticate.";
        let result = scrubber.scrub(text);

        assert!(result.has_scrubbed_content());
        assert!(!result.scrubbed_text.contains("API_KEY_12345"));
    }

    #[test]
    fn test_p0_prompt_creation() {
        let scrubber = EvidenceScrubber::default_scrubber();
        let context = r#"The code shows:
```rust
fn validate() -> bool { true }
```
This validates the input."#;

        let p0_prompt = create_p0_prompt(context, "The validate function returns a boolean", &scrubber);

        assert!(!p0_prompt.prompt.contains("fn validate"));
        assert!(p0_prompt.prompt.contains("REDACTED"));
        assert!(!p0_prompt.hidden_evidence().is_empty());
    }

    #[test]
    fn test_empty_scrub() {
        let scrubber = EvidenceScrubber::default_scrubber();
        let text = "A simple sentence with no code or citations.";

        let result = scrubber.scrub(text);
        // May or may not have scrubbed content depending on patterns
        assert_eq!(result.scrubbed_text.len(), text.len() - result.total_chars_scrubbed() + result.scrubbed_count() * "[EVIDENCE REDACTED]".len());
    }

    #[test]
    fn test_items_by_type() {
        let mut config = ScrubConfig::default();
        config.targets.insert(ScrubTarget::Paths);
        let scrubber = EvidenceScrubber::new(config);

        let text = r#"See `/path/file.txt` and `some_code()` for details."#;
        let result = scrubber.scrub(text);

        let code_items = result.items_by_type(ScrubTarget::Code);
        assert!(!code_items.is_empty());
    }
}
