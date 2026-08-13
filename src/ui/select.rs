use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config as MatcherConfig, Matcher, Utf32String};
use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::theme::Colors;
use crate::ui;
use crate::ui::editor::Editor;

/// The display string and its matcher candidate, both built once at startup.
struct Candidate {
    display: String,
    haystack: Utf32String,
}

pub enum Outcome {
    Continue,
    Selected(usize),
    Cancelled,
}

pub struct Select {
    title: String,
    candidates: Vec<Candidate>,
    order: Vec<usize>,
    cursor: usize,
    query: Editor,
    matcher: Matcher,
    colors: Colors,
}

impl Select {
    pub fn new(title: impl Into<String>, entries: Vec<String>, colors: Colors) -> Self {
        let candidates: Vec<Candidate> = entries
            .into_iter()
            .map(|display| Candidate {
                haystack: Utf32String::from(display.as_str()),
                display,
            })
            .collect();

        let mut query = Editor::default();
        query.set_style(Style::new().fg(colors.success).add_modifier(Modifier::BOLD));

        Self {
            title: title.into(),
            order: (0..candidates.len()).collect(),
            candidates,
            cursor: 0,
            query,
            matcher: Matcher::new(MatcherConfig::DEFAULT),
            colors,
        }
    }

    pub fn set_entries(&mut self, entries: Vec<String>) {
        *self = Self::new(std::mem::take(&mut self.title), entries, self.colors);
    }

    /// The list owns the arrows, and every other key edits the filter.
    ///
    /// Home, End, and the word keys therefore work inside the filter, as they do in a text step.
    /// Cmd and an arrow reach the ends of the list.
    pub fn handle(&mut self, key: KeyEvent) -> Outcome {
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        let command = key.modifiers.contains(KeyModifiers::SUPER);

        match key.code {
            KeyCode::Esc => return Outcome::Cancelled,
            KeyCode::Enter => {
                return match self.order.get(self.cursor) {
                    Some(&index) => Outcome::Selected(index),
                    None => Outcome::Continue,
                };
            }
            KeyCode::Up if command => self.cursor = 0,
            KeyCode::Down if command => self.cursor = self.order.len().saturating_sub(1),
            KeyCode::Up => self.move_cursor(-1),
            KeyCode::Down => self.move_cursor(1),
            KeyCode::Char('p') if control => self.move_cursor(-1),
            KeyCode::Char('n') if control => self.move_cursor(1),
            _ => {
                if self.query.handle(key) {
                    self.refilter();
                }
            }
        }

        Outcome::Continue
    }

    fn move_cursor(&mut self, step: isize) {
        if self.order.is_empty() {
            self.cursor = 0;
            return;
        }

        let last = self.order.len() - 1;
        self.cursor = match step {
            step if step < 0 => self.cursor.saturating_sub(step.unsigned_abs()),
            step => (self.cursor + step as usize).min(last),
        };
    }

