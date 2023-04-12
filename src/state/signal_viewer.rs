use std::path::PathBuf;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use makai::utils::messages::Messages;
use makai_vcd_reader::parser::{VcdHeader, VcdVariable};
use makai_waveform_db::bitvector::BitVectorRadix;
use pyo3::PyErr;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};
use tui_tiling::component::ComponentWidget;

use crate::{
    python::ConfigOwner,
    python::{
        signals::{SignalNodePyInternal, SignalRadixPy},
        utils::{run_config, save_config, SaveConfigError},
    },
    state::waveform_viewer::{WaveformNode, WaveformViewerMessage},
    widgets::browser::*,
};

#[derive(Debug)]
pub(crate) enum SignalViewerError {
    UnsavedSignals,
    ConfigNotAvailable,
    ConfigLoadError(PyErr),
    ConfigSaveError(SaveConfigError),
    VariableNotFound(String),
}

impl From<PyErr> for SignalViewerError {
    fn from(err: PyErr) -> Self {
        Self::ConfigLoadError(err)
    }
}

impl From<SaveConfigError> for SignalViewerError {
    fn from(err: SaveConfigError) -> Self {
        Self::ConfigSaveError(err)
    }
}

#[derive(Debug, Clone)]
pub enum SignalNode {
    Group {
        name: String,
        owner: ConfigOwner,
        saved: bool,
    },
    Vector {
        name: String,
        radix: BitVectorRadix,
        owner: ConfigOwner,
        saved: bool,
    },
    Signal {
        path: Vec<String>,
        vcd_variable: VcdVariable,
        radix: BitVectorRadix,
        index: Option<usize>,
        owner: ConfigOwner,
        saved: bool,
    },
    Spacer {
        owner: ConfigOwner,
        saved: bool,
    },
}

impl SignalNode {
    fn get_owner(&self) -> ConfigOwner {
        match self {
            Self::Group { owner, .. } => *owner,
            Self::Vector { owner, .. } => *owner,
            Self::Signal { owner, .. } => *owner,
            Self::Spacer { owner, .. } => *owner,
        }
    }

    fn is_saved(&self) -> bool {
        match self {
            Self::Group { saved, .. } => *saved,
            Self::Vector { saved, .. } => *saved,
            Self::Signal { saved, .. } => *saved,
            Self::Spacer { saved, .. } => *saved,
        }
    }

    fn set_saved(&mut self, value: bool) {
        match self {
            Self::Group { saved, .. } => *saved = value,
            Self::Vector { saved, .. } => *saved = value,
            Self::Signal { saved, .. } => *saved = value,
            Self::Spacer { saved, .. } => *saved = value,
        }
    }
}

impl std::fmt::Display for SignalNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spacer { .. } => write!(f, ""),
            Self::Group { name, .. } => write!(f, "Group: {} (TODO)", name),
            Self::Vector { name, .. } => write!(f, "Vector: {} (TODO)", name),
            Self::Signal {
                vcd_variable,
                index,
                ..
            } => {
                if let Some(index) = index {
                    write!(f, "{} [{}]", vcd_variable, index)
                } else {
                    write!(f, "{}", vcd_variable)
                }
            }
        }
    }
}

impl Default for SignalNode {
    fn default() -> Self {
        Self::Spacer {
            owner: ConfigOwner::Nalu,
            saved: false,
        }
    }
}

fn create_vcd_variable_node(
    path: Vec<String>,
    variable: VcdVariable,
    radix: BitVectorRadix,
    owner: ConfigOwner,
) -> BrowserNode<SignalNode> {
    log::info!("create_vcd_variable_node {path:?}");
    BrowserNode::from(
        Some(SignalNode::Signal {
            path: path.clone(),
            vcd_variable: variable.clone(),
            radix,
            index: None,
            owner,
            saved: false,
        }),
        Visibility::Collapsed,
        if variable.get_bit_width() > 1 {
            (0..variable.get_bit_width())
                .into_iter()
                .map(|i| SignalNode::Signal {
                    path: path.clone(),
                    vcd_variable: variable.clone(),
                    radix,
                    index: Some(i),
                    owner,
                    saved: false,
                })
                .map(|n| BrowserNode::new(n))
                .collect()
        } else {
            Vec::new()
        },
    )
}

