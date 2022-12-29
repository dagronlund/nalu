use std::sync::Arc;

use pyo3::prelude::*;

use vcd_parser::parser::{VcdHeader, VcdVariable};

#[pyclass]
pub struct VcdVariablePy {
    value: VcdVariable,
}

impl VcdVariablePy {
    pub fn new(value: VcdVariable) -> Self {
        Self { value }
    }
}

#[pymethods]
impl VcdVariablePy {
    #[pyo3(name = "get_name")]
    fn get_name_py(self_: PyRef<'_, Self>) -> PyResult<String> {
        Ok(self_.value.get_name().clone())
    }

    #[pyo3(name = "get_bit_width")]
    pub fn get_bit_width_py(self_: PyRef<'_, Self>) -> PyResult<usize> {
        Ok(self_.value.get_bit_width())
    }

    #[pyo3(name = "get_idcode")]
    pub fn get_idcode_py(self_: PyRef<'_, Self>) -> PyResult<usize> {
        Ok(self_.value.get_idcode())
    }
}

#[pyclass]
pub struct VcdHeaderPy {
    value: Arc<VcdHeader>,
}

impl VcdHeaderPy {
    pub fn new(value: Arc<VcdHeader>) -> Self {
        Self { value }
    }
}

#[pymethods]
impl VcdHeaderPy {
    #[pyo3(name = "get_variable")]
    pub fn get_variable_py(self_: PyRef<'_, Self>, path: &str) -> PyResult<Option<VcdVariablePy>> {
        if let Some(result) = self_.value.get_variable(path) {
            Ok(Some(VcdVariablePy::new(result.clone())))
        } else {
            Ok(None)
        }
    }

    #[pyo3(name = "get_version")]
    pub fn get_version_py(self_: PyRef<'_, Self>) -> PyResult<Option<String>> {
        Ok(self_.value.get_version().clone())
    }

    #[pyo3(name = "get_date")]
    pub fn get_date_py(self_: PyRef<'_, Self>) -> PyResult<Option<String>> {
        Ok(self_.value.get_date().clone())
    }

    #[pyo3(name = "get_timescale")]
    pub fn get_timescale_py(self_: PyRef<'_, Self>) -> PyResult<Option<i32>> {
        Ok(self_.value.get_timescale().clone())
    }
}
