pub mod filter;
pub mod netlist_viewer;
pub mod signal_viewer;
pub mod waveform_viewer;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use vcd_parser::parser::VcdHeader;
use vcd_parser::utils::*;
use vcd_parser::waveform::Waveform;

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEventKind};
use tui::layout::Rect;

use crate::resize::LayoutResize;
use crate::state::netlist_viewer::NetlistViewerState;
use crate::state::signal_viewer::SignalViewerState;
use crate::state::waveform_viewer::WaveformViewerState;

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
        KeyCode::Char(c @ ('a'..='z' | 'A'..='Z' | '0'..='9' | '_' | ' ' | '*' | '.')) => {
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
    vcd_handle: Option<JoinHandle<VcdResult<(VcdHeader, Waveform)>>>,
    overlay: NaluOverlay,
    focus: NaluFocus,
    resizing: LayoutResize<3>,
    progress: Arc<Mutex<(usize, usize)>>,
    vcd_header: VcdHeader,
    filter_input: String,
    palette_input: String,
    done: Option<String>,
    netlist_state: NetlistViewerState,
    signal_state: SignalViewerState,
    waveform_state: WaveformViewerState,
}

impl NaluState {
    pub fn new(vcd_path: PathBuf) -> Self {
        // Load initial VCD file, TODO: Handle file loading error
        let bytes = std::fs::read_to_string(&vcd_path).unwrap();
        let progress = Arc::new(Mutex::new((0, 0)));
        let handle = load_multi_threaded(bytes, 4, progress.clone());
        Self {
            vcd_path: vcd_path,
            vcd_handle: Some(handle),
            overlay: NaluOverlay::Loading,
            focus: NaluFocus::None,
            resizing: LayoutResize::new([1, 1, 2], 2),
            progress: progress,
            vcd_header: VcdHeader::new(),
            filter_input: String::new(),
            palette_input: String::new(),
            done: None,
            netlist_state: NetlistViewerState::new(),
            signal_state: SignalViewerState::new(),
            waveform_state: WaveformViewerState::new(),
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
                    self.netlist_state.handle_mouse_click(
                        x - sizing.browser.left() - 1,
                        y - sizing.browser.top() - 1,
                    );
                }
                _ => self.focus = NaluFocus::Browser(NaluFocusType::Full),
            }
        }

        if sizing.list.intersects(coord) {
            match self.focus {
                NaluFocus::List(NaluFocusType::Full) => {
                    self.signal_state
                        .handle_mouse_click(x - sizing.list.left() - 1, y - sizing.list.top() - 1);
                }
                _ => self.focus = NaluFocus::List(NaluFocusType::Full),
            }
        }

        if sizing.viewer.intersects(coord) {
            match self.focus {
                // TODO: Handle passing mouse event to component
                NaluFocus::Viewer(NaluFocusType::Full) => {}
                _ => self.focus = NaluFocus::Viewer(NaluFocusType::Full),
            }
        }

        if sizing.filter.intersects(coord) {
            match self.focus {
                // TODO: Handle passing mouse event to component
                NaluFocus::Filter(NaluFocusType::Full) => {}
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

    fn handle_mouse_scroll(&mut self, scroll_up: bool) {
        self.resizing.handle_mouse_done();
        match self.focus {
            NaluFocus::Browser(NaluFocusType::Full) => {
                self.netlist_state.handle_mouse_scroll(scroll_up)
            }
            // TODO: Handle passing mouse event to component(s)
            NaluFocus::List(NaluFocusType::Full) => {
                self.signal_state.handle_mouse_scroll(scroll_up)
            }
            NaluFocus::Viewer(NaluFocusType::Full) => {}
            NaluFocus::Filter(NaluFocusType::Full) => {}
            _ => {}
        }
    }

    pub fn handle_mouse(&mut self, x: u16, y: u16, kind: MouseEventKind, sizing: NaluSizing) {
        match self.overlay {
            NaluOverlay::None => {}
            _ => return,
        }
        match kind {
            MouseEventKind::Down(MouseButton::Left) => self.handle_mouse_click(x, y, sizing),
            MouseEventKind::Drag(MouseButton::Left) => self.handle_mouse_drag(x, y, sizing),
            MouseEventKind::ScrollDown => self.handle_mouse_scroll(false),
            MouseEventKind::ScrollUp => self.handle_mouse_scroll(true),
            _ => self.resizing.handle_mouse_done(),
        }
    }

    fn handle_key_non_overlay(&mut self, event: KeyEvent) {
        match &self.focus {
            NaluFocus::Browser(focus_type) => {
                if let NaluFocusType::Full = focus_type {
                    match event.code.clone() {
                        KeyCode::Esc => self.focus = NaluFocus::Browser(NaluFocusType::Partial),
                        _ => {
                            self.netlist_state.handle_key_press(event);
                        }
                    }
                } else {
                    match event.code {
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
                    match event.code.clone() {
                        KeyCode::Esc => self.focus = NaluFocus::Filter(NaluFocusType::Partial),
                        key => {
                            edit_string(key, &mut self.filter_input);
                            self.netlist_state.update_filter(self.filter_input.clone());
                        }
                    }
                } else {
                    match event.code {
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
                    match event.code.clone() {
                        KeyCode::Esc => self.focus = NaluFocus::List(NaluFocusType::Partial),
                        _ => self.signal_state.handle_key_press(event),
                    }
                } else {
                    match event.code {
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
                    match event.code.clone() {
                        KeyCode::Esc => self.focus = NaluFocus::Viewer(NaluFocusType::Partial),
                        _ => self.waveform_state.handle_key_press(event),
                    }
                } else {
                    match event.code {
                        KeyCode::Enter => self.focus = NaluFocus::Viewer(NaluFocusType::Full),
                        KeyCode::Esc => self.overlay = NaluOverlay::QuitPrompt,
                        KeyCode::Left => self.focus = NaluFocus::List(NaluFocusType::Partial),
                        _ => {}
                    }
                }
            }
            NaluFocus::None => match event.code {
                KeyCode::Enter | KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                    self.focus = NaluFocus::Browser(NaluFocusType::Partial)
                }
                KeyCode::Esc => self.overlay = NaluOverlay::QuitPrompt,
                _ => {}
            },
        }
    }

    pub fn handle_key(&mut self, event: KeyEvent) {
        let key_code = event.code.clone();
        match self.overlay {
            NaluOverlay::Loading => match key_code {
                KeyCode::Char('q') => self.done = Some(String::new()),
                _ => {}
            },
            NaluOverlay::Palette => match key_code {
                KeyCode::Esc => self.overlay = NaluOverlay::None,
                key => edit_string(key, &mut self.palette_input),
            },
            NaluOverlay::HelpPrompt => match key_code {
                KeyCode::Char('q') => self.done = Some(String::new()),
                KeyCode::Esc => self.overlay = NaluOverlay::None,
                _ => {}
            },
            NaluOverlay::QuitPrompt => match key_code {
                KeyCode::Char('q') => self.done = Some(String::new()),
                KeyCode::Esc => self.overlay = NaluOverlay::None,
                _ => {}
            },
            NaluOverlay::None => match key_code {
                KeyCode::Char('q') => self.done = Some(String::new()),
                KeyCode::Char('h') => self.overlay = NaluOverlay::HelpPrompt,
                KeyCode::Char('p') => self.overlay = NaluOverlay::Palette,
                KeyCode::Char('r') => {
                    self.overlay = NaluOverlay::Loading;
                    self.handle_reload();
                }
                _ => self.handle_key_non_overlay(event),
            },
        }
    }

    pub fn handle_requests(&mut self) {
        for request in self.waveform_state.get_requests() {
            self.signal_state.handle_key_press(request.0);
        }
        self.signal_state
            .browser_request(self.netlist_state.get_requests());
        self.waveform_state
            .signal_request(self.signal_state.get_requests());
    }

    pub fn handle_reload(&mut self) {
        *self.progress.lock().unwrap() = (0, 0);
        // Load updated VCD file, TODO: Handle file loading error
        let bytes = std::fs::read_to_string(&self.vcd_path).unwrap();
        let handle = load_multi_threaded(bytes, 4, self.progress.clone());
        self.vcd_handle = Some(handle);
    }

    pub fn handle_vcd(&mut self) {
        // Check if we have a handle to work with
        if let None = &mut self.vcd_handle {
            return;
        }
        // Wait for the thread to complete
        let (current, total) = *self.progress.lock().unwrap();
        if current < total || total == 0 {
            return;
        }
        // Replace existing handle with none and extract values
        let mut vcd_handle_swap = None;
        std::mem::swap(&mut vcd_handle_swap, &mut self.vcd_handle);
        match vcd_handle_swap.unwrap().join().unwrap() {
            Ok((vcd_header, waveform)) => {
                self.overlay = NaluOverlay::None;
                self.vcd_header = vcd_header;
                self.netlist_state
                    .update_scopes(&self.vcd_header.get_scopes());
                let timescale = self.vcd_header.get_timescale();
                self.waveform_state.load_waveform(
                    waveform,
                    match timescale {
                        Some(timescale) => *timescale,
                        None => 0,
                    },
                );
            }
            Err(err) => {
                self.done = Some(format!("VCD Loading Error: {:?}", err));
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

    pub fn get_netlist_state(&self) -> &NetlistViewerState {
        &self.netlist_state
    }

    pub fn get_netlist_state_mut(&mut self) -> &mut NetlistViewerState {
        &mut self.netlist_state
    }

    pub fn get_signal_state(&self) -> &SignalViewerState {
        &self.signal_state
    }

    pub fn get_signal_state_mut(&mut self) -> &mut SignalViewerState {
        &mut self.signal_state
    }

    pub fn get_waveform_state(&self) -> &WaveformViewerState {
        &self.waveform_state
    }

    pub fn get_waveform_state_mut(&mut self) -> &mut WaveformViewerState {
        &mut self.waveform_state
    }

    pub fn get_done(&self) -> Option<String> {
        self.done.clone()
    }
}
