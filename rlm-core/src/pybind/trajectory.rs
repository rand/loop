//! Python bindings for trajectory types.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::trajectory::{TrajectoryEvent, TrajectoryEventType};

/// Python enum for TrajectoryEventType.
#[pyclass(name = "TrajectoryEventType", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PyTrajectoryEventType {
    RlmStart = 0,
    Analyze = 1,
    ReplExec = 2,
    ReplResult = 3,
    Reason = 4,
    RecurseStart = 5,
    RecurseEnd = 6,
    Final = 7,
    Error = 8,
    ToolUse = 9,
    CostReport = 10,
    VerifyStart = 11,
    ClaimExtracted = 12,
    EvidenceChecked = 13,
    BudgetComputed = 14,
    HallucinationFlag = 15,
    VerifyComplete = 16,
    Memory = 17,
    Externalize = 18,
    Decompose = 19,
    Synthesize = 20,
    AdversarialStart = 21,
    CriticInvoked = 22,
    IssueFound = 23,
    AdversarialComplete = 24,
}

impl From<TrajectoryEventType> for PyTrajectoryEventType {
    fn from(t: TrajectoryEventType) -> Self {
        match t {
            TrajectoryEventType::RlmStart => PyTrajectoryEventType::RlmStart,
            TrajectoryEventType::Analyze => PyTrajectoryEventType::Analyze,
            TrajectoryEventType::ReplExec => PyTrajectoryEventType::ReplExec,
            TrajectoryEventType::ReplResult => PyTrajectoryEventType::ReplResult,
            TrajectoryEventType::Reason => PyTrajectoryEventType::Reason,
            TrajectoryEventType::RecurseStart => PyTrajectoryEventType::RecurseStart,
            TrajectoryEventType::RecurseEnd => PyTrajectoryEventType::RecurseEnd,
            TrajectoryEventType::Final => PyTrajectoryEventType::Final,
            TrajectoryEventType::Error => PyTrajectoryEventType::Error,
            TrajectoryEventType::ToolUse => PyTrajectoryEventType::ToolUse,
            TrajectoryEventType::CostReport => PyTrajectoryEventType::CostReport,
            TrajectoryEventType::VerifyStart => PyTrajectoryEventType::VerifyStart,
            TrajectoryEventType::ClaimExtracted => PyTrajectoryEventType::ClaimExtracted,
            TrajectoryEventType::EvidenceChecked => PyTrajectoryEventType::EvidenceChecked,
            TrajectoryEventType::BudgetComputed => PyTrajectoryEventType::BudgetComputed,
            TrajectoryEventType::HallucinationFlag => PyTrajectoryEventType::HallucinationFlag,
            TrajectoryEventType::VerifyComplete => PyTrajectoryEventType::VerifyComplete,
            TrajectoryEventType::Memory => PyTrajectoryEventType::Memory,
            TrajectoryEventType::Externalize => PyTrajectoryEventType::Externalize,
            TrajectoryEventType::Decompose => PyTrajectoryEventType::Decompose,
            TrajectoryEventType::Synthesize => PyTrajectoryEventType::Synthesize,
            TrajectoryEventType::AdversarialStart => PyTrajectoryEventType::AdversarialStart,
            TrajectoryEventType::CriticInvoked => PyTrajectoryEventType::CriticInvoked,
            TrajectoryEventType::IssueFound => PyTrajectoryEventType::IssueFound,
            TrajectoryEventType::AdversarialComplete => PyTrajectoryEventType::AdversarialComplete,
        }
    }
}

