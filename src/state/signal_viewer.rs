use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use makai::utils::messages::Messages;
use makai_vcd_reader::parser::VcdVariable;
use makai_waveform_db::bitvector::BitVectorRadix;
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};
use tui_tiling::component::ComponentWidget;

use crate::{state::waveform_viewer::WaveformViewerMessage, widgets::browser::*};

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
                .map(|i| SignalNode::VectorSignal(path.clone(), variable.clone(), radix, Some(i)))
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

#[derive(Debug, Clone)]
pub struct SignalViewerEntry {
    pub(crate) idcode: usize,
    pub(crate) index: Option<usize>,
    pub(crate) radix: BitVectorRadix,
    pub(crate) is_selected: bool,
}

pub(crate) enum SignalViewerMessage {
    NetlistAppend(Vec<String>, VcdVariable),
    NetlistInsert(Vec<String>, VcdVariable),
    WaveformKey(KeyEvent),
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
            node: BrowserNode::from_expanded(None, true, Vec::new()),
            messages,
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
            .push(create_variable_node(path, variable, radix));
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
                        radix: *radix,
                        is_selected,
                    })
                }
                _ => None,
            });
        }
        self.messages
            .push(WaveformViewerMessage::UpdateSignals(request.clone()));
    }
}

impl ComponentWidget for SignalViewerState {
    fn handle_mouse(&mut self, _x: u16, y: u16, kind: MouseEventKind) -> bool {
        match kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if self.browser.select_absolute(&self.node, y as isize, true) {
                    self.modify(ListAction::Expand);
                }
                self.push_request();
            }
            MouseEventKind::ScrollDown => {
                self.browser.select_relative(&self.node, 5, true);
                self.push_request();
            }
            MouseEventKind::ScrollUp => {
                self.browser.select_relative(&self.node, -5, true);
                self.push_request();
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
        self.push_request();
        true
    }

    fn handle_update(&mut self) -> bool {
        let mut updated = false;
        for message in self.messages.get::<SignalViewerMessage>() {
            match message {
                SignalViewerMessage::NetlistAppend(path, variable) => {
                    self.browser_request_append(path, variable, BitVectorRadix::Hexadecimal);
                    updated = true;
                }
                SignalViewerMessage::NetlistInsert(_, _) => {}
                SignalViewerMessage::WaveformKey(e) => updated |= self.handle_key(e),
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
