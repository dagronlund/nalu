use std::ops::Range;

use makai_waveform_db::{
    bitvector::{BitVectorRadix, Logic},
    Waveform, WaveformSearchMode, WaveformValueResult,
};

use tui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Paragraph, Widget},
};

use super::timescale::TimescaleState;

pub struct WaveformWidget<'a> {
    /// The timescale range and cursor position to render
    timescale_state: &'a TimescaleState,
    /// The waveform container to query
    waveform: &'a Waveform,
    /// The idcode of the signal to render
    idcode: usize,
    /// Optionally what bit-index of a multi-bit vector to render
    bit_index: Option<usize>,
    /// How to render the signal values
    radix: BitVectorRadix,
    /// If the signal itself is selected
    is_selected: bool,
}

impl<'a> WaveformWidget<'a> {
    pub fn new(
        timescale_state: &'a TimescaleState,
        waveform: &'a Waveform,
        idcode: usize,
        bit_index: Option<usize>,
        radix: BitVectorRadix,
        is_selected: bool,
    ) -> Self {
        Self {
            timescale_state,
            waveform,
            idcode,
            bit_index,
            radix,
            is_selected,
        }
    }
}

#[derive(Clone, Debug)]
enum WaveformQuery {
    SingleEdge(WaveformValueResult, usize),
    MultipleEdge(usize),
    Static(WaveformValueResult, usize),
    StaticVoid(WaveformValueResult, usize),
    None(usize),
}

impl WaveformQuery {
    fn get_span(&self, radix: BitVectorRadix, _is_selected: bool) -> (String, Style) {
        let (value, width, is_void, is_delta) = match self {
            Self::Static(value, width) => (value, width, false, false),
            Self::StaticVoid(value, width) => (value, width, true, false),
            Self::SingleEdge(value, width) => (value, width, false, true),
            Self::MultipleEdge(width) => {
                return (
                    "#".repeat(*width),
                    Style::default().fg(Color::Black).bg(Color::Gray),
                )
            }
            Self::None(width) => {
                return (
                    " ".repeat(*width),
                    Style::default().fg(Color::White).bg(Color::Black),
                )
            }
        };

        let style = if is_void {
            Style::default().fg(Color::Gray).bg(Color::Gray)
        } else if value.is_unknown() {
            Style::default().fg(Color::Red).bg(Color::Black)
        } else if value.is_high_impedance() {
            Style::default().fg(Color::Blue).bg(Color::Black)
        } else {
            Style::default().fg(Color::White).bg(Color::Black)
        };

        let raw = match value {
            WaveformValueResult::Vector(bv, _) => {
                if bv.get_bit_width() <= 1 {
                    match bv.get_bit(0) {
                        Logic::Zero => "_".repeat(*width),
                        Logic::One => "█".repeat(*width),
                        Logic::Unknown => "X".repeat(*width),
                        Logic::HighImpedance => "Z".repeat(*width),
                    }
                } else if is_delta {
                    format!("|{}", bv.to_string_radix(radix))
                } else {
                    bv.to_string_radix(radix)
                }
            }
            WaveformValueResult::Real(f, _) => {
                if is_delta {
                    format!("|{}", f)
                } else {
                    format!("{}", f)
                }
            }
        };

        let mut sized = String::with_capacity(raw.as_bytes().len());
        let mut chars = 0;
        for (i, c) in raw.chars().enumerate() {
            if i < *width {
                sized.push(c);
                chars += 1;
            }
        }
        if chars < *width {
            for _ in 0..(*width - chars) {
                sized.push(' ');
            }
        }
        (sized, style)
    }
}