/// Returns ConfigOwner::User only if the node and all children are owned by the
/// user scripting
fn get_node_owner(node: &BrowserNode<SignalNode>) -> ConfigOwner {
    if let Some(entry) = node.get_entry() {
        if entry.get_owner() == ConfigOwner::Nalu {
            return ConfigOwner::Nalu;
        }
    }
    for child in node.get_children() {
        if get_node_owner(child) == ConfigOwner::Nalu {
            return ConfigOwner::Nalu;
        }
    }
    ConfigOwner::User
}

/// Returns true only if the node and all children have been saved (user owned
/// nodes are always considered saved)
fn is_node_saved(node: &BrowserNode<SignalNode>) -> bool {
    if get_node_owner(node) == ConfigOwner::User {
        return true;
    }
    if let Some(entry) = node.get_entry() {
        if !entry.is_saved() {
            return false;
        }
    }
    for child in node.get_children() {
        if !is_node_saved(child) {
            return false;
        }
    }
    true
}

/// Recursively sets a node and its children to be saved
fn set_node_saved(node: &mut BrowserNode<SignalNode>, saved: bool) {
    if get_node_owner(node) == ConfigOwner::User {
        return;
    }
    if let Some(entry) = node.get_entry_mut() {
        entry.set_saved(saved);
    }
    for child in node.get_children_mut() {
        set_node_saved(child, saved);
    }
}

impl From<BitVectorRadix> for SignalRadixPy {
    fn from(radix: BitVectorRadix) -> Self {
        match radix {
            BitVectorRadix::Binary => Self::Binary,
            BitVectorRadix::Octal => Self::Octal,
            BitVectorRadix::Decimal => Self::Decimal,
            BitVectorRadix::Hexadecimal => Self::Hexadecimal,
        }
    }
}

impl From<SignalRadixPy> for BitVectorRadix {
    fn from(radix: SignalRadixPy) -> Self {
        match radix {
            SignalRadixPy::Binary => Self::Binary,
            SignalRadixPy::Octal => Self::Octal,
            SignalRadixPy::Decimal => Self::Decimal,
            SignalRadixPy::Hexadecimal => Self::Hexadecimal,
        }
    }
}

impl From<bool> for Visibility {
    fn from(expanded: bool) -> Self {
        if expanded {
            Visibility::Expanded
        } else {
            Visibility::Collapsed
        }
    }
}

