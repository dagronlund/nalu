use std::ops::Range;

use waveform_db::bitvector::{BitVector, BitVectorRadix, Logic};

use tui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Paragraph, Widget},
};

use super::timescale::TimescaleState;

#[derive(Clone, Debug)]
pub enum SignalValue {
    Vector(BitVector),
    Real(f64),
}

impl SignalValue {
    pub fn is_unknown(&self) -> bool {
        match self {
            Self::Vector(bv) => bv.is_unknown(),
            Self::Real(_) => false,
        }
    }

    pub fn is_high_impedance(&self) -> bool {
        match self {
            Self::Vector(bv) => bv.is_high_impedance(),
            Self::Real(_) => false,
        }
    }
}

pub trait SignalStorage {
    /// Returns the value at or immediately before the given timestamp and its
    /// timestamp index
    fn get_value(&self, timestamp_index: usize) -> Option<(usize, SignalValue)>;

    /// Binary search for the index of the requested timestamp, or if not
    /// found the timestamp immediately before it
    fn search_timestamp(&self, timestamp: u64) -> Option<usize>;

    /// Binary search for the index of the requested timestamp, or if not
    /// found the timestamp immediately after it
    fn search_timestamp_after(&self, timestamp: u64) -> Option<usize>;

    /// Returns the range of timestamp indices that either contains the given
    /// timestamps (greedy) or is contained by the given timestamps (non-greedy)
    fn search_timestamp_range(
        &self,
        timestamp_range: std::ops::Range<u64>,
        greedy: bool,
    ) -> Option<std::ops::Range<usize>>;

    fn get_timestamps(&self) -> &Vec<u64>;
}

pub struct Signal<'a, S> {
    /// The timescale range and cursor position to render
    state: &'a TimescaleState,
    /// The signal values across time to render
    storage: S,
    /// How to render the signal values
    radix: BitVectorRadix,
    /// If the signal itself is selected
    selected: bool,
}

impl<'a, S> Signal<'a, S> {
    pub fn new(
        state: &'a TimescaleState,
        storage: S,
        radix: BitVectorRadix,
        selected: bool,
    ) -> Self {
        Self {
            state,
            storage,
            radix,
            selected,
        }
    }
}

#[derive(Clone, Debug)]
enum SignalQuery {
    SingleEdge(usize, SignalValue, usize),
    MultipleEdge(usize),
    Static(SignalValue, usize),
    StaticVoid(SignalValue, usize),
    None(usize),
}

