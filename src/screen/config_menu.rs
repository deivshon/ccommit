use std::fmt;
use std::path::PathBuf;

use ratatui::Frame;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::config::{self, Config};
use crate::message::{MessageLimit, NO_LIMIT};
use crate::theme::{self, HexColor};
use crate::ui::input::{self, Input};
use crate::ui::{self, Flow, Screen};

/// The sixteen keys the menu can write, in the order the rows appear.
const ROWS: [Row; 16] = [
    Row::Path(PathRow::CommitTypes),
    Row::Path(PathRow::Gitmojis),
    Row::Boolean(BoolRow::CommitOnFinish),
    Row::Boolean(BoolRow::PreviewOnFinish),
    Row::Boolean(BoolRow::RealEmoji),
    Row::Boolean(BoolRow::ScopeStep),
    Row::Boolean(BoolRow::GitmojiStep),
    Row::Boolean(BoolRow::LongDescriptionStep),
    Row::Boolean(BoolRow::BreakingChangeStep),
    Row::Boolean(BoolRow::DenyCliLongMessage),
    Row::Limit,
    Row::Color(ColorRow::Accent),
    Row::Color(ColorRow::Muted),
    Row::Color(ColorRow::Success),
    Row::Color(ColorRow::Warning),
    Row::Color(ColorRow::Error),
];

#[derive(Clone, Copy)]
enum Row {
    Path(PathRow),
    Boolean(BoolRow),
    Color(ColorRow),
    Limit,
}

/// A row the user changes through a text input.
#[derive(Clone, Copy)]
enum EditRow {
    Path(PathRow),
    Color(ColorRow),
    Limit,
}

#[derive(Clone, Copy)]
enum PathRow {
    CommitTypes,
    Gitmojis,
}

#[derive(Clone, Copy)]
enum BoolRow {
    CommitOnFinish,
    PreviewOnFinish,
    RealEmoji,
    ScopeStep,
    GitmojiStep,
    LongDescriptionStep,
    BreakingChangeStep,
    DenyCliLongMessage,
}

#[derive(Clone, Copy)]
enum ColorRow {
    Accent,
    Muted,
    Success,
    Warning,
    Error,
}

impl Row {
    fn key(self) -> &'static str {
        match self {
            Row::Path(row) => row.key(),
            Row::Boolean(row) => row.key(),
            Row::Color(row) => row.key(),
            Row::Limit => config::KEY_SHORT_MESSAGE_LIMIT,
        }
    }
}

impl EditRow {
    fn key(self) -> &'static str {
        match self {
            EditRow::Path(row) => row.key(),
            EditRow::Color(row) => row.key(),
            EditRow::Limit => config::KEY_SHORT_MESSAGE_LIMIT,
        }
    }

    /// The hint that follows the key on the prompt of the input.
    fn prompt(self) -> String {
        let hint = match self {
            EditRow::Path(_) => "an empty path means the embedded file".to_string(),
            EditRow::Color(_) => "an empty color means the default color".to_string(),
            EditRow::Limit => format!("a number, `{NO_LIMIT}`, or empty for the default"),
        };
        format!("{} ({hint})", self.key())
    }
}

impl PathRow {
    fn key(self) -> &'static str {
        match self {
            PathRow::CommitTypes => config::KEY_COMMIT_TYPES,
            PathRow::Gitmojis => config::KEY_GITMOJIS,
        }
    }

    fn value(self, config: &Config) -> Option<&PathBuf> {
        match self {
            PathRow::CommitTypes => config.commit_types.as_ref(),
            PathRow::Gitmojis => config.gitmojis.as_ref(),
        }
    }

    fn field(self, config: &mut Config) -> &mut Option<PathBuf> {
        match self {
            PathRow::CommitTypes => &mut config.commit_types,
            PathRow::Gitmojis => &mut config.gitmojis,
        }
    }
}

