pub mod logging;
pub mod python;
pub mod state;
pub mod widgets;

use std::io::{stdout, Stdout, Write};
use std::path::PathBuf;
use std::thread;
use std::time::{self, Duration};

use clap::Parser;
use crossbeam::channel::{unbounded, Sender};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
    QueueableCommand, Result as CrosstermResult,
};
use makai::utils::messages::Messages;
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Direction, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame, Terminal,
};
use tui_tiling::{
    component::{simple::ComponentWidgetSimple, Component, ComponentBaseWidget},
    container::{list::ContainerList, Container, ContainerChild},
    ResizeError,
};

use crate::{
    logging::FrameTimestamps,
    state::netlist_viewer::NetlistViewerState,
    state::signal_viewer::SignalViewerState,
    state::waveform_viewer::WaveformViewerState,
    state::{NaluOverlay, NaluState},
};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct NaluArgs {
    /// VCD file that will be loaded
    vcd_file: String,
    #[clap(long)]
    /// Optional python program that can be run
    python: Option<String>,
}

fn spawn_input_listener(tx: Sender<CrosstermEvent>) {
    thread::spawn(move || loop {
        if event::poll(time::Duration::from_millis(100)).unwrap() {
            tx.send(event::read().unwrap()).unwrap();
        }
    });
}

fn get_tui(messages: &Messages) -> Result<Box<dyn Container>, ResizeError> {
    let netlist_main =
        ContainerList::new("netlist_main".to_string(), Direction::Vertical, false, 0, 0)
            .from_children(vec![
                ContainerChild::from(Component::new(
                    "netlist".to_string(),
                    1,
                    Box::new(NetlistViewerState::new(messages.clone())),
                )),
                ContainerChild::from(
                    Component::new(
                        "filter".to_string(),
                        1,
                        Box::new(ComponentWidgetSimple::new().text("TODO: Filter".to_string())),
                    )
                    .fixed_height(Some(3)),
                ),
            ])?;

    let main = ContainerList::new("main".to_string(), Direction::Horizontal, true, 0, 0)
        .from_children(vec![
            ContainerChild::from(netlist_main),
            ContainerChild::from(Component::new(
                "signal".to_string(),
                1,
                Box::new(SignalViewerState::new(messages.clone())),
            )),
            ContainerChild::from(Component::new(
                "waveform".to_string(),
                1,
                Box::new(WaveformViewerState::new(messages.clone())),
            )),
        ])?;

    let nalu = ContainerList::new("nalu".to_string(), Direction::Vertical, false, 0, 0)
        .from_children(vec![
            ContainerChild::from(
                Component::new(
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
                )
                .fixed_height(Some(1)),
            ),
            ContainerChild::from(main),
        ])?;

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

fn nalu_main(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> CrosstermResult<String> {
    let args = NaluArgs::parse();

    let mut nalu_state = NaluState::new(
        PathBuf::from(args.vcd_file.clone()),
        args.python.map(PathBuf::from),
    );
    let mut tui = get_tui(nalu_state.get_messages()).unwrap();
    nalu_state.handle_load();

    // Setup event listeners
    let (tx_input, rx_input) = unbounded();
    spawn_input_listener(tx_input);

    loop {
        let mut frame_duration = FrameTimestamps::new();

        terminal.draw(|frame| {
            tui.as_base_mut().invalidate();
            if let Err(err) = tui
                .as_base_mut()
                .resize(frame.size().width, frame.size().height)
            {
                log::error!("Resizing Error! ({err:?})");
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

        // Wait while there is no input events and no message events
        while rx_input.is_empty() && nalu_state.get_messages().is_empty() {
            thread::sleep(Duration::from_millis(10));
        }
        frame_duration.timestamp(String::from("sleep"));

        // Handle input events (if any)
        while !rx_input.is_empty() {
            if let Some(event) = nalu_state.handle_input(rx_input.recv().unwrap()) {
                tui.as_base_mut().handle_input(event);
            }
        }
        frame_duration.timestamp(String::from("input"));

        // Handle message events
        while !nalu_state.get_messages().is_empty() {
            nalu_state.handle_update();
            tui.as_base_mut().handle_update();
        }
        if let Some(msg) = nalu_state.get_done() {
            cleanup_terminal(terminal)?;
            return Ok(msg);
        }
        frame_duration.timestamp(String::from("updates"));

        log::trace!(
            "Frame: {:?}, (Total: {:?})",
            frame_duration.get_sections(),
            frame_duration.total()
        );
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
    // Parse args once to exit before setting up TUI if necessary
    let _ = NaluArgs::parse();

    if !stdout().is_tty() {
        println!("Error: Cannot open viewer when not TTY!");
        return Ok(());
    }

    simple_logging::log_to_file(".nalu.log", log::LevelFilter::Info)?;
    log::info!("Starting Nalu...");

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