/// Converts python provided nodes to BrowserNode compatible nodes
fn convert_from_config_node(
    node: &SignalNodePyInternal,
    vcd_header: &VcdHeader,
) -> Result<BrowserNode<SignalNode>, SignalViewerError> {
    match node {
        SignalNodePyInternal::Group {
            name,
            children,
            expanded,
            owner,
        } => {
            let children = children
                .into_iter()
                .map(|c| convert_from_config_node(c, vcd_header))
                .collect::<Result<Vec<BrowserNode<SignalNode>>, SignalViewerError>>()?;
            Ok(BrowserNode::from(
                Some(SignalNode::Group {
                    name: name.clone(),
                    owner: *owner,
                    saved: true,
                }),
                Visibility::from(*expanded),
                children,
            ))
        }
        SignalNodePyInternal::Vector {
            name,
            children,
            radix,
            expanded,
            owner,
        } => {
            let children = children
                .into_iter()
                .map(|c| convert_from_config_node(c, vcd_header))
                .collect::<Result<Vec<BrowserNode<SignalNode>>, SignalViewerError>>()?;
            Ok(BrowserNode::from(
                Some(SignalNode::Vector {
                    name: name.clone(),
                    radix: BitVectorRadix::from(*radix),
                    owner: *owner,
                    saved: true,
                }),
                Visibility::from(*expanded),
                children,
            ))
        }
        SignalNodePyInternal::Signal {
            path,
            radix,
            index,
            expanded,
            owner,
        } => {
            // Find vcd variable for this path if it exists
            let Some(vcd_variable) = vcd_header.get_variable(path).cloned() else {
                return Err(SignalViewerError::VariableNotFound(path.to_string()));
            };
            let path = path
                .split(".")
                .map(|s| s.to_string())
                .collect::<Vec<String>>();
            let children = if vcd_variable.get_bit_width() > 1 && index.is_none() {
                (0..vcd_variable.get_bit_width())
                    .into_iter()
                    .map(|i| SignalNode::Signal {
                        path: path.clone(),
                        vcd_variable: vcd_variable.clone(),
                        radix: BitVectorRadix::from(*radix),
                        index: Some(i),
                        owner: *owner,
                        saved: true,
                    })
                    .map(|n| BrowserNode::new(n))
                    .collect()
            } else {
                Vec::new()
            };
            Ok(BrowserNode::from(
                Some(SignalNode::Signal {
                    path: path.clone(),
                    vcd_variable: vcd_variable.clone(),
                    radix: BitVectorRadix::from(*radix),
                    index: *index,
                    owner: *owner,
                    saved: true,
                }),
                Visibility::from(*expanded),
                children,
            ))
        }
        SignalNodePyInternal::Spacer { owner } => Ok(BrowserNode::new(SignalNode::Spacer {
            owner: *owner,
            saved: true,
        })),
    }
}

/// Converts BrowserNode compatible nodes to python provided nodes
fn convert_to_config_node(
    node: &BrowserNode<SignalNode>,
    owner: ConfigOwner,
) -> Option<SignalNodePyInternal> {
    if get_node_owner(node) != owner {
        return None;
    }
    match node.get_entry() {
        Some(SignalNode::Group { name, owner, .. }) => Some(SignalNodePyInternal::Group {
            name: name.clone(),
            expanded: node.get_visibility() == Visibility::Expanded,
            owner: *owner,
            children: node
                .get_children()
                .iter()
                .filter_map(|child| convert_to_config_node(child, *owner))
                .collect::<Vec<SignalNodePyInternal>>(),
        }),
        Some(SignalNode::Vector {
            name, radix, owner, ..
        }) => Some(SignalNodePyInternal::Vector {
            name: name.clone(),
            radix: SignalRadixPy::from(*radix),
            expanded: node.get_visibility() == Visibility::Expanded,
            owner: *owner,
            children: node
                .get_children()
                .iter()
                .filter_map(|child| convert_to_config_node(child, *owner))
                .collect::<Vec<SignalNodePyInternal>>(),
        }),
        Some(SignalNode::Signal {
            path,
            radix,
            index,
            owner,
            ..
        }) => Some(SignalNodePyInternal::Signal {
            path: path.join("."),
            radix: SignalRadixPy::from(*radix),
            index: *index,
            expanded: node.get_visibility() == Visibility::Expanded,
            owner: *owner,
        }),
        Some(SignalNode::Spacer { owner, .. }) => {
            Some(SignalNodePyInternal::Spacer { owner: *owner })
        }
        None => None,
    }
}

#[derive(Clone)]
enum ListAction {
    Group,
    Delete,
    Expand,
}

pub(crate) enum SignalViewerMessage {
    NetlistAppend(Vec<String>, VcdVariable),
    NetlistInsert(Vec<String>, VcdVariable),
    WaveformKey(KeyEvent),
    SaveConfig {
        python_path: Option<PathBuf>,
        force: bool,
    },
    LoadConfig {
        vcd_header: Arc<VcdHeader>,
        python_path: Option<PathBuf>,
        force: bool,
    },
}

