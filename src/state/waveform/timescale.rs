use core::ops::Range;

use tui::{
    style::Style,
    text::{Span, Spans, Text},
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

pub struct WaveformTime {
    timestamp_range: Range<u64>,
    timestamp_cursor: u64,
    timescale: i32,
    timestamp_max: u64,
}

impl WaveformTime {
    pub fn new() -> Self {
        Self {
            timestamp_range: 0..1000000, // Actual time is timestamp*10^(-timescale)
            timestamp_cursor: 0,
            timescale: 6,
            timestamp_max: 1000000,
        }
    }

    pub fn load_waveform(&mut self, new_range: Range<u64>, timestamp_max: u64, timescale: i32) {
        self.timescale = timescale;
        // TODO: Keep old timescale range if it still makes sense and timescales same
        self.timestamp_range = new_range;
        self.timestamp_max = timestamp_max;
    }

    pub fn zoom_left(&mut self, cursor: bool) {
        let width = self.get_width();
        if self.timestamp_range.start > width / 2 {
            self.timestamp_range = (self.timestamp_range.start - (width / 2))
                ..(self.timestamp_range.end - (width / 2));
        } else {
            self.timestamp_range = 0..width;
        }
    }

    pub fn zoom_right(&mut self, cursor: bool) {
        let width = self.get_width();
        if self.timestamp_range.end < (self.timestamp_max + width / 2) {
            self.timestamp_range = (self.timestamp_range.start + (width / 2))
                ..(self.timestamp_range.end + (width / 2));
        } else {
            self.timestamp_range =
                (self.timestamp_max - width / 2)..(self.timestamp_max + width / 2);
        }
    }

    pub fn zoom_in(&mut self, cursor: bool) {
        // TODO: Support zooming in around cursor
        // Find the center of the timestamp range and then average start/end with the center
        let center = self.get_center();
        self.timestamp_range =
            ((self.timestamp_range.start + center) / 2)..((self.timestamp_range.end + center) / 2);
    }

    pub fn zoom_out(&mut self, cursor: bool) {
        // TODO: Support zooming out around cursor
        let center = self.get_center();
        let width = self.get_width();
        if center >= width {
            self.timestamp_range = (center - width)..(center + width);
        } else {
            self.timestamp_range = 0..(width * 2);
        }
    }

    pub fn render(&self, width: usize) -> Text<'static> {
        let mut text = Spans::from(Vec::new());
        if self.timestamp_range.start == self.timestamp_range.end {
            text.0
                .push(Span::from(format!("|{}|", self.timestamp_range.start)));
            return Text::from(text);
        }
        let mut timestamp_current = self.timestamp_range.start;
        let timestamp_step = if width > 0 {
            (self.timestamp_range.end - self.timestamp_range.start) / width as u64
        } else {
            0
        };
        // text.0.push(Span::from(format!(
        //     "<{}><{}>",
        //     (timestamp_current as f64) * 10.0f64.powi(-self.timescale),
        //     (timestamp_step as f64) * 10.0f64.powi(-self.timescale),
        // )));
        while text.width() < width {
            let s = format!(
                "|{}",
                render_time(timestamp_current, timestamp_step, self.timescale)
            );
            timestamp_current += timestamp_step * s.len() as u64;
            text.0.push(Span::from(s));
        }
        Text::from(text)
    }

    fn get_width(&self) -> u64 {
        if self.timestamp_range.start < self.timestamp_range.end {
            self.timestamp_range.end - self.timestamp_range.start
        } else {
            1
        }
    }

    fn get_center(&self) -> u64 {
        (self.timestamp_range.start + self.timestamp_range.end) / 2
    }
}
