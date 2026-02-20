//! Spec coverage tracking for DP integration.
//!
//! This module provides scanning and tracking of SPEC-XX.YY coverage
//! across Lean formalizations and tests.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::error::Result;

use super::types::{
    CoverageReport, CoverageSummary, ProofStatus, SpecCoverage, SpecId, TestTrace, TheoremInfo,
};

/// Scanner for spec coverage across project files.
pub struct CoverageScanner {
    /// Project root directory.
    project_root: PathBuf,
    /// Patterns for Lean files.
    lean_patterns: Vec<String>,
    /// Patterns for spec files.
    spec_patterns: Vec<String>,
    /// Patterns for test files.
    test_patterns: Vec<String>,
}

impl CoverageScanner {
    /// Create a new coverage scanner.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            lean_patterns: vec!["**/*.lean".to_string()],
            spec_patterns: vec![
                "docs/spec/**/*.md".to_string(),
                "spec/**/*.md".to_string(),
                "specs/**/*.md".to_string(),
            ],
            test_patterns: vec![
                "**/*_test.rs".to_string(),
                "**/test_*.rs".to_string(),
                "**/tests/**/*.rs".to_string(),
                "**/*_test.go".to_string(),
                "**/test_*.py".to_string(),
                "**/tests/**/*.py".to_string(),
            ],
        }
    }

    /// Add a Lean file pattern.
    pub fn with_lean_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.lean_patterns.push(pattern.into());
        self
    }

    /// Add a spec file pattern.
    pub fn with_spec_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.spec_patterns.push(pattern.into());
        self
    }

    /// Add a test file pattern.
    pub fn with_test_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.test_patterns.push(pattern.into());
        self
    }

    /// Scan and generate a coverage report.
    pub fn scan(&self) -> Result<CoverageReport> {
        let mut report = CoverageReport::new(&self.project_root);

        // Find all files
        let lean_files = self.find_files(&self.lean_patterns)?;
        let spec_files = self.find_files(&self.spec_patterns)?;
        let test_files = self.find_files(&self.test_patterns)?;

        report.lean_files_scanned = lean_files.clone();
        report.spec_files_scanned = spec_files.clone();

        // Extract specs from spec files
        let mut specs_map: HashMap<SpecId, SpecCoverage> = HashMap::new();
        for spec_file in &spec_files {
            let extracted = self.extract_specs_from_file(spec_file)?;
            for (spec_id, text, line) in extracted {
                let mut coverage = SpecCoverage::new(spec_id.clone(), text);
                coverage.spec_source = Some(spec_file.clone());
                coverage.spec_line = Some(line);
                specs_map.insert(spec_id, coverage);
            }
        }

        // Extract theorem-spec mappings from Lean files
        for lean_file in &lean_files {
            let theorems = self.extract_theorems_from_lean(lean_file)?;
            for (spec_id, theorem) in theorems {
                if let Some(coverage) = specs_map.get_mut(&spec_id) {
                    coverage.add_theorem(theorem);
                } else {
                    // Spec found in Lean but not in spec files
                    let mut coverage = SpecCoverage::new(spec_id.clone(), "(from Lean file)");
                    coverage.add_theorem(theorem);
                    specs_map.insert(spec_id, coverage);
                }
            }
        }

        // Extract test traces
        for test_file in &test_files {
            let traces = self.extract_test_traces(test_file)?;
            for (spec_id, trace) in traces {
                if let Some(coverage) = specs_map.get_mut(&spec_id) {
                    coverage.add_test_trace(trace);
                }
            }
        }

        // Build report
        let mut specs: Vec<_> = specs_map.into_values().collect();
        specs.sort_by(|a, b| {
            a.spec_id
                .major
                .cmp(&b.spec_id.major)
                .then(a.spec_id.minor.cmp(&b.spec_id.minor))
        });

        for spec in specs {
            report.add_spec(spec);
        }

        Ok(report)
    }

    /// Find files matching patterns.
    fn find_files(&self, patterns: &[String]) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();

        for pattern in patterns {
            let full_pattern = self.project_root.join(pattern);
            let pattern_str = full_pattern.to_string_lossy();

            if let Ok(paths) = glob::glob(&pattern_str) {
                for entry in paths.flatten() {
                    if entry.is_file() && !files.contains(&entry) {
                        files.push(entry);
                    }
                }
            }
        }

        Ok(files)
    }

    /// Extract SPEC-XX.YY definitions from a spec file.
    fn extract_specs_from_file(&self, path: &Path) -> Result<Vec<(SpecId, String, u32)>> {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let mut specs = Vec::new();

        // Pattern: [SPEC-XX.YY]: Description or SPEC-XX.YY: Description
        let re = Regex::new(r"(?m)^\s*\[?(SPEC-\d+\.\d+)\]?[:\s]+(.+?)(?:\n|$)").unwrap();

        for (line_num, line) in content.lines().enumerate() {
            if let Some(caps) = re.captures(line) {
                if let Some(spec_id) = SpecId::parse(caps.get(1).unwrap().as_str()) {
                    let text = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                    specs.push((spec_id, text.to_string(), (line_num + 1) as u32));
                }
            }
        }

        Ok(specs)
    }

    /// Extract theorems and their SPEC references from a Lean file.
    fn extract_theorems_from_lean(&self, path: &Path) -> Result<Vec<(SpecId, TheoremInfo)>> {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let mut results = Vec::new();

        // Track current namespace
        let mut current_namespace: Option<String> = None;

        // Pattern for namespace
        let ns_re = Regex::new(r"^\s*namespace\s+(\w+)").unwrap();
        let end_ns_re = Regex::new(r"^\s*end\s+(\w+)").unwrap();

        // Pattern for SPEC reference in doc comment: /-- SPEC-XX.YY: ... -/
        let spec_comment_re =
            Regex::new(r"(?s)/\-\-[^-]*?(SPEC-\d+\.\d+)[:\s]*([^-]*?)\-/").unwrap();

        // Pattern for theorem/lemma declaration
        let theorem_re = Regex::new(r"^\s*(theorem|lemma)\s+(\w+)").unwrap();

        // Pattern for sorry
        let sorry_re = Regex::new(r"\bsorry\b").unwrap();

        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Track namespace
            if let Some(caps) = ns_re.captures(line) {
                current_namespace = Some(caps.get(1).unwrap().as_str().to_string());
            } else if end_ns_re.is_match(line) {
                current_namespace = None;
            }

            // Look for SPEC reference in preceding doc comment
            // Check if this line or recent lines have a doc comment with SPEC
            let mut spec_id: Option<SpecId> = None;
            let mut spec_desc = String::new();

            // Look backwards for doc comment
            let lookback_start = i.saturating_sub(10);
            let lookback_text = lines[lookback_start..=i].join("\n");

            if let Some(caps) = spec_comment_re.captures(&lookback_text) {
                spec_id = SpecId::parse(caps.get(1).unwrap().as_str());
                spec_desc = caps
                    .get(2)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default();
            }

            // Check for theorem declaration
            if let Some(caps) = theorem_re.captures(line) {
                let theorem_name = caps.get(2).unwrap().as_str().to_string();

                // Find the proof body to check for sorry
                let proof_start = i;
                let mut proof_end = i;
                let mut brace_count = 0;
                let mut in_proof = false;

                for j in i..lines.len() {
                    let proof_line = lines[j];
                    if proof_line.contains(":=") || proof_line.contains("where") {
                        in_proof = true;
                    }
                    if in_proof {
                        brace_count += proof_line.matches('{').count();
                        brace_count = brace_count.saturating_sub(proof_line.matches('}').count());

                        // Simple heuristic: proof ends at blank line or next theorem
                        if j > proof_start
                            && (proof_line.trim().is_empty() || theorem_re.is_match(proof_line))
                            && brace_count == 0
                        {
                            break;
                        }
                    }
                    proof_end = j;
                }

                let proof_text = lines[proof_start..=proof_end].join("\n");
                let sorry_count = sorry_re.find_iter(&proof_text).count() as u32;

                let status = if sorry_count > 0 {
                    ProofStatus::HasSorry
                } else if proof_text.contains(":=") || proof_text.contains("by") {
                    ProofStatus::Complete
                } else {
                    ProofStatus::Stated
                };

                // If we found a SPEC reference, link it
                if let Some(sid) = spec_id {
                    let mut theorem = TheoremInfo::new(&theorem_name, path, (i + 1) as u32)
                        .with_status(status)
                        .with_sorry_count(sorry_count);

                    if let Some(ref ns) = current_namespace {
                        theorem = theorem.with_namespace(ns);
                    }

                    if !spec_desc.is_empty() {
                        theorem = theorem.with_statement(spec_desc);
                    }

                    results.push((sid, theorem));
                }
            }

            i += 1;
        }

        Ok(results)
    }

    /// Extract test traces from a test file.
    fn extract_test_traces(&self, path: &Path) -> Result<Vec<(SpecId, TestTrace)>> {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let mut results = Vec::new();

        // Pattern for @trace SPEC-XX.YY or // trace: SPEC-XX.YY
        let trace_re = Regex::new(r"(?:@trace|//\s*trace:?)\s*(SPEC-\d+\.\d+)").unwrap();

        // Pattern for test function names (Rust, Go, Python)
        let test_fn_re = Regex::new(
            r"(?m)^\s*(?:#\[test\]|func\s+Test|def\s+test_)\s*(?:\n\s*)?(?:fn\s+|)(\w+)",
        )
        .unwrap();

        let lines: Vec<&str> = content.lines().collect();
        let mut current_test: Option<(String, u32)> = None;

        for (line_num, line) in lines.iter().enumerate() {
            // Track test function
            if let Some(caps) = test_fn_re.captures(line) {
                let test_name = caps.get(1).unwrap().as_str().to_string();
                current_test = Some((test_name, (line_num + 1) as u32));
            }

            // Look for trace comment
            if let Some(caps) = trace_re.captures(line) {
                if let Some(spec_id) = SpecId::parse(caps.get(1).unwrap().as_str()) {
                    let (test_name, test_line) = current_test
                        .clone()
                        .unwrap_or(("unknown".to_string(), (line_num + 1) as u32));

                    let trace = TestTrace::new(test_name, path, test_line);
                    results.push((spec_id, trace));
                }
            }
        }

        Ok(results)
    }
}