pub struct SignalViewerState {
    browser: BrowserState,
    node: BrowserNode<SignalNode>,
    messages: Messages,
}

impl SignalViewerState {
    pub fn new(messages: Messages) -> Self {
        Self {
            browser: BrowserState::new(true, true, false),
            node: BrowserNode::new_container(),
            messages,
        }
    }

    fn append_signal(&mut self, path: Vec<String>, variable: VcdVariable, radix: BitVectorRadix) {
        log::info!("Appending signal {path:?}...");
        self.node.get_children_mut().push(create_vcd_variable_node(
            path,
            variable,
            radix,
            ConfigOwner::Nalu,
        ));
        self.update_waveform_viewer();
    }

    fn load_config(
        &mut self,
        vcd_header: Arc<VcdHeader>,
        python_path: Option<PathBuf>,
        force: bool,
    ) -> Result<(), SignalViewerError> {
        // Check that all nalu-edited nodes are saved (or being forced), otherwise this will
        // overwrite nodes
        if !is_node_saved(&self.node) && !force {
            return Err(SignalViewerError::UnsavedSignals);
        }
        // Check that a configuration file was provided
        let Some(python_path) = python_path else {
            return Err(SignalViewerError::ConfigNotAvailable);
        };
        // Run the configuration file
        let mut nodes_nalu =
            run_config(python_path.clone(), vcd_header.clone(), ConfigOwner::Nalu)?
                .iter()
                .map(|node| convert_from_config_node(node, &vcd_header))
                .collect::<Result<Vec<BrowserNode<SignalNode>>, SignalViewerError>>()?;
        let mut nodes_user = run_config(python_path, vcd_header.clone(), ConfigOwner::User)?
            .iter()
            .map(|node| convert_from_config_node(node, &vcd_header))
            .collect::<Result<Vec<BrowserNode<SignalNode>>, SignalViewerError>>()?;
        // Replace existing nodes
        self.node.get_children_mut().clear();
        self.node.get_children_mut().append(&mut nodes_nalu);
        self.node.get_children_mut().append(&mut nodes_user);
        self.update_waveform_viewer();
        Ok(())
    }

    fn save_config(
        &mut self,
        python_path: Option<PathBuf>,
        force: bool,
    ) -> Result<(), SignalViewerError> {
        // Check that a configuration file was provided
        let Some(python_path) = python_path else {
            return Err(SignalViewerError::ConfigNotAvailable);
        };
        // Convert browser nodes to compatible configuration nodes
        let nodes_nalu = self
            .node
            .get_children()
            .iter()
            .filter_map(|node| convert_to_config_node(node, ConfigOwner::Nalu))
            .collect::<Vec<SignalNodePyInternal>>();
        save_config(python_path, &nodes_nalu, force)?;
        set_node_saved(&mut self.node, true);
        self.update_waveform_viewer();
        Ok(())
    }

    pub fn set_size(&mut self, size: &Rect, border_width: u16) {
        // Handle extra room above/below hierarchy in browser
        let margin = border_width as isize * 2;
        self.browser
            .set_height((size.height as isize - margin).max(0));
        self.browser.scroll_relative(&self.node, 0);
        self.update_waveform_viewer();
    }

