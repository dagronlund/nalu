pub mod python;
pub mod state;
pub mod widgets;

use std::io::{stdout, Stdout, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{self, Duration, Instant};

use clap::Parser;
use crossbeam::channel::{unbounded, Sender};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
    QueueableCommand, Result as CrosstermResult,
};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Direction, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame, Terminal,
};
use tui_layout::{
    component::{simple::ComponentWidgetSimple, Component, ComponentBaseWidget},
    container::{list::ContainerList, search::ContainerSearch, Container},
    ResizeError,
};
use waveform_db::Waveform;

use crate::state::netlist_viewer::NetlistViewerState;
use crate::state::signal_viewer::SignalViewerState;
use crate::state::waveform_viewer::WaveformViewerState;
use crate::state::*;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct NaluArgs {
    vcd_file: String,
}

fn spawn_input_listener(tx: Sender<CrosstermEvent>) {
    thread::spawn(move || loop {
        if event::poll(time::Duration::from_millis(100)).unwrap() {
            tx.send(event::read().unwrap()).unwrap();
        }
    });
}

fn get_tui() -> Result<Box<dyn Container>, ResizeError> {
    let mut netlist_main =
        ContainerList::new("netlist_main".to_string(), Direction::Vertical, false, 0, 0);
    netlist_main.add_component(Component::new(
        "netlist".to_string(),
        1,
        Box::new(NetlistViewerState::new()),
    ))?;
    let mut netlist_filter = Component::new(
        "filter".to_string(),
        1,
        Box::new(ComponentWidgetSimple::new().text(format!("TODO: Filter"))),
    );
    netlist_filter.set_fixed_height(Some(3));
    netlist_main.add_component(netlist_filter)?;

    let mut main = ContainerList::new("main".to_string(), Direction::Horizontal, true, 0, 0);
    main.add_container(Box::new(netlist_main))?;
    main.add_component(Component::new(
        "signal".to_string(),
        1,
        Box::new(SignalViewerState::new()),
    ))?;
    main.add_component(Component::new(
        "waveform".to_string(),
        1,
        Box::new(WaveformViewerState::new(Arc::new(Waveform::new()))),
    ))?;

    let mut nalu = ContainerList::new("nalu".to_string(), Direction::Vertical, false, 0, 0);
    let mut header = Component::new(
        "header".to_string(),
        0,
        Box::new(
            ComponentWidgetSimple::new()
                .text(format!(
                    "nalu v{} (Press h for help, p for palette, r to reload, q to quit)",
                    option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0")
                ))
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Left),
        ),
    );
    header.set_fixed_height(Some(1));
    nalu.add_component(header)?;
    nalu.add_container(Box::new(main))?;

    Ok(Box::new(nalu))
}

fn get_overlay_rect(frame_rect: Rect, overlay_height: u16) -> Rect {
    let (y, height) = if frame_rect.height <= overlay_height {
        (0, frame_rect.height)
    } else {
        ((frame_rect.height - overlay_height) / 2, overlay_height)
    };
    let (x, width) = if frame_rect.width <= 4 {
        (0, frame_rect.width)
    } else {
        (1, frame_rect.width - 2)
    };
    Rect::new(x, y, width, height)
}

fn render_overlay_layout(
    frame: &mut Frame<CrosstermBackend<std::io::Stdout>>,
    nalu_state: &NaluState,
) {
    match &nalu_state.get_overlay() {
        NaluOverlay::Loading => frame.render_widget(
            Gauge::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .border_type(BorderType::Rounded)
                        .title("Loading"),
                )
                .gauge_style(Style::default().fg(Color::LightCyan))
                .percent(nalu_state.get_percent() as u16),
            get_overlay_rect(frame.size(), 3),
        ),
        NaluOverlay::HelpPrompt => frame.render_widget(
            Paragraph::new("<Insert Help Messages>")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .border_type(BorderType::Rounded)
                        .title("Help"),
                )
                .style(Style::default().fg(Color::LightCyan)),
            get_overlay_rect(frame.size(), 10),
        ),
        NaluOverlay::QuitPrompt => frame.render_widget(
            Paragraph::new("Press q to quit, esc to not...")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .border_type(BorderType::Rounded)
                        .title("Quit?"),
                )
                .style(Style::default().fg(Color::LightCyan)),
            get_overlay_rect(frame.size(), 3),
        ),
        NaluOverlay::Palette => frame.render_widget(
            Paragraph::new(nalu_state.get_palette())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .border_type(BorderType::Rounded)
                        .title("Palette"),
                )
                .style(Style::default().fg(Color::LightCyan)),
            get_overlay_rect(frame.size(), 10),
        ),
        NaluOverlay::None => {}
    }
}

fn setup_terminal() -> CrosstermResult<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.backend_mut().queue(EnableMouseCapture)?;
    terminal.backend_mut().queue(EnterAlternateScreen)?;
    terminal.backend_mut().flush()?;
    terminal.clear()?;
    Ok(terminal)
}

fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> CrosstermResult<()> {
    terminal.backend_mut().queue(DisableMouseCapture)?;
    terminal.backend_mut().queue(LeaveAlternateScreen)?;
    terminal.backend_mut().flush()?;
    disable_raw_mode()?;
    terminal.show_cursor()?;
    Ok(())
}

fn cleanup_terminal_force() -> CrosstermResult<()> {
    cleanup_terminal(&mut Terminal::new(CrosstermBackend::new(stdout()))?)
}

