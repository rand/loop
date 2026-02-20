# SPEC-24: BootstrapFewShot Optimizer

> DSPy-style automatic prompt optimization

**Status**: Partially implemented (bootstrap optimization core, metric suite, reasoning capture summaries, and save/load persistence are implemented; advanced metric trait/object-safety refinements remain deferred)
**Created**: 2026-01-20
**Epic**: loop-zcx (DSPy-Inspired RLM Improvements)
**Task**: loop-o9r
**Depends On**: SPEC-20 (Typed Signatures)

---

## Overview

Implement DSPy-style BootstrapFewShot optimizer that automatically improves prompts by selecting high-quality demonstrations from training data.

## Implementation Snapshot (2026-02-20)

| Section | Status | Runtime Evidence |
|---|---|---|
| SPEC-24.01 Optimizer trait and compile flow | Implemented | `Optimizer` + `BootstrapFewShot::compile` in `rlm-core/src/module/optimize.rs` |
| SPEC-24.02 Bootstrap configuration/presets | Implemented | `BootstrapFewShot::{default,new,greedy,thorough}` and builder methods |
| SPEC-24.03 OptimizedModule + persistence helpers | Implemented | `OptimizedModule::{save,load}` + roundtrip test in `rlm-core/src/module/optimize.rs` |
| SPEC-24.04 Optimization process and selection | Implemented | thresholding, dedupe, round stats, and demo selection in `compile` |
| SPEC-24.05 Metric functions | Implemented | `metrics` module (`exact_match`, `f1_score`, `jaccard_similarity`, `edit_distance_similarity`, `combine_weighted`) |
| Reasoning capture parity (`M7-T07`) | Implemented (summary capture, toggleable via config) | `build_bootstrap_reasoning_summary`, `build_labeled_reasoning_summary`, reasoning on selected demonstrations |

## Background

DSPy's optimization approach:
1. Run module on training data with temperature=1.0 (diverse sampling)
2. Evaluate outputs with a metric function
3. Filter examples meeting metric threshold
4. Compose demonstrations from successful traces
5. Return module with injected few-shot examples

## Requirements

### SPEC-24.01: Optimizer Trait

Base trait for all optimizers.

```rust
/// Trait for prompt optimizers
pub trait Optimizer: Send + Sync {
    /// Compile a module with training data
    fn compile<S, M>(
        &self,
        module: M,
        trainset: &[Example<S>],
        metric: &dyn Metric<S>,
    ) -> Result<OptimizedModule<S, M>, OptimizeError>
    where
        S: Signature,
        M: Module<Signature = S>;

    /// Async version
    async fn compile_async<S, M>(
        &self,
        module: M,
        trainset: &[Example<S>],
        metric: &dyn Metric<S>,
    ) -> Result<OptimizedModule<S, M>, OptimizeError>
    where
        S: Signature,
        M: Module<Signature = S>;
}

/// Training example with optional gold output
pub struct Example<S: Signature> {
    pub inputs: S::Inputs,
    pub gold_outputs: Option<S::Outputs>,
}

/// Metric for evaluating outputs
pub trait Metric<S: Signature>: Send + Sync {
    fn score(&self, predicted: &S::Outputs, gold: &S::Outputs) -> f64;
    fn name(&self) -> &str;
}

#[derive(Debug)]
pub enum OptimizeError {
    NoExamples,
    AllExamplesFailed,
    MetricError(String),
    ModuleError(ModuleError),
}
```

**Acceptance Criteria**:
- [ ] Optimizer trait is object-safe where needed
- [ ] Example struct supports optional gold
- [ ] Metric trait flexible for different tasks

### SPEC-24.02: BootstrapFewShot Configuration

Configuration for bootstrap optimization.

```rust
/// BootstrapFewShot optimizer configuration
#[derive(Debug, Clone)]
pub struct BootstrapFewShot {
    /// Maximum bootstrapped demonstrations to include
    pub max_bootstrapped_demos: usize,
    /// Maximum labeled demonstrations from trainset
    pub max_labeled_demos: usize,
    /// Number of bootstrap rounds
    pub max_rounds: usize,
    /// Minimum metric score for inclusion
    pub metric_threshold: f64,
    /// Temperature for diverse sampling
    pub temperature: f64,
    /// Random seed for reproducibility
    pub seed: Option<u64>,
}

impl Default for BootstrapFewShot {
    fn default() -> Self {
        Self {
            max_bootstrapped_demos: 4,
            max_labeled_demos: 16,
            max_rounds: 1,
            metric_threshold: 0.0,
            temperature: 1.0,
            seed: None,
        }
    }
}

impl BootstrapFewShot {
    pub fn with_max_demos(mut self, bootstrapped: usize, labeled: usize) -> Self {
        self.max_bootstrapped_demos = bootstrapped;
        self.max_labeled_demos = labeled;
        self
    }

    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.metric_threshold = threshold;
        self
    }

    pub fn with_rounds(mut self, rounds: usize) -> Self {
        self.max_rounds = rounds;
        self
    }
}
```

