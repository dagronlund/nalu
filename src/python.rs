mod bitvector;
mod buffer;
mod vcd_header;
mod waveform;

#[test]
fn test_pyo3() -> pyo3::prelude::PyResult<()> {
    use std::sync::Arc;

    use pyo3::prelude::*;
    use pyo3::types::IntoPyDict;

    let nalu_user = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/nalu_user.py"));

    Python::with_gil(|py| {
        let sys = py.import("sys")?;
        let version: String = sys.getattr("version")?.extract()?;

        let locals = [("os", py.import("os")?)].into_py_dict(py);
        let code = "os.getenv('USER') or os.getenv('USERNAME') or 'Unknown'";
        let user: String = py.eval(code, None, Some(&locals))?.extract()?;
        println!("Hello {}, I'm Python {}", user, version);

        #[pyclass]
        struct ClassTest {
            string: std::sync::Arc<String>,
        }

        impl ClassTest {
            fn new(string: String) -> Self {
                Self {
                    string: std::sync::Arc::new(string),
                }
            }
        }

        #[pymethods]
        impl ClassTest {
            #[pyo3(name = "test_method")]
            fn test_method_py(self_: PyRef<'_, Self>, arg: Option<usize>) -> PyResult<usize> {
                if let Some(arg) = arg {
                    println!("Some({})", arg);
                } else {
                    println!("None");
                }
                println!("{}", self_.string);
                Ok(self_.string.len())
            }

            #[pyo3(name = "mut_method")]
            fn mut_method_py(mut self_: PyRefMut<'_, Self>) -> PyResult<usize> {
                self_.string = Arc::new(format!("{}{}", self_.string, self_.string));
                println!("{}", self_.string);
                Ok(self_.string.len())
            }
        }

        #[pyfunction]
        #[pyo3(name = "no_args")]
        fn no_args_py() -> usize {
            println!("From rust!!!");
            42
        }

        let nalu = PyModule::new(py, "nalu")?;
        nalu.add_function(wrap_pyfunction!(no_args_py, nalu)?)?;

        py.import("sys")?
            .getattr("modules")?
            .set_item("nalu", nalu)?;

        let main: Py<PyAny> = PyModule::from_code(py, nalu_user, "nalu_user", "nalu_user")?
            .getattr("main")?
            .into();

        let test_value = ClassTest::new(String::from("hello from rust class!"));

        let result = main.call1(py, (test_value,))?;

        println!("nalu_user.py returned {result}");

        Ok(())
    })
}
