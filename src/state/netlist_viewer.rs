use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use lazy_static::*;
use makai::utils::messages::Messages;
use makai_vcd_reader::parser::{VcdHeader, VcdScope, VcdVariable};
use tui::widgets::Widget;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};
use tui_tiling::component::ComponentWidget;

use crate::widgets::browser::Visibility;
use crate::{
    state::filter::{construct_filter, BrowserFilterSection},
    state::signal_viewer::SignalViewerMessage,
    widgets::browser::{Browser, BrowserNode, BrowserState},
};

#[derive(Clone)]
pub enum NetlistNode {
    Scope(String),
    Variable(VcdVariable),
}

impl std::fmt::Display for NetlistNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scope(name) => write!(f, "{}", name),
            Self::Variable(variable) => write!(f, "{}", variable),
        }
    }
}

impl Default for NetlistNode {
    fn default() -> Self {
        Self::Scope(String::new())
    }
}

lazy_static! {
    static ref DEFAULT_NODE: BrowserNode<NetlistNode> = BrowserNode::default();
}

// Looks for a node with the same name, first in the expected place, then
// through all nodes, and if that fails returns a default node
fn search_nodes<'a>(
    nodes: &'a [BrowserNode<NetlistNode>],
    name: &String,
    expected_index: usize,
) -> &'a BrowserNode<NetlistNode> {
    for node in nodes.get(expected_index).into_iter().chain(nodes.iter()) {
        if &node.to_string() == name {
            return node;
        }
    }
    &DEFAULT_NODE
}

fn generate_new_node(
    old_node: &BrowserNode<NetlistNode>,
    new_scope: &VcdScope,
) -> BrowserNode<NetlistNode> {
    // Search through the old node's children for matches to the new scope children
    let mut new_scopes = new_scope
        .get_scopes()
        .iter()
        .enumerate()
        .map(|(i, s)| generate_new_node(search_nodes(old_node.get_children(), s.get_name(), i), s))
        .collect::<Vec<BrowserNode<NetlistNode>>>();

    // Sort the new child scope nodes
    new_scopes.sort_by(|a, b| alphanumeric_sort::compare_str(&a.to_string(), &b.to_string()));
    // Create a copy of the variables and sort them separately
    let mut new_variables = new_scope
        .get_variables()
        .iter()
        .map(|v| BrowserNode::new(NetlistNode::Variable(v.clone())))
        .collect::<Vec<BrowserNode<NetlistNode>>>();
    new_variables.sort_by(|a, b| alphanumeric_sort::compare_str(&a.to_string(), &b.to_string()));
    // Create new node with proper expansion and the new scopes followed by new variables
    let entry = NetlistNode::Scope(new_scope.get_name().clone());
    new_scopes.append(&mut new_variables);
    BrowserNode::from(Some(entry), old_node.get_visibility(), new_scopes)
}

fn generate_new_nodes(
    old_nodes: &BrowserNode<NetlistNode>,
    new_scopes: &[VcdScope],
) -> BrowserNode<NetlistNode> {
    // Search through the old node's children for matches to the new scope children
    let mut new_scopes = new_scopes
        .iter()
        .enumerate()
        .map(|(i, s)| generate_new_node(search_nodes(old_nodes.get_children(), s.get_name(), i), s))
        .collect::<Vec<BrowserNode<NetlistNode>>>();
    // Sort the new child scope nodes
    new_scopes.sort_by(|a, b| alphanumeric_sort::compare_str(&a.to_string(), &b.to_string()));
    BrowserNode::from(None, Visibility::Expanded, new_scopes)
}

#[derive(Clone)]
enum NetlistViewerAction {
    Append,
    Insert,
    Expand,
}

pub(crate) enum NetlistViewerMessage {
    WaveformUpdate { vcd_header: Arc<VcdHeader> },
}

pub struct NetlistViewerState {
    state: BrowserState,
    node: BrowserNode<NetlistNode>,
    filters: Vec<BrowserFilterSection>,
    border_width: u16,
    messages: Messages,
}

impl NetlistViewerState {
    pub fn new(messages: Messages) -> Self {
        Self {
            state: BrowserState::new(true, true, false),
            node: BrowserNode::new_container(),
            filters: Vec::new(),
            border_width: 1,
            messages,
        }
    }

    pub fn update_filter(&mut self, filter: String) {
        self.filters = construct_filter(filter);
    }

