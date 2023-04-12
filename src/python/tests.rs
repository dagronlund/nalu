#[test]
fn test_config_load() {
    use crate::python::signals::{SignalNodePyInternal, SignalRadixPy};
    use crate::python::utils::run_config;
    use crate::python::ConfigOwner;
    use makai_vcd_reader::parser::VcdHeader;
    use std::{path::PathBuf, sync::Arc};

    let nodes_nalu = run_config(
        PathBuf::from("res/gecko.py"),
        Arc::new(VcdHeader::new()),
        ConfigOwner::Nalu,
    )
    .unwrap();
    assert_eq!(
        nodes_nalu,
        vec![SignalNodePyInternal::Signal {
            path: String::from("TOP.clk"),
            radix: SignalRadixPy::Hexadecimal,
            index: None,
            expanded: false,
            owner: ConfigOwner::Nalu
        }]
    );

    let nodes_user = run_config(
        PathBuf::from("res/gecko.py"),
        Arc::new(VcdHeader::new()),
        ConfigOwner::User,
    )
    .unwrap();
    assert_eq!(
        nodes_user,
        vec![SignalNodePyInternal::Signal {
            path: String::from("TOP.rst"),
            radix: SignalRadixPy::Hexadecimal,
            index: None,
            expanded: false,
            owner: ConfigOwner::User,
        }]
    );
}

#[test]
fn test_config_save() {
    use crate::python::signals::{SignalNodePyInternal, SignalRadixPy};
    use crate::python::utils::{run_config, save_config};
    use crate::python::ConfigOwner;
    use makai_vcd_reader::parser::VcdHeader;
    use std::io::Write;
    use std::{path::PathBuf, sync::Arc};

    // Copy template python to tempfile
    let template = std::fs::read(PathBuf::from("res/nalu.py")).unwrap();
    let mut write_tmp = tempfile::NamedTempFile::new().unwrap();
    write_tmp.write(&template).unwrap();

    // Check template tempfile is correct
    assert_eq!(
        run_config(
            write_tmp.path().to_path_buf(),
            Arc::new(VcdHeader::new()),
            ConfigOwner::Nalu,
        )
        .unwrap(),
        vec![]
    );
    assert_eq!(
        run_config(
            write_tmp.path().to_path_buf(),
            Arc::new(VcdHeader::new()),
            ConfigOwner::User,
        )
        .unwrap(),
        vec![]
    );

    // Add two signals, one should be ignored
    let mut nodes_nalu = Vec::new();
    nodes_nalu.push(SignalNodePyInternal::Signal {
        path: String::from("TOP.clk"),
        radix: SignalRadixPy::Hexadecimal,
        index: None,
        expanded: false,
        owner: ConfigOwner::Nalu,
    });
    nodes_nalu.push(SignalNodePyInternal::Signal {
        path: String::from("TOP.rst"),
        radix: SignalRadixPy::Hexadecimal,
        index: None,
        expanded: false,
        owner: ConfigOwner::User,
    });

    // Add a signal group
    nodes_nalu.push(SignalNodePyInternal::Group {
        name: String::from("my_group"),
        children: vec![SignalNodePyInternal::Signal {
            path: String::from("TOP.rst"),
            radix: SignalRadixPy::Hexadecimal,
            index: None,
            expanded: false,
            owner: ConfigOwner::Nalu,
        }],
        expanded: false,
        owner: ConfigOwner::Nalu,
    });

    // Save new config to tempfile
    save_config(write_tmp.path().to_path_buf(), &nodes_nalu, false).unwrap();

    // Check updated tempfile is correct
    assert_eq!(
        run_config(
            write_tmp.path().to_path_buf(),
            Arc::new(VcdHeader::new()),
            ConfigOwner::Nalu,
        )
        .unwrap(),
        vec![
            SignalNodePyInternal::Signal {
                path: String::from("TOP.clk"),
                radix: SignalRadixPy::Hexadecimal,
                index: None,
                expanded: false,
                owner: ConfigOwner::Nalu,
            },
            SignalNodePyInternal::Group {
                name: String::from("my_group"),
                children: vec![SignalNodePyInternal::Signal {
                    path: String::from("TOP.rst"),
                    radix: SignalRadixPy::Hexadecimal,
                    index: None,
                    expanded: false,
                    owner: ConfigOwner::Nalu,
                }],
                expanded: false,
                owner: ConfigOwner::Nalu,
            }
        ]
    );
    assert_eq!(
        run_config(
            write_tmp.path().to_path_buf(),
            Arc::new(VcdHeader::new()),
            ConfigOwner::User,
        )
        .unwrap(),
        vec![]
    );
}

#[test]
fn test_config_save_force() {
    use crate::python::signals::{SignalNodePyInternal, SignalRadixPy};
    use crate::python::utils::{run_config, save_config};
    use crate::python::ConfigOwner;
    use makai_vcd_reader::parser::VcdHeader;
    use std::io::Write;
    use std::sync::Arc;

    // Create empty tempfile
    let mut write_tmp = tempfile::NamedTempFile::new().unwrap();
    write_tmp.write(&Vec::new()).unwrap();

    // Add two signals, one should be ignored
    let mut nodes_nalu = Vec::new();
    nodes_nalu.push(SignalNodePyInternal::Signal {
        path: String::from("TOP.clk"),
        radix: SignalRadixPy::Hexadecimal,
        index: None,
        expanded: false,
        owner: ConfigOwner::Nalu,
    });
    nodes_nalu.push(SignalNodePyInternal::Signal {
        path: String::from("TOP.rst"),
        radix: SignalRadixPy::Hexadecimal,
        index: None,
        expanded: false,
        owner: ConfigOwner::User,
    });

    // Add a signal group
    nodes_nalu.push(SignalNodePyInternal::Group {
        name: String::from("my_group"),
        children: vec![SignalNodePyInternal::Signal {
            path: String::from("TOP.rst"),
            radix: SignalRadixPy::Hexadecimal,
            index: None,
            expanded: false,
            owner: ConfigOwner::Nalu,
        }],
        expanded: false,
        owner: ConfigOwner::Nalu,
    });

    // Save new config to tempfile (no force)
    save_config(write_tmp.path().to_path_buf(), &nodes_nalu, false).unwrap_err();

    // Check file temp is empty
    assert!(std::fs::read(write_tmp.path()).unwrap().is_empty());

    // Save new config to tempfile (force)
    save_config(write_tmp.path().to_path_buf(), &nodes_nalu, true).unwrap();

    // Check updated tempfile is correct
    assert_eq!(
        run_config(
            write_tmp.path().to_path_buf(),
            Arc::new(VcdHeader::new()),
            ConfigOwner::Nalu,
        )
        .unwrap(),
        vec![
            SignalNodePyInternal::Signal {
                path: String::from("TOP.clk"),
                radix: SignalRadixPy::Hexadecimal,
                index: None,
                expanded: false,
                owner: ConfigOwner::Nalu,
            },
            SignalNodePyInternal::Group {
                name: String::from("my_group"),
                children: vec![SignalNodePyInternal::Signal {
                    path: String::from("TOP.rst"),
                    radix: SignalRadixPy::Hexadecimal,
                    index: None,
                    expanded: false,
                    owner: ConfigOwner::Nalu,
                }],
                expanded: false,
                owner: ConfigOwner::Nalu,
            }
        ]
    );
    assert_eq!(
        run_config(
            write_tmp.path().to_path_buf(),
            Arc::new(VcdHeader::new()),
            ConfigOwner::User,
        )
        .unwrap(),
        vec![]
    );
}
