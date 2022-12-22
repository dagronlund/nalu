use crossterm::event::{KeyCode, KeyEvent, MouseEventKind};
use tui::{
    buffer::Buffer,
    layout::{Direction, Rect},
};

use crate::tui_layout::{
    container::search::ContainerSearch, container::*, pos::*, Border, Focus, FocusResult,
    ResizeError,
};

pub(crate) fn orientation_scalar(orientation: &Direction, width: u16, height: u16) -> u16 {
    match orientation {
        Direction::Horizontal => width,
        Direction::Vertical => height,
    }
}

enum Resize {
    _Left(u16, usize),
    _Right(u16, usize),
    None,
}

pub struct ContainerList {
    name: String,
    orientation: Direction,
    _resizable: bool,
    _resize: Resize,
    width: u16,
    height: u16,
    children: Vec<ContainerChild>,
}

impl ContainerList {
    pub fn new(
        name: String,
        orientation: Direction,
        _resizable: bool,
        width: u16,
        height: u16,
    ) -> Self {
        Self {
            name,
            orientation,
            _resizable,
            _resize: Resize::None,
            width,
            height,
            children: Vec::new(),
        }
    }

    /// Gets the ratios of all the component sizes to the total
    fn get_ratios(&self) -> Vec<f64> {
        let total = orientation_scalar(&self.orientation, self.get_width(), self.get_height());
        let any_zero = self.children.iter().any(|c| {
            orientation_scalar(
                &self.orientation,
                c.as_base().get_width(),
                c.as_base().get_height(),
            ) == 0
        });
        // Return a default ratio list if any sub-component is zero or the
        // container is
        if total == 0 || any_zero {
            return vec![1.0f64].repeat(self.children.len());
        }
        self.children
            .iter()
            .map(|c| {
                orientation_scalar(
                    &self.orientation,
                    c.as_base().get_width(),
                    c.as_base().get_height(),
                ) as f64
                    / total as f64
            })
            .collect()
    }

    /// Sets all the children to be proportioned sizes in the container
    fn calculate_sizes(ratios: Vec<f64>, total_size: u16) -> Vec<u16> {
        let sum = ratios.iter().sum::<f64>();
        let ratios = ratios.iter().map(|r| r / sum).collect::<Vec<f64>>();
        let mut sizes = Vec::new();
        let mut sub_total = 0;
        for i in 0..ratios.len() {
            let calculated = if i < ratios.len() - 1 {
                (ratios[i] * total_size as f64) as u16
            } else {
                total_size - sub_total
            };
            sizes.push(calculated);
            sub_total += calculated;
        }
        sizes
    }

    /// Adds a new component to the container, resizes the existing children
    /// to fit the new component, and returns false if there was no room
    /// available for the component
    pub fn add_component(&mut self, child: Component) -> Result<(), ResizeError> {
        self.children.push(ContainerChild::Component(child));
        self.resize(self.get_width(), self.get_height())
    }

    /// Adds a new container to the container, resizes the existing children
    /// to fit the new container, and returns false if there was no room
    /// available for the container
    pub fn add_container(&mut self, child: Box<dyn Container>) -> Result<(), ResizeError> {
        self.children.push(ContainerChild::Container(child));
        self.resize(self.get_width(), self.get_height())
    }

    pub fn get_orientation(&self) -> Direction {
        self.orientation.clone()
    }
}

impl Container for ContainerList {
    fn get_children(&self) -> &Vec<ContainerChild> {
        &self.children
    }

    fn get_children_mut(&mut self) -> &mut Vec<ContainerChild> {
        &mut self.children
    }

    fn get_children_rectangles(&self) -> Vec<Rect> {
        let mut pos = ComponentPos::default();
        let mut rects = Vec::new();
        for child in &self.children {
            rects.push(Rect {
                x: pos.x,
                y: pos.y,
                width: child.as_base().get_width(),
                height: child.as_base().get_height(),
            });
            match self.orientation {
                Direction::Horizontal => pos.x += child.as_base().get_width(),
                Direction::Vertical => pos.y += child.as_base().get_height(),
            }
        }
        rects
    }

    fn as_base(&self) -> &dyn ComponentBase {
        self
    }

    fn as_base_mut(&mut self) -> &mut dyn ComponentBase {
        self
    }

    fn is_resizable(&self) -> bool {
        self._resizable
    }

    fn as_container(&self) -> &dyn Container {
        self
    }

    fn as_container_mut(&mut self) -> &mut dyn Container {
        self
    }
}

fn find_next_pos(
    pos: ComponentPos,
    border: Border,
    component_width: u16,
    component_height: u16,
    container_width: u16,
    container_height: u16,
) -> Option<ComponentPos> {
    match border {
        Border::Top => {
            if pos.y == 0 {
                return None;
            }
            Some(ComponentPos {
                x: pos.x,
                y: pos.y - 1,
            })
        }
        Border::Bottom => {
            if pos.y + component_height >= container_height {
                return None;
            }
            Some(ComponentPos {
                x: pos.x,
                y: pos.y + component_height,
            })
        }
        Border::Left => {
            if pos.x == 0 {
                return None;
            }
            Some(ComponentPos {
                x: pos.x - 1,
                y: pos.y,
            })
        }
        Border::Right => {
            if pos.x + component_width >= container_width {
                return None;
            }
            Some(ComponentPos {
                x: pos.x + component_width,
                y: pos.y,
            })
        }
    }
}

