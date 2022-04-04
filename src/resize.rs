use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};

pub struct LayoutResize<const N: usize> {
    lengths: [u16; N],
    min_length: u16,
    moving_border: Option<(usize, u16)>,
}

impl<const N: usize> LayoutResize<N> {
    pub fn new(lengths: [u16; N], min_length: u16) -> Self {
        Self {
            lengths: lengths,
            min_length: min_length,
            moving_border: None,
        }
    }

    pub fn get_container_length(&self) -> u16 {
        let mut total_length = 0;
        for length in self.lengths {
            total_length += length;
        }
        total_length
    }

    pub fn resize_container(&mut self, get_container_length: u16) {
        if self.lengths.len() == 0 {
            return;
        }
        let old_container_length = self.get_container_length();
        // Scale all but last length in container by new size
        let mut new_lengths = self.lengths.clone();
        let mut all_but_last_length = 0;
        for i in 0..(self.lengths.len() - 1) {
            let new_length = ((self.lengths[i] * get_container_length) as u32
                / old_container_length as u32) as u16;
            if new_length > self.min_length {
                new_lengths[i] = new_length;
                all_but_last_length += new_length;
            } else {
                return;
            }
        }
        // Use remaining space for last length
        if all_but_last_length + self.min_length <= get_container_length {
            new_lengths[self.lengths.len() - 1] = get_container_length - all_but_last_length;
        } else {
            return;
        }
        self.lengths = new_lengths;
        assert!(self.get_container_length() == get_container_length)
    }

    pub fn handle_mouse_down(&mut self, pos: u16, border_size: u16) {
        let mut offset = 0;
        for border in 1..self.lengths.len() {
            offset += self.lengths[border - 1];
            if (offset - border_size) <= pos && pos < (offset + border_size) {
                self.moving_border = Some((border, pos));
                break;
            }
        }
    }

    pub fn handle_mouse_drag(&mut self, pos: u16) {
        if let Some((border, old_pos)) = self.moving_border {
            let diff = (pos as i16) - (old_pos as i16);
            let left_length = (self.lengths[border - 1] as i16) + diff;
            let right_length = (self.lengths[border] as i16) - diff;
            if left_length >= (self.min_length as i16) && right_length >= (self.min_length as i16) {
                self.lengths[border - 1] = left_length as u16;
                self.lengths[border] = right_length as u16;
            }
            self.moving_border = Some((border, pos));
        }
    }

    pub fn handle_mouse_done(&mut self) {
        self.moving_border = None;
    }

    pub fn constrain_layout(&self, layout: Layout) -> Layout {
        let mut constraints = Vec::new();
        for length in self.lengths {
            constraints.push(Constraint::Length(length));
        }
        layout.constraints(constraints)
    }
}
