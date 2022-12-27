use crossterm::event::{KeyCode, KeyEvent, MouseEventKind};
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Widget},
};
use tui_layout::component::ComponentWidget;

use vcd_parser::waveform::{bitvector::BitVector, Waveform, WaveformSignalResult};

use crate::signal_viewer::{SignalViewerEntry, SignalViewerRequest};
use crate::widgets::signal::*;
use crate::widgets::timescale::*;

pub struct WaveformViewerRequest(pub KeyEvent);

pub struct WaveformViewerState {
    width: usize,
    waveform: Waveform,
    timescale_state: TimescaleState,
    signal_entries: Vec<Option<SignalViewerEntry>>,
    requests: Vec<WaveformViewerRequest>,
}

impl WaveformViewerState {
    pub fn new() -> Self {
        Self {
            width: 0,
            waveform: Waveform::new(),
            timescale_state: TimescaleState::new(),
            signal_entries: Vec::new(),
            requests: Vec::new(),
        }
    }

    pub fn load_waveform(&mut self, waveform: Waveform, timescale: i32) {
        self.waveform = waveform;
        let range = self.waveform.get_timestamp_range();
        self.timescale_state
            .load_waveform(range.clone(), range.end, timescale);
    }

    pub fn set_size(&mut self, size: &Rect, border_width: u16) {
        self.width = if size.width > (border_width * 2) {
            (size.width - (border_width * 2)) as usize
        } else {
            0
        };
    }

    pub fn handle_key_press(&mut self, event: KeyEvent) {
        // let ctrl = event.modifiers.contains(KeyModifiers::CONTROL);
        // let shift = event.modifiers.contains(KeyModifiers::SHIFT);
        match event.clone().code {
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
            | KeyCode::Delete => self.requests.push(WaveformViewerRequest(event)),
            _ => {}
        }
    }

    pub fn get_waveform_widget<'a>(&'a self) -> WaveformWidget<'a> {
        let mut signal_widgets = Vec::new();
        for signal_entry in &self.signal_entries {
            if let Some(signal_entry) = signal_entry {
                signal_widgets.push(Some(Signal::new(
                    &self.timescale_state,
                    WaveformEntry {
                        storage: &self.waveform,
                        idcode: signal_entry.idcode,
                        index: signal_entry.index,
                    },
                    signal_entry.radix,
                    signal_entry.is_selected,
                )));
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

    pub fn signal_request(&mut self, requests: Vec<SignalViewerRequest>) {
        for request in requests {
            self.signal_entries = request.0;
        }
    }

    // fn modify_list(&mut self, action: ListAction) {
    //     // let mut select_offset = self.tree_select.get_primary_selected();

    //     // let selected = match self.tree.get_selected_mut(&mut select_offset) {
    //     //     Some((_, selected)) => selected,
    //     //     None => return,
    //     // };

    //     // let variables = match selected.get_value() {
    //     //     NodeValue::Variable(variable) => vec![variable.clone()],
    //     //     NodeValue::Scope(_) => get_scope_variables(selected.get_nodes()),
    //     // };

    //     match action {
    //         ListAction::Group => {}
    //         ListAction::Delete => {}
    //         ListAction::Expand => {
    //             let path = self.list_state.get_primary_selected_path(&self.node);
    //             if let Some(node) = self.node.get_node_mut(&path) {
    //                 node.set_expanded(!node.is_expanded());
    //             }
    //         }
    //     }
    // }

    pub fn get_requests(&mut self) -> Vec<WaveformViewerRequest> {
        let mut requests = Vec::new();
        std::mem::swap(&mut requests, &mut self.requests);
        requests
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

impl ComponentWidget for WaveformViewerState {
    fn handle_mouse(&mut self, _x: u16, _y: u16, _kind: MouseEventKind) {}

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
        self.get_waveform_widget()
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