/// Tracker for spec coverage with incremental updates.
pub struct SpecCoverageTracker {
    /// Current coverage data.
    coverage: HashMap<SpecId, SpecCoverage>,
    /// Project root.
    project_root: PathBuf,
}

impl SpecCoverageTracker {
    /// Create a new tracker.
    pub fn new(project_root: impl Into<PathBuf>) -> Self {
        Self {
            coverage: HashMap::new(),
            project_root: project_root.into(),
        }
    }

    /// Load from an existing report.
    pub fn from_report(report: CoverageReport) -> Self {
        let coverage = report
            .specs
            .into_iter()
            .map(|s| (s.spec_id.clone(), s))
            .collect();

        Self {
            coverage,
            project_root: report.project_root,
        }
    }

    /// Register a spec requirement.
    pub fn register_spec(&mut self, spec_id: SpecId, text: impl Into<String>) {
        self.coverage
            .entry(spec_id.clone())
            .or_insert_with(|| SpecCoverage::new(spec_id, text.into()));
    }

    /// Register a theorem for a spec.
    pub fn register_theorem(&mut self, spec_id: &SpecId, theorem: TheoremInfo) {
        if let Some(coverage) = self.coverage.get_mut(spec_id) {
            coverage.add_theorem(theorem);
        } else {
            let mut coverage = SpecCoverage::new(spec_id.clone(), "(auto-discovered)");
            coverage.add_theorem(theorem);
            self.coverage.insert(spec_id.clone(), coverage);
        }
    }

