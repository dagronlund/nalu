use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crossbeam::channel::{unbounded, Receiver};

use vcd_parser::parser::VcdHeader;
use vcd_parser::waveform::Waveform;

use crate::resize::LayoutResize;
use crate::vcd::*;

use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use tui::layout::Rect;

#[derive(Debug, Clone, PartialEq)]
pub enum NaluOverlay {
    Loading,
    Palette,
    HelpPrompt,
    QuitPrompt,
    None,
}

fn edit_string(key: KeyCode, string: &mut String) {
    match key {
        KeyCode::Backspace => {
            if string.len() > 0 {
                string.remove(string.len() - 1);
            }
        }
        KeyCode::Char(c @ ('a'..='z' | 'A'..='Z' | '0'..='9' | '_' | ' ')) => {
            string.push(c);
        }
        _ => {}
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NaluPanes {
    Browser,
    List,
    Viewer,
    Filter,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NaluFocusType {
    Partial,
    Full,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NaluFocus {
    Browser(NaluFocusType),
    List(NaluFocusType),
    Viewer(NaluFocusType),
    Filter(NaluFocusType),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NaluSizing {
    browser: Rect,
    list: Rect,
    viewer: Rect,
    filter: Rect,
}

impl NaluSizing {
    pub fn new(browser: Rect, list: Rect, viewer: Rect, filter: Rect) -> Self {
        Self {
            browser: browser,
            list: list,
            viewer: viewer,
            filter: filter,
        }
    }
}

pub struct NaluState {
    vcd_path: PathBuf,
    vcd_rx: Receiver<VcdResult<(VcdHeader, Waveform)>>,
    overlay: NaluOverlay,
    focus: NaluFocus,
    resizing: LayoutResize<3>,
    progress: Arc<Mutex<(usize, usize)>>,
    vcd_header: VcdHeader,
    waveform: Waveform,
    filter_input: String,
    palette_input: String,
    done: Option<String>,
}

impl NaluState {
    pub fn new(vcd_path: PathBuf) -> Self {
        // Load initial VCD file
        let progress = Arc::new(Mutex::new((0, 0)));
        let (tx_vcd, rx_vcd) = unbounded();
        load_vcd(vcd_path.clone(), tx_vcd, progress.clone());
        Self {
            vcd_path: vcd_path,
            vcd_rx: rx_vcd,
            overlay: NaluOverlay::Loading,
            focus: NaluFocus::None,
            resizing: LayoutResize::new([1, 1, 2], 2),
            progress: progress,
            vcd_header: VcdHeader::new(),
            waveform: Waveform::new(),
            filter_input: String::new(),
            palette_input: String::new(),
            done: None,
        }
    }

    fn handle_mouse_click(&mut self, x: u16, y: u16, sizing: NaluSizing) {
        let coord = Rect::new(x, y, 1, 1);
        // Handle resizing
        if sizing.browser.intersects(coord)
            || sizing.filter.intersects(coord)
            || sizing.list.intersects(coord)
            || sizing.viewer.intersects(coord)
        {
            self.resizing.handle_mouse_down(x, 1);
        } else {
            self.resizing.handle_mouse_done();
        }

        if sizing.browser.intersects(coord) {
            match self.focus {
                NaluFocus::Browser(NaluFocusType::Full) => {
                    // TODO: Handle passing mouse event to component
                }
                _ => self.focus = NaluFocus::Browser(NaluFocusType::Full),
            }
        }

        if sizing.list.intersects(coord) {
            match self.focus {
                NaluFocus::List(NaluFocusType::Full) => {
                    // TODO: Handle passing mouse event to component
                }
                _ => self.focus = NaluFocus::List(NaluFocusType::Full),
            }
        }

        if sizing.viewer.intersects(coord) {
            match self.focus {
                NaluFocus::Viewer(NaluFocusType::Full) => {
                    // TODO: Handle passing mouse event to component
                }
                _ => self.focus = NaluFocus::Viewer(NaluFocusType::Full),
            }
        }

        if sizing.filter.intersects(coord) {
            match self.focus {
                NaluFocus::Filter(NaluFocusType::Full) => {
                    // TODO: Handle passing mouse event to component
                }
                _ => self.focus = NaluFocus::Filter(NaluFocusType::Full),
            }
        }
    }

    fn handle_mouse_drag(&mut self, x: u16, y: u16, sizing: NaluSizing) {
        let coord = Rect::new(x, y, 1, 1);
        if sizing.browser.intersects(coord)
            || sizing.filter.intersects(coord)
            || sizing.list.intersects(coord)
            || sizing.viewer.intersects(coord)
        {
            self.resizing.handle_mouse_drag(x);
        } else {
            self.resizing.handle_mouse_done();
        }
    }

    fn handle_mouse_other(&mut self) {
        self.resizing.handle_mouse_done();
    }

    pub fn handle_mouse(&mut self, x: u16, y: u16, kind: MouseEventKind, sizing: NaluSizing) {
        match self.overlay {
            NaluOverlay::None => {}
            _ => return,
        }
        match kind {
            MouseEventKind::Down(MouseButton::Left) => self.handle_mouse_click(x, y, sizing),
            MouseEventKind::Drag(MouseButton::Left) => self.handle_mouse_drag(x, y, sizing),
            _ => self.handle_mouse_other(),
        }
    }

    fn handle_key_browser(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.focus = NaluFocus::Browser(NaluFocusType::Partial),
            _ => {
                // TODO: Handle passing key event to component
            }
        }
    }
    fn handle_key_filter(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.focus = NaluFocus::Filter(NaluFocusType::Partial),
            _ => {
                // TODO: Handle passing key event to component
            }
        }
    }
    fn handle_key_list(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.focus = NaluFocus::List(NaluFocusType::Partial),
            _ => {
                // TODO: Handle passing key event to component
            }
        }
    }
    fn handle_key_viewer(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => self.focus = NaluFocus::Viewer(NaluFocusType::Partial),
            _ => {
                // TODO: Handle passing key event to component
            }
        }
    }

    fn handle_key_non_overlay(&mut self, key: KeyCode) {
        match &self.focus {
            NaluFocus::Browser(focus_type) => {
                if let NaluFocusType::Full = focus_type {
                    self.handle_key_browser(key);
                } else {
                    match key {
                        KeyCode::Enter => self.focus = NaluFocus::Browser(NaluFocusType::Full),
                        KeyCode::Esc => self.overlay = NaluOverlay::QuitPrompt,
                        KeyCode::Down => self.focus = NaluFocus::Filter(NaluFocusType::Partial),
                        KeyCode::Right => self.focus = NaluFocus::List(NaluFocusType::Partial),
                        _ => {}
                    }
                }
            }
            NaluFocus::Filter(focus_type) => {
                if let NaluFocusType::Full = focus_type {
                    self.handle_key_filter(key);
                } else {
                    match key {
                        KeyCode::Enter => self.focus = NaluFocus::Filter(NaluFocusType::Full),
                        KeyCode::Esc => self.overlay = NaluOverlay::QuitPrompt,
                        KeyCode::Up => self.focus = NaluFocus::Browser(NaluFocusType::Partial),
                        KeyCode::Right => self.focus = NaluFocus::List(NaluFocusType::Partial),
                        _ => {}
                    }
                }
            }
            NaluFocus::List(focus_type) => {
                if let NaluFocusType::Full = focus_type {
                    self.handle_key_list(key);
                } else {
                    match key {
                        KeyCode::Enter => self.focus = NaluFocus::List(NaluFocusType::Full),
                        KeyCode::Esc => self.overlay = NaluOverlay::QuitPrompt,
                        KeyCode::Left => self.focus = NaluFocus::Browser(NaluFocusType::Partial),
                        KeyCode::Right => self.focus = NaluFocus::Viewer(NaluFocusType::Partial),
                        _ => {}
                    }
                }
            }
            NaluFocus::Viewer(focus_type) => {
                if let NaluFocusType::Full = focus_type {
                    self.handle_key_viewer(key);
                } else {
                    match key {
                        KeyCode::Enter => self.focus = NaluFocus::Viewer(NaluFocusType::Full),
                        KeyCode::Esc => self.overlay = NaluOverlay::QuitPrompt,
                        KeyCode::Left => self.focus = NaluFocus::List(NaluFocusType::Partial),
                        _ => {}
                    }
                }
            }
            NaluFocus::None => match key {
                KeyCode::Enter | KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                    self.focus = NaluFocus::Browser(NaluFocusType::Partial)
                }
                KeyCode::Esc => self.overlay = NaluOverlay::QuitPrompt,
                _ => {}
            },
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        match self.overlay {
            NaluOverlay::Loading => match key {
                KeyCode::Char('q') => self.done = Some(String::new()),
                _ => {}
            },
            NaluOverlay::Palette => match key {
                KeyCode::Esc => self.overlay = NaluOverlay::None,
                key => edit_string(key, &mut self.palette_input),
            },
            NaluOverlay::HelpPrompt => match key {
                KeyCode::Char('q') => self.done = Some(String::new()),
                KeyCode::Esc => self.overlay = NaluOverlay::None,
                _ => {}
            },
            NaluOverlay::QuitPrompt => match key {
                KeyCode::Char('q') => self.done = Some(String::new()),
                KeyCode::Esc => self.overlay = NaluOverlay::None,
                _ => {}
            },
            NaluOverlay::None => match key {
                KeyCode::Char('q') => self.done = Some(String::new()),
                // KeyCode::Esc => self.overlay = NaluOverlay::QuitPrompt,
                KeyCode::Char('h') => self.overlay = NaluOverlay::HelpPrompt,
                KeyCode::Char('p') => self.overlay = NaluOverlay::Palette,
                KeyCode::Char('r') => {
                    self.overlay = NaluOverlay::Loading;
                    self.handle_reload();
                }
                key => self.handle_key_non_overlay(key),
            },
        }
    }

    pub fn handle_reload(&mut self) {
        *self.progress.lock().unwrap() = (0, 0);
        let (tx_vcd, rx_vcd) = unbounded();
        load_vcd(self.vcd_path.clone(), tx_vcd, self.progress.clone());
        self.vcd_rx = rx_vcd;
    }

    pub fn handle_vcd(&mut self) {
        if !self.vcd_rx.is_empty() {
            match self.vcd_rx.recv().unwrap() {
                Ok((vcd_header, waveform)) => {
                    self.overlay = NaluOverlay::None;
                    // Handle dropping large objects in another thread
                    let mut vcd_header = vcd_header;
                    let mut waveform = waveform;
                    std::mem::swap(&mut self.vcd_header, &mut vcd_header);
                    std::mem::swap(&mut self.waveform, &mut waveform);
                    std::thread::spawn(move || {
                        drop(waveform);
                        drop(vcd_header);
                    });
                }
                Err(err) => {
                    self.done = Some(format!("VCD Loading Error: {:?}", err));
                }
            }
        }
    }

    pub fn get_focus(&self, pane: NaluPanes) -> Option<NaluFocusType> {
        match (pane, self.focus.clone()) {
            (NaluPanes::Browser, NaluFocus::Browser(focus_type)) => Some(focus_type),
            (NaluPanes::List, NaluFocus::List(focus_type)) => Some(focus_type),
            (NaluPanes::Viewer, NaluFocus::Viewer(focus_type)) => Some(focus_type),
            (NaluPanes::Filter, NaluFocus::Filter(focus_type)) => Some(focus_type),
            _ => None,
        }
    }

    pub fn get_resize(&self) -> &LayoutResize<3> {
        &self.resizing
    }

    pub fn get_resize_mut(&mut self) -> &mut LayoutResize<3> {
        &mut self.resizing
    }

    pub fn get_overlay(&self) -> &NaluOverlay {
        &self.overlay
    }

    pub fn get_percent(&self) -> usize {
        let (current, total) = *self.progress.lock().unwrap();
        if total == 0 {
            0
        } else {
            current * 100 / total
        }
    }

    pub fn get_filter(&self) -> String {
        self.filter_input.clone()
    }

    pub fn get_palette(&self) -> String {
        self.palette_input.clone()
    }

    pub fn get_done(&self) -> Option<String> {
        self.done.clone()
    }
}
