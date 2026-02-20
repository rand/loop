//! Proof status extraction from Lean files.
//!
//! This module provides detailed analysis of proof status in Lean files,
//! including sorry detection, proof completeness, and evidence gathering.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::lean::{LeanRepl, LeanReplConfig};
use crate::repl::ReplEnvironment;

use super::types::{ProofStatus, SpecId, TheoremInfo};

/// Evidence gathered about a proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEvidence {
    /// The theorem this evidence is for.
    pub theorem_name: String,
    /// Lean file path.
    pub file: PathBuf,
    /// Line number.
    pub line: u32,
    /// Proof status.
    pub status: ProofStatus,
    /// Number of sorry placeholders.
    pub sorry_count: u32,
    /// Specific sorry locations (line numbers).
    pub sorry_locations: Vec<u32>,
    /// Tactics used in the proof.
    pub tactics_used: Vec<String>,
    /// Dependencies (other theorems referenced).
    pub dependencies: Vec<String>,
    /// Type-checking result (if verified).
    pub type_check_ok: Option<bool>,
    /// Type-checking error message.
    pub type_check_error: Option<String>,
    /// Proof text (truncated).
    pub proof_text: Option<String>,
}

impl ProofEvidence {
    /// Create new evidence for a theorem.
    pub fn new(theorem_name: impl Into<String>, file: impl Into<PathBuf>, line: u32) -> Self {
        Self {
            theorem_name: theorem_name.into(),
            file: file.into(),
            line,
            status: ProofStatus::Stated,
            sorry_count: 0,
            sorry_locations: Vec::new(),
            tactics_used: Vec::new(),
            dependencies: Vec::new(),
            type_check_ok: None,
            type_check_error: None,
            proof_text: None,
        }
    }

    /// Update status based on analysis.
    pub fn update_status(&mut self) {
        if let Some(false) = self.type_check_ok {
            self.status = ProofStatus::Failed;
        } else if self.sorry_count > 0 {
            self.status = ProofStatus::HasSorry;
        } else if self.proof_text.is_some() {
            self.status = ProofStatus::Complete;
        } else {
            self.status = ProofStatus::Stated;
        }
    }
}

/// Scanner for proof status in Lean files.
pub struct LeanProofScanner {
    /// Lean REPL for verification (optional).
    repl: Option<LeanRepl>,
    /// Whether to verify proofs with REPL.
    verify_proofs: bool,
    /// Common tactics to detect.
    #[allow(dead_code)] // Reserved for future tactic analysis
    known_tactics: Vec<String>,
}

impl LeanProofScanner {
    /// Create a new proof scanner.
    pub fn new() -> Self {
        Self {
            repl: None,
            verify_proofs: false,
            known_tactics: vec![
                "intro".to_string(),
                "apply".to_string(),
                "exact".to_string(),
                "simp".to_string(),
                "rfl".to_string(),
                "trivial".to_string(),
                "decide".to_string(),
                "omega".to_string(),
                "aesop".to_string(),
                "cases".to_string(),
                "induction".to_string(),
                "constructor".to_string(),
                "exists".to_string(),
                "have".to_string(),
                "let".to_string(),
                "show".to_string(),
                "calc".to_string(),
                "rw".to_string(),
                "rewrite".to_string(),
                "unfold".to_string(),
                "ring".to_string(),
                "linarith".to_string(),
                "norm_num".to_string(),
            ],
        }
    }

    /// Enable proof verification with Lean REPL.
    pub fn with_verification(mut self, config: LeanReplConfig) -> Result<Self> {
        self.repl = Some(LeanRepl::spawn(config)?);
        self.verify_proofs = true;
        Ok(self)
    }