    /// The list returns to the full set when the query is empty.
    fn refilter(&mut self) {
        let query = self.query.text();
        let Self {
            candidates,
            matcher,
            order,
            cursor,
            ..
        } = self;

        if query.is_empty() {
            *order = (0..candidates.len()).collect();
        } else {
            let pattern = Pattern::parse(&query, CaseMatching::Ignore, Normalization::Smart);
            let mut scored: Vec<(u32, usize)> = candidates
                .iter()
                .enumerate()
                .filter_map(|(index, candidate)| {
                    pattern
                        .score(candidate.haystack.slice(..), matcher)
                        .map(|score| (score, index))
                })
                .collect();

            scored.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));
            *order = scored.into_iter().map(|(_, index)| index).collect();
        }

        // A new query moves the cursor to the best match, because that is what the typing asked
        // for. An old index would point at an unrelated entry.
        *cursor = 0;
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        let [title_area, list_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(area);

        self.draw_title(frame, title_area);

        let height = list_area.height as usize;
        let offset = self.window(height);

        let lines: Vec<Line> = self
            .order
            .iter()
            .skip(offset)
            .take(height)
            .enumerate()
            .map(|(row, &index)| {
                let display = self.candidates[index].display.as_str();
                ui::menu_row(display, offset + row == self.cursor, self.colors)
            })
            .collect();

        frame.render_widget(Paragraph::new(lines).block(Block::new()), list_area);
    }

    /// The filter sits on the title row, and it draws a cursor of its own.
    ///
    /// A widget needs an area, so the row holds the title on the left and the filter on the right.
    fn draw_title(&self, frame: &mut Frame, area: Rect) {
        let mut spans = vec![Span::styled(&self.title, ui::title_style(self.colors))];
        if !self.query.is_empty() {
            spans.push(Span::raw("  /"));
        }

        let line = Line::from(spans);
        let width = (line.width() as u16).min(area.width);
        frame.render_widget(Paragraph::new(line), area);

        if self.query.is_empty() {
            return;
        }

        let query_area = Rect {
            x: area.x + width,
            width: area.width - width,
            ..area
        };
        frame.render_widget(self.query.widget(), query_area);
    }

    fn window(&self, height: usize) -> usize {
        if height == 0 || self.cursor < height {
            0
        } else {
            self.cursor + 1 - height
        }
    }

    pub fn entry(&self, index: usize) -> &str {
        &self.candidates[index].display
    }

    /// Clears the filter and keeps the highlighted entry highlighted.
    pub fn reset_filter(&mut self) {
        if self.query.is_empty() {
            return;
        }

        let highlighted = self.order.get(self.cursor).copied();
        self.query.set_text("");
        self.refilter();

        // An empty query gives every entry back in its original order.
        if let Some(index) = highlighted {
            self.cursor = index;
        }
    }

    /// The text of the filter.
    pub fn query(&self) -> String {
        self.query.text()
    }

    /// The entries the list shows for the current query, in order.
    pub fn visible(&self) -> Vec<&str> {
        self.order
            .iter()
            .map(|&index| self.candidates[index].display.as_str())
            .collect()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn control(character: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
    }

    fn command(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::SUPER)
    }

    fn option(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::ALT)
    }

    fn select() -> Select {
        Select::new(
            "Select commit type",
            vec![
                "feat (Feature; A new feature)".to_string(),
                "fix (Bug Fixes; A bug fix)".to_string(),
                "docs (Documentation; Documentation only changes)".to_string(),
            ],
            Colors::default(),
        )
    }

    fn type_text(select: &mut Select, text: &str) {
        for character in text.chars() {
            select.handle(key(KeyCode::Char(character)));
        }
    }

    /// Builds a menu with a two-word filter, with the cursor at the end of it.
    fn filtered() -> Select {
        let mut select = select();
        type_text(&mut select, "a bug");
        select
    }

    #[test]
    fn enter_gives_the_index_of_the_original_entry() {
        let mut select = select();
        select.handle(key(KeyCode::Down));

        match select.handle(key(KeyCode::Enter)) {
            Outcome::Selected(index) => assert_eq!(index, 1),
            _ => panic!("enter selects"),
        }
    }

    #[test]
    fn typing_filters_the_list_and_the_index_stays_correct() {
        let mut select = select();
        for character in "docs".chars() {
            select.handle(key(KeyCode::Char(character)));
        }

        assert_eq!(select.visible().len(), 1);

        match select.handle(key(KeyCode::Enter)) {
            Outcome::Selected(index) => assert_eq!(index, 2),
            _ => panic!("enter selects"),
        }
    }

    #[test]
    fn an_empty_query_gives_the_full_list_back() {
        let mut select = select();
        select.handle(key(KeyCode::Char('d')));
        assert_eq!(select.visible().len(), 1);

        select.handle(key(KeyCode::Backspace));
        assert_eq!(select.visible().len(), 3);
    }

    #[test]
    fn a_query_with_no_match_leaves_nothing_to_select() {
        let mut select = select();
        for character in "zzzz".chars() {
            select.handle(key(KeyCode::Char(character)));
        }

        assert!(select.visible().is_empty());
        assert!(matches!(
            select.handle(key(KeyCode::Enter)),
            Outcome::Continue
        ));
    }

    #[test]
    fn the_cursor_stops_at_both_ends() {
        let mut select = select();
        select.handle(key(KeyCode::Up));
        assert_eq!(select.cursor(), 0);

        for _ in 0..10 {
            select.handle(key(KeyCode::Down));
        }
        assert_eq!(select.cursor(), 2);
    }

    #[test]
    fn a_new_query_moves_the_cursor_to_the_best_match() {
        let mut select = select();

        // Move away from the top, then filter.
        select.handle(key(KeyCode::Down));
        select.handle(key(KeyCode::Down));
        assert_eq!(select.cursor(), 2);

        for character in "fix".chars() {
            select.handle(key(KeyCode::Char(character)));
        }
        assert_eq!(select.cursor(), 0);

        match select.handle(key(KeyCode::Enter)) {
            Outcome::Selected(index) => assert_eq!(index, 1, "fix is entry 1"),
            _ => panic!("enter selects"),
        }
    }

    #[test]
    fn reset_filter_keeps_the_highlighted_entry() {
        let mut select = select();
        for character in "docs".chars() {
            select.handle(key(KeyCode::Char(character)));
        }
        assert_eq!(select.visible()[select.cursor()], select.entry(2));

        select.reset_filter();

        assert_eq!(select.visible().len(), 3);
        assert_eq!(select.cursor(), 2, "docs stays highlighted");
    }

    #[test]
    fn reset_filter_does_nothing_when_the_query_is_empty() {
        let mut select = select();
        select.handle(key(KeyCode::Down));

        select.reset_filter();

        assert_eq!(select.cursor(), 1);
        assert_eq!(select.visible().len(), 3);
    }

    #[test]
    fn esc_cancels() {
        let mut select = select();
        assert!(matches!(
            select.handle(key(KeyCode::Esc)),
            Outcome::Cancelled
        ));
    }

    #[test]
    fn ctrl_a_and_ctrl_e_reach_both_ends_of_the_filter() {
        let mut select = filtered();

        select.handle(control('a'));
        type_text(&mut select, "z");
        assert_eq!(select.query(), "za bug");

        select.handle(control('e'));
        type_text(&mut select, "z");
        assert_eq!(select.query(), "za bugz");
    }

    #[test]
    fn home_and_end_reach_both_ends_of_the_filter() {
        let mut select = filtered();
        select.handle(key(KeyCode::Down));

        select.handle(key(KeyCode::Home));
        type_text(&mut select, "z");
        assert_eq!(select.query(), "za bug");

        select.handle(key(KeyCode::End));
        type_text(&mut select, "z");
        assert_eq!(select.query(), "za bugz");
    }

    #[test]
    fn option_and_an_arrow_move_the_filter_by_one_word() {
        let mut select = filtered();

        select.handle(option(KeyCode::Left));
        type_text(&mut select, "z");
        assert_eq!(select.query(), "a zbug");

        select.handle(option(KeyCode::Right));
        type_text(&mut select, "z");
        assert_eq!(select.query(), "a zbugz");
    }

    #[test]
    fn cmd_and_an_arrow_reach_both_ends_of_the_filter() {
        let mut select = filtered();

        select.handle(command(KeyCode::Left));
        type_text(&mut select, "z");
        assert_eq!(select.query(), "za bug");

        select.handle(command(KeyCode::Right));
        type_text(&mut select, "z");
        assert_eq!(select.query(), "za bugz");
    }

    #[test]
    fn the_word_keys_erase_one_word_of_the_filter() {
        let mut select = filtered();
        select.handle(option(KeyCode::Backspace));
        assert_eq!(select.query(), "a ");

        let mut select = filtered();
        select.handle(control('w'));
        assert_eq!(select.query(), "a ");

        let mut select = filtered();
        select.handle(control('a'));
        select.handle(option(KeyCode::Delete));
        assert_eq!(select.query(), " bug");
    }

    #[test]
    fn the_line_keys_erase_the_whole_filter() {
        let mut select = filtered();
        select.handle(command(KeyCode::Backspace));
        assert_eq!(select.query(), "");

        let mut select = filtered();
        select.handle(control('u'));
        assert_eq!(select.query(), "");

        let mut select = filtered();
        select.handle(control('a'));
        select.handle(command(KeyCode::Delete));
        assert_eq!(select.query(), "");
    }

    #[test]
    fn ctrl_k_erases_to_the_end_of_the_filter() {
        let mut select = filtered();
        select.handle(option(KeyCode::Left));
        select.handle(control('k'));

        assert_eq!(select.query(), "a ");
    }

    #[test]
    fn an_editing_key_filters_the_list_again() {
        let mut select = select();
        type_text(&mut select, "docs");
        assert_eq!(select.visible().len(), 1);

        select.handle(control('u'));
        assert_eq!(select.visible().len(), 3);
    }

    #[test]
    fn a_move_inside_the_filter_leaves_the_list_alone() {
        let mut select = select();
        type_text(&mut select, "i");
        select.handle(key(KeyCode::Down));
        let visible = select.visible().len();
        let cursor = select.cursor();

        select.handle(control('a'));

        assert_eq!(select.visible().len(), visible);
        assert_eq!(select.cursor(), cursor);
    }

    #[test]
    fn cmd_and_an_arrow_reach_both_ends_of_the_list() {
        let mut select = select();

        select.handle(command(KeyCode::Down));
        assert_eq!(select.cursor(), 2);

        select.handle(command(KeyCode::Up));
        assert_eq!(select.cursor(), 0);
    }

    #[test]
    fn ctrl_p_and_ctrl_n_move_the_list_and_leave_the_filter_alone() {
        let mut select = select();

        select.handle(control('n'));
        assert_eq!(select.cursor(), 1);

        select.handle(control('p'));
        assert_eq!(select.cursor(), 0);
        assert_eq!(select.query(), "");
    }
}