impl BoolRow {
    fn key(self) -> &'static str {
        match self {
            BoolRow::CommitOnFinish => config::KEY_COMMIT_ON_FINISH,
            BoolRow::PreviewOnFinish => config::KEY_PREVIEW_ON_FINISH,
            BoolRow::RealEmoji => config::KEY_REAL_EMOJI,
            BoolRow::ScopeStep => config::KEY_SCOPE_STEP,
            BoolRow::GitmojiStep => config::KEY_GITMOJI_STEP,
            BoolRow::LongDescriptionStep => config::KEY_LONG_DESCRIPTION_STEP,
            BoolRow::BreakingChangeStep => config::KEY_BREAKING_CHANGE_STEP,
            BoolRow::DenyCliLongMessage => config::KEY_DENY_CLI_LONG_MESSAGE,
        }
    }

    fn value(self, config: &Config) -> bool {
        match self {
            BoolRow::CommitOnFinish => config.commit_on_finish,
            BoolRow::PreviewOnFinish => config.preview_on_finish,
            BoolRow::RealEmoji => config.real_emoji,
            BoolRow::ScopeStep => config.scope_step,
            BoolRow::GitmojiStep => config.gitmoji_step,
            BoolRow::LongDescriptionStep => config.long_description_step,
            BoolRow::BreakingChangeStep => config.breaking_change_step,
            BoolRow::DenyCliLongMessage => config.deny_cli_long_message,
        }
    }

    fn field(self, config: &mut Config) -> &mut bool {
        match self {
            BoolRow::CommitOnFinish => &mut config.commit_on_finish,
            BoolRow::PreviewOnFinish => &mut config.preview_on_finish,
            BoolRow::RealEmoji => &mut config.real_emoji,
            BoolRow::ScopeStep => &mut config.scope_step,
            BoolRow::GitmojiStep => &mut config.gitmoji_step,
            BoolRow::LongDescriptionStep => &mut config.long_description_step,
            BoolRow::BreakingChangeStep => &mut config.breaking_change_step,
            BoolRow::DenyCliLongMessage => &mut config.deny_cli_long_message,
        }
    }
}

impl ColorRow {
    fn key(self) -> &'static str {
        match self {
            ColorRow::Accent => config::KEY_ACCENT_COLOR,
            ColorRow::Muted => config::KEY_MUTED_COLOR,
            ColorRow::Success => config::KEY_SUCCESS_COLOR,
            ColorRow::Warning => config::KEY_WARNING_COLOR,
            ColorRow::Error => config::KEY_ERROR_COLOR,
        }
    }

    fn value(self, config: &Config) -> Option<HexColor> {
        match self {
            ColorRow::Accent => config.accent_color,
            ColorRow::Muted => config.muted_color,
            ColorRow::Success => config.success_color,
            ColorRow::Warning => config.warning_color,
            ColorRow::Error => config.error_color,
        }
    }

    fn field(self, config: &mut Config) -> &mut Option<HexColor> {
        match self {
            ColorRow::Accent => &mut config.accent_color,
            ColorRow::Muted => &mut config.muted_color,
            ColorRow::Success => &mut config.success_color,
            ColorRow::Warning => &mut config.warning_color,
            ColorRow::Error => &mut config.error_color,
        }
    }

    /// The color the run uses when the file says nothing.
    fn default_color(self) -> Color {
        match self {
            ColorRow::Accent => theme::DEFAULT_ACCENT,
            ColorRow::Muted => theme::DEFAULT_MUTED,
            ColorRow::Success => theme::DEFAULT_SUCCESS,
            ColorRow::Warning => theme::DEFAULT_WARNING,
            ColorRow::Error => theme::DEFAULT_ERROR,
        }
    }
}

/// The text of a value the file does not hold, so the reader sees what the run uses.
fn from_default(value: impl fmt::Display) -> String {
    format!("{value} (default)")
}

pub struct ConfigMenu {
    path: PathBuf,
    config: Config,
    cursor: usize,
    editing: Option<(EditRow, Input)>,
    error: Option<String>,
}

impl ConfigMenu {
    pub fn new(path: PathBuf, config: Config) -> Self {
        Self {
            path,
            config,
            cursor: 0,
            editing: None,
            error: None,
        }
    }

    fn value(&self, row: Row) -> String {
        match row {
            Row::Path(row) => match row.value(&self.config) {
                Some(path) => path.display().to_string(),
                None => "(embedded)".to_string(),
            },
            Row::Boolean(row) => row.value(&self.config).to_string(),
            Row::Color(row) => match row.value(&self.config) {
                Some(color) => color.to_string(),
                None => from_default(theme::default_name(row.default_color())),
            },
            Row::Limit => match self.config.short_message_limit {
                Some(limit) => limit.to_string(),
                None => from_default(self.config.message_limit()),
            },
        }
    }