    /// Update proof status for a theorem.
    pub fn update_proof_status(
        &mut self,
        spec_id: &SpecId,
        theorem_name: &str,
        status: ProofStatus,
    ) {
        if let Some(coverage) = self.coverage.get_mut(spec_id) {
            for theorem in &mut coverage.theorems {
                if theorem.name == theorem_name {
                    theorem.status = status;
                }
            }
            // Recalculate overall status
            coverage.proof_status = Self::calculate_status(&coverage.theorems);
        }
    }

    /// Calculate overall status from theorems.
    fn calculate_status(theorems: &[TheoremInfo]) -> ProofStatus {
        if theorems.is_empty() {
            return ProofStatus::NotFormalized;
        }

        let mut has_failed = false;
        let mut has_sorry = false;
        let mut has_stated = false;
        let mut all_complete = true;

        for theorem in theorems {
            match theorem.status {
                ProofStatus::Failed => has_failed = true,
                ProofStatus::HasSorry => has_sorry = true,
                ProofStatus::Stated => has_stated = true,
                ProofStatus::Complete => {}
                ProofStatus::NotFormalized => all_complete = false,
            }
            if !matches!(theorem.status, ProofStatus::Complete) {
                all_complete = false;
            }
        }

        if has_failed {
            ProofStatus::Failed
        } else if has_sorry {
            ProofStatus::HasSorry
        } else if has_stated {
            ProofStatus::Stated
        } else if all_complete {
            ProofStatus::Complete
        } else {
            ProofStatus::Stated
        }
    }

    /// Get coverage for a specific spec.
    pub fn get(&self, spec_id: &SpecId) -> Option<&SpecCoverage> {
        self.coverage.get(spec_id)
    }

    /// Get all specs.
    pub fn all_specs(&self) -> Vec<&SpecCoverage> {
        self.coverage.values().collect()
    }

    /// Get specs by status.
    pub fn specs_by_status(&self, status: ProofStatus) -> Vec<&SpecCoverage> {
        self.coverage
            .values()
            .filter(|s| s.proof_status == status)
            .collect()
    }

