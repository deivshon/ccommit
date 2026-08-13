use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::message::DescriptionLimit;
use crate::theme::Colors;
use crate::ui;
use crate::ui::editor::Editor;

pub enum Outcome {
    Continue,
    Submitted,
    Cancelled,
}

pub struct Input {
    prompt: String,
    editor: Editor,
    multiline: bool,
    limit: Option<DescriptionLimit>,
    refuse_empty: bool,
    colors: Colors,
}

impl Input {
    /// One line, Enter submits.
    pub fn single_line(prompt: impl Into<String>, refuse_empty: bool, colors: Colors) -> Self {
        Self::build(prompt, false, refuse_empty, colors)
    }

    /// Many lines, Enter submits, so Shift+Enter and Option+Enter insert a newline.
    ///
    /// A terminal reports Shift+Enter only through the kitty keyboard protocol. Option+Enter needs
    /// no protocol, so it is the fallback.
    pub fn multi_line(prompt: impl Into<String>, refuse_empty: bool, colors: Colors) -> Self {
        Self::build(prompt, true, refuse_empty, colors)
    }

    fn build(
        prompt: impl Into<String>,
        multiline: bool,
        refuse_empty: bool,
        colors: Colors,
    ) -> Self {
        Self {
            prompt: prompt.into(),
            editor: Editor::default(),
            multiline,
            limit: None,
            refuse_empty,
            colors,
        }
    }

    pub fn text(&self) -> String {
        self.editor.text()
    }

    pub fn is_empty(&self) -> bool {
        self.editor.is_empty()
    }

    fn length(&self) -> usize {
        self.text().chars().count()
    }

    pub fn set_text(&mut self, text: &str) {
        self.editor.set_text(text);
    }

    /// Cuts the kept text when a new limit is shorter than it. `None` removes the limit.
    pub fn set_limit(&mut self, limit: Option<DescriptionLimit>) {
        self.limit = limit;

        let Some(limit) = limit else {
            return;
        };

        if self.length() > limit.characters {
            let text: String = self.text().chars().take(limit.characters).collect();
            self.editor.set_text(&text);
        }
    }

    pub fn handle(&mut self, key: KeyEvent) -> Outcome {
        let new_line = key
            .modifiers
            .intersects(KeyModifiers::SHIFT | KeyModifiers::ALT);

        match key.code {
            KeyCode::Esc => return Outcome::Cancelled,
            KeyCode::Enter if self.multiline && new_line => {
                self.editor.insert_newline();
                return Outcome::Continue;
            }
            KeyCode::Enter => return self.submit(),
            _ => {}
        }

        if !self.rejects(key) {
            self.editor.handle(key);
        }

        Outcome::Continue
    }

    fn submit(&self) -> Outcome {
        if self.refuse_empty && self.is_empty() {
            Outcome::Continue
        } else {
            Outcome::Submitted
        }
    }

    /// The limit is hard, so a plain character stops at the last allowed position.
    fn rejects(&self, key: KeyEvent) -> bool {
        let Some(limit) = self.limit else {
            return false;
        };

        let plain = !key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER);

