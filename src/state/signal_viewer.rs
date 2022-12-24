use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};
use tui_layout::component::ComponentWidget;

use vcd_parser::parser::VcdVariable;
use vcd_parser::waveform::bitvector::BitVectorRadix;

use crate::state::netlist_viewer::NetlistViewerRequest;
use crate::widgets::browser::*;

#[derive(Clone)]
pub enum SignalNode {
    Spacer,
    Group(String),
    VectorSignal(Vec<String>, VcdVariable, BitVectorRadix, Option<usize>),
}

impl std::fmt::Display for SignalNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spacer => write!(f, ""),
            Self::Group(name) => write!(f, "{}", name),
            Self::VectorSignal(_, variable, _, index) => {
                if let Some(index) = index {
                    write!(f, "{} [{}]", variable, index)
                } else {
                    write!(f, "{}", variable)
                }
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

fn create_variable_node(
    path: Vec<String>,
    variable: VcdVariable,
    radix: BitVectorRadix,
) -> BrowserNode<SignalNode> {
    BrowserNode::from(
        Some(SignalNode::VectorSignal(
            path.clone(),
            variable.clone(),
            radix,
            None,
        )),
        if variable.get_bit_width() > 1 {
            (0..variable.get_bit_width())
                .into_iter()
                .map(|i| {
                    SignalNode::VectorSignal(path.clone(), variable.clone(), radix.clone(), Some(i))
                })
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

pub struct SignalViewerEntry {
    pub(crate) idcode: usize,
    pub(crate) index: Option<usize>,
    pub(crate) radix: BitVectorRadix,
    pub(crate) is_selected: bool,
}

pub struct SignalViewerRequest(pub(crate) Vec<Option<SignalViewerEntry>>);

pub struct SignalViewerState {
    browser: BrowserState,
    node: BrowserNode<SignalNode>,
    requests: Vec<SignalViewerRequest>,
}

impl SignalViewerState {
    pub fn new() -> Self {
        Self {
            browser: BrowserState::new(true, true, false),
            node: BrowserNode::from_expanded(None, true, Vec::new()),
            requests: Vec::new(),
        }
    }

    fn browser_request_append(
        &mut self,
        path: Vec<String>,
        variable: VcdVariable,
        radix: BitVectorRadix,
    ) {
        self.node
            .get_children_mut()
            .push(create_variable_node(path.clone(), variable, radix));
    }

    fn browser_request_insert(
        &mut self,
        _path: Vec<String>,
        _variable: VcdVariable,
        _insert_path: BrowserNodePath,
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

    pub fn browser_request(&mut self, requests: Vec<NetlistViewerRequest>) {
        let mut insert_path = self.browser.get_primary_selected_path(&self.node);
        for request in requests {
            match request {
                NetlistViewerRequest::Append(path, variable) => {
                    self.browser_request_append(path, variable, BitVectorRadix::Hexadecimal);
                }
                NetlistViewerRequest::Insert(path, variable) => {
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
        self.push_request();
    }

    pub fn set_size(&mut self, size: &Rect, border_width: u16) {
        // Handle extra room above/below hierarchy in browser
        let margin = border_width as isize * 2;
        self.browser
            .set_height((size.height as isize - margin).max(0));
        self.browser.scroll_relative(&self.node, 0);
        self.push_request();
    }

    pub fn handle_key_press(&mut self, event: KeyEvent) {
        let shift = event.modifiers.contains(KeyModifiers::SHIFT);
        match event.code {
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
            _ => {}
        }
        self.push_request();
    }

    pub fn handle_mouse_click(&mut self, _: u16, y: u16) {
        if self.browser.select_absolute(&self.node, y as isize, true) {
            self.modify(ListAction::Expand);
        }
        self.push_request();
    }

    pub fn handle_mouse_scroll(&mut self, scroll_up: bool) {
        self.browser
            .select_relative(&self.node, if scroll_up { -5 } else { 5 }, true);
        self.push_request();
    }

    pub fn get_browser<'a>(&'a self) -> Browser<'a, SignalNode> {
        Browser::new(&self.browser, &self.node)
    }

    pub fn get_browser_state(&self) -> &BrowserState {
        &self.browser
    }

    pub fn get_node(&self) -> &BrowserNode<SignalNode> {
        &self.node
    }

    fn modify(&mut self, action: ListAction) {
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
                let path = self.browser.get_primary_selected_path(&self.node);
                if let Some(node) = self.node.get_node_mut(&path) {
                    node.set_expanded(!node.is_expanded());
                }
            }
        }
        self.push_request();
    }

    pub fn push_request(&mut self) {
        let mut request = Vec::new();
        for path in self.browser.get_visible_paths(&self.node) {
            let is_selected = self.browser.get_primary_selected_path(&self.node) == path;
            let Some(node) = self.node.get_node(&path) else {
                request.push(None);
                continue;
            };
            request.push(match node.get_entry().as_ref().unwrap() {
                SignalNode::VectorSignal(_, vcd_variable, radix, index) => {
                    Some(SignalViewerEntry {
                        idcode: vcd_variable.get_idcode(),
                        index: *index,
                        radix: radix.clone(),
                        is_selected,
                    })
                }
                _ => None,
            });
        }
        self.requests.push(SignalViewerRequest(request));
    }

    pub fn get_requests(&mut self) -> Vec<SignalViewerRequest> {
        let mut requests = Vec::new();
        std::mem::swap(&mut requests, &mut self.requests);
        requests
    }
}

impl ComponentWidget for SignalViewerState {
    fn handle_mouse(&mut self, x: u16, y: u16, kind: MouseEventKind) {
        match kind {
            MouseEventKind::Down(MouseButton::Left) => self.handle_mouse_click(x, y),
            MouseEventKind::ScrollDown => self.handle_mouse_scroll(false),
            MouseEventKind::ScrollUp => self.handle_mouse_scroll(true),
            _ => {}
        }
    }

    fn handle_key(&mut self, e: KeyEvent) {
        self.handle_key_press(e);
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
}
