use tui::layout::Rect;

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentPos {
    pub x: u16,
    pub y: u16,
}

impl Default for ComponentPos {
    fn default() -> Self {
        Self { x: 0, y: 0 }
    }
}

impl std::ops::Add<ComponentPos> for ComponentPos {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub<ComponentPos> for ComponentPos {
    type Output = Option<Self>;

    fn sub(self, rhs: Self) -> Self::Output {
        if self.x >= rhs.x && self.y >= rhs.y {
            Some(Self {
                x: self.x - rhs.x,
                y: self.y - rhs.y,
            })
        } else {
            None
        }
    }
}

impl std::ops::Add<(i16, i16)> for ComponentPos {
    type Output = Option<Self>;

    fn add(self, rhs: (i16, i16)) -> Self::Output {
        let x = self.x as i16 + rhs.0;
        let y = self.y as i16 + rhs.1;
        if x >= 0 && y >= 0 {
            Some(Self {
                x: x as u16,
                y: y as u16,
            })
        } else {
            None
        }
    }
}

impl From<ComponentPos> for Rect {
    fn from(pos: ComponentPos) -> Self {
        Rect {
            x: pos.x,
            y: pos.y,
            width: 1,
            height: 1,
        }
    }
}

impl ComponentPos {
    pub fn intersects_rect(&self, r: Rect) -> bool {
        self.x >= r.x && self.x < (r.x + r.width) && self.y >= r.y && self.y < (r.y + r.height)
    }
}
