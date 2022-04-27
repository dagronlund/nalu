use vcd_parser::parser::*;

use crossterm::event::KeyCode;

use tui::{layout::Rect, style::Style, text::Text};

use crate::state::filter::*;
use crate::state::tree::*;
use crate::state::utils::*;

fn search_nodes<'a>(
    nodes: &'a Vec<TreeNode<VcdVariable, ()>>,
    name: &String,
) -> Option<&'a TreeNode<VcdVariable, ()>> {
    for node in nodes {
        if node.get_name() == name {
            return Some(node);
        }
    }
    None
}

fn generate_new_tree(
    old_tree: &Vec<TreeNode<VcdVariable, ()>>,
    new_scopes: &Vec<VcdScope>,
) -> TreeNodes<VcdVariable, ()> {
    let mut generated_nodes = TreeNodes::new();

    for (i, new_scope) in new_scopes.into_iter().enumerate() {
        let empty_node = TreeNode::default();
        let old_scope = if old_tree.len() > i && new_scope.get_name() == old_tree[i].get_name() {
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

        generated_nodes.get_nodes_mut().push(TreeNode::new_from(
            new_scope.get_name().clone(),
            (),
            old_scope.is_expanded(),
            generate_new_tree(old_scope.get_nodes(), &new_scopes[i].get_scopes()).into_nodes(),
            variables,
        ));
    }

    generated_nodes
        .get_nodes_mut()
        .sort_by(|a, b| alphanumeric_sort::compare_str(&a.get_name(), &b.get_name()));
    generated_nodes
}

#[derive(Clone)]
enum BrowserAction {
    Append,
    Insert,
    Expand,
}

pub enum BrowserRequest {
    Append(Vec<VcdVariable>),
    Insert(Vec<VcdVariable>),
    None,
}

pub struct BrowserState {
    width: usize,
    context: SelectContext,
    tree: TreeNodes<VcdVariable, ()>,
    filters: Vec<BrowserFilterSection>,
}

impl BrowserState {
    pub fn new() -> Self {
        Self {
            width: 0,
            context: SelectContext::new(),
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
        let line_count = self.rendered_len();
        self.context.select_relative(0, line_count);
    }

    pub fn set_size(&mut self, size: &Rect, border_width: u16) {
        self.width = if size.width > (border_width * 2) {
            (size.width - (border_width * 2)) as usize
        } else {
            0
        };
        // Handle extra room above/below hierarchy in browser
        let margin = border_width as isize * 2 + 2;
        self.context.set_height(size.height as isize - margin);
    }

    pub fn handle_key(&mut self, key: KeyCode) -> BrowserRequest {
        let line_count = self.rendered_len();
        match key {
            KeyCode::Up => self.context.select_relative(-1, line_count),
            KeyCode::Down => self.context.select_relative(1, line_count),
            KeyCode::PageDown => self.context.select_relative(20, line_count),
            KeyCode::PageUp => self.context.select_relative(-20, line_count),
            KeyCode::Enter => return self.modify(BrowserAction::Expand),
            KeyCode::Char('a') => return self.modify(BrowserAction::Append),
            KeyCode::Char('i') => return self.modify(BrowserAction::Insert),
            _ => {}
        }
        BrowserRequest::None
    }

    pub fn handle_mouse_click(&mut self, _: u16, y: u16) -> BrowserRequest {
        let line_count = self.rendered_len();
        if self.context.select_absolute(y as isize - 1, line_count) {
            return self.modify(BrowserAction::Expand);
        }
        BrowserRequest::None
    }

    pub fn handle_mouse_scroll(&mut self, scroll_up: bool) {
        let line_count = self.rendered_len();
        self.context
            .select_relative(if scroll_up { -5 } else { 5 }, line_count);
    }

    pub fn render(&self) -> Text<'static> {
        let mut text = Text::styled(" ", Style::default());
        let mut offsets = self.context.make_render_offsets();
        self.tree.render(&mut text, &mut offsets, self.width);
        text
    }

    fn rendered_len(&self) -> usize {
        self.tree.rendered_len()
    }

    fn modify(&mut self, action: BrowserAction) -> BrowserRequest {
        let mut select_offset = self.context.get_select_offset();
        let selected = self.tree.get_selected_mut(&mut select_offset);

        match selected {
            TreeNodeSelected::Value(value) => match action {
                BrowserAction::Append => BrowserRequest::Append(vec![value.clone()]),
                BrowserAction::Insert => BrowserRequest::Insert(vec![value.clone()]),
                BrowserAction::Expand => BrowserRequest::None,
            },
            TreeNodeSelected::Node(node) => match action {
                BrowserAction::Append => BrowserRequest::Append(node.get_values().clone()),
                BrowserAction::Insert => BrowserRequest::Insert(node.get_values().clone()),
                BrowserAction::Expand => {
                    node.set_expanded(!node.is_expanded());
                    self.context.scroll_relative(0);
                    BrowserRequest::None
                }
            },
            TreeNodeSelected::None => BrowserRequest::None,
        }
    }
}
