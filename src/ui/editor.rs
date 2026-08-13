use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Style;
use tui_textarea::{CursorMove, TextArea};

/// One line of editable text with the key map of a terminal line editor.
///
/// Every text field of the program holds one, so the keys are the same in a text step and in the
/// filter of a menu.
pub struct Editor {
    area: TextArea<'static>,
}

impl Default for Editor {
    fn default() -> Self {
        let mut area = TextArea::default();
        area.set_cursor_line_style(Style::default());

        Self { area }
    }
}

impl Editor {
    pub fn set_style(&mut self, style: Style) {
        self.area.set_style(style);
    }

    /// The widget that draws the text and the cursor.
    pub fn widget(&self) -> &TextArea<'static> {
        &self.area
    }

    pub fn text(&self) -> String {
        self.area.lines().join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.area.lines().iter().all(|line| line.is_empty())
    }

    pub fn cursor(&self) -> (usize, usize) {
        self.area.cursor()
    }

    pub fn set_text(&mut self, text: &str) {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(str::to_string).collect()
        };
        let row = lines.len() - 1;
        let column = lines[row].chars().count();
        self.area.set_lines(lines, (row, column));
    }

    pub fn insert_newline(&mut self) {
        self.area.insert_newline();
    }

    /// Handles one key. Gives `true` when the text changed.
    pub fn handle(&mut self, key: KeyEvent) -> bool {
        match self.shortcut(key) {
            Some(changed) => changed,
            None => self.area.input(key),
        }
    }

    /// Handles the keys the text area does not map. `None` gives the key back to the text area.
    ///
    /// The text area already maps Ctrl+A, Ctrl+E, Ctrl+W, Ctrl+K, Option+Backspace,
    /// Option+Delete, and Ctrl with an arrow. Cmd reaches the program only when the terminal
    /// speaks the kitty keyboard protocol.
    fn shortcut(&mut self, key: KeyEvent) -> Option<bool> {
        let command = key.modifiers.contains(KeyModifiers::SUPER);
        let control = key.modifiers.contains(KeyModifiers::CONTROL);
        let option = key.modifiers.contains(KeyModifiers::ALT);

        let changed = match key.code {
            KeyCode::Backspace if command => self.area.delete_line_by_head(),
            KeyCode::Delete if command => self.area.delete_line_by_end(),
            KeyCode::Left if command => self.move_to(CursorMove::Head),
            KeyCode::Right if command => self.move_to(CursorMove::End),
            // `Top` and `Bottom` keep the column, and Cmd reaches the very start and end.
            KeyCode::Up if command => {
                self.move_to(CursorMove::Top);
                self.move_to(CursorMove::Head)
            }
            KeyCode::Down if command => {
                self.move_to(CursorMove::Bottom);
                self.move_to(CursorMove::End)
            }
            KeyCode::Left if option => self.move_to(CursorMove::WordBack),
            KeyCode::Right if option => self.move_to(CursorMove::WordForward),
            // Every terminal line editor discards to the head of the line on Ctrl+U.
            KeyCode::Char('u') if control => self.area.delete_line_by_head(),
            KeyCode::Char('z') if control => self.area.undo(),
            // No other Cmd key reaches the text, so Cmd with a letter cannot insert that letter.
            _ if command => false,
            _ => return None,
        };

        Some(changed)
    }

    /// Moves the cursor and reports no change, because a move leaves the text alone.
    fn move_to(&mut self, movement: CursorMove) -> bool {
        self.area.move_cursor(movement);
        false
    }
}
