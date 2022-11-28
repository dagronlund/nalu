use std::cmp::Ordering;

use tui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Paragraph, Widget},
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

#[allow(dead_code)]
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

    pub fn from_expanded(entry: Option<E>, expanded: bool, children: Vec<BrowserNode<E>>) -> Self {
        Self {
            entry,
            expanded,
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

    pub fn get_paths(&self, range: std::ops::Range<usize>, condense: bool) -> Vec<BrowserNodePath> {
        let mut paths: Vec<BrowserNodePath> = Vec::new();
        for index in range {
            let path = self.get_path(index);
            if let Some(last_path) = paths.last() {
                if !condense || !last_path.contains(&path) {
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

    pub fn get_node_mut(&mut self, path: &BrowserNodePath) -> Option<&mut BrowserNode<E>> {
        if path.0.len() == 0 {
            None
        } else if path.0.len() == 1 {
            self.children.get_mut(path.0[0])
        } else {
            self.children[path.0[0]].get_node_mut(&BrowserNodePath(path.0[1..].to_vec()))
        }
    }
}

impl<E> BrowserNode<E>
where
    E: std::fmt::Display,
{
    pub fn get_full_name(&self, path: &BrowserNodePath) -> Vec<String> {
        let mut name = if let Some(entry) = &self.entry {
            vec![entry.to_string()]
        } else {
            Vec::new()
        };
        let mut suffix = if path.0.len() == 0 {
            Vec::new()
        } else {
            self.children[path.0[0]].get_full_name(&BrowserNodePath(path.0[1..].to_vec()))
        };
        name.append(&mut suffix);
        name
    }
}

impl<E> std::fmt::Display for BrowserNode<E>
where
    E: std::fmt::Display,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(entry) = &self.entry {
            write!(fmt, "{}", entry)
        } else {
            write!(fmt, "")
        }
    }
}

impl<E> Default for BrowserNode<E> {
    fn default() -> Self {
        Self {
            entry: None,
            expanded: false,
            children: Vec::new(),
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

#[allow(dead_code)]
impl BrowserNodePath {
    pub fn new(path: Vec<usize>) -> Self {
        Self(path)
    }

    pub fn to_vec(self) -> Vec<usize> {
        self.0
    }

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
    bounds_enabled: bool,
    // Enables indenting for hierarchical entries
    indent_enabled: bool,
    // Enables displaying full hierarchical names
    full_name_enabled: bool,
    scroll: isize,
    cursor: isize,
    cursor_secondary: Option<isize>,
    height: isize,
}

#[allow(dead_code)]
impl BrowserState {
    pub fn new(bounds_enabled: bool, indent_enabled: bool, full_name_enabled: bool) -> Self {
        Self {
            bounds_enabled,
            indent_enabled,
            full_name_enabled,
            scroll: 0,
            cursor: 0,
            cursor_secondary: None,
            height: 0,
        }
    }

    pub fn get_selected_range(&self) -> std::ops::Range<usize> {
        if let Some(cursor_secondary) = self.cursor_secondary {
            if self.cursor > cursor_secondary {
                cursor_secondary as usize..(self.cursor + 1) as usize
            } else {
                self.cursor as usize..(cursor_secondary + 1) as usize
            }
        } else {
            self.cursor as usize..(self.cursor + 1) as usize
        }
    }

    pub fn get_primary_selected(&self) -> usize {
        self.cursor as usize
    }

    pub fn get_visible_paths<E>(&self, node: &BrowserNode<E>) -> Vec<BrowserNodePath> {
        node.get_paths(
            self.scroll as usize..(self.scroll + self.height) as usize,
            false,
        )
    }

    pub fn get_selected_paths<E>(
        &self,
        node: &BrowserNode<E>,
        condense: bool,
    ) -> Vec<BrowserNodePath> {
        node.get_paths(self.get_selected_range(), condense)
    }

    pub fn get_primary_selected_path<E>(&self, node: &BrowserNode<E>) -> BrowserNodePath {
        node.get_path(self.get_primary_selected())
    }

    fn clamp_scroll(&mut self, render_height: isize) {
        if self.cursor < self.scroll {
            self.scroll = self.cursor;
        } else if self.cursor > self.scroll + (render_height - 1) {
            self.scroll = self.cursor - (render_height - 1);
        }
    }

    pub fn scroll_relative<E>(&mut self, node: &BrowserNode<E>, delta: isize) {
        let node_height = node.get_render_len();
        let render_height = (self.height - if self.bounds_enabled { 2 } else { 0 }).max(0);
        self.scroll = (self.scroll + delta).clamp(0, (node_height as isize - 1).max(0));
        self.clamp_scroll(render_height);
    }

    pub fn select_absolute<E>(
        &mut self,
        node: &BrowserNode<E>,
        render_offset: isize,
        primary: bool,
    ) -> bool {
        let render_height = (self.height - if self.bounds_enabled { 2 } else { 0 }).max(0);
        let node_height = node.get_render_len();
        let offset = if self.bounds_enabled {
            if render_offset < 1 || render_offset > render_height {
                return false;
            } else {
                render_offset + self.scroll - 1
            }
        } else {
            render_offset + self.scroll
        };
        if offset > node_height as isize {
            return false;
        }
        if primary {
            self.cursor_secondary = None;
            if self.cursor == offset {
                true
            } else {
                self.cursor = offset;
                false
            }
        } else {
            self.cursor_secondary = Some(self.cursor_secondary.unwrap_or(self.cursor));
            false
        }
    }

    pub fn select_relative<E>(&mut self, node: &BrowserNode<E>, delta: isize, primary: bool) {
        let render_height = (self.height - if self.bounds_enabled { 2 } else { 0 }).max(0);
        let node_height = node.get_render_len();
        self.cursor_secondary = if primary {
            None
        } else {
            Some(self.cursor_secondary.unwrap_or(self.cursor))
        };
        self.cursor = (self.cursor + delta).clamp(0, (node_height as isize - 1).max(0));
        self.clamp_scroll(render_height);
    }

    pub fn get_height(&self) -> isize {
        self.height
    }

    pub fn set_height(&mut self, height: isize) {
        self.height = height;
    }

    pub fn is_bounds_enabled(&self) -> bool {
        self.bounds_enabled
    }

    pub fn is_indent_enabled(&self) -> bool {
        self.indent_enabled
    }

    pub fn is_full_name_enabled(&self) -> bool {
        self.full_name_enabled
    }

    pub fn set_bounds_enabled(&mut self, bounds_enabled: bool) {
        self.bounds_enabled = bounds_enabled
    }

    pub fn set_indent_enabled(&mut self, indent_enabled: bool) {
        self.indent_enabled = indent_enabled
    }

    pub fn set_full_name_enabled(&mut self, full_name_enabled: bool) {
        self.full_name_enabled = full_name_enabled
    }
}

pub struct Browser<'a, E> {
    /// The scroll and selection status of the component
    state: &'a BrowserState,
    /// The root node to render
    node: &'a BrowserNode<E>,
    /// A block to wrap the widget in
    block: Option<Block<'a>>,
    /// Widget style
    style: Style,
}

impl<'a, E> Browser<'a, E> {
    pub fn new(state: &'a BrowserState, node: &'a BrowserNode<E>) -> Self {
        Self {
            state,
            node,
            block: None,
            style: Default::default(),
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl<'a, E> Widget for Browser<'a, E>
where
    E: std::fmt::Display,
{
    fn render(self, area: Rect, buf: &mut Buffer) {
        let height = if self.state.bounds_enabled {
            if area.height < 2 {
                return;
            } else {
                area.height - 2
            }
        } else {
            area.height
        };
        let mut text = Text::raw("");
        let line_range = if self.state.bounds_enabled {
            if self.state.scroll > 0 {
                text.extend(Text::raw("↑".repeat(area.width as usize)));
            } else {
                text.extend(Text::raw(" ".repeat(area.width as usize)));
            }
            self.state.scroll..(self.state.scroll + height as isize - 2)
        } else {
            self.state.scroll..(self.state.scroll + height as isize)
        };
        for line_index in line_range {
            let path = self.node.get_path(line_index as usize);
            let sub_node = if let Some(sub_node) = self.node.get_node(&path) {
                sub_node
            } else {
                text.extend(Text::raw("    "));
                continue;
            };
            let indents = if self.state.indent_enabled {
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
            let content = if self.state.full_name_enabled {
                self.node.get_full_name(&path).join(".")
            } else {
                if let Some(entry) = sub_node.get_entry() {
                    entry.to_string()
                } else {
                    String::new()
                }
            };
            let node_raw = format!("{}{}{}", indents, expander, content);
            let padding = String::from(" ").repeat(if node_raw.len() < area.width as usize {
                area.width as usize - node_raw.len()
            } else {
                0
            });
            let node_raw = format!("{}{}", node_raw, padding);
            let is_selected = self
                .state
                .get_selected_range()
                .contains(&(line_index as usize));
            let is_primary_selected = line_index == self.state.get_primary_selected() as isize;
            text.extend(Text::styled(
                node_raw,
                get_selected_style(is_selected, is_primary_selected),
            ));
        }
        if self.state.bounds_enabled {
            if self.node.get_render_len() as isize - self.state.scroll > height as isize - 2 {
                text.extend(Text::raw("↓".repeat(area.width as usize)));
            } else {
                text.extend(Text::raw(" ".repeat(area.width as usize)));
            }
        }

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Left)
            .style(self.style);
        if let Some(block) = self.block {
            paragraph.block(block)
        } else {
            paragraph
        }
        .render(area, buf)
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
        nodes.get_paths(1..4, true),
        vec![
            BrowserNodePath(vec![0, 0]),
            BrowserNodePath(vec![0, 1]),
            BrowserNodePath(vec![1]),
        ]
    );

    assert_eq!(
        nodes.get_paths(0..4, true),
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

    assert!(nodes.get_full_name(&BrowserNodePath(vec![])).is_empty());
    assert_eq!(nodes.get_full_name(&BrowserNodePath(vec![0])), vec!["A"]);
    assert_eq!(
        nodes.get_full_name(&BrowserNodePath(vec![0, 0])),
        vec!["A", "1"]
    );
    assert_eq!(
        nodes.get_full_name(&BrowserNodePath(vec![0, 1])),
        vec!["A", "2"]
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

    let _text = browser.render(
        Rect::new(0, 0, 10, 10),
        &mut Buffer::empty(Rect::new(0, 0, 10, 10)),
    );
}
