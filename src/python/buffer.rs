use pyo3::prelude::*;

#[pyclass]
pub struct BufferPy {
    width: u16,
    height: u16,
    buffer: Vec<char>,
}

impl BufferPy {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            buffer: vec![' '].repeat((width * height) as usize),
        }
    }

    pub fn get_cell(&self, x: u16, y: u16) -> char {
        if x < self.width && y < self.height {
            self.buffer[(y * self.width + x) as usize]
        } else {
            ' '
        }
    }

    pub fn set_cell(&mut self, x: u16, y: u16, c: char) {
        if x < self.width && y < self.height {
            self.buffer[(y * self.width + x) as usize] = c;
        }
    }
}

#[pymethods]
impl BufferPy {
    #[pyo3(name = "get_cell")]
    fn get_cell_py(self_: PyRef<'_, Self>, x: u16, y: u16) -> PyResult<char> {
        Ok(self_.get_cell(x, y))
    }

    #[pyo3(name = "set_cell")]
    fn set_cell_py(mut self_: PyRefMut<'_, Self>, x: u16, y: u16, c: char) -> PyResult<()> {
        self_.set_cell(x, y, c);
        Ok(())
    }
}
