use std::cmp::Ordering;

// Displays hierarchical data
use tui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Paragraph, Widget},
};

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

pub struct BrowserNode<E> {
    entry: Option<E>,
    expanded: bool,
    children: Vec<BrowserNode<E>>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BrowserNodePath(Vec<usize>);

impl<E> BrowserNode<E> {
    pub fn new(entry: Option<E>) -> Self {
        Self {
            entry,
            expanded: false,
            children: Vec::new(),
        }
    }

    pub fn from(entry: Option<E>, children: Vec<BrowserNode<E>>) -> Self {
        Self {
            entry,
            expanded: false,
            children,
        }
    }

    pub fn is_parent(&self) -> bool {
        self.children.len() > 0
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    pub fn get_children(&self) -> &Vec<BrowserNode<E>> {
        &self.children
    }

    pub fn get_children_mut(&mut self) -> &mut Vec<BrowserNode<E>> {
        &mut self.children
    }

    pub fn get_entry(&self) -> &Option<E> {
        &self.entry
    }

    pub fn get_entry_mut(&mut self) -> &mut Option<E> {
        &mut self.entry
    }

    pub fn get_render_len(&self) -> usize {
        (if self.expanded {
            self.children
                .iter()
                .map(|c| c.get_render_len())
                .sum::<usize>()
        } else {
            0
        }) + (if let Some(_) = &self.entry { 1 } else { 0 })
    }

    pub fn get_path(&self, index: usize) -> BrowserNodePath {
        let mut index = index;
        for (i, c) in (&self.children).iter().enumerate() {
            if index == 0 {
                return BrowserNodePath(vec![i]);
            } else if index < c.get_render_len() {
                let mut v = vec![i];
                v.append(&mut c.get_path(index - 1).0);
                return BrowserNodePath(v);
            } else {
                index -= c.get_render_len();
            }
        }
        BrowserNodePath(Vec::new())
    }

    pub fn get_paths(&self, range: std::ops::Range<usize>) -> Vec<BrowserNodePath> {
        let mut paths: Vec<BrowserNodePath> = Vec::new();
        for index in range {
            let path = self.get_path(index);
            if let Some(last_path) = paths.last() {
                if !last_path.contains(&path) {
                    paths.push(path);
                }
            } else {
                paths.push(path);
            }
        }
        paths
    }

    pub fn get_node(&self, path: &BrowserNodePath) -> Option<&BrowserNode<E>> {
        if path.0.len() == 0 {
            None
        } else if path.0.len() == 1 {
            self.children.get(path.0[0])
        } else {
            self.children[path.0[0]].get_node(&BrowserNodePath(path.0[1..].to_vec()))
        }
    }
}

impl<E> std::ops::Index<usize> for BrowserNode<E> {
    type Output = BrowserNode<E>;
    fn index<'a>(&'a self, i: usize) -> &'a BrowserNode<E> {
        &self.children[i]
    }
}

impl<E> std::ops::IndexMut<usize> for BrowserNode<E> {
    fn index_mut<'a>(&'a mut self, i: usize) -> &'a mut BrowserNode<E> {
        &mut self.children[i]
    }
}

impl BrowserNodePath {
    pub fn condense_paths(paths: Vec<Self>) -> Vec<Self> {
        let mut paths = paths.clone();
        paths.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut i = 0;
        while i < paths.len() - 1 {
            if paths[i].contains(&paths[i + 1]) {
                paths.remove(i + 1);
            } else {
                i += 1
            }
        }
        paths
    }

    // Returns true if self contains other in the hierarchy (comparing the same
    // paths results in true as well)
    pub fn contains(&self, other: &Self) -> bool {
        if other.is_empty() {
            // Cannot contain an empty path
            return false;
        }
        for (i, index) in (&self.0).iter().enumerate() {
            match other.0.get(i) {
                Some(other_index) => {
                    if other_index != index {
                        // self and other has mismatched indices at some level
                        return false;
                    }
                }
                // self is more specific than other, cannot contain it
                None => return false,
            }
        }
        true
    }

    pub fn is_empty(&self) -> bool {
        self.0.len() == 0
    }
}

