use tui::{
    style::{Color, Style},
    text::Text,
};

pub fn get_selected_style(selected: bool) -> Style {
    if selected {
        Style::default().fg(Color::Black).bg(Color::White)
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

pub struct SelectContext {
    height: isize,
    scroll: isize,
    select: isize,
}

impl SelectContext {
    pub fn new() -> Self {
        Self {
            height: 0,
            scroll: 0,
            select: 0,
        }
    }

    pub fn make_render_offsets(&self) -> RenderContext {
        RenderContext {
            render: self.height,
            scroll: self.scroll,
            select: self.select,
            indent: 0,
        }
    }

    pub fn select_relative(&mut self, delta: isize, limit: usize) {
        self.select = clamp_signed(self.select as isize + delta, 0, limit as isize - 1);
        self.scroll_relative(0);
    }

    pub fn select_absolute(&mut self, value: isize, limit: usize) -> bool {
        let select_offset = value + self.scroll;
        if 0 <= select_offset && select_offset < limit as isize {
            let already_selected = select_offset == self.select;
            self.select = select_offset;
            already_selected
        } else {
            false
        }
    }

    pub fn scroll_relative(&mut self, delta: isize) {
        self.scroll = clamp_signed(
            self.scroll as isize + delta,
            if self.select > self.height - 1 {
                self.select - self.height + 1
            } else {
                0
            },
            self.select,
        );
    }

    pub fn set_height(&mut self, height: isize) {
        if height > 0 {
            self.height = height;
        } else {
            self.height = 0;
        }
        self.scroll_relative(0);
    }

    pub fn get_select_offset(&self) -> isize {
        self.select
    }
}

pub struct RenderContext {
    render: isize,
    scroll: isize,
    select: isize,
    indent: isize,
}

impl RenderContext {
    pub fn render_line(&mut self, text: &mut Text<'static>, line: Text<'static>) {
        if self.scroll <= 0 && self.render > 0 {
            text.extend(line);
            self.render -= 1;
        } else {
            self.scroll -= 1;
        }
        self.select -= 1;
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

    pub fn is_selected(&self) -> bool {
        self.select == 0
    }
}