    fn update_scopes(&mut self, new_scopes: &[VcdScope]) {
        // Set new scopes and clear the selected item
        self.node = generate_new_nodes(&self.node, new_scopes);
        self.state.select_relative(&self.node, 0, true);
    }

    pub fn set_size(&mut self, size: &Rect) {
        // Handle extra room above/below hierarchy in browser
        let margin = self.border_width as isize * 2;
        self.state
            .set_height((size.height as isize - margin).max(0));
        self.state.scroll_relative(&self.node, 0);
    }

    pub fn get_browser(&self) -> Browser<'_, NetlistNode> {
        Browser::new(&self.state, &self.node)
    }

    fn get_selected_variables(&self) -> Vec<(Vec<String>, VcdVariable)> {
        self.state
            .get_selected_paths(&self.node, false) // Do not condense
            .iter()
            .map(|p| (p, self.node.get_node(p).unwrap())) // Produce paths
            .filter_map(|(path, node)| match node.get_entry() {
                // Ignore scopes
                Some(NetlistNode::Variable(variable)) => Some((path, variable)),
                _ => None,
            })
            // Convert path to full names
            .map(|(path, variable)| {
                // log::info!("Full name: {:?}", self.node.get_full_name(path));
                (self.node.get_full_name(path), variable.clone())
            })
            .collect()
    }

    fn modify(&mut self, action: NetlistViewerAction) {
        let requests = match action {
            NetlistViewerAction::Append => self
                .get_selected_variables()
                .iter()
                .map(|(full_name, variable)| {
                    SignalViewerMessage::NetlistAppend(full_name.clone(), variable.clone())
                })
                .collect(),
            NetlistViewerAction::Insert => self
                .get_selected_variables()
                .iter()
                .map(|(full_name, variable)| {
                    SignalViewerMessage::NetlistInsert(full_name.clone(), variable.clone())
                })
                .collect(),
            NetlistViewerAction::Expand => {
                let path = self.state.get_primary_selected_path(&self.node);
                if let Some(node) = self.node.get_node_mut(&path) {
                    match node.get_visibility() {
                        Visibility::Collapsed => node.set_visibility(Visibility::Expanded),
                        Visibility::Expanded => node.set_visibility(Visibility::Collapsed),
                    }
                }
                Vec::new()
            }
        };
        self.messages.append(requests);
    }
}

impl ComponentWidget for NetlistViewerState {
    fn handle_mouse(&mut self, _x: u16, y: u16, kind: MouseEventKind) -> bool {
        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if self.state.select_absolute(&self.node, y as isize, true) {
                    self.modify(NetlistViewerAction::Expand);
                }
            }
            MouseEventKind::ScrollDown => self.state.select_relative(&self.node, 5, true),
            MouseEventKind::ScrollUp => self.state.select_relative(&self.node, -5, true),
            _ => return false,
        }
        true
    }

    fn handle_key(&mut self, e: KeyEvent) -> bool {
        let shift = e.modifiers.contains(KeyModifiers::SHIFT);
        match e.code {
            KeyCode::Up => self.state.select_relative(&self.node, -1, !shift),
            KeyCode::Down => self.state.select_relative(&self.node, 1, !shift),
            KeyCode::PageDown => self.state.select_relative(&self.node, 20, !shift),
            KeyCode::PageUp => self.state.select_relative(&self.node, -20, !shift),
            KeyCode::Enter => self.modify(NetlistViewerAction::Expand),
            KeyCode::Char('a') => self.modify(NetlistViewerAction::Append),
            KeyCode::Char('i') => self.modify(NetlistViewerAction::Insert),
            KeyCode::Char('f') => {
                self.state
                    .set_indent_enabled(!self.state.is_full_name_enabled());
                self.state
                    .set_full_name_enabled(!self.state.is_full_name_enabled());
            }
            _ => return false,
        }
        true
    }

    fn handle_update(&mut self) -> bool {
        let mut updated = false;
        for message in self.messages.get::<NetlistViewerMessage>() {
            match message {
                NetlistViewerMessage::WaveformUpdate { vcd_header } => {
                    self.update_scopes(&vcd_header.get_scopes());
                    updated = true;
                }
            }
        }
        updated
    }

    fn resize(&mut self, width: u16, height: u16) {
        self.set_size(&Rect {
            x: 0,
            y: 0,
            width,
            height,
        });
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
