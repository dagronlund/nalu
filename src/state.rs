pub mod filter;
pub mod netlist_viewer;
pub mod signal_viewer;
pub mod waveform_viewer;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crossterm::event::{KeyCode, KeyEvent, MouseEventKind};

use tui_layout::container::{search::ContainerSearch, Container};
use vcd_parser::parser::VcdHeader;
use vcd_parser::utils::*;
use waveform_db::Waveform;

use crate::state::{netlist_viewer::NetlistViewerState, waveform_viewer::WaveformViewerState};

#[derive(Debug, Clone, PartialEq)]
pub enum NaluOverlay {
    Loading,
    Palette,
    HelpPrompt,
    QuitPrompt,
    None,
}

pub struct NaluState {
    vcd_path: PathBuf,
    python_path: Option<PathBuf>,
    vcd_handle: Option<JoinHandle<VcdResult<(VcdHeader, Waveform)>>>,
    overlay: NaluOverlay,
    progress: Arc<Mutex<(usize, usize)>>,
    vcd_header: Arc<VcdHeader>,
    filter_input: String,
    palette_input: String,
    done: Option<String>,
}

impl NaluState {
    pub fn new(vcd_path: PathBuf, python_path: Option<PathBuf>) -> Self {
        // Load initial VCD file, TODO: Handle file loading error
        let bytes = std::fs::read_to_string(&vcd_path).unwrap();
        let progress = Arc::new(Mutex::new((0, 0)));
        let handle = load_multi_threaded(bytes, 4, progress.clone());
        Self {
            vcd_path,
            python_path,
            vcd_handle: Some(handle),
            overlay: NaluOverlay::Loading,
            progress,
            vcd_header: Arc::new(VcdHeader::new()),
            filter_input: String::new(),
            palette_input: String::new(),
            done: None,
        }
    }

    pub fn handle_mouse(
        &mut self,
        x: u16,
        y: u16,
        kind: MouseEventKind,
    ) -> Option<(u16, u16, MouseEventKind)> {
        match self.overlay {
            NaluOverlay::None => Some((x, y, kind)),
            _ => None,
        }
    }

    pub fn handle_key(&mut self, event: KeyEvent) -> Option<KeyEvent> {
        let key_code = event.code.clone();
        match self.overlay {
            NaluOverlay::Loading => match key_code {
                KeyCode::Char('q') => self.done = Some(String::new()),
                _ => {}
            },
            NaluOverlay::Palette => match key_code {
                KeyCode::Esc => self.overlay = NaluOverlay::None,
                _ => {}
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
                _ => return Some(event),
            },
        }
        None
    }

    pub fn handle_reload(&mut self) {
        *self.progress.lock().unwrap() = (0, 0);
        // Load updated VCD file, TODO: Handle file loading error
        let bytes = std::fs::read_to_string(&self.vcd_path).unwrap();
        let handle = load_multi_threaded(bytes, 4, self.progress.clone());
        self.vcd_handle = Some(handle);
    }

    pub fn handle_vcd(&mut self, tui: &mut Box<dyn Container>) {
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
        let (vcd_header, waveform) = match vcd_handle_swap.unwrap().join().unwrap() {
            Ok((vcd_header, waveform)) => (vcd_header, waveform),
            Err(err) => {
                self.done = Some(format!("VCD Loading Error: {:?}", err));
                return;
            }
        };
        self.overlay = NaluOverlay::None;
        self.vcd_header = Arc::new(vcd_header);
        let timescale = self.vcd_header.get_timescale();
        tui.as_container_mut()
            .search_name_widget_mut::<NetlistViewerState>("main.netlist_main.netlist")
            .unwrap()
            .update_scopes(&self.vcd_header.get_scopes());
        tui.as_container_mut()
            .search_name_widget_mut::<WaveformViewerState>("main.waveform")
            .unwrap()
            .load_waveform(
                Arc::new(waveform),
                self.vcd_header.clone(),
                match timescale {
                    Some(timescale) => *timescale,
                    None => 0,
                },
                self.python_path.clone(),
            );
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