    /// Generate a summary.
    pub fn summary(&self) -> CoverageSummary {
        let mut summary = CoverageSummary::default();
        summary.total_specs = self.coverage.len();

        for spec in self.coverage.values() {
            if spec.is_formalized() {
                summary.formalized_count += 1;
            }
            if !spec.test_traces.is_empty() {
                summary.tested_count += 1;
            }

            match spec.proof_status {
                ProofStatus::Complete => summary.complete_count += 1,
                ProofStatus::HasSorry => summary.has_sorry_count += 1,
                ProofStatus::Failed => summary.failed_count += 1,
                ProofStatus::Stated => summary.stated_count += 1,
                ProofStatus::NotFormalized => {}
            }
        }

        summary
    }

    /// Convert to a full report.
    pub fn to_report(&self) -> CoverageReport {
        let mut report = CoverageReport::new(&self.project_root);

        let mut specs: Vec<_> = self.coverage.values().cloned().collect();
        specs.sort_by(|a, b| {
            a.spec_id
                .major
                .cmp(&b.spec_id.major)
                .then(a.spec_id.minor.cmp(&b.spec_id.minor))
        });

        for spec in specs {
            report.add_spec(spec);
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_extract_specs_from_markdown() {
        let dir = TempDir::new().unwrap();
        let spec_content = r#"
# Auth Specification

[SPEC-01.01]: Users must authenticate before accessing resources
[SPEC-01.02]: Sessions expire after 30 minutes of inactivity

## Data Model

SPEC-02.01: User records contain email and hashed password
"#;
        create_test_file(dir.path(), "docs/spec/auth.md", spec_content);

        let scanner = CoverageScanner::new(dir.path());
        let specs = scanner
            .extract_specs_from_file(&dir.path().join("docs/spec/auth.md"))
            .unwrap();

        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].0, SpecId::new(1, 1));
        assert!(specs[0].1.contains("authenticate"));
    }

    #[test]
    fn test_extract_theorems_from_lean() {
        let dir = TempDir::new().unwrap();
        let lean_content = r#"
namespace Auth

/--
SPEC-01.01: Authentication correctness
Users must be authenticated before access.
-/
theorem auth_correct (u : User) : authenticated u -> canAccess u := by
  intro h
  exact h.access

/--
SPEC-01.02: Session timeout
-/
theorem session_timeout : timeout s -> expired s := by
  sorry

end Auth
"#;
        create_test_file(dir.path(), "src/Auth.lean", lean_content);

        let scanner = CoverageScanner::new(dir.path());
        let theorems = scanner
            .extract_theorems_from_lean(&dir.path().join("src/Auth.lean"))
            .unwrap();

        assert_eq!(theorems.len(), 2);

        let (spec1, thm1) = &theorems[0];
        assert_eq!(*spec1, SpecId::new(1, 1));
        assert_eq!(thm1.name, "auth_correct");
        assert_eq!(thm1.status, ProofStatus::Complete);

        let (spec2, thm2) = &theorems[1];
        assert_eq!(*spec2, SpecId::new(1, 2));
        assert_eq!(thm2.name, "session_timeout");
        assert_eq!(thm2.status, ProofStatus::HasSorry);
    }

    #[test]
    fn test_extract_test_traces() {
        let dir = TempDir::new().unwrap();
        let test_content = r#"
#[test]
fn test_authentication() {
    // @trace SPEC-01.01
    let user = User::new("test@example.com");
    assert!(user.authenticate("password").is_ok());
}

#[test]
fn test_session_expiry() {
    // trace: SPEC-01.02
    let session = Session::new();
    session.advance_time(Duration::minutes(31));
    assert!(session.is_expired());
}
"#;
        create_test_file(dir.path(), "tests/auth_test.rs", test_content);

        let scanner = CoverageScanner::new(dir.path());
        let traces = scanner
            .extract_test_traces(&dir.path().join("tests/auth_test.rs"))
            .unwrap();

        assert_eq!(traces.len(), 2);
        assert_eq!(traces[0].0, SpecId::new(1, 1));
        assert_eq!(traces[1].0, SpecId::new(1, 2));
    }

    #[test]
    fn test_coverage_tracker() {
        let mut tracker = SpecCoverageTracker::new("/project");

        tracker.register_spec(SpecId::new(1, 1), "Auth requirement");
        tracker.register_spec(SpecId::new(1, 2), "Session requirement");

        tracker.register_theorem(
            &SpecId::new(1, 1),
            TheoremInfo::new("auth_thm", "auth.lean", 10).with_status(ProofStatus::Complete),
        );

        let summary = tracker.summary();
        assert_eq!(summary.total_specs, 2);
        assert_eq!(summary.formalized_count, 1);
        assert_eq!(summary.complete_count, 1);
    }
}
