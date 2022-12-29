use num_bigint::BigUint;
use pyo3::prelude::*;

use waveform_db::bitvector::{BitVector, Logic};

#[pyclass]
pub enum LogicPy {
    Zero = 0,
    One = 1,
    Unknown = 2,
    HighImpedance = 3,
}

#[pyclass]
pub struct BitVectorPy {
    bitvector: BitVector,
}

impl BitVectorPy {
    pub fn new(bitvector: BitVector) -> Self {
        Self { bitvector }
    }
}

#[pymethods]
impl BitVectorPy {
    #[pyo3(name = "get_value")]
    fn get_value_py(self_: PyRef<'_, Self>) -> PyResult<BigUint> {
        let mut value = BigUint::default();
        for index in 0..self_.bitvector.get_bit_width() {
            match self_.bitvector.get_bit(index) {
                Logic::Zero | Logic::Unknown => value.set_bit(index as u64, false),
                Logic::One | Logic::HighImpedance => value.set_bit(index as u64, true),
            }
        }
        Ok(value)
    }

    #[pyo3(name = "get_mask")]
    fn get_mask_py(self_: PyRef<'_, Self>) -> PyResult<BigUint> {
        let mut mask = BigUint::default();
        for index in 0..self_.bitvector.get_bit_width() {
            match self_.bitvector.get_bit(index) {
                Logic::Zero | Logic::One => mask.set_bit(index as u64, false),
                Logic::Unknown | Logic::HighImpedance => mask.set_bit(index as u64, true),
            }
        }
        Ok(mask)
    }

    #[pyo3(name = "get_logic")]
    fn get_logic_py(self_: PyRef<'_, Self>, index: usize) -> PyResult<Option<LogicPy>> {
        if index >= self_.bitvector.get_bit_width() {
            return Ok(None);
        }
        match self_.bitvector.get_bit(index) {
            Logic::Zero => Ok(Some(LogicPy::Zero)),
            Logic::One => Ok(Some(LogicPy::One)),
            Logic::Unknown => Ok(Some(LogicPy::Unknown)),
            Logic::HighImpedance => Ok(Some(LogicPy::HighImpedance)),
        }
    }

    #[pyo3(name = "get_bit")]
    fn get_bit_py(self_: PyRef<'_, Self>, index: usize) -> PyResult<Option<LogicPy>> {
        if index >= self_.bitvector.get_bit_width() {
            return Ok(None);
        }
        match self_.bitvector.get_bit(index) {
            Logic::Zero => Ok(Some(LogicPy::Zero)),
            Logic::One => Ok(Some(LogicPy::One)),
            _ => Ok(None),
        }
    }
}