impl<'a> WaveformWidget<'a> {
    fn get_query(&self, timestamp_range: Range<u64>) -> WaveformQuery {
        // Find the timestamp indices that are contained by the timestamp range
        if timestamp_range.end == 0 {
            return WaveformQuery::None(1);
        }
        let Some(timestamp_index_start) = self.waveform.search_timestamp(
            timestamp_range.start,
            WaveformSearchMode::After
        ) else {
            return WaveformQuery::None(1);
        };
        let Some(timestamp_index_end) = self.waveform.search_timestamp(
            timestamp_range.end - 1,
            WaveformSearchMode::Before
        ) else {
            return WaveformQuery::None(1);
        };
        // Check if there is a value available
        let Some(result) = self.waveform.search_value_bit_index(
            self.idcode,
            timestamp_index_end,
            WaveformSearchMode::Before,
            self.bit_index,
        ) else {
            return WaveformQuery::None(1);
        };
        if result.get_timestamp_index() < timestamp_index_start {
            // Value changed before range
            return WaveformQuery::Static(result, 1);
        }
        if timestamp_range.start >= self.timescale_state.get_timestamp_max() {
            // Range starts in void time
            return WaveformQuery::StaticVoid(result, 1);
        }
        if result.get_timestamp_index() == 0 {
            // First timestamp index, nothing before
            return WaveformQuery::SingleEdge(result, 1);
        }
        let Some(result_before) = self.waveform.search_value_bit_index(
            self.idcode,
            result.get_timestamp_index() - 1,
            WaveformSearchMode::Before,
            self.bit_index
        ) else {
            return WaveformQuery::SingleEdge(result, 1);
        };
        if result_before.get_timestamp_index() >= timestamp_index_start {
            WaveformQuery::MultipleEdge(1)
        } else {
            WaveformQuery::SingleEdge(result, 1)
        }
    }
}

impl<'a> Widget for WaveformWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let timestamp_width =
            self.timescale_state.get_range().end - self.timescale_state.get_range().start;
        // Create list of queries, one for each character on the screen
        let queries = (0..area.width as u64)
            .map(|i| {
                (i * timestamp_width / area.width as u64)
                    ..((i + 1) * timestamp_width / area.width as u64)
            })
            .map(|range| {
                range.start + self.timescale_state.get_range().start
                    ..range.end + self.timescale_state.get_range().start
            })
            .map(|range| self.get_query(range))
            .collect::<Vec<WaveformQuery>>();

        // Merge queries together when possible
        let mut queries_compressed: Vec<WaveformQuery> = Vec::with_capacity(queries.len());
        for query in queries.into_iter() {
            let query_last = if let Some(query_last) = queries_compressed.pop() {
                query_last
            } else {
                queries_compressed.push(query);
                continue;
            };
            let query = match (&query_last, query) {
                (WaveformQuery::None(width_last), WaveformQuery::None(width)) => {
                    WaveformQuery::None(width_last + width)
                }
                (WaveformQuery::MultipleEdge(width_last), WaveformQuery::MultipleEdge(width)) => {
                    WaveformQuery::MultipleEdge(width_last + width)
                }
                (WaveformQuery::Static(_, width_last), WaveformQuery::Static(value, width)) => {
                    WaveformQuery::Static(value, width_last + width)
                }
                (WaveformQuery::SingleEdge(value, width_last), WaveformQuery::Static(_, width)) => {
                    WaveformQuery::SingleEdge(value.clone(), width_last + width)
                }
                (
                    WaveformQuery::StaticVoid(_, width_last),
                    WaveformQuery::StaticVoid(value, width),
                ) => WaveformQuery::StaticVoid(value, width_last + width),
                (query_last, query) => {
                    queries_compressed.push((*query_last).clone());
                    query
                }
            };
            queries_compressed.push(query);
        }

        // Render queries into a set of styled spans
        let mut spans = Vec::new();
        for query in queries_compressed {
            let (string, style) = query.get_span(self.radix, self.is_selected);
            spans.push(Span::styled(string, style));
        }

        Paragraph::new(Text::from(Spans::from(spans)))
            .alignment(Alignment::Left)
            .render(area, buf)
    }
}