impl From<PyTrajectoryEventType> for TrajectoryEventType {
    fn from(t: PyTrajectoryEventType) -> Self {
        match t {
            PyTrajectoryEventType::RlmStart => TrajectoryEventType::RlmStart,
            PyTrajectoryEventType::Analyze => TrajectoryEventType::Analyze,
            PyTrajectoryEventType::ReplExec => TrajectoryEventType::ReplExec,
            PyTrajectoryEventType::ReplResult => TrajectoryEventType::ReplResult,
            PyTrajectoryEventType::Reason => TrajectoryEventType::Reason,
            PyTrajectoryEventType::RecurseStart => TrajectoryEventType::RecurseStart,
            PyTrajectoryEventType::RecurseEnd => TrajectoryEventType::RecurseEnd,
            PyTrajectoryEventType::Final => TrajectoryEventType::Final,
            PyTrajectoryEventType::Error => TrajectoryEventType::Error,
            PyTrajectoryEventType::ToolUse => TrajectoryEventType::ToolUse,
            PyTrajectoryEventType::CostReport => TrajectoryEventType::CostReport,
            PyTrajectoryEventType::VerifyStart => TrajectoryEventType::VerifyStart,
            PyTrajectoryEventType::ClaimExtracted => TrajectoryEventType::ClaimExtracted,
            PyTrajectoryEventType::EvidenceChecked => TrajectoryEventType::EvidenceChecked,
            PyTrajectoryEventType::BudgetComputed => TrajectoryEventType::BudgetComputed,
            PyTrajectoryEventType::HallucinationFlag => TrajectoryEventType::HallucinationFlag,
            PyTrajectoryEventType::VerifyComplete => TrajectoryEventType::VerifyComplete,
            PyTrajectoryEventType::Memory => TrajectoryEventType::Memory,
            PyTrajectoryEventType::Externalize => TrajectoryEventType::Externalize,
            PyTrajectoryEventType::Decompose => TrajectoryEventType::Decompose,
            PyTrajectoryEventType::Synthesize => TrajectoryEventType::Synthesize,
            PyTrajectoryEventType::AdversarialStart => TrajectoryEventType::AdversarialStart,
            PyTrajectoryEventType::CriticInvoked => TrajectoryEventType::CriticInvoked,
            PyTrajectoryEventType::IssueFound => TrajectoryEventType::IssueFound,
            PyTrajectoryEventType::AdversarialComplete => TrajectoryEventType::AdversarialComplete,
        }
    }
}

#[pymethods]
impl PyTrajectoryEventType {
    fn __repr__(&self) -> &'static str {
        match self {
            PyTrajectoryEventType::RlmStart => "TrajectoryEventType.RlmStart",
            PyTrajectoryEventType::Analyze => "TrajectoryEventType.Analyze",
            PyTrajectoryEventType::ReplExec => "TrajectoryEventType.ReplExec",
            PyTrajectoryEventType::ReplResult => "TrajectoryEventType.ReplResult",
            PyTrajectoryEventType::Reason => "TrajectoryEventType.Reason",
            PyTrajectoryEventType::RecurseStart => "TrajectoryEventType.RecurseStart",
            PyTrajectoryEventType::RecurseEnd => "TrajectoryEventType.RecurseEnd",
            PyTrajectoryEventType::Final => "TrajectoryEventType.Final",
            PyTrajectoryEventType::Error => "TrajectoryEventType.Error",
            PyTrajectoryEventType::ToolUse => "TrajectoryEventType.ToolUse",
            PyTrajectoryEventType::CostReport => "TrajectoryEventType.CostReport",
            PyTrajectoryEventType::VerifyStart => "TrajectoryEventType.VerifyStart",
            PyTrajectoryEventType::ClaimExtracted => "TrajectoryEventType.ClaimExtracted",
            PyTrajectoryEventType::EvidenceChecked => "TrajectoryEventType.EvidenceChecked",
            PyTrajectoryEventType::BudgetComputed => "TrajectoryEventType.BudgetComputed",
            PyTrajectoryEventType::HallucinationFlag => "TrajectoryEventType.HallucinationFlag",
            PyTrajectoryEventType::VerifyComplete => "TrajectoryEventType.VerifyComplete",
            PyTrajectoryEventType::Memory => "TrajectoryEventType.Memory",
            PyTrajectoryEventType::Externalize => "TrajectoryEventType.Externalize",
            PyTrajectoryEventType::Decompose => "TrajectoryEventType.Decompose",
            PyTrajectoryEventType::Synthesize => "TrajectoryEventType.Synthesize",
            PyTrajectoryEventType::AdversarialStart => "TrajectoryEventType.AdversarialStart",
            PyTrajectoryEventType::CriticInvoked => "TrajectoryEventType.CriticInvoked",
            PyTrajectoryEventType::IssueFound => "TrajectoryEventType.IssueFound",
            PyTrajectoryEventType::AdversarialComplete => "TrajectoryEventType.AdversarialComplete",
        }
    }
}

/// Python wrapper for TrajectoryEvent.
#[pyclass(name = "TrajectoryEvent")]
#[derive(Clone)]
pub struct PyTrajectoryEvent {
    pub(crate) inner: TrajectoryEvent,
}

#[pymethods]
impl PyTrajectoryEvent {
    #[new]
    #[pyo3(signature = (event_type, content, depth=0))]
    fn new(event_type: PyTrajectoryEventType, content: String, depth: u32) -> Self {
        Self {
            inner: TrajectoryEvent::new(event_type.into(), depth, content),
        }
    }

    /// Create an RLM start event.
    #[staticmethod]
    fn rlm_start(query: String) -> Self {
        Self {
            inner: TrajectoryEvent::rlm_start(query),
        }
    }

    /// Create an analyze event.
    #[staticmethod]
    #[pyo3(signature = (analysis, depth=0))]
    fn analyze(analysis: String, depth: u32) -> Self {
        Self {
            inner: TrajectoryEvent::analyze(depth, analysis),
        }
    }

