use std::ops::Range;

use tui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    text::{Span, Spans, Text},
    widgets::{Paragraph, Widget},
};

fn render_time(timestamp: u64, resolution: u64, timescale: i32) -> String {
    let mut timestamp = timestamp;
    let mut resolution = resolution;
    let mut offset = 0i32;
    while resolution >= 10 {
        timestamp /= 10;
        resolution /= 10;
        offset += 1;
    }

    let timescale_pow10 = offset - timescale;
    let timescale_pow10_rem = if timescale_pow10 < 0 {
        (timescale_pow10 % 3) + 3
    } else {
        timescale_pow10 % 3
    };

    let timescale_pow10 = timescale_pow10 - timescale_pow10_rem;
    for _ in 0..timescale_pow10_rem {
        timestamp *= 10;
    }

    let mut timestamp_msb_divider = 1u64;
    let mut timestamp_offset = 0i32;
    while timestamp >= (1000 * timestamp_msb_divider) {
        timestamp_msb_divider *= 1000;
        timestamp_offset += 3;
    }

    let timescale_str = match timescale_pow10 + timestamp_offset {
        -15 => "fs",
        -12 => "ps",
        -9 => "ns",
        -6 => "us",
        -3 => "ms",
        0 => "s",
        3 => "Ks",
        6 => "Ms",
        9 => "Gs",
        12 => "Ps",
        15 => "Es",
        _ => "(err)",
    };

    if timestamp_offset > 0 {
        format!(
            "{}.{}{}",
            timestamp / timestamp_msb_divider,
            timestamp % timestamp_msb_divider,
            timescale_str
        )
    } else {
        format!("{}{}", timestamp, timescale_str)
    }
}

pub struct TimescaleState {
    range: Range<u64>,
    _cursor: u64,
    timescale: i32,
    timestamp_max: u64,
}

impl TimescaleState {
    pub fn new() -> Self {
        Self {
            range: 0..1000000, // Actual time is timestamp*10^(-timescale)
            _cursor: 0,
            timescale: 6,
            timestamp_max: 1000000,
        }
    }

    pub fn load_waveform(&mut self, new_range: Range<u64>, timestamp_max: u64, timescale: i32) {
        self.timescale = timescale;
        // TODO: Keep old timescale range if it still makes sense and timescales same
        self.range = new_range;
        self.timestamp_max = timestamp_max;
    }

    pub fn zoom_left(&mut self, _cursor: bool) {
        let width = self.get_width();
        if self.range.start > width / 2 {
            self.range = (self.range.start - (width / 2))..(self.range.end - (width / 2));
        } else {
            self.range = 0..width;
        }
    }

    pub fn zoom_right(&mut self, _cursor: bool) {
        let width = self.get_width();
        if self.range.end < (self.timestamp_max + width / 2) {
            self.range = (self.range.start + (width / 2))..(self.range.end + (width / 2));
        } else {
            self.range = (self.timestamp_max - width / 2)..(self.timestamp_max + width / 2);
        }
    }

    pub fn zoom_in(&mut self, _cursor: bool) {
        // TODO: Support zooming in around cursor
        // Find the center of the timestamp range and then average start/end with the center
        let center = self.get_center();
        self.range = ((self.range.start + center) / 2)..((self.range.end + center) / 2);
    }

    pub fn zoom_out(&mut self, _cursor: bool) {
        // TODO: Support zooming out around cursor
        let center = self.get_center();
        let width = self.get_width();
        if center >= width {
            self.range = (center - width)..(center + width);
        } else {
            self.range = 0..(width * 2);
        }
    }

    fn get_width(&self) -> u64 {
        if self.range.start < self.range.end {
            self.range.end - self.range.start
        } else {
            1
        }
    }

    fn get_center(&self) -> u64 {
        (self.range.start + self.range.end) / 2
    }

    pub fn get_range(&self) -> Range<u64> {
        self.range.clone()
    }

    pub fn get_cursor(&self) -> u64 {
        // TODO: Implement actual cursor
        self.range.start
    }

    pub fn get_timescale(&self) -> i32 {
        self.timescale
    }

    pub fn get_timestamp_max(&self) -> u64 {
        self.timestamp_max
    }
}

impl Default for TimescaleState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Timescale<'a> {
    /// The timescale range and cursor position to render
    state: &'a TimescaleState,
}

impl<'a> Timescale<'a> {
    pub fn new(state: &'a TimescaleState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for Timescale<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let text = if self.state.range.start == self.state.range.end {
            Text::raw(format!("|{}|", self.state.range.start))
        } else {
            let mut text = Spans::from(Vec::new());
            let mut timestamp_current = self.state.range.start;
            let timestamp_step = if area.width > 0 {
                (self.state.range.end - self.state.range.start) / area.width as u64
            } else {
                0
            };
            // text.0.push(Span::from(format!(
            //     "<{}><{}>",
            //     (timestamp_current as f64) * 10.0f64.powi(-self.timescale),
            //     (timestamp_step as f64) * 10.0f64.powi(-self.timescale),
            // )));
            while text.width() < area.width as usize {
                let s = format!(
                    "|{}",
                    render_time(timestamp_current, timestamp_step, self.state.timescale)
                );
                timestamp_current += timestamp_step * s.len() as u64;
                text.0.push(Span::from(s));
            }
            Text::from(text)
        };

        Paragraph::new(text)
            .alignment(Alignment::Left)
            .render(area, buf)
    }
}
