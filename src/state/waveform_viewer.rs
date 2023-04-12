use std::path::PathBuf;
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyEvent, MouseEventKind};
use makai::utils::messages::Messages;
use makai_vcd_reader::parser::VcdHeader;
use makai_waveform_db::{bitvector::BitVectorRadix, Waveform};
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Paragraph, Widget},
};
use tui_tiling::component::ComponentWidget;

use crate::{
    python::utils::run_interactive,
    state::signal_viewer::SignalViewerMessage,
    widgets::timescale::{Timescale, TimescaleState},
    widgets::waveform::WaveformWidget,
};

#[derive(Debug, Clone)]
pub(crate) struct WaveformNode {
    pub(crate) idcode: usize,
    pub(crate) index: Option<usize>,
    pub(crate) radix: BitVectorRadix,
    pub(crate) is_selected: bool,
}

pub(crate) enum WaveformViewerMessage {
    UpdateSignals(Vec<Option<WaveformNode>>),
    WaveformUpdate(Arc<Waveform>, Arc<VcdHeader>, i32, Option<PathBuf>),
}

pub struct WaveformViewerState {
    width: usize,
    height: usize,
    waveform: Arc<Waveform>,
    vcd_header: Arc<VcdHeader>,
    timescale_state: TimescaleState,
    signal_entries: Vec<Option<WaveformNode>>,
    python_view: bool,
    python_path: Option<PathBuf>,
    messages: Messages,
}

impl WaveformViewerState {
    pub fn new(messages: Messages) -> Self {
        Self {
            width: 0,
            height: 0,
            waveform: Arc::new(Waveform::default()),
            vcd_header: Arc::new(VcdHeader::default()),
            timescale_state: TimescaleState::new(),
            signal_entries: Vec::new(),
            python_view: false,
            python_path: None,
            messages,
        }
    }

    pub fn load_waveform(
        &mut self,
        waveform: Arc<Waveform>,
        vcd_header: Arc<VcdHeader>,
        timescale: i32,
        python_path: Option<PathBuf>,
    ) {
        self.waveform = waveform;
        self.vcd_header = vcd_header;
        let range = self.waveform.get_timestamp_range();
        self.timescale_state
            .load_waveform(range.clone(), range.end, timescale);
        self.python_path = python_path;
    }

    pub fn set_size(&mut self, size: &Rect, border_width: u16) {
        self.width = if size.width > (border_width * 2) {
            (size.width - (border_width * 2)) as usize
        } else {
            0
        };
        self.height = size.height as usize;
    }

    fn get_waveform_widget(&self) -> WaveformViewerWidget<'_> {
        let signal_widgets = self
            .signal_entries
            .iter()
            .map(|entry| {
                entry.as_ref().map(|entry| {
                    WaveformWidget::new(
                        &self.timescale_state,
                        &self.waveform,
                        entry.idcode,
                        entry.index,
                        entry.radix,
                        entry.is_selected,
                    )
                })
            })
            .collect::<Vec<Option<WaveformWidget>>>();
        WaveformViewerWidget {
            timescale_widget: Timescale::new(&self.timescale_state),
            signal_widgets,
            block: None,
            style: Default::default(),
        }
    }

    fn get_python_widget(&self) -> Paragraph<'_> {
        let Some(python_path) = self.python_path.clone() else {
            return Paragraph::new("No python loaded!");
        };
        match run_interactive(
            python_path,
            self.width as u16,
            self.height as u16,
            self.waveform.clone(),
            self.vcd_header.clone(),
            self.timescale_state.get_cursor(),
        ) {
            Ok(p) => p,
            Err(err) => Paragraph::new(format!("{err:#?}")),
        }
    }
}

pub struct WaveformViewerWidget<'a> {
    timescale_widget: Timescale<'a>,
    signal_widgets: Vec<Option<WaveformWidget<'a>>>,
    /// A block to wrap the widget in
    block: Option<Block<'a>>,
    /// Widget style
    style: Style,
}

impl<'a> WaveformViewerWidget<'a> {
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a> Widget for WaveformViewerWidget<'a> {
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
    fn handle_mouse(&mut self, _x: u16, _y: u16, _kind: MouseEventKind) -> bool {
        false
    }

    fn handle_key(&mut self, e: KeyEvent) -> bool {
        match e.code {
            KeyCode::Char('v') => self.python_view = !self.python_view,
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
            | KeyCode::Delete => {
                self.messages.push(SignalViewerMessage::WaveformKey(e));
            }
            _ => return false,
        }
        true
    }

    fn handle_update(&mut self) -> bool {
        let mut updated = false;
        for message in self.messages.get::<WaveformViewerMessage>() {
            match message {
                WaveformViewerMessage::UpdateSignals(signals) => {
                    self.signal_entries = signals;
                }
                WaveformViewerMessage::WaveformUpdate(
                    waveform,
                    vcd_header,
                    timescale,
                    python_path,
                ) => {
                    self.load_waveform(waveform, vcd_header, timescale, python_path);
                }
            }
            updated = true;
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
        if self.python_view {
            self.get_python_widget()
                .style(Style::default().fg(Color::LightCyan))
                .render(area, buf);
        } else {
            self.get_waveform_widget()
                .style(Style::default().fg(Color::LightCyan))
                .render(area, buf);
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