impl SignalQuery {
    fn get_span(&self, radix: BitVectorRadix, _is_selected: bool) -> (String, Style) {
        let (value, width, is_void, is_delta) = match self {
            Self::Static(value, width) => (value, width, false, false),
            Self::StaticVoid(value, width) => (value, width, true, false),
            Self::SingleEdge(_, value, width) => (value, width, false, true),
            Self::MultipleEdge(width) => {
                return (
                    format!("#").repeat(*width),
                    Style::default().fg(Color::Black).bg(Color::Gray),
                )
            }
            Self::None(width) => {
                return (
                    format!(" ").repeat(*width),
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
            SignalValue::Vector(bv) => {
                if bv.get_bit_width() <= 1 {
                    match bv.get_bit(0) {
                        Logic::Zero => format!("_").repeat(*width),
                        Logic::One => format!("█").repeat(*width),
                        Logic::Unknown => format!("X").repeat(*width),
                        Logic::HighImpedance => format!("Z").repeat(*width),
                    }
                } else {
                    if is_delta {
                        format!("|{}", bv.to_string_radix(radix))
                    } else {
                        bv.to_string_radix(radix)
                    }
                }
            }
            SignalValue::Real(f) => {
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

impl<'a, S> Signal<'a, S>
where
    S: SignalStorage,
{
    fn get_query(&self, timestamp_range: Range<u64>) -> SignalQuery {
        // Find the timestamp indices that are contained by the timestamp range
        let timestamp_index_range = if let Some(range) = self
            .storage
            .search_timestamp_range(timestamp_range.clone(), false)
        {
            range
        } else {
            return SignalQuery::None(1);
        };
        // Check if there is a value available
        let (timestamp_index, value) =
            if let Some(iv) = self.storage.get_value(timestamp_index_range.end) {
                iv
            } else {
                return SignalQuery::None(1);
            };
        if timestamp_index < timestamp_index_range.start {
            // Value changed before range
            return SignalQuery::Static(value, 1);
        }
        if timestamp_range.start >= self.state.get_timestamp_max() {
            // Range starts in void time
            return SignalQuery::StaticVoid(value, 1);
        }
        if timestamp_index == 0 {
            // First timestamp index, nothing before
            return SignalQuery::SingleEdge(timestamp_index, value, 1);
        }
        if let Some((timestamp_index_next, _)) = self.storage.get_value(timestamp_index - 1) {
            if timestamp_index_next >= timestamp_index_range.start {
                SignalQuery::MultipleEdge(1)
            } else {
                SignalQuery::SingleEdge(timestamp_index, value, 1)
            }
        } else {
            SignalQuery::SingleEdge(timestamp_index, value, 1)
        }
    }
}

impl<'a, S> Widget for Signal<'a, S>
where
    S: SignalStorage,
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        let timestamp_width = self.state.get_range().end - self.state.get_range().start;
        // Create list of queries, one for each character on the screen
        let queries = (0..area.width as u64)
            .map(|i| {
                (i * timestamp_width / area.width as u64)
                    ..((i + 1) * timestamp_width / area.width as u64)
            })
            .map(|range| {
                range.start + self.state.get_range().start..range.end + self.state.get_range().start
            })
            .map(|range| {
                let query = self.get_query(range.clone());
                query
            })
            .collect::<Vec<SignalQuery>>();

        // Merge queries together when possible
        let mut queries_compressed: Vec<SignalQuery> = Vec::with_capacity(queries.len());
        for query in queries.into_iter() {
            let query_last = if let Some(query_last) = queries_compressed.pop() {
                query_last
            } else {
                queries_compressed.push(query);
                continue;
            };
            let query = match (&query_last, query) {
                (SignalQuery::None(width_last), SignalQuery::None(width)) => {
                    SignalQuery::None(width_last + width)
                }
                (SignalQuery::MultipleEdge(width_last), SignalQuery::MultipleEdge(width)) => {
                    SignalQuery::MultipleEdge(width_last + width)
                }
                (SignalQuery::Static(_, width_last), SignalQuery::Static(value, width)) => {
                    SignalQuery::Static(value, width_last + width)
                }
                (
                    SignalQuery::SingleEdge(timestamp_index, _, width_last),
                    SignalQuery::Static(value, width),
                ) => SignalQuery::SingleEdge(*timestamp_index, value, width_last + width),
                (SignalQuery::StaticVoid(_, width_last), SignalQuery::StaticVoid(value, width)) => {
                    SignalQuery::StaticVoid(value, width_last + width)
                }
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
            let (string, style) = query.get_span(self.radix, self.selected);
            spans.push(Span::styled(string, style));
        }

        Paragraph::new(Text::from(Spans::from(spans)))
            .alignment(Alignment::Left)
            .render(area, buf)
    }
}

#[test]
fn signal_render_test() {
    use std::sync::{Arc, Mutex};
    use std::thread;
    use waveform_db::*;

    use crate::state::waveform_viewer::*;

    let fname = "res/gecko.vcd";

    // Read VCD file header and build out waveform structure
    let bytes = std::fs::read_to_string(fname).unwrap();
    let status = Arc::new(Mutex::new((0, 0)));
    let handle = vcd_parser::utils::load_multi_threaded(bytes, 4, status.clone());
    loop {
        let (pos, total) = *status.lock().unwrap();
        if pos >= total && total > 0 {
            break;
        }
        thread::sleep(std::time::Duration::from_millis(10));
    }
    let (header, waveform) = handle.join().unwrap().unwrap();

    let timestamp_range = 0u64..100u64;

    let idcode = header.get_variable("TOP.clk").unwrap().get_idcode();
    let signal = match waveform.get_signal(idcode) {
        WaveformSignalResult::Vector(signal) => signal,
        _ => panic!("Cannot find vector signal!"),
    };

    let timestamp_index = waveform.search_timestamp(5).unwrap();
    let mut pos = signal
        .get_history()
        .search_timestamp_index(timestamp_index)
        .unwrap();
    for _ in 0..5 {
        println!(
            "Timestamp Index: {}, Pos: {pos:?}",
            pos.get_index().get_timestamp_index()
        );
        pos = pos.next(&signal.get_history()).unwrap();
    }

    let timestamp_index = waveform.search_timestamp(10).unwrap();
    let mut pos = signal
        .get_history()
        .search_timestamp_index(timestamp_index)
        .unwrap();
    for _ in 0..5 {
        println!(
            "Timestamp Index: {}, Pos: {pos:?}",
            pos.get_index().get_timestamp_index()
        );
        pos = pos.next(&signal.get_history()).unwrap();
    }

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
    Signal::new(
        &timescale_state,
        WaveformEntry::new(&waveform, idcode, None),
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
    Signal::new(
        &timescale_state,
        WaveformEntry::new(&waveform, idcode, None),
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
    Signal::new(
        &timescale_state,
        WaveformEntry::new(&waveform, idcode, None),
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
    Signal::new(
        &timescale_state,
        WaveformEntry::new(&waveform, idcode, None),
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
    Signal::new(
        &timescale_state,
        WaveformEntry::new(&waveform, idcode, None),
        BitVectorRadix::Hexadecimal,
        false,
    )
    .render(rect, &mut buffer);
    for x in 0..rect.width {
        print!("{}", buffer.get(x, 0).symbol);
    }
    println!();
}
