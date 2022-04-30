use tui::text::Text;

use crate::state::utils::*;

pub struct TreeNode<T> {
    value: T,
    expanded: bool,
    nodes: TreeNodes<T>,
}

pub struct TreeNodes<T> {
    nodes: Vec<TreeNode<T>>,
}

impl<T> TreeNode<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: value,
            expanded: false,
            nodes: TreeNodes::new(),
        }
    }

    pub fn new_from(value: T, expanded: bool, nodes: TreeNodes<T>) -> Self {
        Self {
            value: value,
            expanded: expanded,
            nodes: nodes,
        }
    }

    pub fn get_selected_mut<'a>(
        &'a mut self,
        select_offset: &mut isize,
    ) -> Option<&'a mut TreeNode<T>> {
        match *select_offset {
            0 => {
                *select_offset -= 1;
                return Some(self);
            }
            1.. => *select_offset -= 1,
            _ => return None,
        }
        if self.expanded {
            for node in self.nodes.get_nodes_mut() {
                match node.get_selected_mut(select_offset) {
                    Some(node) => return Some(node),
                    None => {}
                }
            }
        }
        None
    }

    pub fn rendered_len(&self) -> usize {
        let mut len = 1;
        if self.expanded {
            for node in self.nodes.get_nodes() {
                len += node.rendered_len();
            }
        }
        len
    }

    pub fn get_nodes(&self) -> &TreeNodes<T> {
        &self.nodes
    }

    pub fn get_nodes_mut(&mut self) -> &mut TreeNodes<T> {
        &mut self.nodes
    }

    pub fn get_value(&self) -> &T {
        &self.value
    }

    pub fn get_value_mut(&mut self) -> &mut T {
        &mut self.value
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }
}

impl<T> TreeNode<T>
where
    T: std::fmt::Display,
{
    pub fn render(&self, text: &mut Text<'static>, offsets: &mut TreeRender, line_width: usize) {
        if !offsets.is_rendering() {
            return;
        }
        let indents = "    ".repeat(offsets.get_indent_offset() as usize);
        let expander = if self.nodes.len() > 0 {
            if self.expanded {
                "[-] "
            } else {
                "[+] "
            }
        } else {
            ""
        };
        let line = Text::styled(
            format!("{}{}{}", indents, expander, self.value),
            get_selected_style(offsets.is_selected()),
        );
        offsets.render_line(text, line);
        if self.expanded {
            offsets.do_indent();
            for node in self.nodes.get_nodes() {
                node.render(text, offsets, line_width);
            }
            offsets.undo_indent();
        }
    }
}

impl<T> Default for TreeNode<T>
where
    T: std::fmt::Display + Default,
{
    fn default() -> Self {
        Self {
            value: T::default(),
            expanded: false,
            nodes: TreeNodes::default(),
        }
    }
}

impl<T> TreeNodes<T> {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn get_selected_mut<'a>(
        &'a mut self,
        select_offset: &mut isize,
    ) -> Option<&'a mut TreeNode<T>> {
        for node in &mut self.nodes {
            match node.get_selected_mut(select_offset) {
                Some(node) => return Some(node),
                None => {}
            }
        }
        None
    }

    pub fn rendered_len(&self) -> usize {
        let mut len = 0;
        for node in &self.nodes {
            len += node.rendered_len();
        }
        len
    }

    pub fn get_nodes(&self) -> &Vec<TreeNode<T>> {
        &self.nodes
    }

    pub fn get_nodes_mut(&mut self) -> &mut Vec<TreeNode<T>> {
        &mut self.nodes
    }

    pub fn into_nodes(self) -> Vec<TreeNode<T>> {
        self.nodes
    }
}

impl<T> TreeNodes<T>
where
    T: std::fmt::Display,
{
    pub fn render(&self, text: &mut Text<'static>, offsets: &mut TreeRender, line_width: usize) {
        for node in &self.nodes {
            node.render(text, offsets, line_width);
        }
    }
}

impl<T> Default for TreeNodes<T>
where
    T: std::fmt::Display,
{
    fn default() -> Self {
        Self {
            nodes: Vec::default(),
        }
    }
}

impl<T> std::ops::Deref for TreeNodes<T> {
    type Target = Vec<TreeNode<T>>;

    fn deref(&self) -> &Self::Target {
        &self.nodes
    }
}

impl<T> std::ops::DerefMut for TreeNodes<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.nodes
    }
}

impl<T> From<Vec<TreeNode<T>>> for TreeNodes<T> {
    fn from(nodes: Vec<TreeNode<T>>) -> Self {
        Self { nodes: nodes }
    }
}