impl PartialOrd for BrowserNodePath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        for (i, a) in self.0.iter().enumerate() {
            match other.0.get(i) {
                Some(b) => {
                    if a > b {
                        return Some(Ordering::Greater);
                    } else if a < b {
                        return Some(Ordering::Less);
                    }
                }
                None => return Some(Ordering::Greater),
            }
        }
        if self.0.len() < other.0.len() {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

pub struct BrowserState {
    // Enables display up and down arrows at the top and bottom of the component
    // to indicate if scrolling is available
    display_bounds: bool,
    // Enables indenting for hierarchical entries
    display_indent: bool,
    // Enables displaying full hierarchical names
    display_full: bool,
    scroll: isize,
    cursor: isize,
    cursor_secondary: Option<isize>,
}

impl BrowserState {
    pub fn new(display_bounds: bool, display_indent: bool, display_full: bool) -> Self {
        Self {
            display_bounds,
            display_indent,
            display_full,
            scroll: 0,
            cursor: 0,
            cursor_secondary: None,
        }
    }

    pub fn get_selected<E>(&self, node: &BrowserNode<E>) -> Vec<BrowserNodePath> {
        todo!()
    }
}

pub struct Browser<'a, 'b, E> {
    state: &'a BrowserState,
    node: &'b BrowserNode<E>,
}

impl<'a, 'b, E> Browser<'a, 'b, E> {
    pub fn new(state: &'a BrowserState, node: &'b BrowserNode<E>) -> Self {
        Self { state, node }
    }
}

impl<'a, 'b, E> Widget for Browser<'a, 'b, E>
where
    E: std::fmt::Display,
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        let height = if self.state.display_bounds {
            if area.height < 2 {
                return;
            } else {
                area.height - 2
            }
        } else {
            area.height
        };
        let mut text = Text::raw("");
        if self.state.display_bounds {
            if self.state.scroll > 0 {
                text.extend(Text::raw("↑↑↑↑"));
            } else {
                text.extend(Text::raw("    "));
            }
        }
        for line_index in self.state.scroll..(self.state.scroll + height as isize) {
            let path = self.node.get_path(line_index as usize);
            let sub_node = if let Some(sub_node) = self.node.get_node(&path) {
                sub_node
            } else {
                text.extend(Text::raw("    "));
                continue;
            };
            let indents = if self.state.display_indent {
                "    ".repeat(path.0.len() - 1)
            } else {
                String::new()
            };
            let expander = if sub_node.is_parent() {
                if sub_node.is_expanded() {
                    "[-] "
                } else {
                    "[+] "
                }
            } else {
                ""
            };
            let content = if let Some(entry) = sub_node.get_entry() {
                entry.to_string()
            } else {
                String::new()
            };
            let node_raw = format!("{}{}{}", indents, expander, content);
            let padding = String::from(" ").repeat(if node_raw.len() < area.width as usize {
                area.width as usize - node_raw.len()
            } else {
                0
            });
            let node_raw = format!("{}{}", node_raw, padding);
            let is_selected = if let Some(cursor_secondary) = self.state.cursor_secondary {
                if self.state.cursor > cursor_secondary {
                    cursor_secondary <= line_index && line_index <= self.state.cursor
                } else {
                    self.state.cursor <= line_index && line_index <= cursor_secondary
                }
            } else {
                line_index == self.state.cursor
            };
            let is_primary_selected = line_index == self.state.cursor;
            text.extend(Text::styled(
                node_raw,
                get_selected_style(is_selected, is_primary_selected),
            ));
        }
        if self.state.display_bounds {
            if self.node.get_render_len() as isize - self.state.scroll > height as isize {
                text.extend(Text::raw("↓↓↓↓"));
            } else {
                text.extend(Text::raw("    "));
            }
        }

        Paragraph::new(text).render(area, buf)
    }
}

