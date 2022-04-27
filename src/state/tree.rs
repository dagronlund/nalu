use tui::text::Text;

use crate::state::utils::*;

pub struct TreeNode<V, N> {
    name: String,
    node_value: N,
    expanded: bool,
    nodes: Vec<TreeNode<V, N>>,
    values: Vec<V>,
}

pub enum TreeNodeSelected<V, N> {
    Value(V),
    Node(N),
    None,
}

impl<V, N> TreeNode<V, N>
where
    V: std::fmt::Display,
    N: Default,
{
    pub fn new(name: String, node_value: N) -> Self {
        Self {
            name: name,
            node_value: node_value,
            expanded: false,
            nodes: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn new_from(
        name: String,
        node_value: N,
        expanded: bool,
        nodes: Vec<TreeNode<V, N>>,
        values: Vec<V>,
    ) -> Self {
        Self {
            name: name,
            node_value: node_value,
            expanded: expanded,
            nodes: nodes,
            values: values,
        }
    }

    pub fn render(&self, text: &mut Text<'static>, offsets: &mut RenderContext, line_width: usize) {
        if !offsets.is_rendering() {
            return;
        }
        let indents = "    ".repeat(offsets.get_indent_offset() as usize);
        let line = Text::styled(
            format!(
                "{}{} {}",
                indents,
                if self.expanded { "[-]" } else { "[+]" },
                self.name
            ),
            get_selected_style(offsets.is_selected()),
        );
        offsets.render_line(text, line);
        if self.expanded {
            offsets.do_indent();
            for node in &self.nodes {
                node.render(text, offsets, line_width);
            }
            for value in &self.values {
                let line = Text::styled(
                    format!("    {}{}", indents, value),
                    get_selected_style(offsets.is_selected()),
                );
                offsets.render_line(text, line);
            }
            offsets.undo_indent();
        }
    }

    pub fn get_selected_mut<'a>(
        &'a mut self,
        select_offset: &mut isize,
    ) -> TreeNodeSelected<&'a mut V, &'a mut TreeNode<V, N>> {
        match *select_offset {
            0 => {
                *select_offset -= 1;
                return TreeNodeSelected::Node(self);
            }
            1.. => *select_offset -= 1,
            _ => return TreeNodeSelected::None,
        }
        if self.expanded {
            for node in &mut self.nodes {
                match node.get_selected_mut(select_offset) {
                    TreeNodeSelected::None => {}
                    selected => return selected,
                }
            }
            for value in &mut self.values {
                match *select_offset {
                    0 => {
                        *select_offset -= 1;
                        return TreeNodeSelected::Value(value);
                    }
                    1.. => *select_offset -= 1,
                    _ => return TreeNodeSelected::None,
                }
            }
        }
        TreeNodeSelected::None
    }

    pub fn rendered_len(&self) -> usize {
        let mut len = 1;
        if self.expanded {
            for node in &self.nodes {
                len += node.rendered_len();
            }
            len += self.values.len();
        }
        len
    }

    pub fn get_nodes(&self) -> &Vec<TreeNode<V, N>> {
        &self.nodes
    }

    pub fn get_nodes_mut(&mut self) -> &mut Vec<TreeNode<V, N>> {
        &mut self.nodes
    }

    pub fn get_values(&self) -> &Vec<V> {
        &self.values
    }

    pub fn get_values_mut(&mut self) -> &mut Vec<V> {
        &mut self.values
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn get_node_value(&self) -> &N {
        &self.node_value
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }
}

impl<V, N> Default for TreeNode<V, N>
where
    V: std::fmt::Display,
    N: Default,
{
    fn default() -> Self {
        Self {
            name: String::default(),
            node_value: N::default(),
            expanded: false,
            nodes: Vec::default(),
            values: Vec::default(),
        }
    }
}

pub struct TreeNodes<V, N> {
    nodes: Vec<TreeNode<V, N>>,
}

impl<V, N> TreeNodes<V, N>
where
    V: std::fmt::Display,
    N: Default,
{
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn render(&self, text: &mut Text<'static>, offsets: &mut RenderContext, line_width: usize) {
        for node in &self.nodes {
            node.render(text, offsets, line_width);
        }
    }

    pub fn get_selected_mut<'a>(
        &'a mut self,
        select_offset: &mut isize,
    ) -> TreeNodeSelected<&'a mut V, &'a mut TreeNode<V, N>> {
        for node in &mut self.nodes {
            match node.get_selected_mut(select_offset) {
                TreeNodeSelected::None => {}
                selected => return selected,
            }
        }
        TreeNodeSelected::None
    }

    pub fn rendered_len(&self) -> usize {
        let mut len = 0;
        for node in &self.nodes {
            len += node.rendered_len();
        }
        len
    }

    pub fn get_nodes(&self) -> &Vec<TreeNode<V, N>> {
        &self.nodes
    }

    pub fn get_nodes_mut(&mut self) -> &mut Vec<TreeNode<V, N>> {
        &mut self.nodes
    }

    pub fn into_nodes(self) -> Vec<TreeNode<V, N>> {
        self.nodes
    }
}
