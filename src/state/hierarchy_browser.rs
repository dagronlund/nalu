use lazy_static::*;

use vcd_parser::parser::*;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use tui::layout::Rect;

use crate::state::filter::*;
use crate::widgets::browser::*;

#[derive(Clone)]
pub enum HierarchyNode {
    Scope(String),
    Variable(VcdVariable),
}

impl std::fmt::Display for HierarchyNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scope(name) => write!(f, "{}", name),
            Self::Variable(variable) => write!(f, "{}", variable),
        }
    }
}

impl Default for HierarchyNode {
    fn default() -> Self {
        Self::Scope(String::new())
    }
}

lazy_static! {
    static ref DEFAULT_NODE: BrowserNode<HierarchyNode> = BrowserNode::default();
}

// Looks for a node with the same name, first in the expected place, then
// through all nodes, and if that fails returns a default node
fn search_nodes<'a>(
    nodes: &'a Vec<BrowserNode<HierarchyNode>>,
    name: &String,
    expected_index: usize,
) -> &'a BrowserNode<HierarchyNode> {
    for node in nodes.get(expected_index).into_iter().chain(nodes.iter()) {
        if &node.to_string() == name {
            return node;
        }
    }
    &DEFAULT_NODE
}

fn generate_new_node(
    old_node: &BrowserNode<HierarchyNode>,
    new_scope: &VcdScope,
) -> BrowserNode<HierarchyNode> {
    // Search through the old node's children for matches to the new scope children
    let mut new_scopes = new_scope
        .get_scopes()
        .into_iter()
        .enumerate()
        .map(|(i, s)| generate_new_node(search_nodes(&old_node.get_children(), s.get_name(), i), s))
        .collect::<Vec<BrowserNode<HierarchyNode>>>();

    // Sort the new child scope nodes
    new_scopes.sort_by(|a, b| alphanumeric_sort::compare_str(&a.to_string(), &b.to_string()));
    // Create a copy of the variables and sort them separately
    let mut new_variables = new_scope
        .get_variables()
        .iter()
        .map(|v| BrowserNode::new(Some(HierarchyNode::Variable(v.clone()))))
        .collect::<Vec<BrowserNode<HierarchyNode>>>();
    new_variables.sort_by(|a, b| alphanumeric_sort::compare_str(&a.to_string(), &b.to_string()));
    // Create new node with proper expansion and the new scopes followed by new variables
    let entry = HierarchyNode::Scope(new_scope.get_name().clone());
    new_scopes.append(&mut new_variables);
    BrowserNode::from_expanded(Some(entry), old_node.is_expanded(), new_scopes)
}

fn generate_new_nodes(
    old_nodes: &BrowserNode<HierarchyNode>,
    new_scopes: &Vec<VcdScope>,
) -> BrowserNode<HierarchyNode> {
    // Search through the old node's children for matches to the new scope children
    let mut new_scopes = new_scopes
        .into_iter()
        .enumerate()
        .map(|(i, s)| {
            generate_new_node(search_nodes(&old_nodes.get_children(), s.get_name(), i), s)
        })
        .collect::<Vec<BrowserNode<HierarchyNode>>>();
    // Sort the new child scope nodes
    new_scopes.sort_by(|a, b| alphanumeric_sort::compare_str(&a.to_string(), &b.to_string()));
    BrowserNode::from_expanded(None, true, new_scopes)
}

#[derive(Clone)]
enum HierarchyBrowserAction {
    Append,
    Insert,
    Expand,
}

pub enum HierarchyBrowserRequest {
    Append(Vec<String>, VcdVariable),
    Insert(Vec<String>, VcdVariable),
}

pub struct HierarchyBrowserState {
    state: BrowserState,
    node: BrowserNode<HierarchyNode>,
    filters: Vec<BrowserFilterSection>,
}

impl HierarchyBrowserState {
    pub fn new() -> Self {
        Self {
            state: BrowserState::new(true, true, false),
            node: BrowserNode::from_expanded(None, true, Vec::new()),
            filters: Vec::new(),
        }
    }