#[test]
fn browser_node_test() {
    let mut nodes = BrowserNode::from(
        None,
        vec![
            BrowserNode::from(
                Some("A"),
                vec![
                    BrowserNode::from(
                        Some("1"),
                        vec![
                            BrowserNode::from(Some("a"), vec![]),
                            BrowserNode::from(Some("b"), vec![]),
                        ],
                    ),
                    BrowserNode::from(Some("2"), vec![]),
                ],
            ),
            BrowserNode::from(Some("B"), vec![BrowserNode::from(Some("1"), vec![])]),
            BrowserNode::from(
                Some("C"),
                vec![
                    BrowserNode::from(Some("1"), vec![]),
                    BrowserNode::from(Some("2"), vec![]),
                    BrowserNode::from(Some("3"), vec![]),
                ],
            ),
        ],
    );
    nodes.set_expanded(true);

    assert_eq!(nodes.get_render_len(), 3);

    nodes[0].set_expanded(true);
    assert_eq!(nodes.get_render_len(), 5);

    assert_eq!(nodes.get_path(0), BrowserNodePath(vec![0]));
    assert_eq!(nodes.get_path(1), BrowserNodePath(vec![0, 0]));
    assert_eq!(nodes.get_path(2), BrowserNodePath(vec![0, 1]));
    assert_eq!(nodes.get_path(3), BrowserNodePath(vec![1]));
    assert_eq!(nodes.get_path(4), BrowserNodePath(vec![2]));

    assert_eq!(
        nodes.get_paths(1..4),
        vec![
            BrowserNodePath(vec![0, 0]),
            BrowserNodePath(vec![0, 1]),
            BrowserNodePath(vec![1]),
        ]
    );

    assert_eq!(
        nodes.get_paths(0..4),
        vec![BrowserNodePath(vec![0]), BrowserNodePath(vec![1]),]
    );

    nodes[0][0].set_expanded(true);
    assert_eq!(nodes.get_path(2), BrowserNodePath(vec![0, 0, 0]));

    assert!(!BrowserNodePath(vec![]).contains(&BrowserNodePath(vec![])));
    assert!(!BrowserNodePath(vec![0]).contains(&BrowserNodePath(vec![])));
    assert!(BrowserNodePath(vec![]).contains(&BrowserNodePath(vec![0])));
    assert!(BrowserNodePath(vec![0]).contains(&BrowserNodePath(vec![0])));

    assert!(BrowserNodePath(vec![0]).contains(&BrowserNodePath(vec![0, 0])));
    assert!(BrowserNodePath(vec![0]).contains(&BrowserNodePath(vec![0, 1])));

    assert!(!BrowserNodePath(vec![0, 0]).contains(&BrowserNodePath(vec![0])));
    assert!(!BrowserNodePath(vec![0, 1]).contains(&BrowserNodePath(vec![0])));

    assert!(!BrowserNodePath(vec![1, 0]).contains(&BrowserNodePath(vec![2, 0])));

    assert!(nodes.get_node(&BrowserNodePath(vec![])).is_none());
    assert_eq!(
        nodes
            .get_node(&BrowserNodePath(vec![0]))
            .unwrap()
            .get_entry(),
        &Some("A")
    );
    assert_eq!(
        nodes
            .get_node(&BrowserNodePath(vec![1]))
            .unwrap()
            .get_entry(),
        &Some("B")
    );
    assert_eq!(
        nodes
            .get_node(&BrowserNodePath(vec![0, 1]))
            .unwrap()
            .get_entry(),
        &Some("2")
    );

    let mut paths = vec![
        BrowserNodePath(vec![0]),
        BrowserNodePath(vec![1, 0]),
        BrowserNodePath(vec![1]),
        BrowserNodePath(vec![2, 3]),
        BrowserNodePath(vec![2]),
        BrowserNodePath(vec![2, 4, 5]),
        BrowserNodePath(vec![3, 4, 5]),
        BrowserNodePath(vec![3, 4]),
    ];
    paths = BrowserNodePath::condense_paths(paths);
    assert_eq!(
        paths,
        vec![
            BrowserNodePath(vec![0]),
            BrowserNodePath(vec![1]),
            BrowserNodePath(vec![2]),
            BrowserNodePath(vec![3, 4]),
        ]
    );
}

#[test]
fn browser_render_test() {
    let mut nodes = BrowserNode::from(
        None,
        vec![
            BrowserNode::from(
                Some("A"),
                vec![
                    BrowserNode::from(
                        Some("1"),
                        vec![
                            BrowserNode::from(Some("a"), vec![]),
                            BrowserNode::from(Some("b"), vec![]),
                        ],
                    ),
                    BrowserNode::from(Some("2"), vec![]),
                ],
            ),
            BrowserNode::from(Some("B"), vec![BrowserNode::from(Some("1"), vec![])]),
            BrowserNode::from(
                Some("C"),
                vec![
                    BrowserNode::from(Some("1"), vec![]),
                    BrowserNode::from(Some("2"), vec![]),
                    BrowserNode::from(Some("3"), vec![]),
                ],
            ),
        ],
    );
    nodes.set_expanded(true);
    nodes[0].set_expanded(true);
    nodes[0][0].set_expanded(true);

    let browser_state = BrowserState::new(true, true, true);

    let browser = Browser::new(&browser_state, &nodes);

    let text = browser.render(
        Rect::new(0, 0, 10, 10),
        &mut Buffer::empty(Rect::new(0, 0, 10, 10)),
    );
}