#[derive(Debug)]
pub struct FrameDuration {
    start: Instant,
    sections: Vec<(String, Duration)>,
}

impl FrameDuration {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            sections: Vec::new(),
        }
    }

    pub fn timestamp(&mut self, name: String) {
        let duration = self.start.elapsed();
        self.sections.push((name, duration));
        self.start = Instant::now();
    }

    pub fn total(&self) -> Duration {
        let mut total = Duration::new(0, 0);
        for (_, d) in &self.sections {
            total += d.clone();
        }
        total
    }
}

fn nalu_main(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> CrosstermResult<String> {
    use std::collections::VecDeque;

    // Setup event listeners
    let args = NaluArgs::parse();
    let mut tui = get_tui().unwrap();
    let mut nalu_state = NaluState::new(PathBuf::from(args.vcd_file.clone()));
    let (tx_input, rx_input) = unbounded();
    spawn_input_listener(tx_input);

    let mut durations: VecDeque<FrameDuration> = VecDeque::new();

    loop {
        let mut frame_duration = FrameDuration::new();
        let frame_start = Instant::now();

        terminal.draw(|frame| {
            tui.as_base_mut().invalidate();
            if let Err(err) = tui
                .as_base_mut()
                .resize(frame.size().width, frame.size().height)
            {
                panic!("Resizing Error! ({err:?})");
            }
            frame.render_stateful_widget(
                ComponentBaseWidget::from(tui.as_base_mut()),
                frame.size(),
                &mut (),
            );
            render_overlay_layout(frame, &nalu_state);
        })?;
        frame_duration.timestamp(String::from("draw"));

        while !rx_input.is_empty() {
            match rx_input.recv().unwrap() {
                CrosstermEvent::Key(key) => {
                    if let Some(key) = nalu_state.handle_key(key) {
                        tui.as_base_mut().handle_key(key);
                    }
                }
                CrosstermEvent::Mouse(event) => {
                    if let Some((x, y, kind)) =
                        nalu_state.handle_mouse(event.column, event.row, event.kind)
                    {
                        tui.as_base_mut().handle_mouse(x, y, Some(kind));
                    }
                }
                CrosstermEvent::Resize(_, _)
                | CrosstermEvent::FocusGained
                | CrosstermEvent::FocusLost
                | CrosstermEvent::Paste(_) => {}
            }
        }
        frame_duration.timestamp(String::from("input"));

        // Handle requests between components
        let netlist_viewer_requests = tui
            .as_container_mut()
            .search_name_widget_mut::<NetlistViewerState>("main.netlist_main.netlist")
            .unwrap()
            .get_requests();
        let signal_viewer_requests = tui
            .as_container_mut()
            .search_name_widget_mut::<SignalViewerState>("main.signal")
            .unwrap()
            .get_requests();
        let waveform_viewer_requests = tui
            .as_container_mut()
            .search_name_widget_mut::<WaveformViewerState>("main.waveform")
            .unwrap()
            .get_requests();

        for request in waveform_viewer_requests {
            tui.as_container_mut()
                .search_name_widget_mut::<SignalViewerState>("main.signal")
                .unwrap()
                .handle_key_press(request.0);
        }
        tui.as_container_mut()
            .search_name_widget_mut::<SignalViewerState>("main.signal")
            .unwrap()
            .browser_request(netlist_viewer_requests);
        tui.as_container_mut()
            .search_name_widget_mut::<WaveformViewerState>("main.waveform")
            .unwrap()
            .signal_request(signal_viewer_requests);
        frame_duration.timestamp(String::from("requests"));

        nalu_state.handle_vcd(&mut tui);
        frame_duration.timestamp(String::from("vcd"));

        if let Some(msg) = nalu_state.get_done() {
            cleanup_terminal(terminal)?;
            for d in durations {
                println!("{:?}, (Total: {:?})", d.sections, d.total());
            }
            return Ok(msg);
        }
        frame_duration.timestamp(String::from("check"));

        // Sleep for unused frame time
        let frame_target = Duration::from_millis(20);
        let frame_elapsed = frame_start.elapsed();
        if frame_elapsed < frame_target {
            thread::sleep(frame_target - frame_start.elapsed());
        }
        frame_duration.timestamp(String::from("sleep"));

        if durations.len() >= 2 {
            durations.pop_front();
        }
        durations.push_back(frame_duration);
    }
}

// Dark magic to capture backtraces from nalu_main, cleanup the terminal state,
// and then print the backtrace on the normal terminal
use backtrace::Backtrace;
use std::cell::RefCell;

thread_local! {
    static BACKTRACE: RefCell<Option<Backtrace>> = RefCell::new(None);
}

fn main() -> CrosstermResult<()> {
    if !stdout().is_tty() {
        println!("Error: Cannot open viewer when not TTY!");
        return Ok(());
    }

    std::panic::set_hook(Box::new(|_| {
        let trace = Backtrace::new();
        BACKTRACE.with(move |b| b.borrow_mut().replace(trace));
    }));

    // Catch any panics and try to cleanup the terminal first
    match std::panic::catch_unwind(|| nalu_main(&mut setup_terminal().unwrap()).unwrap()) {
        Ok(msg) => println!("{}", msg),
        Err(e) => {
            cleanup_terminal_force()?;
            let backtrace = BACKTRACE.with(|b| b.borrow_mut().take()).unwrap();
            println!("Error:\n{:?}\n{:?}", e, backtrace);
        }
    }

    Ok(())
}
