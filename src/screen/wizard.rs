use std::path::PathBuf;

use ratatui::Frame;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout};

use crate::config::Config;
use crate::data::Entry;
use crate::message::{self, ConventionalCommit, MessageLimit};
use crate::scope::{self, NEW_SCOPE, NEW_SCOPE_ONCE, NO_SCOPE};
use crate::theme::Colors;
use crate::ui::input::{self, Input};
use crate::ui::preview;
use crate::ui::select::{self, Select};
use crate::ui::{self, Flow, Screen};

/// The row that means "no gitmoji". The embedded data holds no such row today.
pub const NO_GITMOJI: &str = "None";

const SHORT_PROMPT: &str = "Write a short, imperative tense description of the change";
const LONG_PROMPT: &str = "Write a longer description of the change";
const BREAKING_PROMPT: &str = "List any breaking changes or issues closed by this change";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Type,
    Scope,
    Gitmoji,
    ShortDesc,
    LongDesc,
    Breaking,
    Preview,
}

/// Every step in the order the wizard walks them. The configuration can hide four of them.
const ORDER: [Step; 7] = [
    Step::Type,
    Step::Scope,
    Step::Gitmoji,
    Step::ShortDesc,
    Step::LongDesc,
    Step::Breaking,
    Step::Preview,
];

/// The steps this run shows, in order. A hidden step never appears, and its value stays empty.
fn visible_steps(config: &Config) -> Vec<Step> {
    ORDER
        .into_iter()
        .filter(|step| match step {
            Step::Scope => config.scope_step,
            Step::Gitmoji => config.gitmoji_step,
            Step::LongDesc => config.long_description_step,
            Step::Breaking => config.breaking_change_step,
            Step::Preview => config.preview_on_finish,
            Step::Type | Step::ShortDesc => true,
        })
        .collect()
}

/// Inside the scope step, the text input for a new scope is a sub-mode.
enum ScopeMode {
    Menu,
    NewScope { add: bool },
}

pub struct Wizard {
    config: Config,
    colors: Colors,
    limit: MessageLimit,
    working_directory: PathBuf,
    steps: Vec<Step>,
    position: usize,
    commit_types: Vec<Entry>,
    gitmojis: Vec<Entry>,
    type_select: Select,
    scope_select: Select,
    gitmoji_select: Select,
    scope_mode: ScopeMode,
    new_scope_input: Input,
    short_input: Input,
    long_input: Input,
    breaking_input: Input,
    commit_type: String,
    scope: String,
    gitmoji: String,
    error: Option<String>,
}

impl Wizard {
    pub fn new(
        config: Config,
        working_directory: PathBuf,
        commit_types: Vec<Entry>,
        gitmojis: Vec<Entry>,
    ) -> Self {
        let type_entries = commit_types.iter().map(Entry::menu_entry).collect();
        let gitmoji_entries = gitmojis.iter().map(Entry::menu_entry_with_emoji).collect();
        let scope_entries = scope::menu_entries(&working_directory);
        let colors = config.colors();
        let limit = config.message_limit();

        let mut wizard = Self {
            steps: visible_steps(&config),
            config,
            colors,
            limit,
            working_directory,
            position: 0,
            commit_types,
            gitmojis,
            type_select: Select::new("Select commit type", type_entries, colors),
            scope_select: Select::new("Select scope", scope_entries, colors),
            gitmoji_select: Select::new("Select gitmoji", gitmoji_entries, colors),
            scope_mode: ScopeMode::Menu,
            new_scope_input: Input::single_line("Enter new scope", true, colors),
            short_input: Input::single_line(SHORT_PROMPT, true, colors),
            long_input: Input::multi_line(LONG_PROMPT, false, colors),
            breaking_input: Input::multi_line(BREAKING_PROMPT, false, colors),
            commit_type: String::new(),
            scope: String::new(),
            gitmoji: String::new(),
            error: None,
        };

        wizard.update_limit();
        wizard
    }

    pub fn message(&self) -> ConventionalCommit {
        ConventionalCommit {
            short_message: message::build_short_message(
                &self.commit_type,
                &self.scope,
                &self.gitmoji,
                &self.short_input.text(),
            ),
            long_message: message::build_long_message(
                &self.long_input.text(),
                &self.breaking_input.text(),
            ),
        }
    }

