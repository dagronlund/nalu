#[test]
fn test_component_container() -> Result<(), crate::tui_layout::ResizeError> {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
    use tui::{
        buffer::Buffer,
        layout::{Direction, Rect},
    };

    use crate::tui_layout::{
        component::{Component, ComponentBase, ComponentWidget},
        container::list::ContainerList,
        container::search::ContainerSearch,
        container::Container,
        pos::ComponentPos,
        Border, FocusResult,
    };

    struct TestComponentWidget {
        fill: char,
    }

    impl ComponentWidget for TestComponentWidget {
        fn handle_mouse(&mut self, _: u16, _: u16, _: MouseEventKind) {}
        fn handle_key(&mut self, _: KeyEvent) {}
        fn resize(&mut self, _: u16, _: u16) {}
        fn render(&mut self, area: Rect, buf: &mut Buffer) {
            for x in 0..area.width {
                for y in 0..area.height {
                    buf.get_mut(area.x + x, area.y + y).symbol = format!("{}", self.fill);
                }
            }
        }
    }

    let component_a = Component::new(
        String::from("a"),
        1,
        Box::new(TestComponentWidget { fill: 'a' }),
    );

    let component_b = Component::new(
        String::from("b"),
        1,
        Box::new(TestComponentWidget { fill: 'b' }),
    );

    let component_c = Component::new(
        String::from("c"),
        1,
        Box::new(TestComponentWidget { fill: 'c' }),
    );

    let mut list_vertical =
        ContainerList::new(String::from("vertical"), Direction::Vertical, true, 0, 0);

    let _ = list_vertical.add_component(component_a);
    let _ = list_vertical.add_component(component_b);

    let mut list_horizontal = ContainerList::new(
        String::from("horizontal"),
        Direction::Horizontal,
        true,
        0,
        0,
    );

    let _ = list_horizontal.add_container(Box::new(list_vertical));
    let _ = list_horizontal.add_component(component_c);

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
        "╭a─────────────╮╭c─────────────╮",
        "│aaaaaaaaaaaaaa││cccccccccccccc│",
        "│aaaaaaaaaaaaaa││cccccccccccccc│",
        "╰──────────────╯│cccccccccccccc│",
        "╭b─────────────╮│cccccccccccccc│",
        "│bbbbbbbbbbbbbb││cccccccccccccc│",
        "│bbbbbbbbbbbbbb││cccccccccccccc│",
        "╰──────────────╯╰──────────────╯",
    ];

    for y in 0..rect.height {
        for x in 0..rect.width {
            assert_eq!(
                buffer.get(x, y).symbol.chars().nth(0).unwrap(),
                expected[y as usize].chars().nth(x as usize).unwrap()
            );
        }
    }

    let (comp, pos) = list_horizontal
        .as_container()
        .search_name("c".split(".").map(|s| String::from(s)).collect())
        .unwrap();
    assert_eq!(comp.as_base().get_name(), String::from("c"));
    assert_eq!(pos, ComponentPos { x: 16, y: 0 });

    let (comp, pos) = list_horizontal
        .as_container()
        .search_name("vertical.a".split(".").map(|s| String::from(s)).collect())
        .unwrap();
    assert_eq!(comp.as_base().get_name(), String::from("a"));
    assert_eq!(pos, ComponentPos { x: 0, y: 0 });

    let (comp, pos) = list_horizontal
        .as_container()
        .search_name("vertical.b".split(".").map(|s| String::from(s)).collect())
        .unwrap();
    assert_eq!(comp.as_base().get_name(), String::from("b"));
    assert_eq!(pos, ComponentPos { x: 0, y: 4 });

    if let Some(_) = list_horizontal
        .as_container()
        .search_name("".split(".").map(|s| String::from(s)).collect())
    {
        panic!("<empty> does not exist!");
    }

    if let Some(_) = list_horizontal
        .as_container()
        .search_name("vertical.c".split(".").map(|s| String::from(s)).collect())
    {
        panic!("vertical.c does not exist!");
    }

    if let Some(_) = list_horizontal
        .as_container()
        .search_name("vertical.c".split(".").map(|s| String::from(s)).collect())
    {
        panic!("vertical.b.c does not exist!");
    }

    let (comp, pos) = list_horizontal
        .as_container()
        .search_position(ComponentPos { x: 16, y: 0 })
        .unwrap();
    assert_eq!(comp.get_name(), String::from("c"));
    assert_eq!(pos, ComponentPos { x: 16, y: 0 });

    let (comp, pos) = list_horizontal
        .as_container()
        .search_position(ComponentPos { x: 0, y: 0 })
        .unwrap();
    assert_eq!(comp.get_name(), String::from("a"));
    assert_eq!(pos, ComponentPos { x: 0, y: 0 });

    let (comp, pos) = list_horizontal
        .as_container()
        .search_position(ComponentPos { x: 0, y: 4 })
        .unwrap();
    assert_eq!(comp.get_name(), String::from("b"));
    assert_eq!(pos, ComponentPos { x: 0, y: 4 });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus(_) => panic!("No component should be partial focused!"),
        FocusResult::None => {}
    }

    // Hit enter to partial focus first component
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("a"));
            assert_eq!(pos, ComponentPos { x: 0, y: 0 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit enter to focus
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Enter,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("a"));
            assert_eq!(pos, ComponentPos { x: 0, y: 0 });
        }
        FocusResult::PartialFocus(_) => panic!("No component should be partial focused!"),
        FocusResult::None => panic!("A component should be focused!"),
    }

    // Hit esc to partial focus
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Esc,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("a"));
            assert_eq!(pos, ComponentPos { x: 0, y: 0 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit down to partial focus component below
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Down,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("b"));
            assert_eq!(pos, ComponentPos { x: 0, y: 4 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit right to partial focus component right
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Right,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("c"));
            assert_eq!(pos, ComponentPos { x: 16, y: 0 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    // Hit right to partial focus component right (should not change)
    list_horizontal.handle_key(KeyEvent {
        code: KeyCode::Right,
        modifiers: KeyModifiers::empty(),
    });

    match list_horizontal.as_container().search_focused() {
        FocusResult::Focus(_) => panic!("No component should be focused!"),
        FocusResult::PartialFocus((comp, pos)) => {
            assert_eq!(comp.get_name(), String::from("c"));
            assert_eq!(pos, ComponentPos { x: 16, y: 0 });
        }
        FocusResult::None => panic!("A component should be partial focused!"),
    }

    assert_eq!(list_horizontal.get_border(1, 0), Some(Border::Top));
    assert_eq!(list_horizontal.get_border(0, 1), Some(Border::Left));
    assert_eq!(list_horizontal.get_border(0, 6), Some(Border::Left));
    assert_eq!(list_horizontal.get_border(15, 1), None);
    assert_eq!(list_horizontal.get_border(1, 7), Some(Border::Bottom));
    assert_eq!(list_horizontal.get_border(31, 1), Some(Border::Right));

    Ok(())
}
