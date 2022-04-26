use vcd_parser::parser::*;

use crossterm::event::KeyCode;

use tui::{layout::Rect, style::Style, text::Text};

use crate::state::filter::*;
use crate::state::utils::*;

fn search_scopes_name<'a>(
    scopes: &'a Vec<BrowserScope>,
    name: &String,
) -> Option<&'a BrowserScope> {
    for scope in scopes {
        if &scope.name == name {
            return Some(scope);
        }
    }
    None
}

struct BrowserScope {
    name: String,
    expanded: bool,
    scopes: Vec<BrowserScope>,
    variables: Vec<VcdVariable>,
}

impl BrowserScope {
    fn empty() -> Self {
        Self {
            name: String::new(),
            expanded: false,
            scopes: Vec::new(),
            variables: Vec::new(),
        }
    }

    fn render(&self, text: &mut Text<'static>, offsets: &mut RenderContext) {
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
            for scope in &self.scopes {
                scope.render(text, offsets);
            }
            for variable in &self.variables {
                let line = Text::styled(
                    format!(
                        "{}{} {}",
                        indents,
                        variable.get_name(),
                        variable.get_width()
                    ),
                    get_selected_style(offsets.is_selected()),
                );
                offsets.render_line(text, line);
            }
            offsets.undo_indent();
        }
    }

    fn get_expanded_line_count(&self) -> usize {
        let mut line_count = 1;
        if self.expanded {
            for scope in &self.scopes {
                line_count += scope.get_expanded_line_count();
            }
            line_count += self.variables.len();
        }
        line_count
    }

    fn modify(&mut self, action: BrowserAction, select_offset: &mut isize) -> BrowserRequest {
        *select_offset -= 1;
        match *select_offset {
            0 => match action {
                BrowserAction::Append => return BrowserRequest::Append(self.variables.clone()),
                BrowserAction::Insert => return BrowserRequest::Insert(self.variables.clone()),
                BrowserAction::Expand => {
                    self.expanded = !self.expanded;
                    return BrowserRequest::None;
                }
            },
            1.. => {}
            _ => return BrowserRequest::None,
        }
        if self.expanded {
            for scope in &mut self.scopes {
                match scope.modify(action.clone(), select_offset) {
                    BrowserRequest::None => {}
                    request => return request,
                }
            }
            for variable in &self.variables {
                let v = vec![variable.clone()];
                *select_offset -= 1;
                match *select_offset {
                    0 => match action {
                        BrowserAction::Append => return BrowserRequest::Append(v),
                        BrowserAction::Insert => return BrowserRequest::Insert(v),
                        BrowserAction::Expand => return BrowserRequest::None,
                    },
                    1.. => {}
                    _ => return BrowserRequest::None,
                }
            }
        }
        BrowserRequest::None
    }
}

fn generate_new_scopes(
    old_scopes: &Vec<BrowserScope>,
    new_scopes: &Vec<VcdScope>,
) -> Vec<BrowserScope> {
    let mut generated_scopes = Vec::new();

    for (i, new_scope) in new_scopes.into_iter().enumerate() {
        let empty_scope = BrowserScope::empty();
        let old_scope = if old_scopes.len() > i && new_scope.get_name() == &old_scopes[i].name {
            // The scopes indices lined up from the new to the old
            &old_scopes[i]
        } else if let Some(old_scope) = search_scopes_name(&old_scopes, new_scope.get_name()) {
            // The scope existed in the old scopes, different position
            old_scope
        } else {
            // The scope did not exist in the old scopes
            &empty_scope
        };

        let mut variables = new_scope.get_variables().clone();
        variables.sort_by(|a, b| alphanumeric_sort::compare_str(a.get_name(), b.get_name()));

        generated_scopes.push(BrowserScope {
            name: new_scope.get_name().clone(),
            expanded: old_scope.expanded,
            scopes: generate_new_scopes(&old_scope.scopes, &new_scopes[i].get_scopes()),
            variables: variables,
        });
    }

    generated_scopes.sort_by(|a, b| alphanumeric_sort::compare_str(&a.name, &b.name));
    generated_scopes
}

#[derive(Clone)]
enum BrowserAction {
    Append,
    Insert,
    Expand,
}

pub enum BrowserRequest {
    Append(Vec<VcdVariable>),
    Insert(Vec<VcdVariable>),
    None,
}

pub struct BrowserState {
    width: usize,
    context: SelectContext,
    scopes: Vec<BrowserScope>,
    filters: Vec<BrowserFilterSection>,
}

impl BrowserState {
    pub fn new() -> Self {
        Self {
            width: 0,
            context: SelectContext::new(),
            scopes: Vec::new(),
            filters: Vec::new(),
        }
    }

    pub fn update_filter(&mut self, filter: String) {
        self.filters = construct_filter(filter);
    }

    pub fn update_scopes(&mut self, new_scopes: &Vec<VcdScope>) {
        // Set new scopes and clear the selected item
        self.scopes = generate_new_scopes(&self.scopes, &new_scopes);
        let line_count = self.get_expanded_line_count();
        self.context.select_relative(0, line_count);
    }

    pub fn set_size(&mut self, size: &Rect, border_width: u16) {
        self.width = if size.width > (border_width * 2) {
            (size.width - (border_width * 2)) as usize
        } else {
            0
        };
        // Handle extra room above/below hierarchy in browser
        let margin = border_width as isize * 2 + 2;
        self.context.set_height(size.height as isize - margin);
    }

    pub fn handle_key(&mut self, key: KeyCode) -> BrowserRequest {
        let line_count = self.get_expanded_line_count();
        match key {
            KeyCode::Up => self.context.select_relative(-1, line_count),
            KeyCode::Down => self.context.select_relative(1, line_count),
            KeyCode::PageDown => self.context.select_relative(20, line_count),
            KeyCode::PageUp => self.context.select_relative(-20, line_count),
            KeyCode::Enter => return self.modify(BrowserAction::Expand),
            KeyCode::Char('a') => return self.modify(BrowserAction::Append),
            KeyCode::Char('i') => return self.modify(BrowserAction::Insert),
            _ => {}
        }
        BrowserRequest::None
    }

    pub fn handle_mouse_click(&mut self, _: u16, y: u16) -> BrowserRequest {
        let line_count = self.get_expanded_line_count();
        if self.context.select_absolute(y as isize - 1, line_count) {
            return self.modify(BrowserAction::Expand);
        }
        BrowserRequest::None
    }

    pub fn handle_mouse_scroll(&mut self, scroll_up: bool) {
        let line_count = self.get_expanded_line_count();
        self.context
            .select_relative(if scroll_up { -5 } else { 5 }, line_count);
    }

    pub fn render(&self) -> Text<'static> {
        let mut text = Text::styled(" ", Style::default());
        let mut offsets = self.context.make_render_offsets();
        for scope in &self.scopes {
            scope.render(&mut text, &mut offsets);
        }
        text
    }

    fn get_expanded_line_count(&self) -> usize {
        let mut line_count = 0;
        for scope in &self.scopes {
            line_count += scope.get_expanded_line_count();
        }
        line_count
    }

    fn modify(&mut self, action: BrowserAction) -> BrowserRequest {
        let mut select_offset = self.context.get_select_offset() + 1;
        for scope in &mut self.scopes {
            match scope.modify(action.clone(), &mut select_offset) {
                BrowserRequest::None => {}
                request => {
                    self.context.scroll_relative(0);
                    return request;
                }
            }
        }
        self.context.scroll_relative(0);
        BrowserRequest::None
    }
}
