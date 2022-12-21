use crate::tui_layout::finder::*;
use crate::tui_layout::*;
pub(crate) struct TestComponent {
    fill: char,
    name: String,
    width: u16,
    height: u16,
    border_width: u16,
    invalidated: bool,
    focus: ComponentFocus,
}

impl Component for TestComponent {
    fn as_component_simple(&self) -> &dyn ComponentSimple {
        self
    }

    fn as_component_simple_mut(&mut self) -> &mut dyn ComponentSimple {
        self
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ComponentSimple for TestComponent {
    fn handle_mouse(&mut self, _: u16, _: u16, kind: Option<MouseEventKind>) {
        let Some(_) = kind else {
            if self.focus != ComponentFocus::None {
                self.focus = ComponentFocus::None;
                self.invalidate()
            }
            return;
        };
    }

    fn handle_key(&mut self, _: KeyEvent) {}

    fn invalidate(&mut self) {
        self.invalidated = true;
    }

    fn resize(&mut self, width: u16, height: u16) -> Result<(), ComponentResizeError> {
        let min = std::cmp::max(self.get_border_width() * 2, 1);
        if width < min || height < min {
            return Err(ComponentResizeError {
                name: self.name.clone(),
                width,
                height,
                border_width: self.border_width,
            });
        }
        self.width = width;
        self.height = height;
        Ok(())
    }

    fn get_width(&self) -> u16 {
        self.width
    }
    fn get_height(&self) -> u16 {
        self.height
    }
    fn get_border_width(&self) -> u16 {
        self.border_width
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if !self.invalidated {
            return;
        }
        for x in 0..area.width {
            for y in 0..area.height {
                let c = if (x < self.border_width || x >= (area.width - self.border_width))
                    || (y < self.border_width || y >= (area.height - self.border_width))
                {
                    match self.focus {
                        ComponentFocus::Focus => 'f',
                        ComponentFocus::PartialFocus => 'p',
                        ComponentFocus::None => 'n',
                    }
                } else {
                    self.fill
                };
                buf.get_mut(area.x + x, area.y + y).symbol = format!("{}", c);
            }
        }
        self.invalidated = false;
    }

    fn set_focus(&mut self, focus: ComponentFocus) {
        self.focus = focus;
    }
    fn get_focus(&self) -> ComponentFocus {
        self.focus.clone()
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_border(&self, x: u16, y: u16) -> Option<ComponentBorder> {
        if x >= self.get_width() || y >= self.get_height() {
            return None;
        }
        if x < self.get_border_width() {
            Some(ComponentBorder::Left)
        } else if x >= self.get_width() - self.get_border_width() {
            Some(ComponentBorder::Right)
        } else if y < self.get_border_width() {
            Some(ComponentBorder::Top)
        } else if y >= self.get_height() - self.get_border_width() {
            Some(ComponentBorder::Bottom)
        } else {
            None
        }
    }
}

impl ComponentFinder for TestComponent {
    fn get_focused_component(&self) -> ComponentFocusResult<(&dyn Component, ComponentPos)> {
        match self.focus {
            ComponentFocus::Focus => ComponentFocusResult::Focus((self, ComponentPos::default())),
            ComponentFocus::PartialFocus => {
                ComponentFocusResult::PartialFocus((self, ComponentPos::default()))
            }
            ComponentFocus::None => ComponentFocusResult::None,
        }
    }

    fn get_focused_component_mut(
        &mut self,
    ) -> ComponentFocusResult<(&mut dyn Component, ComponentPos)> {
        match self.focus {
            ComponentFocus::Focus => ComponentFocusResult::Focus((self, ComponentPos::default())),
            ComponentFocus::PartialFocus => {
                ComponentFocusResult::PartialFocus((self, ComponentPos::default()))
            }
            ComponentFocus::None => ComponentFocusResult::None,
        }
    }

    fn locate_component(&self, _: ComponentPos) -> Option<(&dyn Component, ComponentPos)> {
        Some((self, ComponentPos::default()))
    }

    fn locate_component_mut(
        &mut self,
        _: ComponentPos,
    ) -> Option<(&mut dyn Component, ComponentPos)> {
        Some((self, ComponentPos::default()))
    }

    fn search_component(&self, path: Vec<String>) -> Option<(&dyn Component, ComponentPos)> {
        if path.len() > 0 {
            return None;
        }
        Some((self, ComponentPos::default()))
    }

    fn search_component_mut(
        &mut self,
        path: Vec<String>,
    ) -> Option<(&mut dyn Component, ComponentPos)> {
        if path.len() > 0 {
            return None;
        }
        Some((self, ComponentPos::default()))
    }
}

#[test]
fn test_component_container() -> Result<(), ComponentResizeError> {
    use crossterm::event::KeyModifiers;

    use crate::tui_layout::tests::*;

    let component_a = TestComponent {
        fill: 'a',
        name: String::from("a"),
        width: 0,
        height: 0,
        border_width: 1,
        invalidated: true,
        focus: ComponentFocus::None,
    };

    let component_b = TestComponent {
        fill: 'b',
        name: String::from("b"),
        width: 0,
        height: 0,
        border_width: 1,
        invalidated: true,
        focus: ComponentFocus::None,
    };

    let component_c = TestComponent {
        fill: 'c',
        name: String::from("c"),
        width: 0,
        height: 0,
        border_width: 1,
        invalidated: true,
        focus: ComponentFocus::None,
    };

    let mut list_vertical = ComponentList::new(
        String::from("vertical"),
        ComponentListOrientation::Vertical,
        ComponentListResizable::True,
        0,
        0,
    );

    let _ = list_vertical.add_component(Box::new(component_a));
    let _ = list_vertical.add_component(Box::new(component_b));

    let mut list_horizontal = ComponentList::new(
        String::from("horizontal"),
        ComponentListOrientation::Horizontal,
        ComponentListResizable::True,
        0,
        0,
    );

    let _ = list_horizontal.add_component(Box::new(list_vertical));
    let _ = list_horizontal.add_component(Box::new(component_c));

    assert_ne!(list_horizontal.resize(20, 0), Ok(()));
    assert_ne!(list_horizontal.resize(0, 0), Ok(()));
    assert_ne!(list_horizontal.resize(20, 1), Ok(()));
    assert_ne!(list_horizontal.resize(1, 20), Ok(()));

    list_horizontal.resize(20, 10)?;
    list_horizontal.resize(32, 8)?;

    let rect = Rect::new(
        0,
        0,
        list_horizontal.get_width(),
        list_horizontal.get_height(),
    );
    let mut buffer = Buffer::empty(rect.clone());
    list_horizontal.render(rect, &mut buffer);

    let expected = [
        "nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn",
        "naaaaaaaaaaaaaannccccccccccccccn",
        "naaaaaaaaaaaaaannccccccccccccccn",
        "nnnnnnnnnnnnnnnnnccccccccccccccn",
        "nnnnnnnnnnnnnnnnnccccccccccccccn",
        "nbbbbbbbbbbbbbbnnccccccccccccccn",
        "nbbbbbbbbbbbbbbnnccccccccccccccn",
        "nnnnnnnnnnnnnnnnnnnnnnnnnnnnnnnn",
    ];

    for y in 0..rect.height {
        for x in 0..rect.width {
            assert_eq!(
                buffer.get(x, y).symbol.as_bytes()[0],
                expected[y as usize].as_bytes()[x as usize]
            );
        }
    }

    let (comp, pos) = list_horizontal
        .search_component("c".split(".").map(|s| String::from(s)).collect())
        .unwrap();
    assert_eq!(comp.get_name(), String::from("c"));
    assert_eq!(pos, ComponentPos { x: 16, y: 0 });

    let (comp, pos) = list_horizontal
        .search_component("vertical.a".split(".").map(|s| String::from(s)).collect())
        .unwrap();
    assert_eq!(comp.get_name(), String::from("a"));
    assert_eq!(pos, ComponentPos { x: 0, y: 0 });

    let (comp, pos) = list_horizontal
        .search_component("vertical.b".split(".").map(|s| String::from(s)).collect())
        .unwrap();
    assert_eq!(comp.get_name(), String::from("b"));
    assert_eq!(pos, ComponentPos { x: 0, y: 4 });

    if let Some(_) =
        list_horizontal.search_component("".split(".").map(|s| String::from(s)).collect())
    {
        panic!("<empty> does not exist!");
    }

    if let Some(_) =
        list_horizontal.search_component("vertical.c".split(".").map(|s| String::from(s)).collect())
    {
        panic!("vertical.c does not exist!");
    }

    if let Some(_) =
        list_horizontal.search_component("vertical.c".split(".").map(|s| String::from(s)).collect())
    {
        panic!("vertical.b.c does not exist!");
    }

    let (comp, pos) = list_horizontal
        .locate_component(ComponentPos { x: 16, y: 0 })
        .unwrap();
    assert_eq!(comp.get_name(), String::from("c"));
    assert_eq!(pos, ComponentPos { x: 16, y: 0 });

    let (comp, pos) = list_horizontal
        .locate_component(ComponentPos { x: 0, y: 0 })
        .unwrap();
    assert_eq!(comp.get_name(), String::from("a"));
    assert_eq!(pos, ComponentPos { x: 0, y: 0 });

    let (comp, pos) = list_horizontal
        .locate_component(ComponentPos { x: 0, y: 4 })
        .unwrap();
    assert_eq!(comp.get_name(), String::from("b"));
    assert_eq!(pos, ComponentPos { x: 0, y: 4 });

    match list_horizontal.get_focused_component() {
        ComponentFocusResult::Focus(_) => panic!("No component should be focused!"),
        ComponentFocusResult::PartialFocus(_) => panic!("No component should be partial focused!"),
        ComponentFocusResult::None => {}
    }

    // Hit enter to partial focus first component
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.get_focused_component() {
        ComponentFocusResult::Focus(_) => panic!("No component should be focused!"),
        ComponentFocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("a"));
            assert_eq!(pos, ComponentPos { x: 0, y: 0 });
        }
        ComponentFocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit enter to focus
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.get_focused_component() {
        ComponentFocusResult::Focus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("a"));
            assert_eq!(pos, ComponentPos { x: 0, y: 0 });
        }
        ComponentFocusResult::PartialFocus(_) => panic!("No component should be partial focused!"),
        ComponentFocusResult::None => panic!("A component should be focused!"),
    }

