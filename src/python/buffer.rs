use pyo3::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
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

    pub fn get_width(&self) -> u16 {
        self.width
    }

    pub fn get_height(&self) -> u16 {
        self.height
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

    #[pyo3(name = "get_width")]
    fn get_width_py(self_: PyRef<'_, Self>) -> PyResult<u16> {
        Ok(self_.get_width())
    }

    #[pyo3(name = "get_height")]
    fn get_height_py(self_: PyRef<'_, Self>) -> PyResult<u16> {
        Ok(self_.get_height())
    }
}