    pub fn step(&self) -> Step {
        self.steps[self.position]
    }

    pub fn commit_type(&self) -> &str {
        &self.commit_type
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn gitmoji(&self) -> &str {
        &self.gitmoji
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn short_input(&self) -> &Input {
        &self.short_input
    }

    /// The menu of the current step, when the current step holds one.
    pub fn current_select(&self) -> Option<&Select> {
        match self.step() {
            Step::Type => Some(&self.type_select),
            Step::Scope => match self.scope_mode {
                ScopeMode::Menu => Some(&self.scope_select),
                ScopeMode::NewScope { .. } => None,
            },
            Step::Gitmoji => Some(&self.gitmoji_select),
            _ => None,
        }
    }

    /// Moves to a step. A menu always opens with an empty filter, as it did on the first visit.
    fn go_to(&mut self, position: usize) {
        self.position = position;

        match self.step() {
            Step::Type => self.type_select.reset_filter(),
            Step::Scope => self.scope_select.reset_filter(),
            Step::Gitmoji => self.gitmoji_select.reset_filter(),
            _ => {}
        }
    }

    fn update_limit(&mut self) {
        let limit = message::short_description_limit(
            &self.commit_type,
            &self.scope,
            &self.gitmoji,
            self.limit,
        );
        self.short_input.set_limit(limit);
    }

    fn set_scope(&mut self, scope: String) {
        self.scope = scope;
        self.update_limit();
    }

    /// Moves on, or ends the wizard when the current step is the last one.
    fn forward(&mut self) -> Flow<Option<ConventionalCommit>> {
        if self.position + 1 < self.steps.len() {
            self.go_to(self.position + 1);
            Flow::Continue
        } else {
            Flow::Done(Some(self.message()))
        }
    }

    /// Walks back, or quits when the current step is the first one.
    fn backward(&mut self) -> Flow<Option<ConventionalCommit>> {
        match self.position.checked_sub(1) {
            Some(previous) => {
                self.go_to(previous);
                Flow::Continue
            }
            None => Flow::Done(None),
        }
    }

    fn handle_type(&mut self, key: KeyEvent) -> Flow<Option<ConventionalCommit>> {
        match self.type_select.handle(key) {
            select::Outcome::Selected(index) => {
                self.commit_type = self.commit_types[index].name.clone();
                self.update_limit();
                self.forward()
            }
            select::Outcome::Cancelled => self.backward(),
            select::Outcome::Continue => Flow::Continue,
        }
    }

    fn handle_scope(&mut self, key: KeyEvent) -> Flow<Option<ConventionalCommit>> {
        match self.scope_mode {
            ScopeMode::Menu => self.handle_scope_menu(key),
            ScopeMode::NewScope { add } => self.handle_new_scope(key, add),
        }
    }

    fn handle_scope_menu(&mut self, key: KeyEvent) -> Flow<Option<ConventionalCommit>> {
        match self.scope_select.handle(key) {
            select::Outcome::Selected(index) => {
                let selected = self.scope_select.entry(index).to_string();

                match selected.as_str() {
                    NO_SCOPE => self.set_scope(String::new()),
                    NEW_SCOPE => {
                        self.start_new_scope(true);
                        return Flow::Continue;
                    }
                    NEW_SCOPE_ONCE => {
                        self.start_new_scope(false);
                        return Flow::Continue;
                    }
                    _ => self.set_scope(selected),
                }

                self.forward()
            }
            select::Outcome::Cancelled => self.backward(),
            select::Outcome::Continue => Flow::Continue,
        }
    }

    fn start_new_scope(&mut self, add: bool) {
        let prompt = if add {
            "Enter new scope"
        } else {
            "Enter new scope (only use once)"
        };
        self.new_scope_input = Input::single_line(prompt, true, self.colors);
        self.scope_mode = ScopeMode::NewScope { add };
    }

    fn handle_new_scope(&mut self, key: KeyEvent, add: bool) -> Flow<Option<ConventionalCommit>> {
        match self.new_scope_input.handle(key) {
            input::Outcome::Submitted => {
                let scope = self.new_scope_input.text();

                if add {
                    if let Err(error) = scope::add_scope(&self.working_directory, &scope) {
                        self.error = Some(error);
                        return Flow::Continue;
                    }
                    self.scope_select
                        .set_entries(scope::menu_entries(&self.working_directory));
                }

                self.set_scope(scope);
                self.scope_mode = ScopeMode::Menu;
                self.forward()
            }
            // Esc on the text input brings the scope menu back.
            input::Outcome::Cancelled => {
                self.scope_mode = ScopeMode::Menu;
                Flow::Continue
            }
            input::Outcome::Continue => Flow::Continue,
        }
    }

    fn handle_gitmoji(&mut self, key: KeyEvent) -> Flow<Option<ConventionalCommit>> {
        match self.gitmoji_select.handle(key) {
            select::Outcome::Selected(index) => {
                let selection = self.gitmojis[index].selection(self.config.real_emoji);
                self.gitmoji = if selection == NO_GITMOJI {
                    String::new()
                } else {
                    selection
                };
                self.update_limit();
                self.forward()
            }
            select::Outcome::Cancelled => self.backward(),
            select::Outcome::Continue => Flow::Continue,
        }
    }

    fn advance(&mut self, outcome: input::Outcome) -> Flow<Option<ConventionalCommit>> {
        match outcome {
            input::Outcome::Submitted => self.forward(),
            input::Outcome::Cancelled => self.backward(),
            input::Outcome::Continue => Flow::Continue,
        }
    }

    fn handle_preview(&mut self, key: KeyEvent) -> Flow<Option<ConventionalCommit>> {
        match key.code {
            KeyCode::Enter => Flow::Done(Some(self.message())),
            KeyCode::Esc => self.backward(),
            _ => Flow::Continue,
        }
    }

    fn footer(&self) -> &'static str {
        match self.step() {
            Step::Type => "↑↓ move · type to filter · ↵ select · Esc quit",
            Step::Scope => match self.scope_mode {
                ScopeMode::Menu => "↑↓ move · type to filter · ↵ select · Esc back",
                ScopeMode::NewScope { .. } => "↵ submit · Esc back to the menu",
            },
            Step::Gitmoji => "↑↓ move · type to filter · ↵ select · Esc back",
            Step::ShortDesc => "↵ submit · Esc back",
            Step::LongDesc | Step::Breaking => "↵ submit · Shift+↵ or Option+↵ new line · Esc back",
            Step::Preview => {
                if self.config.commit_on_finish {
                    "↵ commit · Esc back"
                } else {
                    "↵ print the message · Esc back"
                }
            }
        }
    }
}

impl Screen for Wizard {
    type Output = Option<ConventionalCommit>;

