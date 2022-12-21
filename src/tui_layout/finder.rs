use crate::tui_layout::*;

pub trait ComponentFinder {
    fn get_focused_component(&self) -> ComponentFocusResult<(&dyn Component, ComponentPos)>;
    fn get_focused_component_mut(
        &mut self,
    ) -> ComponentFocusResult<(&mut dyn Component, ComponentPos)>;

    fn locate_component(&self, pos: ComponentPos) -> Option<(&dyn Component, ComponentPos)>;
    fn locate_component_mut(
        &mut self,
        pos: ComponentPos,
    ) -> Option<(&mut dyn Component, ComponentPos)>;

    fn search_component(&self, path: Vec<String>) -> Option<(&dyn Component, ComponentPos)>;
    fn search_component_mut(
        &mut self,
        path: Vec<String>,
    ) -> Option<(&mut dyn Component, ComponentPos)>;
}

impl ComponentFinder for ComponentList {
    fn get_focused_component(&self) -> ComponentFocusResult<(&dyn Component, ComponentPos)> {
        let mut offset = ComponentPos { x: 0, y: 0 };
        for component in &self.components {
            let (width, height) = (component.get_width(), component.get_height());
            match component.get_focused_component() {
                ComponentFocusResult::Focus((component, pos)) => {
                    return ComponentFocusResult::Focus((component, pos + offset));
                }
                ComponentFocusResult::PartialFocus((component, pos)) => {
                    return ComponentFocusResult::PartialFocus((component, pos + offset));
                }
                ComponentFocusResult::None => {}
            }
            offset = offset + self.orientation.get_offset(width, height);
        }
        ComponentFocusResult::None
    }

    fn get_focused_component_mut(
        &mut self,
    ) -> ComponentFocusResult<(&mut dyn Component, ComponentPos)> {
        let mut offset = ComponentPos { x: 0, y: 0 };
        for component in &mut self.components {
            let (width, height) = (component.get_width(), component.get_height());
            match component.get_focused_component_mut() {
                ComponentFocusResult::Focus((component, pos)) => {
                    return ComponentFocusResult::Focus((component, pos + offset));
                }
                ComponentFocusResult::PartialFocus((component, pos)) => {
                    return ComponentFocusResult::PartialFocus((component, pos + offset));
                }
                ComponentFocusResult::None => {}
            }
            offset = offset + self.orientation.get_offset(width, height);
        }
        ComponentFocusResult::None
    }

    fn locate_component(&self, pos: ComponentPos) -> Option<(&dyn Component, ComponentPos)> {
        let pos_rect = Rect::from(pos.clone());
        let mut offset = ComponentPos { x: 0, y: 0 };
        for component in &self.components {
            let (width, height) = (component.get_width(), component.get_height());
            let component_rect = offset.clone().into_rect(width, height);
            if component_rect.intersects(pos_rect) {
                let pos = (pos.clone() - offset.clone()).unwrap();
                if let Some((component, pos)) = component.locate_component(pos) {
                    return Some((component, pos + offset));
                } else {
                    return None;
                }
            }
            offset = offset + self.orientation.get_offset(width, height)
        }
        None
    }

    fn locate_component_mut(
        &mut self,
        pos: ComponentPos,
    ) -> Option<(&mut dyn Component, ComponentPos)> {
        let pos_rect = Rect::from(pos.clone());
        let mut offset = ComponentPos { x: 0, y: 0 };
        for component in &mut self.components {
            let (width, height) = (component.get_width(), component.get_height());
            let component_rect = offset.clone().into_rect(width, height);
            if component_rect.intersects(pos_rect) {
                let pos = (pos.clone() - offset.clone()).unwrap();
                if let Some((component, pos)) = component.locate_component_mut(pos) {
                    return Some((component, pos + offset));
                } else {
                    return None;
                }
            }
            offset = offset + self.orientation.get_offset(width, height)
        }
        None
    }

    fn search_component(&self, path: Vec<String>) -> Option<(&dyn Component, ComponentPos)> {
        if path.len() == 0 {
            return None;
        }
        let mut offset = ComponentPos { x: 0, y: 0 };
        for component in &self.components {
            let (width, height) = (component.get_width(), component.get_height());
            if path[0] == component.get_name() {
                if let Some((component, pos)) = component.search_component(path[1..].to_vec()) {
                    return Some((component, pos + offset));
                }
            }
            offset = offset + self.orientation.get_offset(width, height)
        }
        None
    }

    fn search_component_mut(
        &mut self,
        path: Vec<String>,
    ) -> Option<(&mut dyn Component, ComponentPos)> {
        if path.len() == 0 {
            return None;
        }
        let mut offset = ComponentPos { x: 0, y: 0 };
        for component in &mut self.components {
            let (width, height) = (component.get_width(), component.get_height());
            if path[0] == component.get_name() {
                if let Some((component, pos)) = component.search_component_mut(path[1..].to_vec()) {
                    return Some((component, pos + offset));
                }
            }
            offset = offset + self.orientation.get_offset(width, height)
        }
        None
    }
}
