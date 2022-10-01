mod component;
mod resize;
mod state;
mod vcd;
mod view;

use std::io::{stdout, Stdout, Write};
use std::path::PathBuf;
use std::thread;
use std::time;

use clap::Parser;
use crossbeam::channel::{unbounded, Sender};

use crate::state::*;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
    QueueableCommand, Result,
};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Gauge, Paragraph},
    Frame, Terminal,
};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct NaluArgs {
    vcd_file: String,
}

fn get_block<'a>(focus: Option<NaluFocusType>, title: Option<&'a str>) -> Block<'a> {
    let color = match focus {
        Some(NaluFocusType::Full) => Color::Green,
        Some(NaluFocusType::Partial) => Color::Yellow,
        None => Color::White,
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(color))
        .border_type(BorderType::Rounded);
    if let Some(title) = title {
        block.title(title)
    } else {
        block
    }
}

fn spawn_input_listener(tx: Sender<CrosstermEvent>) {
    thread::spawn(move || loop {
        if event::poll(time::Duration::from_millis(100)).unwrap() {
            tx.send(event::read().unwrap()).unwrap();
        }
    });
}

fn get_main_layout() -> Layout {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([Constraint::Length(1), Constraint::Min(2)].as_ref())
}

fn get_waveform_layout() -> Layout {
    Layout::default().direction(Direction::Horizontal).margin(0)
}

fn get_browser_layout() -> Layout {
    Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([Constraint::Min(2), Constraint::Length(3)].as_ref())
}

fn render_main_layout(
    frame: &mut Frame<CrosstermBackend<std::io::Stdout>>,
    nalu_state: &NaluState,
    header_rect: Rect,
    browser_rect: Rect,
    filter_rect: Rect,
    list_rect: Rect,
    viewer_rect: Rect,
) {
    let header = Paragraph::new(format!(
        "nalu v{} (Press h for help, p for palette, r to reload, q to quit)",
        option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0")
    ))
    .style(Style::default().fg(Color::LightCyan))
    .alignment(Alignment::Left);

    let waveform_browser = Paragraph::new(nalu_state.get_browser_state().render())
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Left)
        .block(get_block(
            nalu_state.get_focus(NaluPanes::Browser),
            Some("Browser"),
        ));

    let waveform_list = Paragraph::new(nalu_state.get_waveform_state().render_list())
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Left)
        .block(get_block(
            nalu_state.get_focus(NaluPanes::List),
            Some("List"),
        ));

    let waveform_viewer = Paragraph::new(nalu_state.get_waveform_state().render_waveform())
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Left)
        .block(get_block(
            nalu_state.get_focus(NaluPanes::Viewer),
            Some("Viewer"),
        ));

    let filter = Paragraph::new(nalu_state.get_filter())
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Left)
        .block(get_block(
            nalu_state.get_focus(NaluPanes::Filter),
            Some("Filter"),
        ));

    frame.render_widget(header, header_rect);
    frame.render_widget(filter, filter_rect);
    frame.render_widget(waveform_browser, browser_rect);
    frame.render_widget(waveform_list, list_rect);
    frame.render_widget(waveform_viewer, viewer_rect);
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
                .block(get_block(None, Some("Loading")))
                .gauge_style(Style::default().fg(Color::LightCyan))
                .percent(nalu_state.get_percent() as u16),
            get_overlay_rect(frame.size(), 3),
        ),
        NaluOverlay::HelpPrompt => frame.render_widget(
            Paragraph::new("<Insert Help Messages>")
                .block(get_block(None, Some("Help")))
                .style(Style::default().fg(Color::LightCyan)),
            get_overlay_rect(frame.size(), 10),
        ),
        NaluOverlay::QuitPrompt => frame.render_widget(
            Paragraph::new("Press q to quit, esc to not...")
                .block(get_block(None, Some("Quit?")))
                .style(Style::default().fg(Color::LightCyan)),
            get_overlay_rect(frame.size(), 3),
        ),
        NaluOverlay::Palette => frame.render_widget(
            Paragraph::new(nalu_state.get_palette())
                .block(get_block(None, Some("Palette")))
                .style(Style::default().fg(Color::LightCyan)),
            get_overlay_rect(frame.size(), 10),
        ),
        NaluOverlay::None => {}
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode().unwrap();
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    terminal.backend_mut().queue(EnableMouseCapture)?;
    terminal.backend_mut().queue(EnterAlternateScreen)?;
    terminal.backend_mut().flush()?;
    terminal.clear()?;
    Ok(terminal)
}