    fn handle_list(&mut self, key: KeyEvent) -> Flow<()> {
        match key.code {
            KeyCode::Esc => return Flow::Done(()),
            KeyCode::Up => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Down => self.cursor = (self.cursor + 1).min(ROWS.len() - 1),
            KeyCode::Enter => match ROWS[self.cursor] {
                Row::Boolean(row) => self.toggle(row),
                Row::Path(row) => self.start_edit(EditRow::Path(row)),
                Row::Color(row) => self.start_edit(EditRow::Color(row)),
                Row::Limit => self.start_edit(EditRow::Limit),
            },
            _ => {}
        }

        Flow::Continue
    }

    fn toggle(&mut self, row: BoolRow) {
        let field = row.field(&mut self.config);
        *field = !*field;
        self.write();
    }

    fn start_edit(&mut self, row: EditRow) {
        let mut input = Input::single_line(row.prompt(), false, self.config.colors());
        let current = match row {
            EditRow::Path(row) => row
                .value(&self.config)
                .map_or(String::new(), |path| path.display().to_string()),
            EditRow::Color(row) => row
                .value(&self.config)
                .map_or(String::new(), |color| color.to_string()),
            EditRow::Limit => self
                .config
                .short_message_limit
                .map_or(String::new(), |limit| limit.to_string()),
        };
        input.set_text(&current);
        self.editing = Some((row, input));
    }

    fn handle_edit(&mut self, key: KeyEvent) -> Flow<()> {
        let Some((row, input)) = self.editing.as_mut() else {
            return Flow::Continue;
        };
        let row = *row;

        match input.handle(key) {
            input::Outcome::Submitted => {
                let text = input.text();

                match row {
                    EditRow::Path(row) => self.submit_path(row, &text),
                    EditRow::Color(row) => self.submit_color(row, &text),
                    EditRow::Limit => self.submit_limit(&text),
                }
            }
            input::Outcome::Cancelled => self.editing = None,
            input::Outcome::Continue => {}
        }

        Flow::Continue
    }

    fn submit_path(&mut self, row: PathRow, text: &str) {
        *row.field(&mut self.config) = (!text.is_empty()).then(|| PathBuf::from(text));
        self.write();
        self.editing = None;
    }

    /// A text that is not a hex color keeps the input open with the failure under it.
    fn submit_color(&mut self, row: ColorRow, text: &str) {
        let color = if text.is_empty() {
            None
        } else {
            match text.parse::<HexColor>() {
                Ok(color) => Some(color),
                Err(error) => {
                    self.error = Some(error);
                    return;
                }
            }
        };

        *row.field(&mut self.config) = color;
        self.write();
        self.editing = None;
    }

    /// A text that is neither a number nor the word for no limit keeps the input open.
    fn submit_limit(&mut self, text: &str) {
        let limit = if text.is_empty() {
            None
        } else {
            match text.parse::<MessageLimit>() {
                Ok(limit) => Some(limit),
                Err(error) => {
                    self.error = Some(error);
                    return;
                }
            }
        };

        self.config.short_message_limit = limit;
        self.write();
        self.editing = None;
    }

    /// Each edit reaches the file at once, and the file keeps every key.
    fn write(&mut self) {
        self.error = config::write(&self.path, &self.config).err();
    }

    fn footer(&self) -> &'static str {
        if self.editing.is_some() {
            "↵ save · Esc cancel"
        } else {
            "↑↓ move · ↵ change · Esc quit"
        }
    }
}

impl Screen for ConfigMenu {
    type Output = ();

    fn handle(&mut self, event: Event) -> Flow<()> {
        let Some(key) = ui::key_press(event) else {
            return Flow::Continue;
        };
        if ui::is_interrupt(key) {
            return Flow::Done(());
        }

        self.error = None;

        if self.editing.is_some() {
            self.handle_edit(key)
        } else {
            self.handle_list(key)
        }
    }

    /// The menu paints itself with the colors of the file, so an edit shows at once.
    fn draw(&self, frame: &mut Frame) {
        let colors = self.config.colors();
        let [title_area, body, footer] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "ccommit configuration",
                    ui::title_style(colors),
                )),
                Line::from(Span::styled(
                    self.path.display().to_string(),
                    Style::new().fg(colors.muted),
                )),
            ]),
            title_area,
        );

        if let Some((_, input)) = &self.editing {
            input.draw(frame, body);
        } else {
            let lines: Vec<Line> = ROWS
                .iter()
                .enumerate()
                .map(|(index, &row)| {
                    let text = format!("{}: {}", row.key(), self.value(row));
                    ui::menu_row(text, index == self.cursor, colors)
                })
                .collect();
            frame.render_widget(Paragraph::new(lines), body);
        }

        ui::draw_footer(frame, footer, self.error.as_deref(), self.footer(), colors);
    }
}
