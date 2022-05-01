use vcd_parser::parser::*;

use crossterm::event::KeyCode;

use tui::{layout::Rect, style::Style, text::Text};

use crate::state::filter::*;
use crate::state::tree::*;
use crate::state::utils::*;

#[derive(Clone)]
enum NodeValue {
    Scope(String),
    Variable(VcdVariable),
}

impl std::fmt::Display for NodeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scope(name) => write!(f, "{}", name),
            Self::Variable(variable) => write!(f, "{}", variable),
        }
    }
}

impl Default for NodeValue {
    fn default() -> Self {
        Self::Scope(String::new())
    }
}

fn search_nodes<'a>(
    nodes: &'a Vec<TreeNode<NodeValue>>,
    name: &String,
) -> Option<&'a TreeNode<NodeValue>> {
    for node in nodes {
        if &node.get_value().to_string() == name {
            return Some(node);
        }
    }
    None
}

fn get_scope_variables(node: &TreeNodes<NodeValue>) -> Vec<VcdVariable> {
    let mut variables = Vec::new();
    for sub_node in node.get_nodes() {
        match sub_node.get_value() {
            NodeValue::Scope(_) => {}
            NodeValue::Variable(variable) => variables.push(variable.clone()),
        }
    }
    variables
}

fn generate_new_tree(
    old_tree: &Vec<TreeNode<NodeValue>>,
    new_scopes: &Vec<VcdScope>,
) -> TreeNodes<NodeValue> {
    let mut generated_nodes = TreeNodes::new();

    for (i, new_scope) in new_scopes.into_iter().enumerate() {
        let empty_node = TreeNode::default();
        let old_scope =
            if old_tree.len() > i && new_scope.get_name() == &old_tree[i].get_value().to_string() {
                // The scopes indices lined up from the new to the old
                &old_tree[i]
            } else if let Some(old_scope) = search_nodes(&old_tree, new_scope.get_name()) {
                // The scope existed in the old scopes, different position
                old_scope
            } else {
                // The scope did not exist in the old scopes
                &empty_node
            };

        let mut variables = new_scope.get_variables().clone();
        variables.sort_by(|a, b| alphanumeric_sort::compare_str(a.get_name(), b.get_name()));
        let mut scope_nodes =
            generate_new_tree(old_scope.get_nodes(), &new_scopes[i].get_scopes()).into_nodes();

        let mut nodes = Vec::new();
        nodes.append(&mut scope_nodes);
        for variable in variables {
            nodes.push(TreeNode::new(NodeValue::Variable(variable)));
        }

        generated_nodes
            .get_nodes_mut()
            .push(TreeNode::from_existing(
                NodeValue::Scope(new_scope.get_name().clone()),
                old_scope.is_expanded(),
                TreeNodes::from(nodes),
            ));
    }

    generated_nodes.get_nodes_mut().sort_by(|a, b| {
        alphanumeric_sort::compare_str(&a.get_value().to_string(), &b.get_value().to_string())
    });
    generated_nodes
}

#[derive(Clone)]
enum BrowserAction {
    Append,
    Insert,
    Expand,
}

pub enum BrowserRequest {
    Append(Vec<String>, Vec<VcdVariable>),
    Insert(Vec<String>, Vec<VcdVariable>),
    None,
}

pub struct BrowserState {
    width: usize,
    select: TreeSelect,
    tree: TreeNodes<NodeValue>,
    filters: Vec<BrowserFilterSection>,
}

impl BrowserState {
    pub fn new() -> Self {
        Self {
            width: 0,
            select: TreeSelect::new(),
            tree: TreeNodes::new(),
            filters: Vec::new(),
        }
    }

    pub fn update_filter(&mut self, filter: String) {
        self.filters = construct_filter(filter);
    }

    pub fn update_scopes(&mut self, new_scopes: &Vec<VcdScope>) {
        // Set new scopes and clear the selected item
        self.tree = generate_new_tree(&self.tree.get_nodes(), &new_scopes);
        // let line_count = self.rendered_len();
        self.select.select_relative(&self.tree, 0);
    }

    pub fn set_size(&mut self, size: &Rect, border_width: u16) {
        self.width = if size.width > (border_width * 2) {
            (size.width - (border_width * 2)) as usize
        } else {
            0
        };
        // Handle extra room above/below hierarchy in browser
        let margin = border_width as isize * 2 + 2;
        self.select.set_height(size.height as isize - margin);
    }

    pub fn handle_key(&mut self, key: KeyCode) -> BrowserRequest {
        match key {
            KeyCode::Up => self.select.select_relative(&self.tree, -1),
            KeyCode::Down => self.select.select_relative(&self.tree, 1),
            KeyCode::PageDown => self.select.select_relative(&self.tree, 20),
            KeyCode::PageUp => self.select.select_relative(&self.tree, -20),
            KeyCode::Enter => return self.modify(BrowserAction::Expand),
            KeyCode::Char('a') => return self.modify(BrowserAction::Append),
            KeyCode::Char('i') => return self.modify(BrowserAction::Insert),
            _ => {}
        }
        BrowserRequest::None
    }

    pub fn handle_mouse_click(&mut self, _: u16, y: u16) -> BrowserRequest {
        if self.select.select_absolute(&self.tree, y as isize - 1) {
            return self.modify(BrowserAction::Expand);
        }
        BrowserRequest::None
    }

    pub fn handle_mouse_scroll(&mut self, scroll_up: bool) {
        self.select
            .select_relative(&self.tree, if scroll_up { -5 } else { 5 });
    }

    pub fn render(&self) -> Text<'static> {
        let mut text = Text::styled(" ", Style::default());
        let mut offsets = self.select.make_render_offsets();
        self.tree
            .render(&mut text, &mut offsets, self.width, &|n| format!("{}", n));
        text
    }

    fn modify(&mut self, action: BrowserAction) -> BrowserRequest {
        let mut select_offset = self.select.get_select_offset();

        let (path, selected) = match self.tree.get_selected_mut(&mut select_offset) {
            Some((path, selected)) => (path, selected),
            None => return BrowserRequest::None,
        };

        let mut path: Vec<String> = path.into_iter().map(|x| x.to_string()).collect();

        let variables = match selected.get_value() {
            NodeValue::Variable(variable) => vec![variable.clone()],
            NodeValue::Scope(name) => {
                path.push(name.clone());
                get_scope_variables(selected.get_nodes())
            }
        };

        match action {
            BrowserAction::Append => BrowserRequest::Append(path, variables),
            BrowserAction::Insert => BrowserRequest::Insert(path, variables),
            BrowserAction::Expand => {
                selected.set_expanded(!selected.is_expanded());
                self.select.scroll_relative(0);
                BrowserRequest::None
            }
        }
    }
}
