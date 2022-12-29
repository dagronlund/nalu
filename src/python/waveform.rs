use std::sync::Arc;

use pyo3::prelude::*;

use waveform_db::{Waveform, WaveformValueResult};

use crate::python::bitvector::BitVectorPy;

#[pyclass]
pub struct WaveformValueResultPy {
    value: WaveformValueResult,
}

impl WaveformValueResultPy {
    pub fn new(value: WaveformValueResult) -> Self {
        Self { value }
    }
}

#[pymethods]
impl WaveformValueResultPy {
    #[pyo3(name = "is_unknown")]
    pub fn is_unknown_py(self_: PyRef<'_, Self>) -> PyResult<bool> {
        Ok(match &self_.value {
            WaveformValueResult::Vector(bv, _) => bv.is_unknown(),
            _ => false,
        })
    }

    #[pyo3(name = "is_high_impedance")]
    pub fn is_high_impedance_py(self_: PyRef<'_, Self>) -> PyResult<bool> {
        Ok(match &self_.value {
            WaveformValueResult::Vector(bv, _) => bv.is_high_impedance(),
            _ => false,
        })
    }

    #[pyo3(name = "get_timestamp_index")]
    pub fn get_timestamp_index_py(self_: PyRef<'_, Self>) -> PyResult<usize> {
        Ok(match &self_.value {
            WaveformValueResult::Vector(_, index) | WaveformValueResult::Real(_, index) => *index,
        })
    }

    #[pyo3(name = "get_vector")]
    pub fn get_vector_py(self_: PyRef<'_, Self>) -> PyResult<Option<BitVectorPy>> {
        Ok(match &self_.value {
            WaveformValueResult::Vector(value, _) => Some(BitVectorPy::new(value.clone())),
            WaveformValueResult::Real(_, _) => None,
        })
    }

    #[pyo3(name = "get_real")]
    pub fn get_real_py(self_: PyRef<'_, Self>) -> PyResult<Option<f64>> {
        Ok(match &self_.value {
            WaveformValueResult::Vector(_, _) => None,
            WaveformValueResult::Real(value, _) => Some(*value),
        })
    }
}

#[pyclass]
pub struct WaveformPy {
    waveform: Arc<Waveform>,
}

impl WaveformPy {
    pub fn new(waveform: Arc<Waveform>) -> Self {
        Self { waveform }
    }
}

#[pymethods]
impl WaveformPy {
    #[pyo3(name = "get_timestamp_range")]
    fn get_timestamp_range_py(self_: PyRef<'_, Self>) -> PyResult<(u64, u64)> {
        let result = self_.waveform.get_timestamp_range();
        Ok((result.start, result.end))
    }

    #[pyo3(name = "search_timestamp")]
    fn search_timestamp_py(self_: PyRef<'_, Self>, timestamp: u64) -> PyResult<Option<usize>> {
        Ok(self_.waveform.search_timestamp(timestamp))
    }

    #[pyo3(name = "search_timestamp_after")]
    fn search_timestamp_after_py(
        self_: PyRef<'_, Self>,
        timestamp: u64,
    ) -> PyResult<Option<usize>> {
        Ok(self_.waveform.search_timestamp_after(timestamp))
    }

    #[pyo3(name = "search_timestamp_range")]
    fn search_timestamp_range_py(
        self_: PyRef<'_, Self>,
        timestamp_range: (u64, u64),
        greedy: bool,
    ) -> PyResult<Option<(usize, usize)>> {
        if let Some(result) = self_
            .waveform
            .search_timestamp_range(timestamp_range.0..timestamp_range.1, greedy)
        {
            Ok(Some((result.start, result.end)))
        } else {
            Ok(None)
        }
    }

    #[pyo3(name = "search_value")]
    fn search_value_py(
        self_: PyRef<'_, Self>,
        idcode: usize,
        timestamp_index: usize,
        bit_index: Option<usize>,
    ) -> PyResult<Option<WaveformValueResultPy>> {
        if let Some(value) =
            self_
                .waveform
                .search_value_bit_index(idcode, timestamp_index, bit_index)
        {
            Ok(Some(WaveformValueResultPy::new(value)))
        } else {
            Ok(None)
        }
    }
}