    /// Scan a Lean file for proof status.
    pub fn scan_file(&self, path: &Path) -> Result<Vec<ProofEvidence>> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::Internal(format!("Failed to read file {}: {}", path.display(), e))
        })?;
        self.scan_content(&content, path)
    }

    /// Scan Lean content for proof status.
    pub fn scan_content(&self, content: &str, file_path: &Path) -> Result<Vec<ProofEvidence>> {
        let mut results = Vec::new();

        // Regex patterns
        let theorem_re = Regex::new(r"(?m)^\s*(theorem|lemma)\s+(\w+)").unwrap();
        let sorry_re = Regex::new(r"\bsorry\b").unwrap();
        let tactic_re = Regex::new(r"\b(intro|apply|exact|simp|rfl|trivial|decide|omega|aesop|cases|induction|constructor|exists|have|let|show|calc|rw|rewrite|unfold|ring|linarith|norm_num)\b").unwrap();
        let dep_re = Regex::new(r"\b([A-Z]\w*\.\w+|\w+_spec|\w+_correct|\w+_valid)\b").unwrap();

        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            if let Some(caps) = theorem_re.captures(line) {
                let theorem_name = caps.get(2).unwrap().as_str();
                let mut evidence = ProofEvidence::new(theorem_name, file_path, (i + 1) as u32);

                // Find the proof body
                let proof_body = self.extract_proof_body(&lines, i);

                if let Some(ref body) = proof_body {
                    // Count sorries
                    evidence.sorry_count = sorry_re.find_iter(body).count() as u32;

                    // Find sorry line numbers
                    for (j, proof_line) in body.lines().enumerate() {
                        if sorry_re.is_match(proof_line) {
                            evidence.sorry_locations.push((i + 1 + j) as u32);
                        }
                    }

                    // Extract tactics
                    for caps in tactic_re.captures_iter(body) {
                        let tactic = caps.get(1).unwrap().as_str().to_string();
                        if !evidence.tactics_used.contains(&tactic) {
                            evidence.tactics_used.push(tactic);
                        }
                    }

                    // Extract dependencies
                    for caps in dep_re.captures_iter(body) {
                        let dep = caps.get(1).unwrap().as_str().to_string();
                        if !evidence.dependencies.contains(&dep) && dep != theorem_name {
                            evidence.dependencies.push(dep);
                        }
                    }

                    // Store truncated proof text
                    let truncated = if body.len() > 500 {
                        format!("{}...", &body[..500])
                    } else {
                        body.to_string()
                    };
                    evidence.proof_text = Some(truncated);
                }

                evidence.update_status();
                results.push(evidence);
            }
        }

        Ok(results)
    }

    /// Extract the proof body for a theorem starting at line index.
    fn extract_proof_body(&self, lines: &[&str], start: usize) -> Option<String> {
        let mut in_proof = false;
        let mut brace_depth = 0;
        let mut proof_lines = Vec::new();

        for line in lines.iter().skip(start) {
            let trimmed = line.trim();

            // Start of proof
            if !in_proof && (trimmed.contains(":=") || trimmed.contains(" by")) {
                in_proof = true;
            }

            if in_proof {
                proof_lines.push(*line);

                // Track braces for structured proofs
                brace_depth += line.matches('{').count();
                brace_depth = brace_depth.saturating_sub(line.matches('}').count());

                // End conditions
                if brace_depth == 0 {
                    // Simple proof ended
                    if trimmed.is_empty() && proof_lines.len() > 1 {
                        break;
                    }
                    // Next theorem/definition
                    if proof_lines.len() > 1
                        && (trimmed.starts_with("theorem")
                            || trimmed.starts_with("lemma")
                            || trimmed.starts_with("def")
                            || trimmed.starts_with("structure")
                            || trimmed.starts_with("namespace")
                            || trimmed.starts_with("end"))
                    {
                        proof_lines.pop(); // Remove the next declaration
                        break;
                    }
                }
            }
        }

        if proof_lines.is_empty() {
            None
        } else {
            Some(proof_lines.join("\n"))
        }
    }

    /// Verify a theorem using the Lean REPL.
    pub fn verify_theorem(&mut self, file_path: &Path, theorem_name: &str) -> Result<bool> {
        let Some(ref mut repl) = self.repl else {
            return Ok(true); // No REPL, assume OK
        };

        let content = std::fs::read_to_string(file_path).map_err(|e| {
            Error::Internal(format!(
                "Failed to read file {}: {}",
                file_path.display(),
                e
            ))
        })?;

        // Try to type-check the file
        let response = repl.execute(&content)?;

        // Check if there were errors related to our theorem
        if let Some(ref error) = response.error {
            if error.contains(theorem_name) {
                return Ok(false);
            }
        }

        // Also check stderr for error messages about the theorem
        if response.stderr.contains(theorem_name) && response.stderr.contains("error") {
            return Ok(false);
        }

        Ok(response.success)
    }

    /// Scan multiple files and return combined results.
    pub fn scan_files(&self, paths: &[PathBuf]) -> Result<HashMap<PathBuf, Vec<ProofEvidence>>> {
        let mut results = HashMap::new();

        for path in paths {
            let evidence = self.scan_file(path)?;
            if !evidence.is_empty() {
                results.insert(path.clone(), evidence);
            }
        }

        Ok(results)
    }

    /// Get aggregate statistics.
    pub fn statistics(&self, evidence_list: &[ProofEvidence]) -> ProofStatistics {
        let mut stats = ProofStatistics::default();
        stats.total_theorems = evidence_list.len();

        for evidence in evidence_list {
            match evidence.status {
                ProofStatus::Complete => stats.complete += 1,
                ProofStatus::HasSorry => {
                    stats.has_sorry += 1;
                    stats.total_sorries += evidence.sorry_count as usize;
                }
                ProofStatus::Stated => stats.stated += 1,
                ProofStatus::Failed => stats.failed += 1,
                ProofStatus::NotFormalized => stats.not_formalized += 1,
            }

            for tactic in &evidence.tactics_used {
                *stats.tactic_usage.entry(tactic.clone()).or_insert(0) += 1;
            }
        }

        stats
    }
}

