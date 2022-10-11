mod signal;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use tui::{layout::Rect, style::Style, text::Text};

use vcd_parser::parser::*;
use vcd_parser::waveform::Waveform;

use crate::state::hierarchy_browser::HierarchyBrowserRequest;
use crate::widgets::browser::*;
use crate::widgets::timescale::*;

#[derive(Clone)]
pub enum SignalNode {
    Spacer,
    Group(String),
    VectorSignal(Vec<String>, VcdVariable),
    VectorSignalComponent(Vec<String>, VcdVariable, usize),
}

impl std::fmt::Display for SignalNode {
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

impl Default for SignalNode {
    fn default() -> Self {
        Self::Spacer
    }
}

// impl SignalNode {
//     fn print_path(&self) -> String {
//         let mut s = String::new();
//         match self {
//             Self::Spacer | Self::Group(_) => {}
//             Self::VectorSignal(paths, _) | Self::VectorSignalComponent(paths, _, _) => {
//                 for path in paths {
//                     s.push_str(&path);
//                     s.push('.');
//                 }
//             }
//         }
//         s
//     }
// }

fn create_variable_node(path: Vec<String>, variable: VcdVariable) -> BrowserNode<SignalNode> {
    BrowserNode::from(
        Some(SignalNode::VectorSignal(path.clone(), variable.clone())),
        if variable.get_bit_width() > 1 {
            (0..variable.get_bit_width())
                .into_iter()
                .map(|i| SignalNode::VectorSignalComponent(path.clone(), variable.clone(), i))
                .map(|n| BrowserNode::new(Some(n)))
                .collect()
        } else {
            Vec::new()
        },
    )
}

#[derive(Clone)]
enum ListAction {
    Group,
    Delete,
    Expand,
}

pub struct WaveformState {
    viewer_width: usize,
    list_state: BrowserState,
    node: BrowserNode<SignalNode>,
    waveform: Waveform,
    timescale_state: TimescaleState,
}

impl WaveformState {
    pub fn new() -> Self {
        Self {
            viewer_width: 0,
            list_state: BrowserState::new(true, true, false),
            node: BrowserNode::from_expanded(None, true, Vec::new()),
            waveform: Waveform::new(),
            timescale_state: TimescaleState::new(),
        }
    }

    pub fn load_waveform(&mut self, waveform: Waveform, timescale: i32) {
        self.waveform = waveform;
        let (min, max) = match (
            self.waveform.get_timestamps().first(),
            self.waveform.get_timestamps().last(),
        ) {
            (Some(start), Some(end)) => (*start, *end),
            _ => (0, 0),
        };
        self.timescale_state.load_waveform(min..max, max, timescale);
    }

    fn browser_request_append(&mut self, path: Vec<String>, variable: VcdVariable) {
        self.node
            .get_children_mut()
            .push(create_variable_node(path.clone(), variable));
    }

    fn browser_request_insert(
        &mut self,
        path: Vec<String>,
        variable: VcdVariable,
        insert_path: BrowserNodePath,
    ) -> BrowserNodePath {
        // self.node
        // .get_children_mut()
        // .push(create_variable_node(path.clone(), variable));

        // if let Some((index, primary_parent)) = insert_path.clone().to_vec().split_last() {
        // } else {
        //     self.browser_request_append(path, variable);
        // }

        // let insert_node = self.node.get_node_mut(&primary_path);

        BrowserNodePath::new(Vec::new())
    }

    pub fn browser_request(&mut self, requests: Vec<HierarchyBrowserRequest>) {
        let mut insert_path = self.list_state.get_primary_selected_path(&self.node);
        for request in requests {
            match request {
                HierarchyBrowserRequest::Append(path, variable) => {
                    self.browser_request_append(path, variable);
                }
                HierarchyBrowserRequest::Insert(path, variable) => {
                    insert_path = self.browser_request_insert(path, variable, insert_path);

                    // let mut select_offset = self.tree_select.get_primary_selected();
                    // let selected = self.tree.get_selected_mut(&mut select_offset);
                    // let selected = match selected {
                    //     Some(selected) => selected,
                    //     None => return,
                    // };
                }
            }
        }
    }

