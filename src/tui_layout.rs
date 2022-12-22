mod component;
mod container;
mod pos;
mod tests;

#[derive(Debug, Clone, PartialEq)]
pub struct ResizeError {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub border_width: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Focus,
    PartialFocus,
    None,
}

pub enum FocusResult<T> {
    Focus(T),
    PartialFocus(T),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Border {
    Top,
    Bottom,
    Left,
    Right,
}
