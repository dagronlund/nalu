use crossterm::event::KeyCode;

use tui::{layout::Rect, style::Style, text::Text};

use vcd_parser::parser::*;

use crate::state::browser::BrowserRequest;
use crate::state::utils::*;

enum WaveformNode {
    Group,
    Spacer,
    VectorSignal,
}

struct WaveformSignal {
    variable: VcdVariable,
    expanded: bool,
}

impl WaveformSignal {
    fn new(variable: VcdVariable) -> Self {
        Self {
            variable: variable,
            expanded: false,
        }
    }

    fn render(&self, text: &mut Text<'static>, offsets: &mut RenderContext) {
        if !offsets.is_rendering() {
            return;
        }
        let line = Text::styled(
            format!(
                "{} {} {}",
                if self.expanded { "[-]" } else { "[+]" },
                self.variable.get_name(),
                self.variable.get_width(),
            ),
            get_selected_style(offsets.is_selected()),
        );
        offsets.render_line(text, line);
        if self.expanded {
            for bit in 0..self.variable.get_bit_width() {
                let line = Text::styled(
                    format!("    {} [{}]", self.variable.get_name(), bit),
                    get_selected_style(offsets.is_selected()),
                );
                offsets.render_line(text, line);
            }
        }
    }
}

pub struct WaveformState {
    list_width: usize,
    viewer_width: usize,
    context: SelectContext,
    signals: Vec<WaveformSignal>,
}

impl WaveformState {
    pub fn new() -> Self {
        Self {
            list_width: 0,
            viewer_width: 0,
            context: SelectContext::new(),
            signals: Vec::new(),
        }
    }

    pub fn browser_request(&mut self, request: BrowserRequest) {
        match request {
            BrowserRequest::Append(variables) => {
                for v in variables {
                    self.signals.push(WaveformSignal::new(v));
                }
            }
            BrowserRequest::Insert(_) => {}
            BrowserRequest::None => {}
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
        self.context.set_height(list_size.height as isize - margin);
    }

    pub fn handle_key_list(&mut self, key: KeyCode) {
        let line_count = self.get_expanded_line_count();
        match key {
            KeyCode::Up => self.context.select_relative(-1, line_count),
            KeyCode::Down => self.context.select_relative(1, line_count),
            KeyCode::PageDown => self.context.select_relative(20, line_count),
            KeyCode::PageUp => self.context.select_relative(-20, line_count),
            KeyCode::Enter => {
                self.toggle_expanded();
                self.context.scroll_relative(0);
            }
            _ => {}
        }
    }

    pub fn handle_mouse_click_list(&mut self, _: u16, y: u16) {
        let line_count = self.get_expanded_line_count();
        if self.context.select_absolute(y as isize - 1, line_count) {
            self.toggle_expanded();
        }
    }

    pub fn handle_mouse_scroll_list(&mut self, scroll_up: bool) {
        let line_count = self.get_expanded_line_count();
        self.context
            .select_relative(if scroll_up { -5 } else { 5 }, line_count);
    }

    pub fn render_list(&self) -> Text<'static> {
        let mut text = Text::styled(" ", Style::default());
        let mut offsets = self.context.make_render_offsets();
        for signal in &self.signals {
            signal.render(&mut text, &mut offsets);
        }
        text
    }

    pub fn render_waveform(&self) -> Text<'static> {
        Text::styled(" ", Style::default())
    }

    fn get_expanded_line_count(&self) -> usize {
        let mut line_count = 0;
        for signal in &self.signals {
            line_count += 1;
            if signal.expanded {
                line_count += signal.variable.get_bit_width();
            }
        }
        line_count
    }

    fn toggle_expanded(&mut self) {
        let mut select_offset = self.context.get_select_offset();
        for signal in &mut self.signals {
            if select_offset == 0 {
                signal.expanded = !signal.expanded;
                return;
            }
            select_offset -= 1;
            if signal.expanded {
                select_offset -= signal.variable.get_bit_width() as isize;
            }
        }
    }
}