    pub fn set_list_size(&mut self, size: &Rect, border_width: u16) {
        // Handle extra room above/below hierarchy in browser
        let margin = border_width as isize * 2;
        self.list_state
            .set_height((size.height as isize - margin).max(0));
        self.list_state.scroll_relative(&self.node, 0);
    }

    pub fn set_waveform_size(&mut self, size: &Rect, border_width: u16) {
        self.viewer_width = if size.width > (border_width * 2) {
            (size.width - (border_width * 2)) as usize
        } else {
            0
        };
    }

    pub fn handle_key_list(&mut self, event: KeyEvent) {
        let shift = event.modifiers.contains(KeyModifiers::SHIFT);
        match event.code {
            KeyCode::Up => self.list_state.select_relative(&self.node, -1, !shift),
            KeyCode::Down => self.list_state.select_relative(&self.node, 1, !shift),
            KeyCode::PageDown => self.list_state.select_relative(&self.node, 20, !shift),
            KeyCode::PageUp => self.list_state.select_relative(&self.node, -20, !shift),
            KeyCode::Enter => self.modify_list(ListAction::Expand),
            KeyCode::Char('g') => self.modify_list(ListAction::Group),
            KeyCode::Char('f') => {
                self.list_state
                    .set_indent_enabled(!self.list_state.is_full_name_enabled());
                self.list_state
                    .set_full_name_enabled(!self.list_state.is_full_name_enabled());
            }
            KeyCode::Delete => self.modify_list(ListAction::Delete),
            _ => {}
        }
    }

    pub fn handle_key_viewer(&mut self, event: KeyEvent) {
        // let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        // let shift = event.modifiers.contains(KeyModifiers::SHIFT);
        match event.code {
            KeyCode::Char('-') => self.timescale_state.zoom_out(false),
            KeyCode::Char('=') => self.timescale_state.zoom_in(false),
            KeyCode::Char('[') => self.timescale_state.zoom_left(false),
            KeyCode::Char(']') => self.timescale_state.zoom_right(false),
            KeyCode::Char('_') => self.timescale_state.zoom_out(true),
            KeyCode::Char('+') => self.timescale_state.zoom_in(true),
            KeyCode::Char('{') => self.timescale_state.zoom_left(true),
            KeyCode::Char('}') => self.timescale_state.zoom_right(true),
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageDown
            | KeyCode::PageUp
            | KeyCode::Enter
            | KeyCode::Char('g')
            | KeyCode::Char('f')
            | KeyCode::Delete => self.handle_key_list(event),
            _ => {}
        }
    }

    pub fn handle_mouse_click_list(&mut self, _: u16, y: u16) {
        if self
            .list_state
            .select_absolute(&self.node, y as isize, true)
        {
            self.modify_list(ListAction::Expand);
        }
    }

    pub fn handle_mouse_scroll_list(&mut self, scroll_up: bool) {
        self.list_state
            .select_relative(&self.node, if scroll_up { -5 } else { 5 }, true);
    }

    pub fn get_list_browser<'a>(&'a self) -> Browser<'a, SignalNode> {
        Browser::new(&self.list_state, &self.node)
    }

    pub fn get_timescale<'a>(&'a self) -> Timescale<'a> {
        Timescale::new(&self.timescale_state)
    }

    pub fn render_waveform(&self) -> Text<'static> {
        let mut text = Text::styled("", Style::default());
        text.extend(self.timescale_state.render(self.viewer_width));
        text
    }

    fn modify_list(&mut self, action: ListAction) {
        // let mut select_offset = self.tree_select.get_primary_selected();

        // let selected = match self.tree.get_selected_mut(&mut select_offset) {
        //     Some((_, selected)) => selected,
        //     None => return,
        // };

        // let variables = match selected.get_value() {
        //     NodeValue::Variable(variable) => vec![variable.clone()],
        //     NodeValue::Scope(_) => get_scope_variables(selected.get_nodes()),
        // };

        match action {
            ListAction::Group => {}
            ListAction::Delete => {}
            ListAction::Expand => {
                let path = self.list_state.get_primary_selected_path(&self.node);
                if let Some(node) = self.node.get_node_mut(&path) {
                    node.set_expanded(!node.is_expanded());
                }
            }
        }
    }
}
