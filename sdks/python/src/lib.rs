use pyo3::prelude::*;

mod evaluator;
mod types;

/// Tenor contract evaluator Python module.
#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<evaluator::TenorEvaluator>()?;
    Ok(())
}
