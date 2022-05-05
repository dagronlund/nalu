use tui::{
    style::{Color, Style},
    text::Text,
};

use crate::state::tree::*;

pub fn get_selected_style(is_selected: bool, is_primary: bool) -> Style {
    if is_selected {
        if is_primary {
            Style::default().fg(Color::Black).bg(Color::White)
        } else {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Rgb(128, 128, 128))
        }
    } else {
        Style::default()
    }
}

pub fn clamp_signed(value: isize, min: isize, max: isize) -> isize {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[derive(Clone)]
struct TreeCursor {
    primary: isize,
    secondary: Option<isize>,
}

impl TreeCursor {
    fn get_primary(&self) -> isize {
        self.primary
    }

    fn get_selected(&self) -> core::ops::Range<isize> {
        if let Some(secondary) = self.secondary {
            if self.primary < secondary {
                self.primary..secondary + 1
            } else {
                secondary..self.primary + 1
            }
        } else {
            self.primary..self.primary + 1
        }
    }

    fn is_primary_selected(&self) -> bool {
        self.primary == 0
    }

    fn is_selected(&self) -> bool {
        self.get_selected().contains(&0)
    }

    fn add(&mut self, value: isize) {
        self.primary += value;
        self.secondary = if let Some(secondary) = self.secondary {
            Some(secondary + value)
        } else {
            None
        };
    }
}

pub struct TreeDisplay {
    height: isize,
    scroll: isize,
    cursor: TreeCursor,
}

impl TreeDisplay {
    pub fn new() -> Self {
        Self {
            height: 0,
            scroll: 0,
            cursor: TreeCursor {
                primary: 0,
                secondary: None,
            },
        }
    }

    pub fn make_render_offsets(&self) -> TreeRender {
        TreeRender {
            render: self.height,
            scroll: self.scroll,
            cursor: self.cursor.clone(),
            indent: 0,
        }
    }

    pub fn select_relative<T>(&mut self, tree: &TreeNodes<T>, delta: isize, is_primary: bool) {
        if is_primary {
            self.cursor.secondary = None;
            self.cursor.primary = clamp_signed(
                self.cursor.primary + delta,
                0,
                tree.rendered_len() as isize - 1,
            );
            self.scroll_relative(0, self.cursor.primary);
        } else {
            let secondary = if let Some(secondary) = self.cursor.secondary {
                secondary
            } else {
                self.cursor.get_primary()
            };
            let secondary = clamp_signed(secondary + delta, 0, tree.rendered_len() as isize - 1);
            self.cursor.secondary = Some(secondary);
            self.scroll_relative(0, secondary);
        }
    }

    pub fn select_absolute<T>(
        &mut self,
        tree: &TreeNodes<T>,
        value: isize,
        is_primary: bool,
    ) -> bool {
        let select_offset = value + self.scroll;
        if is_primary {
            self.cursor.secondary = None;
            if (0..tree.rendered_len() as isize).contains(&select_offset) {
                let already_selected = select_offset == self.cursor.primary;
                self.cursor.primary = select_offset;
                already_selected
            } else {
                false
            }
        } else {
            self.cursor.secondary = Some(select_offset);
            false
        }
    }

    pub fn scroll_relative(&mut self, delta: isize, cursor: isize) {
        self.scroll = clamp_signed(
            self.scroll as isize + delta,
            if cursor > self.height - 1 {
                cursor - self.height + 1
            } else {
                0
            },
            cursor,
        );
    }

    pub fn set_height(&mut self, height: isize) {
        let changed = self.height != height;
        if height > 0 {
            self.height = height;
        } else {
            self.height = 0;
        }
        if changed {
            self.scroll_relative(0, self.cursor.get_primary());
        }
    }

    pub fn get_primary_selected(&self) -> isize {
        self.cursor.get_primary()
    }

    pub fn get_selected(&self) -> core::ops::Range<isize> {
        self.cursor.get_selected()
    }
}

pub struct TreeRender {
    render: isize,
    scroll: isize,
    cursor: TreeCursor,
    indent: isize,
}

impl TreeRender {
    pub fn render_line(&mut self, text: &mut Text<'static>, line: Text<'static>) {
        if self.scroll <= 0 && self.render > 0 {
            text.extend(line);
            self.render -= 1;
        } else {
            self.scroll -= 1;
        }
        self.cursor.add(-1);
    }

    pub fn do_indent(&mut self) {
        self.indent += 1;
    }

    pub fn undo_indent(&mut self) {
        self.indent -= 1;
    }

    pub fn get_indent_offset(&self) -> isize {
        self.indent
    }

    pub fn is_rendering(&self) -> bool {
        self.render > 0
    }

    pub fn is_primary_selected(&self) -> bool {
        self.cursor.is_primary_selected()
    }

    pub fn is_selected(&self) -> bool {
        self.cursor.is_selected()
    }
}