impl Default for LeanProofScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregate statistics about proofs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProofStatistics {
    /// Total theorems analyzed.
    pub total_theorems: usize,
    /// Complete proofs.
    pub complete: usize,
    /// Proofs with sorry.
    pub has_sorry: usize,
    /// Stated but not proven.
    pub stated: usize,
    /// Failed proofs.
    pub failed: usize,
    /// Not formalized.
    pub not_formalized: usize,
    /// Total sorry count across all proofs.
    pub total_sorries: usize,
    /// Tactic usage counts.
    pub tactic_usage: HashMap<String, usize>,
}

impl ProofStatistics {
    /// Completion percentage.
    pub fn completion_percentage(&self) -> f64 {
        if self.total_theorems == 0 {
            0.0
        } else {
            (self.complete as f64 / self.total_theorems as f64) * 100.0
        }
    }

    /// Most used tactics.
    pub fn top_tactics(&self, n: usize) -> Vec<(String, usize)> {
        let mut tactics: Vec<_> = self
            .tactic_usage
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        tactics.sort_by(|a, b| b.1.cmp(&a.1));
        tactics.truncate(n);
        tactics
    }
}

/// Map SPEC-XX.YY to proof evidence.
pub fn map_specs_to_evidence(
    specs: &HashMap<SpecId, TheoremInfo>,
    evidence: &[ProofEvidence],
) -> HashMap<SpecId, ProofEvidence> {
    let evidence_by_name: HashMap<&str, &ProofEvidence> = evidence
        .iter()
        .map(|e| (e.theorem_name.as_str(), e))
        .collect();

    let mut result = HashMap::new();

    for (spec_id, theorem_info) in specs {
        if let Some(&ev) = evidence_by_name.get(theorem_info.name.as_str()) {
            result.insert(spec_id.clone(), ev.clone());
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_lean_content() {
        let content = r#"
theorem simple_proof : 1 + 1 = 2 := by
  rfl

theorem with_sorry : forall n, n >= 0 := by
  intro n
  sorry

theorem complex_proof (h : P) : P ∨ Q := by
  apply Or.intro_left
  exact h
"#;

        let scanner = LeanProofScanner::new();
        let evidence = scanner
            .scan_content(content, Path::new("test.lean"))
            .unwrap();

        assert_eq!(evidence.len(), 3);

        // simple_proof - complete
        assert_eq!(evidence[0].theorem_name, "simple_proof");
        assert_eq!(evidence[0].status, ProofStatus::Complete);
        assert_eq!(evidence[0].sorry_count, 0);

        // with_sorry - has sorry
        assert_eq!(evidence[1].theorem_name, "with_sorry");
        assert_eq!(evidence[1].status, ProofStatus::HasSorry);
        assert_eq!(evidence[1].sorry_count, 1);

        // complex_proof - complete
        assert_eq!(evidence[2].theorem_name, "complex_proof");
        assert_eq!(evidence[2].status, ProofStatus::Complete);
        assert!(evidence[2].tactics_used.contains(&"apply".to_string()));
        assert!(evidence[2].tactics_used.contains(&"exact".to_string()));
    }

    #[test]
    fn test_proof_statistics() {
        let evidence = vec![
            ProofEvidence {
                theorem_name: "t1".to_string(),
                file: PathBuf::from("t.lean"),
                line: 1,
                status: ProofStatus::Complete,
                sorry_count: 0,
                sorry_locations: vec![],
                tactics_used: vec!["simp".to_string(), "rfl".to_string()],
                dependencies: vec![],
                type_check_ok: Some(true),
                type_check_error: None,
                proof_text: Some("proof".to_string()),
            },
            ProofEvidence {
                theorem_name: "t2".to_string(),
                file: PathBuf::from("t.lean"),
                line: 10,
                status: ProofStatus::HasSorry,
                sorry_count: 2,
                sorry_locations: vec![12, 15],
                tactics_used: vec!["intro".to_string(), "simp".to_string()],
                dependencies: vec![],
                type_check_ok: Some(true),
                type_check_error: None,
                proof_text: Some("proof with sorry".to_string()),
            },
        ];

        let scanner = LeanProofScanner::new();
        let stats = scanner.statistics(&evidence);

        assert_eq!(stats.total_theorems, 2);
        assert_eq!(stats.complete, 1);
        assert_eq!(stats.has_sorry, 1);
        assert_eq!(stats.total_sorries, 2);
        assert_eq!(stats.tactic_usage.get("simp"), Some(&2));
    }

    #[test]
    fn test_proof_evidence_status_update() {
        let mut evidence = ProofEvidence::new("test", "test.lean", 1);

        // No proof text -> Stated
        evidence.update_status();
        assert_eq!(evidence.status, ProofStatus::Stated);

        // With proof text -> Complete
        evidence.proof_text = Some("by rfl".to_string());
        evidence.update_status();
        assert_eq!(evidence.status, ProofStatus::Complete);

        // With sorry -> HasSorry
        evidence.sorry_count = 1;
        evidence.update_status();
        assert_eq!(evidence.status, ProofStatus::HasSorry);

        // Type check failed -> Failed
        evidence.type_check_ok = Some(false);
        evidence.update_status();
        assert_eq!(evidence.status, ProofStatus::Failed);
    }
}