fn cleanup_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>, msg: String) -> Result<()> {
    terminal.backend_mut().queue(DisableMouseCapture)?;
    terminal.backend_mut().queue(LeaveAlternateScreen)?;
    terminal.backend_mut().flush()?;
    disable_raw_mode()?;
    terminal.show_cursor()?;
    println!("{}", msg);
    Ok(())
}

// h for help menu, q to quit, esc to main menu
fn main() -> Result<()> {
    if !stdout().is_tty() {
        println!("Error: Cannot open viewer when not TTY!");
        return Ok(());
    }

    // Setup event listeners
    let args = NaluArgs::parse();
    let (tx_input, rx_input) = unbounded();
    spawn_input_listener(tx_input);

    // Setup terminal for the TUI
    let mut terminal = setup_terminal()?;

    let mut nalu_state = NaluState::new(PathBuf::from(args.vcd_file.clone()));

    loop {
        let frame_start = std::time::Instant::now();

        let mut browser_rect = Rect::new(0, 0, 0, 0);
        let mut list_rect = Rect::new(0, 0, 0, 0);
        let mut viewer_rect = Rect::new(0, 0, 0, 0);
        let mut filter_rect = Rect::new(0, 0, 0, 0);

        terminal.draw(|frame| {
            // Resolve layout
            let main_rects = get_main_layout().split(frame.size());
            nalu_state
                .get_resize_mut()
                .resize_container(main_rects[1].width);
            let waveform_layout = nalu_state
                .get_resize()
                .constrain_layout(get_waveform_layout());
            let waveform_rects = waveform_layout.split(main_rects[1]);
            let browser_filter_rects = get_browser_layout().split(waveform_rects[0]);

            browser_rect = browser_filter_rects[0];
            filter_rect = browser_filter_rects[1];
            list_rect = waveform_rects[1];
            viewer_rect = waveform_rects[2];

            render_main_layout(
                frame,
                &nalu_state,
                main_rects[0],
                browser_rect,
                filter_rect,
                list_rect,
                viewer_rect,
            );
            render_overlay_layout(frame, &nalu_state);
        })?;

        nalu_state.handle_vcd();
        nalu_state
            .get_browser_state_mut()
            .set_size(&browser_rect, 1);
        nalu_state
            .get_waveform_state_mut()
            .set_size(&list_rect, &viewer_rect, 1);

        while !rx_input.is_empty() {
            match rx_input.recv().unwrap() {
                CrosstermEvent::Key(key) => nalu_state.handle_key(key),
                CrosstermEvent::Mouse(event) => nalu_state.handle_mouse(
                    event.column,
                    event.row,
                    event.kind,
                    NaluSizing::new(browser_rect, list_rect, viewer_rect, filter_rect),
                ),
                CrosstermEvent::Resize(_, _) => {}
            }
        }

        if let Some(msg) = nalu_state.get_done() {
            cleanup_terminal(&mut terminal, msg)?;
            break;
        }

        // Sleep for unused frame time
        let frame_target = std::time::Duration::from_millis(20);
        let frame_elapsed = frame_start.elapsed();
        if frame_elapsed < frame_target {
            thread::sleep(frame_target - frame_start.elapsed());
        }
    }

    Ok(())
}