    pub fn get_browser(&self) -> Browser<'_, SignalNode> {
        Browser::new(&self.browser, &self.node)
    }

    pub fn get_browser_state(&self) -> &BrowserState {
        &self.browser
    }

    pub fn get_node(&self) -> &BrowserNode<SignalNode> {
        &self.node
    }

    fn modify(&mut self, action: ListAction) {
        match action {
            ListAction::Group => {}
            ListAction::Delete => {}
            ListAction::Expand => {
                let path = self.browser.get_primary_selected_path(&self.node);
                if let Some(node) = self.node.get_node_mut(&path) {
                    match node.get_visibility() {
                        Visibility::Collapsed => node.set_visibility(Visibility::Expanded),
                        Visibility::Expanded => node.set_visibility(Visibility::Collapsed),
                    }
                }
            }
        }
        self.update_waveform_viewer();
    }

    pub fn update_waveform_viewer(&mut self) {
        let selected_path = self.browser.get_primary_selected_path(&self.node);
        let nodes = self
            .browser
            .get_visible_paths(&self.node)
            .into_iter()
            .map(|path| {
                let Some(node) = self.node.get_node(&path) else {
                    return None;
                };
                match node.get_entry().as_ref().unwrap() {
                    SignalNode::Signal {
                        vcd_variable,
                        radix,
                        index,
                        ..
                    } => Some(WaveformNode {
                        idcode: vcd_variable.get_idcode(),
                        index: *index,
                        radix: *radix,
                        is_selected: selected_path == path,
                    }),
                    _ => None,
                }
            })
            .collect::<Vec<Option<WaveformNode>>>();
        self.messages
            .push(WaveformViewerMessage::UpdateSignals(nodes));
    }
}

impl ComponentWidget for SignalViewerState {
    fn handle_mouse(&mut self, _x: u16, y: u16, kind: MouseEventKind) -> bool {
        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if self.browser.select_absolute(&self.node, y as isize, true) {
                    self.modify(ListAction::Expand);
                }
                self.update_waveform_viewer();
            }
            MouseEventKind::ScrollDown => {
                self.browser.select_relative(&self.node, 5, true);
                self.update_waveform_viewer();
            }
            MouseEventKind::ScrollUp => {
                self.browser.select_relative(&self.node, -5, true);
                self.update_waveform_viewer();
            }
            _ => return false,
        }
        true
    }

    fn handle_key(&mut self, e: KeyEvent) -> bool {
        let shift = e.modifiers.contains(KeyModifiers::SHIFT);
        match e.code {
            KeyCode::Up => self.browser.select_relative(&self.node, -1, !shift),
            KeyCode::Down => self.browser.select_relative(&self.node, 1, !shift),
            KeyCode::PageDown => self.browser.select_relative(&self.node, 20, !shift),
            KeyCode::PageUp => self.browser.select_relative(&self.node, -20, !shift),
            KeyCode::Enter => self.modify(ListAction::Expand),
            KeyCode::Char('g') => self.modify(ListAction::Group),
            KeyCode::Char('f') => {
                self.browser
                    .set_indent_enabled(!self.browser.is_full_name_enabled());
                self.browser
                    .set_full_name_enabled(!self.browser.is_full_name_enabled());
            }
            KeyCode::Delete => self.modify(ListAction::Delete),
            _ => return false,
        }
        self.update_waveform_viewer();
        true
    }

    fn handle_update(&mut self) -> bool {
        let mut updated = false;
        for message in self.messages.get::<SignalViewerMessage>() {
            match message {
                SignalViewerMessage::NetlistAppend(path, variable) => {
                    self.append_signal(path, variable, BitVectorRadix::Hexadecimal);
                    updated = true;
                }
                SignalViewerMessage::NetlistInsert(_, _) => {}
                SignalViewerMessage::WaveformKey(e) => updated |= self.handle_key(e),
                SignalViewerMessage::SaveConfig { python_path, force } => {
                    if let Err(err) = self.save_config(python_path, force) {
                        log::warn!("TODO: Handle save config error ({err:?})");
                    }
                }
                SignalViewerMessage::LoadConfig {
                    vcd_header,
                    python_path,
                    force,
                } => {
                    if let Err(err) = self.load_config(vcd_header, python_path, force) {
                        log::warn!("TODO: Handle load config error ({err:?})");
                    }
                }
            }
        }
        updated
    }

    fn resize(&mut self, width: u16, height: u16) {
        self.set_size(
            &Rect {
                x: 0,
                y: 0,
                width,
                height,
            },
            1,
        );
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.get_browser()
            .style(Style::default().fg(Color::LightCyan))
            .render(area, buf);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