    /// Create a REPL exec event.
    #[staticmethod]
    fn repl_exec(depth: u32, code: String) -> Self {
        Self {
            inner: TrajectoryEvent::repl_exec(depth, code),
        }
    }

    /// Create a REPL result event.
    #[staticmethod]
    #[pyo3(signature = (depth, result, success=true))]
    fn repl_result(depth: u32, result: String, success: bool) -> Self {
        Self {
            inner: TrajectoryEvent::repl_result(depth, result, success),
        }
    }

    /// Create a reason event.
    #[staticmethod]
    fn reason(depth: u32, reasoning: String) -> Self {
        Self {
            inner: TrajectoryEvent::reason(depth, reasoning),
        }
    }

    /// Create a recurse start event.
    #[staticmethod]
    fn recurse_start(depth: u32, query: String) -> Self {
        Self {
            inner: TrajectoryEvent::recurse_start(depth, query),
        }
    }

    /// Create a recurse end event.
    #[staticmethod]
    fn recurse_end(depth: u32, result: String) -> Self {
        Self {
            inner: TrajectoryEvent::recurse_end(depth, result),
        }
    }

    /// Create a final answer event.
    #[staticmethod]
    #[pyo3(signature = (answer, depth=0))]
    fn final_answer(answer: String, depth: u32) -> Self {
        Self {
            inner: TrajectoryEvent::final_answer(depth, answer),
        }
    }

    /// Create an error event.
    #[staticmethod]
    fn error(depth: u32, error: String) -> Self {
        Self {
            inner: TrajectoryEvent::error(depth, error),
        }
    }

    #[getter]
    fn event_type(&self) -> PyTrajectoryEventType {
        self.inner.event_type.into()
    }

    #[getter]
    fn depth(&self) -> u32 {
        self.inner.depth
    }

    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    #[getter]
    fn timestamp(&self) -> String {
        self.inner.timestamp.to_rfc3339()
    }

    /// Add metadata to the event.
    fn with_metadata(&mut self, key: String, value: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value = python_to_json(value)?;
        self.inner = self.inner.clone().with_metadata(key, json_value);
        Ok(self.clone())
    }

    /// Get metadata value.
    fn get_metadata(&self, py: Python<'_>, key: &str) -> PyResult<PyObject> {
        match self.inner.metadata.as_ref().and_then(|m| m.get(key)) {
            Some(value) => json_to_python(py, value),
            None => Ok(py.None()),
        }
    }

    /// Format as a log line.
    fn log_line(&self) -> String {
        self.inner.as_log_line()
    }

    /// Export to JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Import from JSON.
    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: TrajectoryEvent = serde_json::from_str(json)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "TrajectoryEvent(type={:?}, depth={}, content={:?})",
            self.inner.event_type,
            self.inner.depth,
            truncate(&self.inner.content, 30)
        )
    }
}

/// Convert a Python value to serde_json::Value.
fn python_to_json(value: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    if value.is_none() {
        Ok(serde_json::Value::Null)
    } else if let Ok(b) = value.extract::<bool>() {
        Ok(serde_json::Value::Bool(b))
    } else if let Ok(i) = value.extract::<i64>() {
        Ok(serde_json::Value::Number(i.into()))
    } else if let Ok(f) = value.extract::<f64>() {
        Ok(serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null))
    } else if let Ok(s) = value.extract::<String>() {
        Ok(serde_json::Value::String(s))
    } else if let Ok(list) = value.downcast::<pyo3::types::PyList>() {
        let arr: PyResult<Vec<serde_json::Value>> =
            list.iter().map(|v| python_to_json(&v)).collect();
        Ok(serde_json::Value::Array(arr?))
    } else if let Ok(dict) = value.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict {
            let key: String = k.extract()?;
            map.insert(key, python_to_json(&v)?);
        }
        Ok(serde_json::Value::Object(map))
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "Unsupported type for JSON conversion",
        ))
    }
}

/// Convert a serde_json::Value to Python.
fn json_to_python(py: Python<'_>, value: &serde_json::Value) -> PyResult<PyObject> {
    use pyo3::IntoPyObject;
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => Ok((*b).into_pyobject(py)?.to_owned().into_any().unbind()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any().unbind())
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_pyobject(py)?.into_any().unbind())
            } else {
                Ok(py.None())
            }
        }
        serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        serde_json::Value::Array(arr) => {
            let list = pyo3::types::PyList::empty(py);
            for item in arr {
                list.append(json_to_python(py, item)?)?;
            }
            Ok(list.into())
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, json_to_python(py, v)?)?;
            }
            Ok(dict.into())
        }
    }
}

/// Truncate a string for display.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
