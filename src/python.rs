pub mod bitvector;
pub mod buffer;
pub mod signals;
mod tests;
pub mod utils;
pub mod vcd_header;
pub mod waveform;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ConfigOwner {
    Nalu,
    User,
}