**Acceptance Criteria**:
- [ ] Sensible defaults
- [ ] Builder pattern for configuration
- [ ] All parameters documented

### SPEC-24.03: OptimizedModule

Wrapper for optimized modules.

```rust
/// A module with optimized demonstrations
pub struct OptimizedModule<S: Signature, M: Module<Signature = S>> {
    /// Original module
    inner: M,
    /// Selected demonstrations
    demonstrations: Vec<Demonstration<S>>,
    /// Optimization statistics
    stats: OptimizationStats,
}

/// A single demonstration (input-output pair with trace)
pub struct Demonstration<S: Signature> {
    pub inputs: S::Inputs,
    pub outputs: S::Outputs,
    pub trace: Option<ReasoningTrace>,
    pub metric_score: f64,
}

/// Statistics from optimization
#[derive(Debug, Clone)]
pub struct OptimizationStats {
    /// Total examples processed
    pub total_examples: usize,
    /// Examples meeting threshold
    pub passing_examples: usize,
    /// Average metric score
    pub avg_score: f64,
    /// Best metric score
    pub best_score: f64,
    /// Optimization duration
    pub duration: Duration,
    /// Rounds completed
    pub rounds_completed: usize,
}

impl<S: Signature, M: Module<Signature = S>> Module for OptimizedModule<S, M> {
    type Signature = S;

    async fn forward(&self, inputs: S::Inputs) -> Result<S::Outputs, ModuleError> {
        // Forward with demonstrations injected
        self.inner.forward_with_demos(inputs, &self.demonstrations).await
    }
}

impl<S: Signature, M: Module<Signature = S>> OptimizedModule<S, M> {
    /// Get demonstrations
    pub fn demonstrations(&self) -> &[Demonstration<S>];

    /// Get optimization stats
    pub fn stats(&self) -> &OptimizationStats;

    /// Save optimized state
    pub fn save(&self, path: &Path) -> Result<()>;

    /// Load optimized state
    pub fn load(module: M, path: &Path) -> Result<Self>;
}
```

**Acceptance Criteria**:
- [ ] Module trait implemented
- [ ] Demonstrations accessible
- [ ] Serialization works

### SPEC-24.04: Optimization Process

The bootstrap optimization algorithm.

```rust
impl Optimizer for BootstrapFewShot {
    fn compile<S, M>(
        &self,
        module: M,
        trainset: &[Example<S>],
        metric: &dyn Metric<S>,
    ) -> Result<OptimizedModule<S, M>, OptimizeError>
    where
        S: Signature,
        M: Module<Signature = S>,
    {
        let mut demonstrations = Vec::new();
        let mut all_results = Vec::new();

        for round in 0..self.max_rounds {
            // 1. Sample from trainset
            let samples = self.sample_trainset(trainset, round);

            // 2. Run module with temperature=1.0 for diversity
            let config = PredictConfig {
                temperature: self.temperature,
                ..Default::default()
            };

            for example in samples {
                let result = module.forward_with_config(&example.inputs, &config)?;

                // 3. Evaluate with metric
                if let Some(gold) = &example.gold_outputs {
                    let score = metric.score(&result, gold);
                    all_results.push((example.inputs.clone(), result, score));
                }
            }
        }

        // 4. Filter by threshold
        let passing: Vec<_> = all_results
            .into_iter()
            .filter(|(_, _, score)| *score >= self.metric_threshold)
            .collect();

        // 5. Rank by score
        let mut ranked = passing;
        ranked.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

        // 6. Select top demos
        let selected: Vec<_> = ranked
            .into_iter()
            .take(self.max_bootstrapped_demos)
            .map(|(inputs, outputs, score)| Demonstration {
                inputs,
                outputs,
                trace: None,
                metric_score: score,
            })
            .collect();

        Ok(OptimizedModule {
            inner: module,
            demonstrations: selected,
            stats: self.compute_stats(&all_results),
        })
    }
}
```

**Algorithm Steps**:
1. For each round, sample from trainset
2. Run module with temperature=1.0 (bypass cache, get diverse outputs)
3. Evaluate each output with metric function
4. Filter examples meeting metric_threshold
5. Rank by metric score
6. Select top max_bootstrapped_demos
7. Wrap module with demonstrations

**Acceptance Criteria**:
- [ ] Multi-round bootstrap works
- [ ] Temperature=1.0 ensures diversity
- [ ] Filtering and ranking correct

### SPEC-24.05: Metric Functions

Built-in metrics for common tasks.

