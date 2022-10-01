mod browser;
mod filter;
mod signals;
mod waves;

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEventKind};
use tui::layout::Rect;

pub enum MouseScrollDirection {
    Up,
    Down,
}

pub trait Component {
    fn handle_mouse_click(&mut self, x: u16, y: u16);
    fn handle_mouse_drag(&mut self, x: u16, y: u16);
    fn handle_mouse_scroll(&mut self, dir: MouseScrollDirection);

    fn handle_key(&mut self, e: KeyEvent);

    fn check_updated(&mut self) -> bool;

    fn get_rect(&self) -> Rect;
}
