//! Python bindings for CloudShift (PyO3).
//!
//! This crate exposes the core transformation engine to Python
//! via the maturin/PyO3 bridge.

use pyo3::prelude::*;
use cloudshift_core::{TransformConfig, SourceCloud, OutputFormat};

// ---------------------------------------------------------------------------
// Python-visible enum-like classes
// ---------------------------------------------------------------------------

/// Source cloud provider (aws, azure, any).
#[pyclass(name = "SourceCloud")]
#[derive(Clone)]
pub struct PySourceCloud {
    inner: SourceCloud,
}

#[pymethods]
impl PySourceCloud {
    #[new]
    fn new(name: &str) -> PyResult<Self> {
        let inner: SourceCloud = name
            .parse()
            .map_err(|e: cloudshift_core::domain::value_objects::DomainError| {
                pyo3::exceptions::PyValueError::new_err(e.to_string())
            })?;
        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        format!("SourceCloud('{}')", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// Output format (diff, json, sarif).
#[pyclass(name = "OutputFormat")]
#[derive(Clone)]
pub struct PyOutputFormat {
    inner: OutputFormat,
}

#[pymethods]
impl PyOutputFormat {
    #[new]
    fn new(name: &str) -> PyResult<Self> {
        let inner: OutputFormat = name
            .parse()
            .map_err(|e: cloudshift_core::domain::value_objects::DomainError| {
                pyo3::exceptions::PyValueError::new_err(e.to_string())
            })?;
        Ok(Self { inner })
    }

    fn __repr__(&self) -> String {
        format!("OutputFormat('{:?}')", self.inner)
    }
}

// ---------------------------------------------------------------------------
// TransformConfig
// ---------------------------------------------------------------------------

/// Configuration for a transformation run.
#[pyclass(name = "TransformConfig")]
#[derive(Clone)]
pub struct PyTransformConfig {
    inner: TransformConfig,
}

#[pymethods]
impl PyTransformConfig {
    #[new]
    #[pyo3(signature = (source_cloud=None, output_format=None, dry_run=None, parallel=None))]
    fn new(
        source_cloud: Option<&PySourceCloud>,
        output_format: Option<&PyOutputFormat>,
        dry_run: Option<bool>,
        parallel: Option<usize>,
    ) -> Self {
        let mut cfg = TransformConfig::default();
        if let Some(sc) = source_cloud {
            cfg.source_cloud = sc.inner;
        }
        if let Some(of) = output_format {
            cfg.output_format = of.inner;
        }
        if let Some(dr) = dry_run {
            cfg.dry_run = dr;
        }
        if let Some(p) = parallel {
            cfg.parallel = p;
        }
        Self { inner: cfg }
    }

    #[getter]
    fn dry_run(&self) -> bool {
        self.inner.dry_run
    }

    #[getter]
    fn parallel(&self) -> usize {
        self.inner.parallel
    }

    fn __repr__(&self) -> String {
        format!(
            "TransformConfig(source_cloud='{}', dry_run={}, parallel={})",
            self.inner.source_cloud, self.inner.dry_run, self.inner.parallel
        )
    }
}

// ---------------------------------------------------------------------------
// Result wrapper classes
// ---------------------------------------------------------------------------

/// Result of transforming a single file.
#[pyclass(name = "TransformResult")]
pub struct PyTransformResult {
    path: String,
    diff: String,
    patterns: Vec<String>,
    confidence: f64,
    warnings: Vec<String>,
    applied: bool,
}

#[pymethods]
impl PyTransformResult {
    #[getter]
    fn path(&self) -> &str {
        &self.path
    }

    #[getter]
    fn diff(&self) -> &str {
        &self.diff
    }

    #[getter]
    fn patterns(&self) -> Vec<String> {
        self.patterns.clone()
    }

    #[getter]
    fn confidence(&self) -> f64 {
        self.confidence
    }

    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.warnings.clone()
    }

    #[getter]
    fn applied(&self) -> bool {
        self.applied
    }

    fn __repr__(&self) -> String {
        format!(
            "TransformResult(path='{}', confidence={:.2}, patterns={})",
            self.path,
            self.confidence,
            self.patterns.len()
        )
    }
}

impl PyTransformResult {
    fn from_core(r: cloudshift_core::TransformResult) -> Self {
        Self {
            path: r.path,
            diff: r.diff,
            patterns: r.patterns.iter().map(|p| p.pattern_id.to_string()).collect(),
            confidence: r.confidence.value(),
            warnings: r.warnings.iter().map(|w| w.message.clone()).collect(),
            applied: r.applied,
        }
    }
}

/// Summary of changes to a single file within a repo transform.
#[pyclass(name = "FileChange")]
pub struct PyFileChange {
    file: String,
    language: String,
    constructs_detected: usize,
    patterns_matched: usize,
    confidence: f64,
    effort: String,
    diff: String,
}

#[pymethods]
impl PyFileChange {
    #[getter]
    fn file(&self) -> &str {
        &self.file
    }

    #[getter]
    fn language(&self) -> &str {
        &self.language
    }

    #[getter]
    fn constructs_detected(&self) -> usize {
        self.constructs_detected
    }

    #[getter]
    fn patterns_matched(&self) -> usize {
        self.patterns_matched
    }

    #[getter]
    fn confidence(&self) -> f64 {
        self.confidence
    }

    #[getter]
    fn effort(&self) -> &str {
        &self.effort
    }

    #[getter]
    fn diff(&self) -> &str {
        &self.diff
    }

    fn __repr__(&self) -> String {
        format!(
            "FileChange(file='{}', confidence={:.2})",
            self.file, self.confidence
        )
    }
}

impl PyFileChange {
    fn from_core(c: &cloudshift_core::FileChange) -> Self {
        Self {
            file: c.file.clone(),
            language: c.language.to_string(),
            constructs_detected: c.constructs_detected,
            patterns_matched: c.patterns_matched,
            confidence: c.confidence.value(),
            effort: c.effort.to_string(),
            diff: c.diff.clone(),
        }
    }
}

/// Report for a full repository transformation.
#[pyclass(name = "RepoReport")]
pub struct PyRepoReport {
    changes: Vec<cloudshift_core::FileChange>,
    total_constructs: usize,
    average_confidence: f64,
}

#[pymethods]
impl PyRepoReport {
    #[getter]
    fn changes(&self) -> Vec<PyFileChange> {
        self.changes.iter().map(PyFileChange::from_core).collect()
    }

    #[getter]
    fn total_constructs(&self) -> usize {
        self.total_constructs
    }

    #[getter]
    fn average_confidence(&self) -> f64 {
        self.average_confidence
    }

    fn __repr__(&self) -> String {
        format!(
            "RepoReport(files={}, constructs={}, avg_confidence={:.2})",
            self.changes.len(),
            self.total_constructs,
            self.average_confidence
        )
    }
}

impl PyRepoReport {
    fn from_core(r: cloudshift_core::RepoReport) -> Self {
        Self {
            total_constructs: r.total_constructs,
            average_confidence: r.average_confidence.value(),
            changes: r.changes,
        }
    }
}

/// Pattern info returned from catalogue search.
#[pyclass(name = "PatternInfo")]
pub struct PyPatternInfo {
    id: String,
    description: String,
    source_cloud: String,
    language: String,
    confidence: f64,
}

#[pymethods]
impl PyPatternInfo {
    #[getter]
    fn id(&self) -> &str {
        &self.id
    }

    #[getter]
    fn description(&self) -> &str {
        &self.description
    }

    #[getter]
    fn source_cloud(&self) -> &str {
        &self.source_cloud
    }

    #[getter]
    fn language(&self) -> &str {
        &self.language
    }

    #[getter]
    fn confidence(&self) -> f64 {
        self.confidence
    }

    fn __repr__(&self) -> String {
        format!("PatternInfo(id='{}', description='{}')", self.id, self.description)
    }
}

// ---------------------------------------------------------------------------
// Exposed Python functions
// ---------------------------------------------------------------------------

/// Transform a single file, returning a TransformResult.
///
/// The core implementation is a stub that will panic (todo!) until the
/// pipeline is fully implemented. The GIL is released during the Rust call.
#[pyfunction]
#[pyo3(name = "transform_file", signature = (path, source_cloud=None, threshold=None, dry_run=None))]
fn transform_file_py(
    py: Python<'_>,
    path: String,
    source_cloud: Option<&PySourceCloud>,
    threshold: Option<f64>,
    dry_run: Option<bool>,
) -> PyResult<PyTransformResult> {
    let mut cfg = TransformConfig::default();
    if let Some(sc) = source_cloud {
        cfg.source_cloud = sc.inner;
    }
    if let Some(dr) = dry_run {
        cfg.dry_run = dr;
    }
    // threshold is stored for future use when auto-apply logic is implemented
    let _ = threshold;

    let result = py
        .allow_threads(|| cloudshift_core::transform_file(&path, &cfg))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    Ok(PyTransformResult::from_core(result))
}

/// Transform an entire repository, returning a RepoReport.
///
/// The core implementation is a stub that will panic (todo!) until the
/// pipeline is fully implemented. The GIL is released during the Rust call.
#[pyfunction]
#[pyo3(name = "transform_repo", signature = (path, source_cloud=None, auto_apply_threshold=None, dry_run=None))]
fn transform_repo_py(
    py: Python<'_>,
    path: String,
    source_cloud: Option<&PySourceCloud>,
    auto_apply_threshold: Option<f64>,
    dry_run: Option<bool>,
) -> PyResult<PyRepoReport> {
    let mut cfg = TransformConfig::default();
    if let Some(sc) = source_cloud {
        cfg.source_cloud = sc.inner;
    }
    if let Some(dr) = dry_run {
        cfg.dry_run = dr;
    }
    // auto_apply_threshold is stored for future use
    let _ = auto_apply_threshold;

    let result = py
        .allow_threads(|| cloudshift_core::transform_repo(&path, &cfg))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    Ok(PyRepoReport::from_core(result))
}

/// Stream-style transform of a repository, returning a list of TransformResults.
///
/// In the future this will yield results lazily via a Python iterator.
/// Currently returns a list (simplified implementation).
#[pyfunction]
#[pyo3(name = "transform_repo_stream", signature = (path))]
fn transform_repo_stream_py(
    py: Python<'_>,
    path: String,
) -> PyResult<Vec<PyTransformResult>> {
    // For now, delegate to transform_repo and split into per-file results.
    // This is a simplified placeholder — the real streaming implementation
    // will use an async iterator once the pipeline supports it.
    let cfg = TransformConfig::default();
    let report = py
        .allow_threads(|| cloudshift_core::transform_repo(&path, &cfg))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    // Build synthetic per-file TransformResults from the RepoReport FileChanges
    let results: Vec<PyTransformResult> = report
        .changes
        .iter()
        .map(|c| PyTransformResult {
            path: c.file.clone(),
            diff: c.diff.clone(),
            patterns: Vec::new(),
            confidence: c.confidence.value(),
            warnings: Vec::new(),
            applied: false,
        })
        .collect();

    Ok(results)
}

/// Search the pattern catalogue by query string.
///
/// Returns a list of matching PatternInfo objects. Currently returns an
/// empty list since the catalogue search is not yet implemented.
#[pyfunction]
#[pyo3(name = "catalogue_search", signature = (query))]
fn catalogue_search_py(
    _py: Python<'_>,
    query: String,
) -> PyResult<Vec<PyPatternInfo>> {
    // Catalogue search is not yet implemented in cloudshift-core.
    // Return an empty list as a placeholder.
    let _ = query;
    Ok(Vec::new())
}

// ---------------------------------------------------------------------------
// Module init
// ---------------------------------------------------------------------------

#[pymodule]
fn _cloudshift_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(transform_file_py, m)?)?;
    m.add_function(wrap_pyfunction!(transform_repo_py, m)?)?;
    m.add_function(wrap_pyfunction!(transform_repo_stream_py, m)?)?;
    m.add_function(wrap_pyfunction!(catalogue_search_py, m)?)?;
    m.add_class::<PyTransformConfig>()?;
    m.add_class::<PySourceCloud>()?;
    m.add_class::<PyOutputFormat>()?;
    Ok(())
}
