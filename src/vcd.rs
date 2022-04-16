use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crossbeam::channel::{unbounded, Sender};

use vcd_parser::errors::*;
use vcd_parser::parser::{VcdEntry, VcdHeader, VcdParser, VcdVariableWidth};
use vcd_parser::tokenizer::Tokenizer;
use vcd_parser::waveform::{errors::*, Waveform};

#[derive(Debug)]
pub enum VcdError {
    Io(io::Error),
    Parser(ParserError),
    Waveform(WaveformError),
}

pub type VcdResult<T> = Result<T, VcdError>;

// Spawns a new thread and hands back a queue where either a progress update is
// given or a result is returned
pub fn load_vcd(
    file_path: PathBuf,
    tx: Sender<VcdResult<(VcdHeader, Waveform)>>,
    status: Arc<Mutex<(usize, usize)>>,
) {
    thread::spawn(move || {
        // Read file from path
        let bytes = match fs::read_to_string(&file_path) {
            Ok(bytes) => bytes,
            Err(err) => {
                tx.send(Err(VcdError::Io(err))).unwrap();
                return;
            }
        };
        let file_size = match fs::metadata(&file_path) {
            Ok(meta) => meta.len() as usize,
            Err(err) => {
                tx.send(Err(VcdError::Io(err))).unwrap();
                return;
            }
        };
        *status.lock().unwrap() = (0, file_size);

        // Create a tokenizer and parser for the file
        let mut tokenizer = Tokenizer::new(&bytes);
        let mut parser = match VcdParser::new(&mut tokenizer) {
            Ok(parser) => parser,
            Err(err) => {
                tx.send(Err(VcdError::Parser(err))).unwrap();
                return;
            }
        };
        *status.lock().unwrap() = (tokenizer.get_position().get_index(), file_size);

        // Create a waveform and fill it out with signal information
        let mut waveform = Waveform::new();
        for (idcode, width) in parser.get_header().get_idcodes_map().iter() {
            match width {
                VcdVariableWidth::Vector { width } => {
                    waveform.initialize_vector(*idcode, *width);
                }
                VcdVariableWidth::Real => {
                    waveform.initialize_real(*idcode);
                }
            }
        }

        // Read events from the VCD file and load them into the waveform
        let waveform = {
            let (wave_tx, wave_rx) = unbounded();
            // Spawn a separate thread to insert events into the waveform
            let t: JoinHandle<Result<Waveform, WaveformError>> = thread::spawn(move || loop {
                match wave_rx.recv().unwrap() {
                    Some(VcdEntry::Timestamp(timestamp)) => waveform.insert_timestamp(timestamp)?,
                    Some(VcdEntry::Vector(bv, id)) => waveform.update_vector(id, bv)?,
                    Some(VcdEntry::Real(r, id)) => waveform.update_real(id, r)?,
                    None => break Ok(waveform),
                }
            });
            // Loop through all events and add them to the queue
            let mut last_index = tokenizer.get_position().get_index();
            loop {
                let entry = match parser.parse_waveform(&mut tokenizer) {
                    Ok(Some(entry)) => entry,
                    Ok(None) => {
                        wave_tx.send(None).unwrap();
                        break;
                    }
                    Err(err) => {
                        wave_tx.send(None).unwrap();
                        tx.send(Err(VcdError::Parser(err))).unwrap();
                        return;
                    }
                };
                wave_tx.send(Some(entry)).unwrap();
                let index = tokenizer.get_position().get_index();
                if (index - last_index) * 200 / file_size > 0 {
                    *status.lock().unwrap() = (index, file_size);
                    last_index = index;
                }
            }
            *status.lock().unwrap() = (file_size, file_size);
            match t.join().unwrap() {
                Ok(waveform) => waveform,
                Err(err) => {
                    tx.send(Err(VcdError::Waveform(err))).unwrap();
                    return;
                }
            }
        };
        *status.lock().unwrap() = (file_size, file_size);
        tx.send(Ok((parser.into_header(), waveform))).unwrap();
    });
}