    fn handle(&mut self, event: Event) -> Flow<Self::Output> {
        let Some(key) = ui::key_press(event) else {
            return Flow::Continue;
        };
        if ui::is_interrupt(key) {
            return Flow::Done(None);
        }

        self.error = None;

        match self.step() {
            Step::Type => self.handle_type(key),
            Step::Scope => self.handle_scope(key),
            Step::Gitmoji => self.handle_gitmoji(key),
            Step::ShortDesc => {
                let outcome = self.short_input.handle(key);
                self.advance(outcome)
            }
            Step::LongDesc => {
                let outcome = self.long_input.handle(key);
                self.advance(outcome)
            }
            Step::Breaking => {
                let outcome = self.breaking_input.handle(key);
                self.advance(outcome)
            }
            Step::Preview => self.handle_preview(key),
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let [body, footer] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

        match self.step() {
            Step::Type => self.type_select.draw(frame, body),
            Step::Scope => match self.scope_mode {
                ScopeMode::Menu => self.scope_select.draw(frame, body),
                ScopeMode::NewScope { .. } => self.new_scope_input.draw(frame, body),
            },
            Step::Gitmoji => self.gitmoji_select.draw(frame, body),
            Step::ShortDesc => self.short_input.draw(frame, body),
            Step::LongDesc => self.long_input.draw(frame, body),
            Step::Breaking => self.breaking_input.draw(frame, body),
            Step::Preview => preview::draw(frame, body, &self.message(), self.colors),
        }

        ui::draw_footer(
            frame,
            footer,
            self.error.as_deref(),
            self.footer(),
            self.colors,
        );
    }
}
