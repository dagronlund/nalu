pub mod filter;
pub mod netlist_viewer;
pub mod signal_viewer;
pub mod waveform_viewer;

use std::path::PathBuf;
use std::sync::Arc;

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, MouseEventKind};

use makai::utils::messages::Messages;
use makai_vcd_reader::parser::VcdHeader;
use makai_vcd_reader::utils::{load_multi_threaded, VcdLoaderMessage, VcdResult};
use makai_waveform_db::Waveform;

use crate::state::netlist_viewer::NetlistViewerMessage;
use crate::state::signal_viewer::SignalViewerMessage;
use crate::state::waveform_viewer::WaveformViewerMessage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NaluOverlay {
    Loading,
    Palette,
    HelpPrompt,
    QuitPrompt,
    None,
}

pub enum NaluMessage {}

pub struct NaluState {
    vcd_path: PathBuf,
    python_path: Option<PathBuf>,
    overlay: NaluOverlay,
    progress: (usize, usize),
    vcd_header: Arc<VcdHeader>,
    palette_input: String,
    done: Option<String>,
    initial_load: bool,
    messages: Messages,
}

impl NaluState {
    pub fn new(vcd_path: PathBuf, python_path: Option<PathBuf>) -> Self {
        Self {
            vcd_path,
            python_path,
            overlay: NaluOverlay::Loading,
            progress: (0, 0),
            vcd_header: Arc::new(VcdHeader::new()),
            palette_input: String::new(),
            done: None,
            initial_load: true,
            messages: Messages::new(),
        }
    }

    pub fn handle_input(&mut self, event: CrosstermEvent) -> Option<CrosstermEvent> {
        match event {
            CrosstermEvent::Key(key) => {
                if let Some(_) = self.handle_key(key) {
                    return Some(event);
                }
            }
            CrosstermEvent::Mouse(mouse) => {
                if let Some(_) = self.handle_mouse(mouse.column, mouse.row, mouse.kind) {
                    return Some(event);
                }
            }
            CrosstermEvent::Resize(_, _)
            | CrosstermEvent::FocusGained
            | CrosstermEvent::FocusLost
            | CrosstermEvent::Paste(_) => {}
        }
        None
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
                KeyCode::Char('s') => {
                    self.handle_save_config();
                }
                _ => return Some(event),
            },
            _ => {}
        }
        None
    }

    pub fn handle_save_config(&mut self) {
        log::info!("Saving config...");
        self.messages.push(SignalViewerMessage::SaveConfig {
            python_path: self.python_path.clone(),
            force: false,
        });
    }

    pub fn handle_load(&mut self) {
        log::info!("Loading {:?}...", self.vcd_path);
        self.progress = (0, 0);
        let bytes = match std::fs::read_to_string(&self.vcd_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                log::error!("VCD Loading Error: {:?}", err);
                self.done = Some(format!("VCD Loading Error: {:?}", err));
                return;
            }
        };
        load_multi_threaded(bytes, 4, self.messages.clone());
    }

    pub fn handle_update(&mut self) {
        for messages in self.messages.get::<VcdLoaderMessage>() {
            match messages {
                VcdLoaderMessage::Status { index, total } => self.progress = (index, total),
                VcdLoaderMessage::Done(result) => self.handle_vcd(result),
            }
        }
        for messages in self.messages.get::<NaluMessage>() {
            match messages {}
        }
    }

    fn handle_vcd(&mut self, result: VcdResult<(VcdHeader, Waveform)>) {
        log::info!("Finished loading!");
        let (vcd_header, waveform) = match result {
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
        self.messages.push(NetlistViewerMessage::WaveformUpdate {
            vcd_header: self.vcd_header.clone(),
        });
        self.messages.push(SignalViewerMessage::LoadConfig {
            vcd_header: self.vcd_header.clone(),
            python_path: self.python_path.clone(),
            force: self.initial_load,
        });
        self.initial_load = false;
        self.messages.push(WaveformViewerMessage::WaveformUpdate(
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
        let (current, total) = self.progress;
        if total == 0 {
            0
        } else {
            current * 100 / total
        }
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
