pub mod list;
pub mod search;

use tui::layout::Rect;

use crate::tui_layout::component::{Component, ComponentBase};

pub enum ContainerChild {
    Container(Box<dyn Container>),
    Component(Component),
}

impl ContainerChild {
    pub fn as_base(&self) -> &dyn ComponentBase {
        match self {
            Self::Container(container) => container.as_base(),
            Self::Component(component) => component,
        }
    }

    pub fn as_base_mut(&mut self) -> &mut dyn ComponentBase {
        match self {
            Self::Container(container) => container.as_base_mut(),
            Self::Component(component) => component,
        }
    }
}

// // TODO: Lifetime issues?
// impl std::ops::Deref for ContainerChild {
//     type Target = dyn ComponentBase;

//     fn deref(&self) -> &Self::Target {
//         match self {
//             Self::Container(container) => container.as_base(),
//             Self::Component(component) => component,
//         }
//     }
// }

pub trait Container
where
    Self: ComponentBase,
{
    fn get_children(&self) -> &Vec<ContainerChild>;
    fn get_children_mut(&mut self) -> &mut Vec<ContainerChild>;
    fn get_children_rectangles(&self) -> Vec<Rect>;

    fn as_base(&self) -> &dyn ComponentBase;
    fn as_base_mut(&mut self) -> &mut dyn ComponentBase;

    fn is_resizable(&self) -> bool;

    fn as_container(&self) -> &dyn Container;
    fn as_container_mut(&mut self) -> &mut dyn Container;
}