    pub fn update_filter(&mut self, filter: String) {
        self.filters = construct_filter(filter);
    }

    pub fn update_scopes(&mut self, new_scopes: &Vec<VcdScope>) {
        // Set new scopes and clear the selected item
        self.node = generate_new_nodes(&self.node, &new_scopes);
        self.state.select_relative(&self.node, 0, true);
    }

    pub fn set_size(&mut self, size: &Rect, border_width: u16) {
        // Handle extra room above/below hierarchy in browser
        let margin = border_width as isize * 2;
        self.state
            .set_height((size.height as isize - margin).max(0));
        self.state.scroll_relative(&self.node, 0);
    }

    pub fn handle_key(&mut self, event: KeyEvent) -> Vec<HierarchyBrowserRequest> {
        let shift = event.modifiers.contains(KeyModifiers::SHIFT);
        match event.code {
            KeyCode::Up => self.state.select_relative(&self.node, -1, !shift),
            KeyCode::Down => self.state.select_relative(&self.node, 1, !shift),
            KeyCode::PageDown => self.state.select_relative(&self.node, 20, !shift),
            KeyCode::PageUp => self.state.select_relative(&self.node, -20, !shift),
            KeyCode::Enter => return self.modify(HierarchyBrowserAction::Expand),
            KeyCode::Char('a') => return self.modify(HierarchyBrowserAction::Append),
            KeyCode::Char('i') => return self.modify(HierarchyBrowserAction::Insert),
            KeyCode::Char('f') => {
                self.state
                    .set_indent_enabled(!self.state.is_full_name_enabled());
                self.state
                    .set_full_name_enabled(!self.state.is_full_name_enabled());
            }
            _ => {}
        }
        Vec::new()
    }

    pub fn handle_mouse_click(&mut self, _: u16, y: u16) -> Vec<HierarchyBrowserRequest> {
        if self.state.select_absolute(&self.node, y as isize, true) {
            return self.modify(HierarchyBrowserAction::Expand);
        }
        Vec::new()
    }

    pub fn handle_mouse_scroll(&mut self, scroll_up: bool) {
        self.state
            .select_relative(&self.node, if scroll_up { -5 } else { 5 }, true);
    }

    pub fn get_browser<'a>(&'a self) -> Browser<'a, HierarchyNode> {
        Browser::new(&self.state, &self.node)
    }

    fn get_selected_variables(&self) -> Vec<(Vec<String>, VcdVariable)> {
        self.state
            .get_selected_paths(&self.node, false) // Do not condense
            .iter()
            .map(|p| (p, self.node.get_node(p).unwrap())) // Produce paths
            .filter_map(|(path, node)| match node.get_entry() {
                // Ignore scopes
                Some(HierarchyNode::Variable(variable)) => Some((path, variable)),
                _ => None,
            })
            // Convert path to full names
            .map(|(path, variable)| (self.node.get_full_name(&path), variable.clone()))
            .collect()
    }

    fn modify(&mut self, action: HierarchyBrowserAction) -> Vec<HierarchyBrowserRequest> {
        match action {
            HierarchyBrowserAction::Append => self
                .get_selected_variables()
                .iter()
                .map(|(full_name, variable)| {
                    HierarchyBrowserRequest::Append(full_name.clone(), variable.clone())
                })
                .collect(),
            HierarchyBrowserAction::Insert => self
                .get_selected_variables()
                .iter()
                .map(|(full_name, variable)| {
                    HierarchyBrowserRequest::Insert(full_name.clone(), variable.clone())
                })
                .collect(),
            HierarchyBrowserAction::Expand => {
                let path = self.state.get_primary_selected_path(&self.node);
                if let Some(node) = self.node.get_node_mut(&path) {
                    node.set_expanded(!node.is_expanded());
                }
                Vec::new()
            }
        }
    }
}