    // Hit esc to partial focus
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.get_focused_component() {
        ComponentFocusResult::Focus(_) => panic!("No component should be focused!"),
        ComponentFocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("a"));
            assert_eq!(pos, ComponentPos { x: 0, y: 0 });
        }
        ComponentFocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit down to partial focus component below
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Down,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.get_focused_component() {
        ComponentFocusResult::Focus(_) => panic!("No component should be focused!"),
        ComponentFocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("b"));
            assert_eq!(pos, ComponentPos { x: 0, y: 4 });
        }
        ComponentFocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit right to partial focus component right
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Right,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.get_focused_component() {
        ComponentFocusResult::Focus(_) => panic!("No component should be focused!"),
        ComponentFocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("c"));
            assert_eq!(pos, ComponentPos { x: 16, y: 0 });
        }
        ComponentFocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit right to partial focus component right (should not change)
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Right,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.get_focused_component() {
        ComponentFocusResult::Focus(_) => panic!("No component should be focused!"),
        ComponentFocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("c"));
            assert_eq!(pos, ComponentPos { x: 16, y: 0 });
        }
        ComponentFocusResult::None => panic!("A component should be partial focused!"),
    }

    assert_eq!(list_horizontal.get_border(1, 0), Some(ComponentBorder::Top));
    assert_eq!(
        list_horizontal.get_border(0, 1),
        Some(ComponentBorder::Left)
    );
    assert_eq!(
        list_horizontal.get_border(0, 6),
        Some(ComponentBorder::Left)
    );
    assert_eq!(list_horizontal.get_border(15, 1), None);
    assert_eq!(
        list_horizontal.get_border(1, 7),
        Some(ComponentBorder::Bottom)
    );
    assert_eq!(
        list_horizontal.get_border(31, 1),
        Some(ComponentBorder::Right)
    );

    Ok(())
}
