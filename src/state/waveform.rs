use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use tui::{layout::Rect, style::Style, text::Text};

use vcd_parser::parser::*;

use crate::state::browser::BrowserRequest;
use crate::state::tree::*;
use crate::state::utils::*;

#[derive(Clone)]
enum WaveformNode {
    Spacer,
    Group(String),
    VectorSignal(Vec<String>, VcdVariable),
    VectorSignalComponent(Vec<String>, VcdVariable, usize),
}

impl std::fmt::Display for WaveformNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spacer => write!(f, ""),
            Self::Group(name) => write!(f, "{}", name),
            Self::VectorSignal(_, variable) => write!(f, "{}", variable),
            Self::VectorSignalComponent(_, variable, index) => {
                write!(f, "{} [{}]", variable, index)
            }
        }
    }
}

impl Default for WaveformNode {
    fn default() -> Self {
        Self::Spacer
    }
}

impl WaveformNode {
    fn print_path(&self) -> String {
        let mut s = String::new();
        match self {
            Self::Spacer | Self::Group(_) => {}
            Self::VectorSignal(paths, _) | Self::VectorSignalComponent(paths, _, _) => {
                for path in paths {
                    s.push_str(&path);
                    s.push('.');
                }
            }
        }
        s
    }
}

fn create_variable_node(path: Vec<String>, variable: VcdVariable) -> TreeNode<WaveformNode> {
    let mut variable_node =
        TreeNode::new(WaveformNode::VectorSignal(path.clone(), variable.clone()));
    for i in 0..variable.get_bit_width() {
        variable_node
            .get_nodes_mut()
            .push(TreeNode::new(WaveformNode::VectorSignalComponent(
                path.clone(),
                variable.clone(),
                i,
            )));
    }
    variable_node
}

#[derive(Clone)]
enum ListAction {
    Group,
    Delete,
    Expand,
}

pub struct WaveformState {
    list_width: usize,
    viewer_width: usize,
    tree_select: TreeDisplay,
    tree: TreeNodes<WaveformNode>,
    full_path: bool,
}

impl WaveformState {
    pub fn new() -> Self {
        Self {
            list_width: 0,
            viewer_width: 0,
            tree_select: TreeDisplay::new(),
            tree: TreeNodes::new(),
            full_path: false,
        }
    }

    pub fn browser_request(&mut self, requests: Vec<BrowserRequest>) {
        for request in requests {
            match request {
                BrowserRequest::Append(path, variables) => {
                    for variable in variables {
                        self.tree.push(create_variable_node(path.clone(), variable));
                    }
                }
                BrowserRequest::Insert(_, _) => {
                    let mut select_offset = self.tree_select.get_primary_selected();
                    let selected = self.tree.get_selected_mut(&mut select_offset);
                    let selected = match selected {
                        Some(selected) => selected,
                        None => return,
                    };
                }
            }
        }
    }

    pub fn set_size(&mut self, list_size: &Rect, viewer_size: &Rect, border_width: u16) {
        self.list_width = if list_size.width > (border_width * 2) {
            (list_size.width - (border_width * 2)) as usize
        } else {
            0
        };
        self.viewer_width = if viewer_size.width > (border_width * 2) {
            (viewer_size.width - (border_width * 2)) as usize
        } else {
            0
        };
        // Handle extra room for timescale
        let margin = border_width as isize * 2 + 1;
        self.tree_select
            .set_height(list_size.height as isize - margin);
    }

    pub fn handle_key_list(&mut self, event: KeyEvent) {
        let shift = event.modifiers.contains(KeyModifiers::SHIFT);
        match event.code {
            KeyCode::Up => self.tree_select.select_relative(&self.tree, -1, !shift),
            KeyCode::Down => self.tree_select.select_relative(&self.tree, 1, !shift),
            KeyCode::PageDown => self.tree_select.select_relative(&self.tree, 20, !shift),
            KeyCode::PageUp => self.tree_select.select_relative(&self.tree, -20, !shift),
            KeyCode::Enter => self.modify_list(ListAction::Expand),
            KeyCode::Char('g') => self.modify_list(ListAction::Group),
            KeyCode::Char('f') => self.full_path = !self.full_path,
            KeyCode::Delete => self.modify_list(ListAction::Delete),
            _ => {}
        }
    }

    pub fn handle_mouse_click_list(&mut self, _: u16, y: u16) {
        if self
            .tree_select
            .select_absolute(&self.tree, y as isize - 1, true)
        {
            self.modify_list(ListAction::Expand);
        }
    }

    pub fn handle_mouse_scroll_list(&mut self, scroll_up: bool) {
        self.tree_select
            .select_relative(&self.tree, if scroll_up { -5 } else { 5 }, true);
    }

    pub fn render_list(&self) -> Text<'static> {
        let mut text = Text::styled(" ", Style::default());
        let mut offsets = self.tree_select.make_render_offsets();
        self.tree
            .render(&mut text, &mut offsets, self.list_width, &|n| {
                if self.full_path {
                    format!("{}{}", n.print_path(), n)
                } else {
                    format!("{}", n)
                }
            });
        text
    }

    pub fn render_waveform(&self) -> Text<'static> {
        Text::styled(" ", Style::default())
    }

    fn modify_list(&mut self, action: ListAction) {
        let mut select_offset = self.tree_select.get_primary_selected();

        let selected = match self.tree.get_selected_mut(&mut select_offset) {
            Some((_, selected)) => selected,
            None => return,
        };

        // let variables = match selected.get_value() {
        //     NodeValue::Variable(variable) => vec![variable.clone()],
        //     NodeValue::Scope(_) => get_scope_variables(selected.get_nodes()),
        // };

        match action {
            ListAction::Group => {}
            ListAction::Delete => {}
            ListAction::Expand => {
                selected.set_expanded(!selected.is_expanded());
                self.tree_select.scroll_relative(0, select_offset);
            }
        }
    }
}
