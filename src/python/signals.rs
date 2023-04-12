use pyo3::prelude::*;

use crate::python::ConfigOwner;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[pyclass(name = "SignalRadix")]
pub enum SignalRadixPy {
    Binary = 0,
    Octal = 1,
    Decimal = 2,
    Hexadecimal = 3,
}

impl SignalRadixPy {
    fn print_python(&self) -> String {
        match self {
            Self::Binary => "SignalRadix.Binary".to_string(),
            Self::Octal => "SignalRadix.Octal".to_string(),
            Self::Decimal => "SignalRadix.Decimal".to_string(),
            Self::Hexadecimal => "SignalRadix.Hexadecimal".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SignalNodePyInternal {
    Group {
        name: String,
        children: Vec<SignalNodePyInternal>,
        expanded: bool,
        owner: ConfigOwner,
    },
    Vector {
        name: String,
        children: Vec<SignalNodePyInternal>,
        radix: SignalRadixPy,
        expanded: bool,
        owner: ConfigOwner,
    },
    // Children will be inferred if a multi-bit signal
    Signal {
        path: String,
        radix: SignalRadixPy,
        index: Option<usize>,
        expanded: bool,
        owner: ConfigOwner,
    },
    Spacer {
        owner: ConfigOwner,
    },
}

impl SignalNodePyInternal {
    pub(crate) fn print_python(&self, indents: usize) -> Vec<String> {
        let spaces = indents * 4;
        let mut v = Vec::new();
        match self {
            Self::Group {
                name,
                children,
                expanded,
                ..
            } => {
                let expanded = if *expanded { "True" } else { "False" };
                if children.is_empty() {
                    v.push(format!(
                        "{:indent$}new_group(\"{}\", {}, []),",
                        "",
                        name,
                        expanded,
                        indent = spaces
                    ));
                } else {
                    v.push(format!(
                        "{:indent$}new_group(\"{}\", {}, [",
                        "",
                        name,
                        expanded,
                        indent = spaces
                    ));
                    for c in children {
                        v.append(&mut c.print_python(indents + 1));
                    }
                    v.push(format!("{:indent$}]),", "", indent = spaces));
                }
            }
            Self::Vector {
                name,
                children,
                radix,
                expanded,
                ..
            } => {
                let expanded = if *expanded { "True" } else { "False" };
                if children.is_empty() {
                    v.push(format!(
                        "{:indent$}new_vector(\"{}\", {}, {}, []),",
                        "",
                        name,
                        radix.print_python(),
                        expanded,
                        indent = spaces
                    ));
                } else {
                    v.push(format!(
                        "{:indent$}new_vector(\"{}\", {}, {}, [",
                        "",
                        name,
                        radix.print_python(),
                        expanded,
                        indent = spaces
                    ));
                    for c in children {
                        v.append(&mut c.print_python(indents + 1));
                    }
                    v.push(format!("{:indent$}]),", "", indent = spaces));
                }
            }
            Self::Signal {
                path,
                radix,
                index,
                expanded,
                ..
            } => {
                let expanded = if *expanded { "True" } else { "False" };
                let index = if let Some(index) = index {
                    format!("{}", *index)
                } else {
                    "None".to_string()
                };
                v.push(format!(
                    "{:indent$}new_signal(\"{}\", {}, {}, {}),",
                    "",
                    path,
                    radix.print_python(),
                    expanded,
                    index,
                    indent = spaces
                ));
            }
            Self::Spacer { .. } => v.push(format!("{:indent$}new_spacer(),", "", indent = spaces)),
        }
        v
    }

    pub(crate) fn get_owner(&self) -> ConfigOwner {
        match self {
            Self::Group { owner, .. } => *owner,
            Self::Vector { owner, .. } => *owner,
            Self::Signal { owner, .. } => *owner,
            Self::Spacer { owner } => *owner,
        }
    }

    pub(crate) fn set_owner(&mut self, value: ConfigOwner) {
        match self {
            Self::Group {
                owner, children, ..
            } => {
                *owner = value;
                for child in children {
                    child.set_owner(value);
                }
            }
            Self::Vector {
                owner, children, ..
            } => {
                *owner = value;
                for child in children {
                    child.set_owner(value);
                }
            }
            Self::Spacer { owner } => *owner = value,
            Self::Signal { owner, .. } => *owner = value,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[pyclass(name = "SignalNode")]
pub struct SignalNodePy(SignalNodePyInternal);

impl SignalNodePy {
    pub(crate) fn into(self) -> SignalNodePyInternal {
        self.0
    }

    fn children(mut self, children_new: Vec<SignalNodePy>) -> Self {
        match &mut self.0 {
            SignalNodePyInternal::Group { children, .. } => {
                children.clear();
                for child in children_new {
                    children.push(child.0);
                }
            }
            SignalNodePyInternal::Vector { children, .. } => {
                children.clear();
                for child in children_new {
                    children.push(child.0);
                }
            }
            _ => {}
        }
        self
    }
}

#[pyfunction(name = "new_group")]
pub fn new_group_py(
    name: String,
    expanded: bool,
    children: Option<Vec<SignalNodePy>>,
) -> SignalNodePy {
    SignalNodePy(SignalNodePyInternal::Group {
        name,
        children: Vec::new(),
        expanded,
        owner: ConfigOwner::Nalu,
    })
    .children(children.unwrap_or(Vec::new()))
}

#[pyfunction(name = "new_vector")]
pub fn new_vector_py(
    name: String,
    radix: SignalRadixPy,
    expanded: bool,
    children: Option<Vec<SignalNodePy>>,
) -> SignalNodePy {
    SignalNodePy(SignalNodePyInternal::Vector {
        name,
        children: Vec::new(),
        radix,
        expanded,
        owner: ConfigOwner::Nalu,
    })
    .children(children.unwrap_or(Vec::new()))
}

#[pyfunction(name = "new_signal")]
pub fn new_signal_py(
    path: String,
    radix: SignalRadixPy,
    expanded: bool,
    index: Option<usize>,
) -> SignalNodePy {
    SignalNodePy(SignalNodePyInternal::Signal {
        path,
        radix,
        index,
        expanded,
        owner: ConfigOwner::Nalu,
    })
}

#[pyfunction(name = "new_spacer")]
pub fn new_spacer_py() -> SignalNodePy {
    SignalNodePy(SignalNodePyInternal::Spacer {
        owner: ConfigOwner::Nalu,
    })
}

#[pymethods]
impl SignalNodePy {
    #[pyo3(name = "is_group")]
    fn is_group_py(self_: PyRef<'_, Self>) -> PyResult<bool> {
        match self_.0 {
            SignalNodePyInternal::Group { .. } => Ok(true),
            _ => Ok(false),
        }
    }

    #[pyo3(name = "is_vector")]
    fn is_vector_py(self_: PyRef<'_, Self>) -> PyResult<bool> {
        match self_.0 {
            SignalNodePyInternal::Vector { .. } => Ok(true),
            _ => Ok(false),
        }
    }

    #[pyo3(name = "is_signal")]
    fn is_signal_py(self_: PyRef<'_, Self>) -> PyResult<bool> {
        match self_.0 {
            SignalNodePyInternal::Signal { .. } => Ok(true),
            _ => Ok(false),
        }
    }

    #[pyo3(name = "is_spacer")]
    fn is_spacer_py(self_: PyRef<'_, Self>) -> PyResult<bool> {
        match self_.0 {
            SignalNodePyInternal::Spacer { .. } => Ok(true),
            _ => Ok(false),
        }
    }

    #[pyo3(name = "add_child")]
    fn add_child(mut self_: PyRefMut<'_, Self>, child: Self) -> PyResult<()> {
        match &mut self_.0 {
            SignalNodePyInternal::Group { children, .. } => children.push(child.0),
            SignalNodePyInternal::Vector { children, .. } => match child.0 {
                child @ (SignalNodePyInternal::Signal { .. }
                | SignalNodePyInternal::Vector { .. }) => children.push(child),
                SignalNodePyInternal::Group { .. } => {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "Adding child node of type group to a vector!",
                    ))
                }
                SignalNodePyInternal::Spacer { .. } => {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "Adding child node of type spacer to a vector!",
                    ))
                }
            },
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "Adding child node to node without children!",
                ))
            }
        }
        Ok(())
    }
}
