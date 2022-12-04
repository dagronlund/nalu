mod finder;
mod pos;
mod tests;

use crossterm::event::{KeyCode, KeyEvent, MouseEventKind};
use tui::{buffer::Buffer, layout::Rect};

use crate::component::finder::*;
use crate::component::pos::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentResizeError {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub border_width: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentFocus {
    Focus,
    PartialFocus,
    None,
}

pub enum ComponentFocusResult<T> {
    Focus(T),
    PartialFocus(T),
    None,
}

pub trait ComponentSimple {
    fn handle_mouse(&mut self, x: u16, y: u16, kind: MouseEventKind);
    fn handle_key(&mut self, e: KeyEvent);

    /// Indicates that this component and all sub-components need to be redrawn
    fn invalidate(&mut self);

    /// Resizes this component to fit in the new size, returning true if this
    /// succeeded, resizing any child components as well, and invalidating this
    /// component and all of its children too
    fn resize(&mut self, width: u16, height: u16) -> Result<(), ComponentResizeError>;

    fn get_width(&self) -> u16;
    fn get_height(&self) -> u16;
    fn get_border_width(&self) -> u16; // Base component-only

    /// Renders the component to the area specified on the buffer, marking the
    /// component as clean when done
    fn render(&mut self, area: Rect, buf: &mut Buffer);

    fn set_focus(&mut self, focus: ComponentFocus); // Base component-only
    fn get_focus(&self) -> ComponentFocus;

    fn get_name(&self) -> String;
}

pub trait Component
where
    Self: ComponentSimple + ComponentFinder,
{
    fn as_component_simple(&self) -> &dyn ComponentSimple;
    fn as_component_simple_mut(&mut self) -> &mut dyn ComponentSimple;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentListOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentListResizable {
    False,
    True,
}

impl ComponentListOrientation {
    fn get_size(&self, component: &dyn ComponentSimple) -> u16 {
        match self {
            Self::Horizontal => component.get_width(),
            Self::Vertical => component.get_height(),
        }
    }

    fn get_offset(&self, width: u16, height: u16) -> ComponentPos {
        match self {
            Self::Horizontal => ComponentPos { x: width, y: 0 },
            Self::Vertical => ComponentPos { x: 0, y: height },
        }
    }
}

pub struct ComponentList {
    name: String,
    orientation: ComponentListOrientation,
    _resizable: ComponentListResizable,
    width: u16,
    height: u16,
    components: Vec<Box<dyn Component>>,
}

impl ComponentList {
    pub fn new(
        name: String,
        orientation: ComponentListOrientation,
        _resizable: ComponentListResizable,
        width: u16,
        height: u16,
    ) -> Self {
        Self {
            name,
            orientation,
            _resizable,
            width,
            height,
            components: Vec::new(),
        }
    }

    /// Gets the ratios of all the component sizes to the total
    fn get_ratios(&self) -> Vec<f64> {
        let total = self.orientation.get_size(self);
        let any_zero = self
            .components
            .iter()
            .any(|c| self.orientation.get_size(c.as_component_simple()) == 0);
        // Return a default ratio list if any sub-component is zero or the
        // container is
        if total == 0 || any_zero {
            return vec![1.0f64].repeat(self.components.len());
        }
        self.components
            .iter()
            .map(|c| self.orientation.get_size(c.as_component_simple()) as f64 / total as f64)
            .collect()
    }

    /// Sets all the components to be proportioned sizes in the container
    pub fn calculate_sizes(ratios: Vec<f64>, total_size: u16) -> Vec<u16> {
        // assert_eq!(self.components.len(), ratios.len());
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

    /// Adds a new component to the container, resizes the existing components
    /// to fit the new component, and returns false if there was no room
    /// available for the component
    pub fn add_component(
        &mut self,
        component: Box<dyn Component>,
    ) -> Result<(), ComponentResizeError> {
        self.components.push(component);
        self.resize(self.get_width(), self.get_height())
    }

    pub fn get_orientation(&self) -> ComponentListOrientation {
        self.orientation.clone()
    }

    pub fn get_components(&self) -> &Vec<Box<dyn Component>> {
        &self.components
    }

    pub fn get_components_mut(&mut self) -> &mut Vec<Box<dyn Component>> {
        &mut self.components
    }
}

impl Component for ComponentList {
    fn as_component_simple(&self) -> &dyn ComponentSimple {
        self
    }

    fn as_component_simple_mut(&mut self) -> &mut dyn ComponentSimple {
        self
    }
}

impl ComponentSimple for ComponentList {
    fn handle_mouse(&mut self, x: u16, y: u16, kind: MouseEventKind) {
        let mouse_pos = ComponentPos { x, y };
        if let Some((component, pos)) = self.locate_component_mut(mouse_pos.clone()) {
            match component.get_focus() {
                ComponentFocus::Focus => {
                    let relative_pos = (mouse_pos - pos).unwrap();
                    component.handle_mouse(relative_pos.x, relative_pos.y, kind);
                    component.invalidate();
                }
                ComponentFocus::PartialFocus | ComponentFocus::None => {
                    component.set_focus(ComponentFocus::Focus);
                    component.invalidate();
                }
            }
        }
    }

    fn handle_key(&mut self, event: KeyEvent) {
        match self.get_focused_component_mut() {
            ComponentFocusResult::Focus((component, _)) => {
                match event.clone().code {
                    KeyCode::Esc => component.set_focus(ComponentFocus::PartialFocus),
                    _ => component.handle_key(event),
                }
                component.invalidate();
            }
            ComponentFocusResult::PartialFocus((component, pos)) => {
                let (width, height) = (component.get_width(), component.get_height());
                let offset = match event.clone().code {
                    KeyCode::Enter => {
                        component.set_focus(ComponentFocus::Focus);
                        component.invalidate();
                        return;
                    }
                    KeyCode::Left => (-1i16, 0i16),
                    KeyCode::Right => (width as i16 + 1i16, 0i16),
                    KeyCode::Up => (0i16, -1i16),
                    KeyCode::Down => (0i16, height as i16 + 1i16),
                    _ => return,
                };
                let Some(pos_next) = pos.clone() + offset else {
                    return;
                };
                component.set_focus(ComponentFocus::None);
                component.invalidate();
                if let Some((component_next, _)) = self.locate_component_mut(pos_next) {
                    component_next.set_focus(ComponentFocus::PartialFocus);
                    component_next.invalidate();
                } else {
                    // There is no component at the target location, undo setting focus to none
                    if let Some((component, _)) = self.locate_component_mut(pos) {
                        component.set_focus(ComponentFocus::PartialFocus);
                        component.invalidate();
                    }
                }
            }
            ComponentFocusResult::None => match event.clone().code {
                KeyCode::Enter | KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                    if let Some((component, _)) =
                        self.locate_component_mut(ComponentPos { x: 0, y: 0 })
                    {
                        component.set_focus(ComponentFocus::PartialFocus);
                        component.invalidate();
                    }
                }
                _ => {}
            },
        }
    }

    fn invalidate(&mut self) {
        for component in &mut self.components {
            component.invalidate();
        }
    }

    fn resize(&mut self, width: u16, height: u16) -> Result<(), ComponentResizeError> {
        if self.width == width && self.height == height {
            return Ok(());
        }
        let ratios = self.get_ratios();
        self.width = width;
        self.height = height;
        let sizes = ComponentList::calculate_sizes(ratios, self.orientation.get_size(self));
        for i in 0..self.components.len() {
            let (width, height) = match self.orientation {
                ComponentListOrientation::Horizontal => (sizes[i], self.get_height()),
                ComponentListOrientation::Vertical => (self.get_width(), sizes[i]),
            };
            self.components[i].resize(width, height)?;
        }
        self.invalidate();
        Ok(())
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        assert_eq!(area.width, self.width);
        assert_eq!(area.height, self.height);
        let mut sub_area = area.clone();
        for component in &mut self.components {
            match self.orientation {
                ComponentListOrientation::Horizontal => sub_area.width = component.get_width(),
                ComponentListOrientation::Vertical => sub_area.height = component.get_height(),
            }
            component.render(sub_area, buf);
            match self.orientation {
                ComponentListOrientation::Horizontal => sub_area.x += component.get_width(),
                ComponentListOrientation::Vertical => sub_area.y += component.get_height(),
            }
        }
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_width(&self) -> u16 {
        self.width
    }

    fn get_height(&self) -> u16 {
        self.height
    }

    fn get_border_width(&self) -> u16 {
        panic!("The border width of containers is undefined!");
    }

    fn get_focus(&self) -> ComponentFocus {
        for component in &self.components {
            match component.get_focus() {
                ComponentFocus::Focus => return ComponentFocus::Focus,
                ComponentFocus::PartialFocus => return ComponentFocus::PartialFocus,
                _ => {}
            }
        }
        ComponentFocus::None
    }

    fn set_focus(&mut self, _: ComponentFocus) {
        panic!("The focus should not be set directly on containers!");
    }
}