impl ComponentBase for ContainerList {
    fn handle_mouse(&mut self, x: u16, y: u16, kind: Option<MouseEventKind>) {
        let (width, height, orientation) = (
            self.get_width(),
            self.get_height(),
            self.orientation.clone(),
        );
        let (mut offset_x, mut offset_y) = (0, 0);
        for component in &mut self.children {
            if x >= width || y >= height || x < offset_x || y < offset_y {
                component.as_base_mut().handle_mouse(0, 0, None);
            } else {
                component
                    .as_base_mut()
                    .handle_mouse(x - offset_x, y - offset_y, kind);
            }
            match orientation {
                Direction::Horizontal => offset_x += component.as_base().get_width(),
                Direction::Vertical => offset_y += component.as_base().get_height(),
            }
        }
    }

    fn handle_key(&mut self, event: KeyEvent) -> Option<Border> {
        let (container_width, container_height) = (self.get_width(), self.get_height());
        // Send key event to the (partially) focused component
        match self.as_container_mut().search_focused_mut() {
            FocusResult::Focus((component, pos)) | FocusResult::PartialFocus((component, pos)) => {
                let Some(border) = component.handle_key(event) else {
                    return None;
                };
                // Look for position of next component to focus
                let next_pos = find_next_pos(
                    pos,
                    border,
                    component.get_width(),
                    component.get_height(),
                    container_width,
                    container_height,
                );
                // Check the position actually exists
                let Some(next_pos) = next_pos else {
                    component.set_focus(Focus::PartialFocus);
                    return None;
                };
                // Partially focus component
                if let Some((component_next, _)) =
                    self.as_container_mut().search_position_mut(next_pos)
                {
                    component_next.set_focus(Focus::PartialFocus);
                }
            }
            FocusResult::None => {
                // If nothing has focus, check if the right keys were pressed
                match event.clone().code {
                    KeyCode::Enter
                    | KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Left
                    | KeyCode::Right => {}
                    _ => return None,
                }
                // Find something to focus
                if let Some((component, _)) = self
                    .as_container_mut()
                    .search_position_mut(ComponentPos { x: 0, y: 0 })
                {
                    component.set_focus(Focus::PartialFocus);
                }
            }
        }
        None
    }

    fn invalidate(&mut self) {
        for component in &mut self.children {
            component.as_base_mut().invalidate();
        }
    }

    fn resize(&mut self, width: u16, height: u16) -> Result<(), ResizeError> {
        if self.width == width && self.height == height {
            return Ok(());
        }
        let ratios = self.get_ratios();
        self.width = width;
        self.height = height;
        let sizes = ContainerList::calculate_sizes(
            ratios,
            orientation_scalar(&self.orientation, self.get_width(), self.get_height()),
        );
        for i in 0..self.children.len() {
            let (width, height) = match self.orientation {
                Direction::Horizontal => (sizes[i], self.get_height()),
                Direction::Vertical => (self.get_width(), sizes[i]),
            };
            self.children[i].as_base_mut().resize(width, height)?;
        }
        self.invalidate();
        Ok(())
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        assert_eq!(area.width, self.width);
        assert_eq!(area.height, self.height);
        let mut sub_area = area.clone();
        for component in &mut self.children {
            match self.orientation {
                Direction::Horizontal => sub_area.width = component.as_base().get_width(),
                Direction::Vertical => sub_area.height = component.as_base().get_height(),
            }
            component.as_base_mut().render(sub_area, buf);
            match self.orientation {
                Direction::Horizontal => sub_area.x += component.as_base().get_width(),
                Direction::Vertical => sub_area.y += component.as_base().get_height(),
            }
        }
    }

    fn get_width(&self) -> u16 {
        self.width
    }

    fn get_height(&self) -> u16 {
        self.height
    }

    fn get_focus(&self) -> Focus {
        for component in &self.children {
            match component.as_base().get_focus() {
                Focus::Focus => return Focus::Focus,
                Focus::PartialFocus => return Focus::PartialFocus,
                _ => {}
            }
        }
        Focus::None
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_border(&self, x: u16, y: u16) -> Option<Border> {
        let (mut offset_x, mut offset_y) = (0, 0);
        for (i, component) in (&self.children).iter().enumerate() {
            if x >= self.get_width() || y >= self.get_height() || x < offset_x || y < offset_y {
                return None;
            }
            match self.orientation {
                Direction::Horizontal => {
                    match component.as_base().get_border(x - offset_x, y - offset_y) {
                        Some(Border::Top) => return Some(Border::Top),
                        Some(Border::Bottom) => return Some(Border::Bottom),
                        Some(Border::Left) => {
                            if i == 0 {
                                return Some(Border::Left);
                            }
                        }
                        Some(Border::Right) => {
                            if i == self.children.len() - 1 {
                                return Some(Border::Right);
                            }
                        }
                        None => {}
                    }
                    offset_x += component.as_base().get_width();
                }
                Direction::Vertical => {
                    match component.as_base().get_border(x - offset_x, y - offset_y) {
                        Some(Border::Top) => {
                            if i == 0 {
                                return Some(Border::Top);
                            }
                        }
                        Some(Border::Bottom) => {
                            if i == self.children.len() - 1 {
                                return Some(Border::Bottom);
                            }
                        }
                        Some(Border::Left) => return Some(Border::Left),
                        Some(Border::Right) => return Some(Border::Right),
                        None => {}
                    }
                    offset_y += component.as_base().get_height();
                }
            }
        }
        None
    }
}