        matches!(key.code, KeyCode::Char(_)) && plain && self.length() >= limit.characters
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        let [prompt_area, text_area, counter_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(area);

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                &self.prompt,
                ui::title_style(self.colors),
            ))),
            prompt_area,
        );

        frame.render_widget(self.editor.widget(), text_area);

        if let Some(limit) = self.limit {
            let style = Style::new()
                .fg(self.counter_color().unwrap_or(self.colors.success))
                .add_modifier(Modifier::BOLD);

            // The count after the limit is what the parts before the description already hold.
            let counter = format!(
                "{}/{} (+{})",
                self.length(),
                limit.characters,
                limit.occupied
            );
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(counter, style))),
                counter_area,
            );
        }
    }

    /// The success color, then the warning one above half the limit, then the error one above
    /// three quarters.
    pub fn counter_color(&self) -> Option<Color> {
        self.limit.map(|limit| {
            let length = self.length();
            let limit = limit.characters;

            if length * 4 > limit * 3 {
                self.colors.error
            } else if length * 2 > limit {
                self.colors.warning
            } else {
                self.colors.success
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_text(input: &mut Input, text: &str) {
        for character in text.chars() {
            input.handle(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn control(character: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
    }

    fn shift_enter() -> KeyEvent {
        KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)
    }

    fn command(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::SUPER)
    }

    fn option(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::ALT)
    }

    fn cursor(input: &Input) -> (usize, usize) {
        input.editor.cursor()
    }

    /// A limit with a prefix of eight characters, as `feat: ✨ ` gives.
    fn limit(characters: usize) -> DescriptionLimit {
        DescriptionLimit {
            characters,
            occupied: 8,
        }
    }

    /// Builds a one-line input that holds the text, with the cursor at the end.
    fn filled(text: &str) -> Input {
        let mut input = Input::single_line("Write a description", true, Colors::default());
        type_text(&mut input, text);
        input
    }

    #[test]
    fn a_single_line_input_submits_on_enter() {
        let mut input = Input::single_line("Write a description", true, Colors::default());
        type_text(&mut input, "add thing");

        assert!(matches!(
            input.handle(key(KeyCode::Enter)),
            Outcome::Submitted
        ));
        assert_eq!(input.text(), "add thing");
    }

    #[test]
    fn a_single_line_input_refuses_to_submit_empty() {
        let mut input = Input::single_line("Write a description", true, Colors::default());
        assert!(matches!(
            input.handle(key(KeyCode::Enter)),
            Outcome::Continue
        ));
    }

    #[test]
    fn a_multi_line_input_keeps_every_line() {
        let mut input = Input::multi_line("Write a longer description", false, Colors::default());
        type_text(&mut input, "first");
        input.handle(shift_enter());
        type_text(&mut input, "second");

        assert!(matches!(
            input.handle(key(KeyCode::Enter)),
            Outcome::Submitted
        ));
        assert_eq!(input.text(), "first\nsecond");
    }

    #[test]
    fn option_and_enter_also_add_a_new_line() {
        let mut input = Input::multi_line("Write a longer description", false, Colors::default());
        type_text(&mut input, "first");
        input.handle(option(KeyCode::Enter));
        type_text(&mut input, "second");

        assert_eq!(input.text(), "first\nsecond");
    }

    #[test]
    fn shift_and_enter_submit_a_single_line_input() {
        let mut input = Input::single_line("Write a description", true, Colors::default());
        type_text(&mut input, "add thing");

        assert!(matches!(input.handle(shift_enter()), Outcome::Submitted));
        assert!(matches!(
            input.handle(option(KeyCode::Enter)),
            Outcome::Submitted
        ));
    }

    #[test]
    fn a_multi_line_input_submits_empty() {
        let mut input = Input::multi_line("Write a longer description", false, Colors::default());
        assert!(matches!(
            input.handle(key(KeyCode::Enter)),
            Outcome::Submitted
        ));
        assert_eq!(input.text(), "");
    }

    #[test]
    fn the_limit_stops_the_text_at_the_last_allowed_character() {
        let mut input = Input::single_line("Write a description", true, Colors::default());
        input.set_limit(Some(limit(5)));
        type_text(&mut input, "abcdefghij");

        assert_eq!(input.text(), "abcde");
    }

    #[test]
    fn backspace_still_works_at_the_limit() {
        let mut input = Input::single_line("Write a description", true, Colors::default());
        input.set_limit(Some(limit(3)));
        type_text(&mut input, "abc");
        input.handle(key(KeyCode::Backspace));
        type_text(&mut input, "z");

        assert_eq!(input.text(), "abz");
    }

    #[test]
    fn a_shorter_limit_cuts_the_kept_text() {
        let mut input = Input::single_line("Write a description", true, Colors::default());
        input.set_limit(Some(limit(20)));
        type_text(&mut input, "a long description");
        input.set_limit(Some(limit(6)));

        assert_eq!(input.text(), "a long");
    }

    #[test]
    fn no_limit_takes_every_character_and_draws_no_counter() {
        let mut input = Input::single_line("Write a description", true, Colors::default());
        input.set_limit(Some(limit(5)));
        type_text(&mut input, "abcdefghij");
        assert_eq!(input.text(), "abcde");

        input.set_limit(None);
        type_text(&mut input, "fghij");

        assert_eq!(input.text(), "abcdefghij");
        assert_eq!(input.counter_color(), None, "the counter is gone");
    }

    #[test]
    fn the_counter_walks_the_three_colors() {
        let mut input = Input::single_line("Write a description", true, Colors::default());
        input.set_limit(Some(limit(8)));
        let colors = Colors::default();
        assert_eq!(input.counter_color(), Some(colors.success));

        type_text(&mut input, "abcde");
        assert_eq!(input.counter_color(), Some(colors.warning));

        type_text(&mut input, "fgh");
        assert_eq!(input.counter_color(), Some(colors.error));
    }

    #[test]
    fn the_counter_walks_the_three_colors_of_the_theme() {
        let colors = Colors {
            success: Color::Rgb(1, 1, 1),
            warning: Color::Rgb(2, 2, 2),
            error: Color::Rgb(3, 3, 3),
            ..Colors::default()
        };
        let mut input = Input::single_line("Write a description", true, colors);
        input.set_limit(Some(limit(8)));
        assert_eq!(input.counter_color(), Some(colors.success));

        type_text(&mut input, "abcde");
        assert_eq!(input.counter_color(), Some(colors.warning));

        type_text(&mut input, "fgh");
        assert_eq!(input.counter_color(), Some(colors.error));
    }

    #[test]
    fn esc_cancels() {
        let mut input = Input::single_line("Write a description", true, Colors::default());
        assert!(matches!(
            input.handle(key(KeyCode::Esc)),
            Outcome::Cancelled
        ));
    }

    #[test]
    fn cmd_and_backspace_erase_to_the_head_of_the_line() {
        let mut input = filled("add the whole thing");
        input.handle(command(KeyCode::Backspace));

        assert_eq!(input.text(), "");
    }

    #[test]
    fn cmd_and_backspace_keep_the_text_after_the_cursor() {
        let mut input = filled("add the whole thing");
        input.handle(option(KeyCode::Left));
        input.handle(command(KeyCode::Backspace));

        assert_eq!(input.text(), "thing");
    }

    #[test]
    fn ctrl_u_erases_to_the_head_of_the_line() {
        let mut input = filled("add the whole thing");
        input.handle(control('u'));

        assert_eq!(input.text(), "");
    }

    #[test]
    fn cmd_and_delete_erase_to_the_end_of_the_line() {
        let mut input = filled("add the whole thing");
        input.handle(control('a'));
        input.handle(command(KeyCode::Delete));

        assert_eq!(input.text(), "");
    }

    #[test]
    fn option_and_backspace_erase_one_word() {
        let mut input = filled("add the whole thing");
        input.handle(option(KeyCode::Backspace));
        assert_eq!(input.text(), "add the whole ");

        input.handle(option(KeyCode::Backspace));
        assert_eq!(input.text(), "add the ");
    }

    #[test]
    fn ctrl_w_erases_one_word() {
        let mut input = filled("add the whole thing");
        input.handle(control('w'));

        assert_eq!(input.text(), "add the whole ");
    }

    #[test]
    fn option_and_delete_erase_the_next_word() {
        let mut input = filled("add the whole thing");
        input.handle(control('a'));
        input.handle(option(KeyCode::Delete));

        // The text area erases the word and leaves the space that follows it.
        assert_eq!(input.text(), " the whole thing");
    }

    #[test]
    fn ctrl_a_and_ctrl_e_reach_both_ends() {
        let mut input = filled("add thing");
        assert_eq!(cursor(&input), (0, 9));

        input.handle(control('a'));
        assert_eq!(cursor(&input), (0, 0));

        input.handle(control('e'));
        assert_eq!(cursor(&input), (0, 9));
    }

    #[test]
    fn home_and_end_reach_both_ends() {
        let mut input = filled("add thing");

        input.handle(key(KeyCode::Home));
        assert_eq!(cursor(&input), (0, 0));

        input.handle(key(KeyCode::End));
        assert_eq!(cursor(&input), (0, 9));
    }

    #[test]
    fn cmd_and_an_arrow_reach_both_ends() {
        let mut input = filled("add thing");

        input.handle(command(KeyCode::Left));
        assert_eq!(cursor(&input), (0, 0));

        input.handle(command(KeyCode::Right));
        assert_eq!(cursor(&input), (0, 9));
    }

    #[test]
    fn option_and_an_arrow_move_by_one_word() {
        let mut input = filled("add the thing");
        assert_eq!(cursor(&input), (0, 13));

        input.handle(option(KeyCode::Left));
        assert_eq!(cursor(&input), (0, 8));

        input.handle(option(KeyCode::Left));
        assert_eq!(cursor(&input), (0, 4));

        input.handle(option(KeyCode::Right));
        assert_eq!(cursor(&input), (0, 8));
    }

    #[test]
    fn ctrl_and_an_arrow_move_by_one_word() {
        let mut input = filled("add the thing");

        input.handle(KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL));
        assert_eq!(cursor(&input), (0, 8));

        input.handle(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL));
        assert_eq!(cursor(&input), (0, 13));
    }

    #[test]
    fn ctrl_k_erases_to_the_end_of_the_line() {
        let mut input = filled("add the whole thing");
        input.handle(option(KeyCode::Left));
        input.handle(control('k'));

        assert_eq!(input.text(), "add the whole ");
    }

    #[test]
    fn ctrl_z_undoes_the_last_edit() {
        let mut input = filled("add the whole thing");
        input.handle(control('u'));
        assert_eq!(input.text(), "");

        input.handle(control('z'));
        assert_eq!(input.text(), "add the whole thing");
    }

    #[test]
    fn cmd_with_a_letter_does_not_insert_that_letter() {
        let mut input = filled("add");

        for letter in ['k', 'w', 'q', 's', 'v', 'a'] {
            input.handle(KeyEvent::new(KeyCode::Char(letter), KeyModifiers::SUPER));
        }

        assert_eq!(input.text(), "add");
    }

    #[test]
    fn cmd_and_an_arrow_reach_both_ends_of_a_long_body() {
        let mut input = Input::multi_line("Write a longer description", false, Colors::default());
        type_text(&mut input, "first");
        input.handle(shift_enter());
        type_text(&mut input, "second");
        input.handle(shift_enter());
        type_text(&mut input, "third");

        input.handle(command(KeyCode::Up));
        assert_eq!(cursor(&input), (0, 0));

        input.handle(command(KeyCode::Down));
        assert_eq!(cursor(&input), (2, 5));
    }

    #[test]
    fn cmd_and_backspace_erase_one_line_of_a_long_body() {
        let mut input = Input::multi_line("Write a longer description", false, Colors::default());
        type_text(&mut input, "first");
        input.handle(shift_enter());
        type_text(&mut input, "second line");

        input.handle(command(KeyCode::Backspace));

        assert_eq!(input.text(), "first\n");
    }

    #[test]
    fn an_editing_key_never_submits_or_cancels() {
        let mut input = filled("add thing");

        for event in [
            command(KeyCode::Backspace),
            command(KeyCode::Delete),
            command(KeyCode::Left),
            option(KeyCode::Left),
            control('u'),
            control('z'),
            control('a'),
            control('e'),
        ] {
            assert!(matches!(input.handle(event), Outcome::Continue));
        }
    }

    #[test]
    fn a_word_key_still_works_at_the_limit() {
        let mut input = Input::single_line("Write a description", true, Colors::default());
        input.set_limit(Some(limit(9)));
        type_text(&mut input, "add thing");

        input.handle(option(KeyCode::Backspace));
        assert_eq!(input.text(), "add ");

        type_text(&mut input, "other");
        assert_eq!(input.text(), "add other");
    }

    #[test]
    fn a_character_outside_ascii_reaches_the_text() {
        let mut input = Input::single_line("Write a description", true, Colors::default());
        type_text(&mut input, "città ✨");

        assert_eq!(input.text(), "città ✨");
    }
}
