use std::{path::PathBuf, sync::Arc};

use makai_vcd_reader::parser::VcdHeader;
use makai_waveform_db::Waveform;
use pyo3::{exceptions::PyFileNotFoundError, prelude::*};
use tui::{text::Spans, widgets::Paragraph};

use crate::python::{
    buffer::BufferPy,
    signals::{
        new_group_py, new_signal_py, new_spacer_py, new_vector_py, SignalNodePy,
        SignalNodePyInternal, SignalRadixPy,
    },
    vcd_header::VcdHeaderPy,
    waveform::{WaveformPy, WaveformSearchModePy},
    ConfigOwner,
};

fn add_nalu_module(py: Python) -> PyResult<()> {
    let nalu = PyModule::new(py, "nalu")?;
    nalu.add_class::<WaveformSearchModePy>()?;
    nalu.add_class::<SignalRadixPy>()?;
    nalu.add_function(wrap_pyfunction!(new_group_py, nalu)?)?;
    nalu.add_function(wrap_pyfunction!(new_vector_py, nalu)?)?;
    nalu.add_function(wrap_pyfunction!(new_signal_py, nalu)?)?;
    nalu.add_function(wrap_pyfunction!(new_spacer_py, nalu)?)?;
    py.import("sys")?
        .getattr("modules")?
        .set_item("nalu", nalu)?;
    Ok(())
}

fn load_python_file(path: PathBuf) -> PyResult<String> {
    let Ok(python_bytes) = std::fs::read(path.clone()) else {
        log::warn!("Config {path:?} not found!");
        return Err(PyFileNotFoundError::new_err(format!("{path:?} not found!")));
    };
    Ok(String::from_utf8_lossy(&python_bytes).to_string())
}

pub fn run_interactive(
    path: PathBuf,
    width: u16,
    height: u16,
    waveform: Arc<Waveform>,
    vcd_header: Arc<VcdHeader>,
    cursor: u64,
) -> PyResult<Paragraph<'static>> {
    let file = load_python_file(path.clone())?;
    let file_name = std::fs::canonicalize(path.clone())?;
    let file_name = file_name.to_str().unwrap_or("");
    let module_name = path.file_name().map_or("", |m| {
        m.to_str()
            .unwrap_or("")
            .split(".")
            .collect::<Vec<&str>>()
            .first()
            .map_or("", |m| m)
    });

    let buffer: BufferPy = Python::with_gil(|py| {
        add_nalu_module(py)?;
        let main: Py<PyAny> = PyModule::from_code(py, &file, file_name, module_name)?
            .getattr("interactive")?
            .into();
        main.call1(
            py,
            (
                BufferPy::new(width, height),
                WaveformPy::new(waveform),
                VcdHeaderPy::new(vcd_header),
                cursor,
            ),
        )?
        .extract::<BufferPy>(py)
    })?;

    let mut spans = Vec::new();
    for y in 0..buffer.get_height() {
        let mut string = String::new();
        for x in 0..buffer.get_width() {
            string.push(buffer.get_cell(x, y));
        }
        spans.push(Spans::from(string.trim().to_string()));
    }
    Ok(Paragraph::new(spans))
}

pub fn run_config(
    path: PathBuf,
    vcd_header: Arc<VcdHeader>,
    owner: ConfigOwner,
) -> PyResult<Vec<SignalNodePyInternal>> {
    match owner {
        ConfigOwner::Nalu => log::info!("Loading nalu config {path:?} ..."),
        ConfigOwner::User => log::info!("Loading user config {path:?} ..."),
    }
    let file = load_python_file(path.clone())?;
    let file_name = std::fs::canonicalize(path.clone())?;
    let file_name = file_name.to_str().unwrap_or("");
    let module_name = path.file_name().map_or("", |m| {
        m.to_str()
            .unwrap_or("")
            .split(".")
            .collect::<Vec<&str>>()
            .first()
            .map_or("", |m| m)
    });

    let nodes: Vec<SignalNodePy> = Python::with_gil(|py| {
        add_nalu_module(py)?;
        let main: Py<PyAny> = PyModule::from_code(py, &file, file_name, module_name)?
            .getattr(match owner {
                ConfigOwner::Nalu => "nalu_config",
                ConfigOwner::User => "user_config",
            })?
            .into();
        main.call1(py, (VcdHeaderPy::new(vcd_header),))?
            .extract::<Vec<SignalNodePy>>(py)
    })?;

    let mut nodes = nodes
        .into_iter()
        .map(|n| n.into())
        .collect::<Vec<SignalNodePyInternal>>();

    for node in &mut nodes {
        node.set_owner(owner);
    }
    Ok(nodes)
}

#[derive(Debug)]
pub enum SaveConfigError {
    MangledFile,
    Io(std::io::Error),
}

impl From<std::io::Error> for SaveConfigError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

fn split_generated(string: String) -> Result<(Vec<String>, Vec<String>), SaveConfigError> {
    #[derive(Debug, PartialEq, Eq)]
    enum Stage {
        Pre,
        Generated,
        Post,
    }

    // Split configuration file apart
    let mut stage = Stage::Pre;
    let mut pre = Vec::new();
    let mut post = Vec::new();
    for line in string.lines() {
        match stage {
            Stage::Pre => {
                if line.trim() == "### BEGIN NALU GENERATED CODE ###" {
                    stage = Stage::Generated;
                } else {
                    pre.push(line.to_string());
                }
            }
            Stage::Generated => {
                if line.trim() == "### END NALU GENERATED CODE ###" {
                    stage = Stage::Post;
                }
            }
            Stage::Post => post.push(line.to_string()),
        }
    }

    if stage != Stage::Post {
        Err(SaveConfigError::MangledFile)
    } else {
        Ok((pre, post))
    }
}

pub fn save_config(
    path: PathBuf,
    nodes: &Vec<SignalNodePyInternal>,
    force: bool,
) -> Result<(), SaveConfigError> {
    use std::io::Write;

    // Use configuration file if it exists, otherwise use template
    let default_str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/res/nalu.py"));
    let string = if let Ok(bytes) = std::fs::read(path.clone()) {
        String::from_utf8_lossy(&bytes).to_string()
    } else {
        default_str.to_string()
    };

    // Try splitting the file, and if that fails and force is enabled, do it on
    // the default file
    let (pre, post) = match split_generated(string) {
        Ok((pre, post)) => (pre, post),
        Err(_) if force => split_generated(default_str.to_string())?,
        Err(err) => return Err(err),
    };

    // Build new configuration python
    let mut generated = Vec::new();
    generated.push("### BEGIN NALU GENERATED CODE ###".to_string());
    generated.push("# fmt: off".to_string());
    generated.push("def nalu_config(vcd_header):".to_string());
    generated.push("    \"\"\"Nalu generated waveform config\"\"\"".to_string());
    if nodes.is_empty() {
        generated.push("    return []".to_string());
    } else {
        generated.push("    return [".to_string());
        for node in nodes {
            if node.get_owner() == ConfigOwner::Nalu {
                generated.append(&mut node.print_python(2));
            }
        }
        generated.push("    ]".to_string());
    }
    generated.push("# fmt: on".to_string());
    generated.push("### END NALU GENERATED CODE ###".to_string());

    // Write configuration python to file
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    for line in pre {
        file.write(line.as_bytes())?;
        file.write("\n".as_bytes())?;
    }
    for line in generated {
        file.write(line.as_bytes())?;
        file.write("\n".as_bytes())?;
    }
    for line in post {
        file.write(line.as_bytes())?;
        file.write("\n".as_bytes())?;
    }
    file.flush()?;
    Ok(())
}
