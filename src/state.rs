pub mod filter;
pub mod netlist_viewer;
pub mod signal_viewer;
pub mod waveform_viewer;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crossterm::event::{KeyCode, KeyEvent, MouseEventKind};

use makai::utils::messages::Messages;
use makai_vcd_reader::parser::VcdHeader;
use makai_vcd_reader::utils::*;
use makai_waveform_db::Waveform;

use crate::state::netlist_viewer::NetlistViewerMessage;
use crate::state::waveform_viewer::WaveformViewerMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
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
    messages: Messages,
}

impl NaluState {
    pub fn new(vcd_path: PathBuf, python_path: Option<PathBuf>) -> Self {
        Self {
            vcd_path,
            python_path,
            vcd_handle: None,
            overlay: NaluOverlay::Loading,
            progress: Arc::new(Mutex::new((0, 0))),
            vcd_header: Arc::new(VcdHeader::new()),
            filter_input: String::new(),
            palette_input: String::new(),
            done: None,
            messages: Messages::new(),
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
        match self.overlay {
            NaluOverlay::Loading if event.code == KeyCode::Char('q') => {
                self.done = Some(String::new());
            }
            NaluOverlay::Palette if event.code == KeyCode::Esc => {
                self.overlay = NaluOverlay::None;
            }
            NaluOverlay::HelpPrompt => match event.code {
                KeyCode::Char('q') => self.done = Some(String::new()),
                KeyCode::Esc => self.overlay = NaluOverlay::None,
                _ => {}
            },
            NaluOverlay::QuitPrompt => match event.code {
                KeyCode::Char('q') => self.done = Some(String::new()),
                KeyCode::Esc => self.overlay = NaluOverlay::None,
                _ => {}
            },
            NaluOverlay::None => match event.code {
                KeyCode::Char('q') => self.done = Some(String::new()),
                KeyCode::Char('h') => self.overlay = NaluOverlay::HelpPrompt,
                KeyCode::Char('p') => self.overlay = NaluOverlay::Palette,
                KeyCode::Char('r') => {
                    self.overlay = NaluOverlay::Loading;
                    self.handle_load();
                }
                _ => return Some(event),
            },
            _ => {}
        }
        None
    }

    pub fn handle_load(&mut self) {
        log::info!("Loading {:?}...", self.vcd_path);
        *self.progress.lock().unwrap() = (0, 0);
        let bytes = match std::fs::read_to_string(&self.vcd_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                log::error!("VCD Loading Error: {:?}", err);
                self.done = Some(format!("VCD Loading Error: {:?}", err));
                return;
            }
        };
        let handle = load_multi_threaded(bytes, 4, self.progress.clone());
        self.vcd_handle = Some(handle);
    }

    pub fn handle_vcd(&mut self) {
        // Check if we have a handle to work with
        if self.vcd_handle.is_none() {
            return;
        }
        // Wait for the thread to complete
        let (current, total) = *self.progress.lock().unwrap();
        if current < total || total == 0 {
            return;
        }
        log::info!("Finished loading!");
        // Replace existing handle with none and extract values
        let mut vcd_handle_swap = None;
        std::mem::swap(&mut vcd_handle_swap, &mut self.vcd_handle);
        let (vcd_header, waveform) = match vcd_handle_swap.unwrap().join().unwrap() {
            Ok((vcd_header, waveform)) => (vcd_header, waveform),
            Err(err) => {
                log::error!("VCD Loading Error: {:?}", err);
                self.done = Some(format!("VCD Loading Error: {:?}", err));
                return;
            }
        };
        self.overlay = NaluOverlay::None;
        self.vcd_header = Arc::new(vcd_header);
        let timescale = match self.vcd_header.get_timescale() {
            Some(timescale) => *timescale,
            None => 0,
        };
        self.messages.push(NetlistViewerMessage::UpdateScopes(
            self.vcd_header.get_scopes().clone(),
        ));
        self.messages.push(WaveformViewerMessage::UpdateWaveform(
            Arc::new(waveform),
            self.vcd_header.clone(),
            timescale,
            self.python_path.clone(),
        ));
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

    pub fn get_messages(&self) -> &Messages {
        &self.messages
    }
}
