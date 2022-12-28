#[test]
fn test_pyo3() -> pyo3::prelude::PyResult<()> {
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
        let result = main.call0(py)?;

        println!("nalu_user.py returned {result}");

        Ok(())
    })
}
