use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use tui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Widget},
};

use vcd_parser::waveform::bitvector::BitVector;
use vcd_parser::waveform::{bitvector::BitVectorRadix, Waveform};
use vcd_parser::{parser::*, waveform::WaveformSignalResult};

use crate::widgets::browser::*;
use crate::widgets::signal::*;
use crate::widgets::timescale::*;

use crate::state::hierarchy_browser::HierarchyBrowserRequest;

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
        let range = self.waveform.get_timestamp_range();
        self.timescale_state
            .load_waveform(range.clone(), range.end, timescale);
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

    pub fn browser_request(&mut self, requests: Vec<HierarchyBrowserRequest>) {
        let mut insert_path = self.list_state.get_primary_selected_path(&self.node);
        for request in requests {
            match request {
                HierarchyBrowserRequest::Append(path, variable) => {
                    self.browser_request_append(path, variable, BitVectorRadix::Hexadecimal);
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

    pub fn get_waveform_widget<'a>(&'a self) -> WaveformWidget<'a> {
        let mut signal_widgets = Vec::new();
        for path in self.list_state.get_visible_paths(&self.node) {
            let is_selected = self.list_state.get_primary_selected_path(&self.node) == path;
            if let Some(node) = self.node.get_node(&path) {
                match node.get_entry().as_ref().unwrap() {
                    SignalNode::VectorSignal(_, vcd_variable, radix, index) => {
                        let waveform_entry = WaveformEntry {
                            storage: &self.waveform,
                            idcode: vcd_variable.get_idcode(),
                            index: *index,
                        };
                        signal_widgets.push(Some(Signal::new(
                            &self.timescale_state,
                            waveform_entry,
                            radix.clone(),
                            is_selected,
                        )));
                    }
                    _ => signal_widgets.push(None),
                }
            } else {
                signal_widgets.push(None);
            }
        }
        WaveformWidget {
            timescale_widget: Timescale::new(&self.timescale_state),
            signal_widgets,
            block: None,
            style: Default::default(),
        }
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

pub struct WaveformEntry<'a> {
    storage: &'a Waveform,
    idcode: usize,
    index: Option<usize>,
}

impl<'a> WaveformEntry<'a> {
    pub fn new(storage: &'a Waveform, idcode: usize, index: Option<usize>) -> Self {
        Self {
            storage,
            idcode,
            index,
        }
    }
}

impl<'a> SignalStorage for WaveformEntry<'a> {
    fn get_value(&self, timestamp_index: usize) -> Option<(usize, SignalValue)> {
        match self.storage.get_signal(self.idcode) {
            WaveformSignalResult::Vector(signal) => {
                if let Some(pos) = signal.get_history().search_timestamp_index(timestamp_index) {
                    let pos = pos.get_index();
                    let bv = signal.get_bitvector(pos.get_value_index());
                    let bv = if let Some(index) = self.index {
                        BitVector::from(bv.get_bit(index))
                    } else {
                        bv
                    };
                    Some((pos.get_timestamp_index(), SignalValue::Vector(bv)))
                } else {
                    None
                }
            }
            WaveformSignalResult::Real(signal) => {
                if let Some(pos) = signal.get_history().search_timestamp_index(timestamp_index) {
                    let pos = pos.get_index();
                    let r = signal.get_real(pos.get_value_index());
                    Some((pos.get_timestamp_index(), SignalValue::Real(r)))
                } else {
                    None
                }
            }
            WaveformSignalResult::None => None,
        }
    }

    fn search_timestamp(&self, timestamp: u64) -> Option<usize> {
        self.storage.search_timestamp(timestamp)
    }

    fn search_timestamp_after(&self, timestamp: u64) -> Option<usize> {
        self.storage.search_timestamp_after(timestamp)
    }

    fn search_timestamp_range(
        &self,
        timestamp_range: std::ops::Range<u64>,
        greedy: bool,
    ) -> Option<std::ops::Range<usize>> {
        self.storage.search_timestamp_range(timestamp_range, greedy)
    }

    fn get_timestamps(&self) -> &Vec<u64> {
        self.storage.get_timestamps()
    }
}

pub struct WaveformWidget<'a> {
    timescale_widget: Timescale<'a>,
    signal_widgets: Vec<Option<Signal<'a, WaveformEntry<'a>>>>,
    /// A block to wrap the widget in
    block: Option<Block<'a>>,
    /// Widget style
    style: Style,
}

impl<'a> WaveformWidget<'a> {
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a> Widget for WaveformWidget<'a> {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, self.style);
        let area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        if area.height < 1 {
            return;
        }

        let mut area_line = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        self.timescale_widget.render(area_line, buf);
        for (i, signal_widget) in self.signal_widgets.into_iter().enumerate() {
            if (i + 1) as u16 >= area.height {
                break;
            }
            area_line.y = area.y + (i + 1) as u16;
            if let Some(signal_widget) = signal_widget {
                signal_widget.render(area_line, buf);
            }
        }
    }
}