```rust
pub mod metrics {
    use super::*;

    /// Exact match metric (classification)
    pub struct ExactMatch;

    impl<S: Signature> Metric<S> for ExactMatch
    where
        S::Outputs: PartialEq,
    {
        fn score(&self, predicted: &S::Outputs, gold: &S::Outputs) -> f64 {
            if predicted == gold { 1.0 } else { 0.0 }
        }

        fn name(&self) -> &str { "exact_match" }
    }

    /// F1 score metric (extraction)
    pub struct F1Score;

    impl<S: Signature> Metric<S> for F1Score
    where
        S::Outputs: AsRef<[String]>,
    {
        fn score(&self, predicted: &S::Outputs, gold: &S::Outputs) -> f64 {
            let pred_set: HashSet<_> = predicted.as_ref().iter().collect();
            let gold_set: HashSet<_> = gold.as_ref().iter().collect();

            let intersection = pred_set.intersection(&gold_set).count() as f64;
            let precision = intersection / pred_set.len() as f64;
            let recall = intersection / gold_set.len() as f64;

            if precision + recall == 0.0 {
                0.0
            } else {
                2.0 * (precision * recall) / (precision + recall)
            }
        }

        fn name(&self) -> &str { "f1_score" }
    }

    /// Semantic similarity metric (requires embeddings)
    pub struct SemanticSimilarity<E: Embedder> {
        embedder: E,
        threshold: f64,
    }

    impl<S: Signature, E: Embedder> Metric<S> for SemanticSimilarity<E>
    where
        S::Outputs: AsRef<str>,
    {
        fn score(&self, predicted: &S::Outputs, gold: &S::Outputs) -> f64 {
            let pred_emb = self.embedder.embed(predicted.as_ref());
            let gold_emb = self.embedder.embed(gold.as_ref());
            cosine_similarity(&pred_emb, &gold_emb)
        }

        fn name(&self) -> &str { "semantic_similarity" }
    }

    /// Composite metric (combine multiple)
    pub struct CompositeMetric<S: Signature> {
        metrics: Vec<(Box<dyn Metric<S>>, f64)>,  // (metric, weight)
    }

    impl<S: Signature> Metric<S> for CompositeMetric<S> {
        fn score(&self, predicted: &S::Outputs, gold: &S::Outputs) -> f64 {
            let total_weight: f64 = self.metrics.iter().map(|(_, w)| w).sum();
            self.metrics
                .iter()
                .map(|(m, w)| m.score(predicted, gold) * w)
                .sum::<f64>()
                / total_weight
        }

        fn name(&self) -> &str { "composite" }
    }
}
```

**Acceptance Criteria**:
- [ ] ExactMatch works for classification
- [ ] F1Score works for extraction
- [ ] SemanticSimilarity computes correctly
- [ ] CompositeMetric combines weights

---

## Usage Example

```rust
// Define signature
#[derive(Signature)]
#[signature(instructions = "Classify sentiment")]
struct SentimentClassifier {
    #[input(desc = "Text to classify")]
    text: String,
    #[output(desc = "Sentiment: positive/negative/neutral")]
    sentiment: String,
}

// Create module
let module = Predict::<SentimentClassifier>::new();

// Create trainset
let trainset = vec![
    Example {
        inputs: SentimentClassifierInputs { text: "Great!".into() },
        gold_outputs: Some(SentimentClassifierOutputs { sentiment: "positive".into() }),
    },
    // ... more examples
];

// Optimize
let optimizer = BootstrapFewShot::default()
    .with_threshold(0.8)
    .with_max_demos(4, 16);

let optimized = optimizer.compile(module, &trainset, &ExactMatch)?;

// Use optimized module
let result = optimized.forward(inputs).await?;
```

---

## Test Plan

| Test | Description | Spec |
|------|-------------|------|
| `test_compile_captures_reasoning_when_enabled` | Reasoning summaries are captured for selected demos | SPEC-24.04 |
| `test_compile_skips_reasoning_when_disabled` | Reasoning capture toggle is honored | SPEC-24.02, SPEC-24.04 |
| `test_optimized_module_save_and_load_roundtrip` | Save/load persistence roundtrip for optimized state | SPEC-24.03 |
| `test_metrics_exact_match` | Exact match metric | SPEC-24.05 |
| `test_metrics_f1_score` | F1 score metric | SPEC-24.05 |
| `test_metrics_combine_weighted` | Weighted composite metric | SPEC-24.05 |
| `test_optimization_stats` | Optimization stats accounting | SPEC-24.03, SPEC-24.04 |

---

## References

- [DSPy BootstrapFewShot](https://dspy.ai/api/optimizers/BootstrapFewShot/)
- [DSPy Teleprompters](https://github.com/stanfordnlp/dspy/tree/main/dspy/teleprompt)
- SPEC-20: Typed Signatures (prerequisite)