#[test]
fn signal_render_test() {
    use makai::utils::messages::Messages;
    use makai_vcd_reader::utils::{load_multi_threaded, VcdLoaderMessage};
    use std::thread;

    let fname = "res/gecko.vcd";

    // Read VCD file header and build out waveform structure
    let bytes = std::fs::read_to_string(fname).unwrap();
    let messages = Messages::new();
    let handle = load_multi_threaded(bytes, 4, messages.clone());
    let (header, waveform) = loop {
        let mut result = None;
        for messages in messages.get::<VcdLoaderMessage>() {
            match messages {
                VcdLoaderMessage::Done(r) => result = Some(r.unwrap()),
                _ => {}
            }
        }
        if let Some(result) = result {
            break result;
        }
        thread::sleep(std::time::Duration::from_millis(10));
    };
    handle.join().unwrap();

    let timestamp_range = 0u64..100u64;

    let idcode = header.get_variable("TOP.clk").unwrap().get_idcode();

    let mut timescale_state = TimescaleState::new();
    timescale_state.load_waveform(
        timestamp_range.clone(),
        waveform.get_timestamp_range().end,
        header.get_timescale().unwrap(),
    );

    println!("Timescale: {}", header.get_timescale().unwrap());
    println!("Timestamp range: {:?}", waveform.get_timestamp_range());

    timescale_state.load_waveform(
        0..100,
        waveform.get_timestamp_range().end,
        header.get_timescale().unwrap(),
    );

    let rect = Rect::new(0, 0, 50, 1);
    let mut buffer = Buffer::empty(rect.clone());
    WaveformWidget::new(
        &timescale_state,
        &waveform,
        idcode,
        None,
        BitVectorRadix::Hexadecimal,
        false,
    )
    .render(rect, &mut buffer);
    for x in 0..rect.width {
        print!("{}", buffer.get(x, 0).symbol);
    }
    println!();

    let rect = Rect::new(0, 0, 100, 1);
    let mut buffer = Buffer::empty(rect.clone());
    WaveformWidget::new(
        &timescale_state,
        &waveform,
        idcode,
        None,
        BitVectorRadix::Hexadecimal,
        false,
    )
    .render(rect, &mut buffer);
    for x in 0..rect.width {
        print!("{}", buffer.get(x, 0).symbol);
    }
    println!();

    timescale_state.load_waveform(
        0..500,
        waveform.get_timestamp_range().end,
        header.get_timescale().unwrap(),
    );

    let rect = Rect::new(0, 0, 400, 1);
    let mut buffer = Buffer::empty(rect.clone());
    WaveformWidget::new(
        &timescale_state,
        &waveform,
        idcode,
        None,
        BitVectorRadix::Hexadecimal,
        false,
    )
    .render(rect, &mut buffer);
    for x in 0..rect.width {
        print!("{}", buffer.get(x, 0).symbol);
    }
    println!();

    let idcode = header.get_variable("TOP.rst").unwrap().get_idcode();

    let rect = Rect::new(0, 0, 400, 1);
    let mut buffer = Buffer::empty(rect.clone());
    WaveformWidget::new(
        &timescale_state,
        &waveform,
        idcode,
        None,
        BitVectorRadix::Hexadecimal,
        false,
    )
    .render(rect, &mut buffer);
    for x in 0..rect.width {
        print!("{}", buffer.get(x, 0).symbol);
    }
    println!();

    let idcode = header
        .get_variable("TOP.gecko_nano_wrapper.inst.core.data_request.write_enable")
        .unwrap()
        .get_idcode();

    timescale_state.load_waveform(
        0..1000,
        waveform.get_timestamp_range().end,
        header.get_timescale().unwrap(),
    );

    let rect = Rect::new(0, 0, 400, 1);
    let mut buffer = Buffer::empty(rect.clone());
    WaveformWidget::new(
        &timescale_state,
        &waveform,
        idcode,
        None,
        BitVectorRadix::Hexadecimal,
        false,
    )
    .render(rect, &mut buffer);
    for x in 0..rect.width {
        print!("{}", buffer.get(x, 0).symbol);
    }
    println!();
}
