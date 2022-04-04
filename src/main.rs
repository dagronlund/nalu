use std::io::{stdout, Write};
use std::sync::mpsc;
use std::thread;
use std::time;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent, KeyCode,
        MouseButton, MouseEvent, MouseEventKind,
    },
    style::{Attribute, Stylize},
    terminal::{
        self, disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
    },
    tty::IsTty,
    ExecutableCommand, QueueableCommand, Result,
};
use tui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Terminal,
};

enum BorderSelection {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    BottomLeft,
    TopRight,
    BottomRight,
}

fn is_border_selection(rect: &Rect, margin: u16, x: u16, y: u16) -> Option<BorderSelection> {
    let left_right = if x >= rect.left() && x < (rect.left() + margin) {
        -1
    } else if x >= (rect.right() - margin) && x < rect.right() {
        1
    } else {
        0
    };
    let top_bottom = if y >= rect.top() && y < (rect.top() + margin) {
        -1
    } else if y >= (rect.bottom() - margin) && y < rect.bottom() {
        1
    } else {
        0
    };
    match (left_right, top_bottom) {
        (-1, -1) => Some(BorderSelection::TopLeft),
        (-1, 1) => Some(BorderSelection::BottomLeft),
        (1, -1) => Some(BorderSelection::TopRight),
        (1, 1) => Some(BorderSelection::BottomRight),
        (-1, _) => Some(BorderSelection::Left),
        (1, _) => Some(BorderSelection::Right),
        (_, -1) => Some(BorderSelection::Top),
        (_, 1) => Some(BorderSelection::Bottom),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
enum NaluState {
    MainWindow,
    HelpWindow,
    QuitWindow,
    Quit,
}

impl NaluState {
    fn next_from_key(self, key: KeyCode) -> Self {
        match self {
            Self::MainWindow => match key {
                KeyCode::Char('q') => Self::Quit,
                KeyCode::Char('h') => Self::HelpWindow,
                KeyCode::Esc => Self::QuitWindow,
                _ => Self::MainWindow,
            },
            Self::HelpWindow => match key {
                KeyCode::Char('q') => Self::Quit,
                KeyCode::Esc => Self::MainWindow,
                _ => Self::HelpWindow,
            },
            Self::QuitWindow => match key {
                KeyCode::Enter | KeyCode::Char('q') => Self::Quit,
                KeyCode::Esc => Self::MainWindow,
                _ => Self::QuitWindow,
            },
            _ => self,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum NaluFocus {
    Browser,
    List,
    Viewer,
    Filter,
    BrowserPartial,
    ListPartial,
    ViewerPartial,
    FilterPartial,
    None,
}

impl NaluFocus {
    fn next_from_mouse(
        self,
        browser_clicked: bool,
        list_clicked: bool,
        viewer_clicked: bool,
        filter_clicked: bool,
    ) -> Self {
        if browser_clicked {
            Self::Browser
        } else if list_clicked {
            Self::List
        } else if viewer_clicked {
            Self::Viewer
        } else if filter_clicked {
            Self::Filter
        } else {
            Self::None
        }
    }

    fn next_from_key(self, key: KeyCode) -> Self {
        match self {
            Self::Browser => match key {
                KeyCode::Esc => Self::BrowserPartial,
                _ => self,
            },
            Self::List => match key {
                KeyCode::Esc => Self::ListPartial,
                _ => self,
            },
            Self::Viewer => match key {
                KeyCode::Esc => Self::ViewerPartial,
                _ => self,
            },
            Self::Filter => match key {
                KeyCode::Esc => Self::FilterPartial,
                _ => self,
            },
            Self::BrowserPartial => match key {
                KeyCode::Enter => Self::Browser,
                KeyCode::Esc => Self::None,
                KeyCode::Right => Self::ListPartial,
                KeyCode::Down => Self::FilterPartial,
                _ => self,
            },
            Self::ListPartial => match key {
                KeyCode::Enter => Self::List,
                KeyCode::Esc => Self::None,
                KeyCode::Left => Self::BrowserPartial,
                KeyCode::Right => Self::ViewerPartial,
                KeyCode::Down => Self::FilterPartial,
                _ => self,
            },
            Self::ViewerPartial => match key {
                KeyCode::Enter => Self::Viewer,
                KeyCode::Esc => Self::None,
                KeyCode::Left => Self::ListPartial,
                KeyCode::Down => Self::FilterPartial,
                _ => self,
            },
            Self::FilterPartial => match key {
                KeyCode::Enter => Self::Filter,
                KeyCode::Esc => Self::None,
                KeyCode::Up => Self::BrowserPartial,
                _ => self,
            },
            Self::None => match key {
                KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down | KeyCode::Enter => {
                    Self::BrowserPartial
                }
                _ => Self::None,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum NaluDrag {
    BrowserRight,
    ListLeft,
    ListRight,
    ViewerLeft,
    None,
}

// h for help menu, q to quit, esc to main menu
fn main() -> Result<()> {
    enable_raw_mode().expect("can run in raw mode");

    let mut stdout = stdout();

    let (tx, rx) = mpsc::channel();
    // let tick_rate = Duration::from_millis(200);
    thread::spawn(move || {
        // let mut last_tick = Instant::now();
        loop {
            // let timeout = tick_rate
            //     .checked_sub(last_tick.elapsed())
            //     .unwrap_or_else(|| Duration::from_secs(0));

            if event::poll(time::Duration::from_millis(100)).expect("poll works") {
                tx.send(event::read().expect("can read events"))
                    .expect("can send events");

                // if let CrosstermEvent::Key(key) = event::read().expect("can read events") {
                // }
            }

            // if last_tick.elapsed() >= tick_rate {
            //     if let Ok(_) = tx.send(Event::Tick) {
            //         last_tick = Instant::now();
            //     }
            // }

            // thread::sleep(std::time::Duration::from_millis(10));
        }
    });

    // for y in 0..40 {
    //     for x in 0..150 {
    //         if (y == 0 || y == 40 - 1) || (x == 0 || x == 150 - 1) {
    //             // in this loop we are more efficient by not flushing the buffer.
    //             stdout
    //                 .queue(cursor::MoveTo(x, y))?
    //                 .queue(style::PrintStyledContent("█".magenta()))?;
    //         }
    //     }
    // }
    // stdout.flush()?;

    // let (cols, rows) = size()?;
    // // Resize terminal and scroll up.
    // execute!(stdout, SetSize(10, 10), ScrollUp(5))?;

    // // Be a good citizen, cleanup
    // execute!(stdout, SetSize(cols, rows))?;

    stdout.execute(EnableMouseCapture);
    stdout.execute(EnterAlternateScreen);
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let block_default = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .border_type(BorderType::Rounded);

    let block_partial_focus = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Yellow))
        .border_type(BorderType::Rounded);

    let block_focus = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Red))
        .border_type(BorderType::Rounded);

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Min(2),
                Constraint::Length(3),
            ]
            .as_ref(),
        );

    let waveform_layout = Layout::default()
        .direction(Direction::Horizontal)
        .margin(0)
        .constraints(
            [
                Constraint::Ratio(1, 5),
                Constraint::Ratio(1, 5),
                Constraint::Ratio(3, 5),
            ]
            .as_ref(),
        );

    let mut coord = (0, 0);
    let mut mouse_event = None;
    let mut key_event = None;
    let mut nalu_state = NaluState::MainWindow;
    let mut nalu_focus = NaluFocus::None;
    let mut nalu_drag = NaluDrag::None;
    let mut sizing_ratio_browser = (1, 5);
    let mut sizing_ratio_list = (1, 5);
    loop {
        terminal.draw(|rect| {
            let main_chunks = main_layout.split(rect.size());
            // Work out what the size of the waveform window should be
            let window_width = main_chunks[1].width;
            sizing_ratio_browser = (
                sizing_ratio_browser.0 * window_width / sizing_ratio_browser.1,
                window_width,
            );
            sizing_ratio_list = (
                sizing_ratio_list.0 * window_width / sizing_ratio_list.1,
                window_width,
            );
            let viewer_width = window_width - sizing_ratio_browser.0 - sizing_ratio_list.0;
            let sizing_ratio_viewer = (viewer_width, window_width);
            let waveform_layout = waveform_layout.clone().constraints(
                [
                    Constraint::Length(sizing_ratio_browser.0),
                    Constraint::Length(sizing_ratio_list.0),
                    Constraint::Length(viewer_width),
                ]
                .as_ref(),
            );

            let waveform_chunks = waveform_layout.split(main_chunks[1]);

            if let Some((x, y, mouse_kind)) = mouse_event {
                match mouse_kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        coord = (x, y);
                        let r = Rect::new(x, y, 1, 1);
                        nalu_focus = nalu_focus.clone().next_from_mouse(
                            waveform_chunks[0].intersects(r),
                            waveform_chunks[1].intersects(r),
                            waveform_chunks[2].intersects(r),
                            main_chunks[2].intersects(r),
                        );
                        if waveform_chunks[0].intersects(r) {
                            if waveform_chunks[0].right() - 1 == x {
                                nalu_drag = NaluDrag::BrowserRight;
                            }
                        }
                        if waveform_chunks[1].intersects(r) {
                            if waveform_chunks[1].left() == x {
                                nalu_drag = NaluDrag::ListLeft;
                            }
                            if waveform_chunks[1].right() - 1 == x {
                                nalu_drag = NaluDrag::ListRight;
                            }
                        }
                        if waveform_chunks[2].intersects(r) {
                            if waveform_chunks[2].left() == x {
                                nalu_drag = NaluDrag::ViewerLeft;
                            }
                        }
                    }
                    MouseEventKind::Drag(MouseButton::Left) => {
                        //
                        match nalu_drag.clone() {
                            NaluDrag::BrowserRight => {
                                let border = waveform_chunks[0].right() - 1;
                                let dragged = x as i16 - border as i16;
                                if (sizing_ratio_browser.0 as i16 + dragged > 2)
                                    && (sizing_ratio_list.0 as i16 - dragged > 2)
                                {
                                    sizing_ratio_browser.0 =
                                        (sizing_ratio_browser.0 as i16 + dragged) as u16;
                                    sizing_ratio_list.0 =
                                        (sizing_ratio_list.0 as i16 - dragged) as u16
                                }
                            }
                            NaluDrag::ListLeft => {
                                let border = waveform_chunks[1].left();
                                let dragged = x as i16 - border as i16;
                                if (sizing_ratio_browser.0 as i16 + dragged > 2)
                                    && (sizing_ratio_list.0 as i16 - dragged > 2)
                                {
                                    sizing_ratio_browser.0 =
                                        (sizing_ratio_browser.0 as i16 + dragged) as u16;
                                    sizing_ratio_list.0 =
                                        (sizing_ratio_list.0 as i16 - dragged) as u16
                                }
                            }
                            NaluDrag::ListRight => {
                                let border = waveform_chunks[1].right() - 1;
                                let dragged = x as i16 - border as i16;
                                if (sizing_ratio_list.0 as i16 + dragged > 2)
                                    && (sizing_ratio_viewer.0 as i16 - dragged > 2)
                                {
                                    sizing_ratio_list.0 =
                                        (sizing_ratio_list.0 as i16 + dragged) as u16;
                                }
                            }
                            NaluDrag::ViewerLeft => {
                                let border = waveform_chunks[2].left();
                                let dragged = x as i16 - border as i16;
                                if (sizing_ratio_list.0 as i16 + dragged > 2)
                                    && (sizing_ratio_viewer.0 as i16 - dragged > 2)
                                {
                                    sizing_ratio_list.0 =
                                        (sizing_ratio_list.0 as i16 + dragged) as u16;
                                }
                            }
                            _ => {}
                        }
                    }
                    _ => {
                        nalu_drag = NaluDrag::None;
                    }
                }
            }

            if let Some(key) = key_event {
                nalu_focus = nalu_focus.clone().next_from_key(key);
                nalu_drag = NaluDrag::None;
            }

            let mouse_rect = Rect::new(coord.0, coord.1, 1, 1);

            // let footer = Paragraph::new("nalu v0.1 (Press q for help)")
            //     .style(Style::default().fg(Color::LightCyan))
            //     .alignment(Alignment::Left);
            // .block(if main_chunks[2].intersects(mouse_rect.clone()) {
            //     block_highlighted.clone()
            // } else {
            //     block_default.clone()
            // });

            let mut text = Text::styled(
                "_One_|__",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::UNDERLINED),
            );

            let spans = Spans::from(vec![
                Span::styled("      ", Style::default().bg(Color::Indexed(245))),
                Span::styled(
                    "|  |    ",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::UNDERLINED),
                ),
                Span::styled(
                    "X      ",
                    Style::default().fg(Color::Black).bg(Color::Indexed(245)),
                ),
                Span::styled("      ", Style::default().bg(Color::Indexed(88))),
                Span::styled("      ", Style::default().bg(Color::Indexed(21))),
            ]);
            text.extend(Text::from(spans));

            // let spans = Spans::from(vec![
            //     Span::styled("         ", Style::default().bg(Color::White)),
            //     Span::styled(
            //         "|      |",
            //         Style::default()
            //             .fg(Color::White)
            //             .add_modifier(Modifier::UNDERLINED),
            //     ),
            //     Span::styled("    ", Style::default().bg(Color::White)),
            // ]);
            // text.extend(Text::from(spans));

            // let spans = Spans::from(vec![
            //     Span::styled("       ", Style::default().bg(Color::White)),
            //     Span::styled(
            //         "\\      /",
            //         Style::default()
            //             .fg(Color::White)
            //             .add_modifier(Modifier::UNDERLINED),
            //     ),
            //     Span::styled("      ", Style::default().bg(Color::White)),
            // ]);
            // text.extend(Text::from(spans));

            text.extend(Text::styled("Two\n", Style::default().fg(Color::White)));
            text.extend(Text::from(Span::styled("Three", Style::default())));

            let header =
                Paragraph::new(format!("nalu v0.1 (Press q for help, Ctrl+p for palette)",))
                    .style(Style::default().fg(Color::LightCyan))
                    .alignment(Alignment::Left);

            let waveform_browser = Paragraph::new("Waveform Browser")
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Left)
                .block(if nalu_focus.clone() == NaluFocus::Browser {
                    block_focus.clone()
                } else if nalu_focus.clone() == NaluFocus::BrowserPartial {
                    block_partial_focus.clone()
                } else {
                    block_default.clone()
                });

            let waveform_list = Paragraph::new("Waveform List")
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Left)
                .block(if nalu_focus.clone() == NaluFocus::List {
                    block_focus.clone()
                } else if nalu_focus.clone() == NaluFocus::ListPartial {
                    block_partial_focus.clone()
                } else {
                    block_default.clone()
                });

            let waveform_viewer = Paragraph::new(text)
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Left)
                .block(if nalu_focus.clone() == NaluFocus::Viewer {
                    block_focus.clone()
                } else if nalu_focus.clone() == NaluFocus::ViewerPartial {
                    block_partial_focus.clone()
                } else {
                    block_default.clone()
                });

            let filter = Paragraph::new("Filter: ")
                .style(Style::default().fg(Color::LightCyan))
                .alignment(Alignment::Left)
                .block(if nalu_focus.clone() == NaluFocus::Filter {
                    block_focus.clone()
                } else if nalu_focus.clone() == NaluFocus::FilterPartial {
                    block_partial_focus.clone()
                } else {
                    block_default.clone()
                });

            rect.render_widget(header, main_chunks[0]);
            rect.render_widget(waveform_browser, waveform_chunks[0]);
            rect.render_widget(waveform_list, waveform_chunks[1]);
            rect.render_widget(waveform_viewer, waveform_chunks[2]);
            rect.render_widget(filter, main_chunks[2]);
            // rect.render_widget(footer, main_chunks[3]);
            // let mouse_test = Paragraph::new(if let Some((x, y)) = coord {
            //     format!("{}", format!("Clicked: {}x{}", x, y))
            // } else {
            //     format!("")
            // })
            // .style(
            //     Style::default().fg(Color::LightCyan), // .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            // )
            // .alignment(Alignment::Left)
            // .block(block_default.clone());

            // let menu = menu_titles
            //     .iter()
            //     .map(|t| {
            //         let (first, rest) = t.split_at(1);
            //         Spans::from(vec![
            //             Span::styled(
            //                 first,
            //                 Style::default()
            //                     .fg(Color::Yellow)
            //                     .add_modifier(Modifier::UNDERLINED),
            //             ),
            //             Span::styled(rest, Style::default().fg(Color::White)),
            //         ])
            //     })
            //     .collect();

            // let tabs = Tabs::new(menu)
            //     .select(active_menu_item.into())
            //     .block(Block::default().title("Menu").borders(Borders::ALL))
            //     .style(Style::default().fg(Color::White))
            //     .highlight_style(Style::default().fg(Color::Yellow))
            //     .divider(Span::raw("|"));

            // rect.render_widget(tabs, main_chunks[0]);
            // rect.render_stateful_widget(left, pets_main_chunks[0], &mut pet_list_state);
            // rect.render_widget(right, pets_main_chunks[1]);
        })?;

        match rx.recv() {
            Ok(event) => {
                key_event = None;
                mouse_event = None;
                match event {
                    CrosstermEvent::Key(key) => {
                        if key.code == KeyCode::Char('q') {
                            break;
                        }
                        key_event = Some(key.code);
                    }
                    CrosstermEvent::Mouse(event) => {
                        mouse_event = Some((event.column, event.row, event.kind));
                    }
                    CrosstermEvent::Resize(_, _) => {}
                }
            }
            _ => {}
        }

        // thread::sleep(std::time::Duration::from_millis(100));

        // match rx.recv()? {
        //     Event::Input(event) => match event.code {
        //         KeyCode::Char('q') => {
        //             disable_raw_mode()?;
        //             terminal.show_cursor()?;
        //             break;
        //         }

        //         _ => {}
        //     },
        //     Event::Tick => {}
        // }
    }

    terminal.backend_mut().queue(DisableMouseCapture)?;
    terminal.backend_mut().queue(LeaveAlternateScreen)?;
    terminal.backend_mut().flush()?;
    disable_raw_mode()?;
    terminal.show_cursor()?;

    // let mut stdout = stdout();
    // println!("TTY: {}", stdout.is_tty());

    Ok(())
}
