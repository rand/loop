//! Python bindings for context types.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::context::{Message, Role, SessionContext, ToolOutput};

/// Python enum for Role.
#[pyclass(name = "Role", eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PyRole {
    User = 0,
    Assistant = 1,
    System = 2,
    Tool = 3,
}

impl From<Role> for PyRole {
    fn from(role: Role) -> Self {
        match role {
            Role::User => PyRole::User,
            Role::Assistant => PyRole::Assistant,
            Role::System => PyRole::System,
            Role::Tool => PyRole::Tool,
        }
    }
}

impl From<PyRole> for Role {
    fn from(role: PyRole) -> Self {
        match role {
            PyRole::User => Role::User,
            PyRole::Assistant => Role::Assistant,
            PyRole::System => Role::System,
            PyRole::Tool => Role::Tool,
        }
    }
}

#[pymethods]
impl PyRole {
    fn __repr__(&self) -> &'static str {
        match self {
            PyRole::User => "Role.User",
            PyRole::Assistant => "Role.Assistant",
            PyRole::System => "Role.System",
            PyRole::Tool => "Role.Tool",
        }
    }
}

/// Python wrapper for Message.
#[pyclass(name = "Message")]
#[derive(Clone)]
pub struct PyMessage {
    pub(crate) inner: Message,
}

#[pymethods]
impl PyMessage {
    #[new]
    #[pyo3(signature = (role, content, timestamp=None))]
    fn new(role: PyRole, content: String, timestamp: Option<String>) -> PyResult<Self> {
        let ts = timestamp
            .map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .map_err(|e| {
                        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Invalid timestamp: {}",
                            e
                        ))
                    })
            })
            .transpose()?;

        Ok(Self {
            inner: Message {
                role: role.into(),
                content,
                timestamp: ts,
                metadata: None,
            },
        })
    }

    /// Create a user message.
    #[staticmethod]
    fn user(content: String) -> Self {
        Self {
            inner: Message::user(content),
        }
    }

    /// Create an assistant message.
    #[staticmethod]
    fn assistant(content: String) -> Self {
        Self {
            inner: Message::assistant(content),
        }
    }

    /// Create a system message.
    #[staticmethod]
    fn system(content: String) -> Self {
        Self {
            inner: Message::system(content),
        }
    }

    /// Create a tool message.
    #[staticmethod]
    fn tool(content: String) -> Self {
        Self {
            inner: Message::tool(content),
        }
    }

    #[getter]
    fn role(&self) -> PyRole {
        self.inner.role.into()
    }

    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    #[getter]
    fn timestamp(&self) -> Option<String> {
        self.inner.timestamp.map(|ts| ts.to_rfc3339())
    }

    fn __repr__(&self) -> String {
        format!(
            "Message(role={:?}, content={:?})",
            self.inner.role,
            truncate(&self.inner.content, 50)
        )
    }
}

/// Python wrapper for ToolOutput.
#[pyclass(name = "ToolOutput")]
#[derive(Clone)]
pub struct PyToolOutput {
    pub(crate) inner: ToolOutput,
}

#[pymethods]
impl PyToolOutput {
    #[new]
    #[pyo3(signature = (tool_name, content, exit_code=None))]
    fn new(tool_name: String, content: String, exit_code: Option<i32>) -> Self {
        let mut output = ToolOutput::new(tool_name, content);
        if let Some(code) = exit_code {
            output = output.with_exit_code(code);
        }
        Self { inner: output }
    }

    #[getter]
    fn tool_name(&self) -> String {
        self.inner.tool_name.clone()
    }

    #[getter]
    fn content(&self) -> String {
        self.inner.content.clone()
    }

    #[getter]
    fn exit_code(&self) -> Option<i32> {
        self.inner.exit_code
    }

    #[getter]
    fn timestamp(&self) -> Option<String> {
        self.inner.timestamp.map(|ts| ts.to_rfc3339())
    }

    /// Check if the tool execution succeeded.
    fn is_success(&self) -> bool {
        self.inner.is_success()
    }

    fn __repr__(&self) -> String {
        format!(
            "ToolOutput(tool={:?}, exit_code={:?})",
            self.inner.tool_name, self.inner.exit_code
        )
    }
}

/// Python wrapper for SessionContext.
#[pyclass(name = "SessionContext")]
#[derive(Clone)]
pub struct PySessionContext {
    pub(crate) inner: SessionContext,
}

#[pymethods]
impl PySessionContext {
    #[new]
    fn new() -> Self {
        Self {
            inner: SessionContext::new(),
        }
    }

    /// Add a message to the context.
    fn add_message(&mut self, message: &PyMessage) {
        self.inner.add_message(message.inner.clone());
    }

    /// Add a user message.
    fn add_user_message(&mut self, content: String) {
        self.inner.add_user_message(content);
    }

    /// Add an assistant message.
    fn add_assistant_message(&mut self, content: String) {
        self.inner.add_assistant_message(content);
    }

    /// Cache a file's contents.
    fn cache_file(&mut self, path: String, content: String) {
        self.inner.cache_file(path, content);
    }

    /// Add a tool output to the context.
    fn add_tool_output(&mut self, output: &PyToolOutput) {
        self.inner.add_tool_output(output.inner.clone());
    }

    /// Set working memory value.
    fn set_memory(&mut self, key: String, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value = python_to_json(value)?;
        self.inner.set_memory(key, json_value);
        Ok(())
    }

    /// Get working memory value.
    fn get_memory(&self, py: Python<'_>, key: &str) -> PyResult<PyObject> {
        match self.inner.get_memory(key) {
            Some(value) => json_to_python(py, value),
            None => Ok(py.None()),
        }
    }

    /// Get all messages.
    fn messages(&self) -> Vec<PyMessage> {
        self.inner
            .messages
            .iter()
            .map(|m| PyMessage { inner: m.clone() })
            .collect()
    }

    /// Get the last N messages.
    fn last_messages(&self, n: usize) -> Vec<PyMessage> {
        self.inner
            .last_messages(n)
            .iter()
            .map(|m| PyMessage { inner: m.clone() })
            .collect()
    }

    /// Get all files as a dict.
    fn files(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (path, content) in &self.inner.files {
            dict.set_item(path, content)?;
        }
        Ok(dict.into())
    }

    /// Get a specific file.
    fn get_file(&self, path: &str) -> Option<String> {
        self.inner.get_file(path).map(|s| s.to_string())
    }

    /// Get all tool outputs.
    fn tool_outputs(&self) -> Vec<PyToolOutput> {
        self.inner
            .tool_outputs
            .iter()
            .map(|o| PyToolOutput { inner: o.clone() })
            .collect()
    }

    /// Get message count.
    fn message_count(&self) -> usize {
        self.inner.messages.len()
    }

    /// Get file count.
    fn file_count(&self) -> usize {
        self.inner.files.len()
    }

    /// Check if files span multiple directories.
    fn spans_multiple_directories(&self) -> bool {
        self.inner.spans_multiple_directories()
    }

    /// Get total approximate tokens in messages.
    fn total_message_tokens(&self) -> usize {
        self.inner.total_message_tokens()
    }

    fn __repr__(&self) -> String {
        format!(
            "SessionContext(messages={}, files={}, tool_outputs={})",
            self.inner.messages.len(),
            self.inner.files.len(),
            self.inner.tool_outputs.len()
        )
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
