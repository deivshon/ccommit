//! Drives the wizard with a scripted event list against `TestBackend`, so no terminal is needed.

use std::path::PathBuf;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier};
use unicode_width::UnicodeWidthStr;

use ccommit::config::Config;
use ccommit::data::{self, Entry};
use ccommit::message::{ConventionalCommit, MessageLimit};
use ccommit::screen::wizard::{Step, Wizard};
use ccommit::theme::HexColor;
use ccommit::ui::{Flow, Screen};

/// Builds the scripted event list.
#[derive(Default)]
struct Keys(Vec<Event>);

impl Keys {
    fn new() -> Self {
        Self::default()
    }

    fn text(mut self, value: &str) -> Self {
        for character in value.chars() {
            self.0
                .push(press(KeyCode::Char(character), KeyModifiers::NONE));
        }
        self
    }

    fn code(mut self, code: KeyCode) -> Self {
        self.0.push(press(code, KeyModifiers::NONE));
        self
    }

    fn enter(self) -> Self {
        self.code(KeyCode::Enter)
    }

    fn esc(self) -> Self {
        self.code(KeyCode::Esc)
    }

    fn shift_enter(mut self) -> Self {
        self.0.push(press(KeyCode::Enter, KeyModifiers::SHIFT));
        self
    }

    fn ctrl(mut self, character: char) -> Self {
        self.0
            .push(press(KeyCode::Char(character), KeyModifiers::CONTROL));
        self
    }

    fn command(mut self, code: KeyCode) -> Self {
        self.0.push(press(code, KeyModifiers::SUPER));
        self
    }

    fn option(mut self, code: KeyCode) -> Self {
        self.0.push(press(code, KeyModifiers::ALT));
        self
    }

    /// The three keys that carry the wizard from the short description to the end.
    fn finish(self) -> Self {
        self.enter().enter().enter()
    }
}

fn press(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}

struct Harness {
    wizard: Wizard,
    terminal: Terminal<TestBackend>,
    directory: PathBuf,
}

impl Harness {
    fn new(config: Config, name: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("ccommit-wizard-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&directory).ok();
        std::fs::create_dir_all(&directory).expect("the directory is created");

        Self {
            wizard: Wizard::new(config, directory.clone(), commit_types(), gitmojis()),
            terminal: Terminal::new(TestBackend::new(80, 24)).expect("the backend starts"),
            directory,
        }
    }

    /// Feeds the events one by one and draws after each one, as the real loop does.
    fn drive(&mut self, keys: Keys) -> Option<Option<ConventionalCommit>> {
        for event in keys.0 {
            self.draw();
            if let Flow::Done(output) = self.wizard.handle(event) {
                return Some(output);
            }
        }
        self.draw();
        None
    }

    fn draw(&mut self) {
        let wizard = &self.wizard;
        self.terminal
            .draw(|frame| wizard.draw(frame))
            .expect("the draw works");
    }

    /// Reads the buffer the way a terminal shows it.
    ///
    /// A double-width glyph covers the next column, and `Buffer::diff_iter` never writes that
    /// column. `TestBackend` keeps the old symbol there, so the reader has to skip it.
    fn screen(&self) -> String {
        let buffer = self.terminal.backend().buffer();
        buffer
            .content()
            .chunks(buffer.area.width as usize)
            .map(|row| {
                let mut text = String::new();
                let mut covered = 0usize;

                for cell in row {
                    if covered > 0 {
                        covered -= 1;
                        continue;
                    }
                    let symbol = cell.symbol();
                    covered = symbol.width().saturating_sub(1);
                    text.push_str(symbol);
                }

                text.trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The foreground color of one cell.
    fn foreground(&self, column: u16, row: u16) -> Color {
        self.cell(column, row).fg
    }

    /// The background color of one cell.
    fn background(&self, column: u16, row: u16) -> Color {
        self.cell(column, row).bg
    }

    fn cell(&self, column: u16, row: u16) -> &ratatui::buffer::Cell {
        self.terminal
            .backend()
            .buffer()
            .cell((column, row))
            .expect("the cell is on the screen")
    }

    /// The columns of the row that carry the cursor mark.
    fn cursor_columns(&self, row: u16) -> Vec<u16> {
        let buffer = self.terminal.backend().buffer();
        (0..buffer.area.width)
            .filter(|&column| {
                buffer
                    .cell((column, row))
                    .is_some_and(|cell| cell.modifier.contains(Modifier::REVERSED))
            })
            .collect()
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.directory).ok();
    }
}

fn commit_types() -> Vec<Entry> {
    data::commit_types(None).expect("the embedded commit types parse")
}

fn gitmojis() -> Vec<Entry> {
    data::gitmojis(None).expect("the embedded gitmojis parse")
}

fn config() -> Config {
    Config::default()
}

fn hex(text: &str) -> HexColor {
    text.parse().expect("the color parses")
}

/// A configuration with one hex color for each of the five roles.
fn colored_config() -> Config {
    Config {
        accent_color: Some(hex("#1e88e5")),
        muted_color: Some(hex("#606060")),
        success_color: Some(hex("#00c853")),
        warning_color: Some(hex("#ffab00")),
        error_color: Some(hex("#d50000")),
        ..config()
    }
}

const ACCENT: Color = Color::Rgb(0x1e, 0x88, 0xe5);
const MUTED: Color = Color::Rgb(0x60, 0x60, 0x60);
const SUCCESS: Color = Color::Rgb(0x00, 0xc8, 0x53);
const WARNING: Color = Color::Rgb(0xff, 0xab, 0x00);
const ERROR: Color = Color::Rgb(0xd5, 0x00, 0x00);

/// The last row of the screen, where the footer sits.
const FOOTER_ROW: u16 = 23;

/// The last row of the body, where the counter of a text step sits.
const COUNTER_ROW: u16 = 22;

#[test]
fn the_wizard_builds_the_message_from_the_seven_steps() {
    let mut harness = Harness::new(config(), "happy");

    let output = harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .enter()
            .text(":sparkles:")
            .enter()
            .text("add thing")
            .enter()
            .finish(),
    );

    let message = output.expect("the wizard ends").expect("a message");
    assert_eq!(message.short_message, "feat: ✨ add thing");
    assert_eq!(message.long_message, "");
}

#[test]
fn the_gitmoji_code_reaches_the_message_when_real_emoji_is_off() {
    let config = Config {
        real_emoji: false,
        ..config()
    };
    let mut harness = Harness::new(config, "code");

    let output = harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .enter()
            .text(":sparkles:")
            .enter()
            .text("add thing")
            .enter()
            .finish(),
    );

    let message = output.expect("the wizard ends").expect("a message");
    assert_eq!(message.short_message, "feat: :sparkles: add thing");
}

#[test]
fn a_body_over_several_lines_reaches_the_message() {
    let mut harness = Harness::new(config(), "body");

    let output = harness.drive(
        Keys::new()
            .text("fix")
            .enter()
            .enter()
            .text(":bug:")
            .enter()
            .text("correct the parser")
            .enter()
            .text("first line")
            .shift_enter()
            .text("second line")
            .enter()
            .text("BREAKING CHANGE: the API moved")
            .enter()
            .enter(),
    );

    let message = output.expect("the wizard ends").expect("a message");
    assert_eq!(message.short_message, "fix: 🐛 correct the parser");
    assert_eq!(
        message.long_message,
        "first line\nsecond line\nBREAKING CHANGE: the API moved"
    );
}

#[test]
fn esc_on_the_first_menu_quits_with_no_message() {
    let mut harness = Harness::new(config(), "quit");

    let output = harness.drive(Keys::new().esc());

    assert!(output.expect("the wizard ends").is_none());
}

#[test]
fn esc_walks_back_through_every_step_and_keeps_the_text() {
    let mut harness = Harness::new(config(), "back");

    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .enter()
            .text(":sparkles:")
            .enter()
            .text("add thing")
            .enter()
            .text("the body"),
    );
    assert_eq!(harness.wizard.step(), Step::LongDesc);

    harness.drive(Keys::new().esc());
    assert_eq!(harness.wizard.step(), Step::ShortDesc);
    assert!(
        harness.screen().contains("add thing"),
        "{}",
        harness.screen()
    );

    harness.drive(Keys::new().esc());
    assert_eq!(harness.wizard.step(), Step::Gitmoji);

    harness.drive(Keys::new().esc());
    assert_eq!(harness.wizard.step(), Step::Scope);

    harness.drive(Keys::new().esc());
    assert_eq!(harness.wizard.step(), Step::Type);

    // Forward again, and the body text is still there.
    harness.drive(
        Keys::new()
            .enter()
            .enter()
            .enter()
            .enter()
            .code(KeyCode::End),
    );
    assert_eq!(harness.wizard.step(), Step::LongDesc);
    assert!(
        harness.screen().contains("the body"),
        "{}",
        harness.screen()
    );
}

#[test]
fn the_type_menu_filter_narrows_the_list() {
    let mut harness = Harness::new(config(), "filter");

    harness.drive(Keys::new().text("docs"));

    let select = harness.wizard.current_select().expect("the type menu");
    assert!(
        select.visible()[0].starts_with("docs"),
        "{:?}",
        select.visible()
    );
    assert!(
        select.visible().len() < commit_types().len(),
        "the filter drops entries"
    );
    assert!(harness.screen().contains("/docs"), "{}", harness.screen());
}

#[test]
fn the_editing_keys_work_inside_the_menu_filter() {
    let mut harness = Harness::new(config(), "editing-filter");

    // Ctrl+A moves to the head of the filter, so the next character lands there.
    harness.drive(Keys::new().text("eat").ctrl('a').text("f"));
    let select = harness.wizard.current_select().expect("the type menu");
    assert_eq!(select.query(), "feat");
    assert!(
        select.visible()[0].starts_with("feat"),
        "{:?}",
        select.visible()
    );

    // Ctrl+E moves to the end, so Option and Backspace erase the whole word.
    harness.drive(Keys::new().ctrl('e').option(KeyCode::Backspace));
    let select = harness.wizard.current_select().expect("the type menu");
    assert_eq!(select.query(), "");
    assert_eq!(select.visible().len(), commit_types().len());
}

#[test]
fn the_menu_filter_draws_the_cursor_after_the_text() {
    let mut harness = Harness::new(config(), "filter-cursor");

    harness.drive(Keys::new().text("docs"));

    // The title row holds `Select commit type  /docs`, so the cursor sits on column 25.
    assert_eq!(harness.cursor_columns(0), vec![25]);

    harness.drive(Keys::new().option(KeyCode::Left));
    assert_eq!(harness.cursor_columns(0), vec![21]);
}

#[test]
fn the_gitmoji_menu_shows_the_real_emoji() {
    let mut harness = Harness::new(config(), "emoji");

    harness.drive(Keys::new().text("feat").enter().enter().text("sparkles"));

    let select = harness.wizard.current_select().expect("the gitmoji menu");
    assert!(
        select.visible()[0].starts_with('✨'),
        "{:?}",
        select.visible()
    );
    assert!(harness.screen().contains('✨'), "{}", harness.screen());
}

#[test]
fn the_counter_turns_red_close_to_the_limit() {
    let mut harness = Harness::new(config(), "counter");

    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .enter()
            .text(":sparkles:")
            .enter(),
    );
    assert_eq!(
        harness.wizard.short_input().counter_color(),
        Some(Color::LightGreen)
    );

    // The prefix is `feat: ✨`, so the limit here is 64 characters.
    harness.drive(Keys::new().text(&"x".repeat(33)));
    assert_eq!(
        harness.wizard.short_input().counter_color(),
        Some(Color::LightYellow)
    );

    harness.drive(Keys::new().text(&"x".repeat(17)));
    assert_eq!(
        harness.wizard.short_input().counter_color(),
        Some(Color::LightRed)
    );

    // The limit is hard, so more keys change nothing.
    harness.drive(Keys::new().text(&"y".repeat(40)));
    assert_eq!(harness.wizard.short_input().text().chars().count(), 64);
    assert_eq!(harness.wizard.message().short_message.chars().count(), 72);
}

#[test]
fn the_default_colors_stay_ansi_colors() {
    let mut harness = Harness::new(config(), "ansi");
    harness.draw();

    assert_eq!(harness.foreground(0, 0), Color::LightBlue, "the title");
    assert_eq!(harness.foreground(2, 1), Color::LightBlue, "the first row");
    assert_eq!(harness.foreground(0, FOOTER_ROW), Color::Gray);
}

#[test]
fn the_accent_and_muted_colors_reach_the_menu() {
    let mut harness = Harness::new(colored_config(), "menu-colors");
    harness.draw();

    assert_eq!(harness.foreground(0, 0), ACCENT, "the title");
    assert_eq!(harness.foreground(0, 1), ACCENT, "the cursor mark");
    assert_eq!(harness.foreground(0, FOOTER_ROW), MUTED, "the key help");
}

/// The terminal swaps the two colors of a reversed cell, so the buffer keeps its own background.
#[test]
fn the_highlighted_row_reverses_the_accent_color() {
    let mut harness = Harness::new(colored_config(), "menu-highlight");
    harness.draw();

    assert_eq!(harness.foreground(2, 1), ACCENT);
    assert_eq!(harness.background(2, 1), Color::Reset);
    assert!(
        harness.cell(2, 1).modifier.contains(Modifier::REVERSED),
        "the highlighted row is reversed"
    );

    // The row under it carries no highlight at all.
    assert_eq!(harness.foreground(2, 2), Color::Reset);
    assert!(!harness.cell(2, 2).modifier.contains(Modifier::REVERSED));
}

#[test]
fn the_success_color_reaches_the_menu_filter() {
    let mut harness = Harness::new(colored_config(), "filter-color");

    harness.drive(Keys::new().text("docs"));

    // The title row holds `Select commit type  /docs`, so the filter starts on column 22.
    assert_eq!(harness.foreground(22, 0), SUCCESS);
}

#[test]
fn the_counter_walks_the_three_colors_of_the_theme() {
    let mut harness = Harness::new(colored_config(), "counter-colors");

    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .enter()
            .text(":sparkles:")
            .enter(),
    );
    assert_eq!(harness.foreground(0, COUNTER_ROW), SUCCESS);
    assert_eq!(harness.foreground(0, 0), ACCENT, "the prompt");

    // The prefix is `feat: ✨`, so the limit here is 64 characters.
    harness.drive(Keys::new().text(&"x".repeat(33)));
    assert_eq!(harness.foreground(0, COUNTER_ROW), WARNING);

    harness.drive(Keys::new().text(&"x".repeat(17)));
    assert_eq!(harness.foreground(0, COUNTER_ROW), ERROR);
}

#[test]
fn the_error_color_reaches_the_failure_line() {
    let mut harness = Harness::new(colored_config(), "error-color");

    // A scope that the settings file cannot take, because the file is a directory.
    std::fs::create_dir_all(harness.directory.join(".vscode/settings.json"))
        .expect("the directory is created");

    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .code(KeyCode::Down)
            .enter()
            .text("cli")
            .enter(),
    );

    assert!(harness.wizard.error().is_some(), "{}", harness.screen());
    assert_eq!(harness.foreground(0, FOOTER_ROW), ERROR);
}

#[test]
fn the_accent_color_reaches_the_preview_title() {
    let mut harness = Harness::new(colored_config(), "preview-color");

    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .enter()
            .text(":sparkles:")
            .enter()
            .text("add thing")
            .enter()
            .enter()
            .enter(),
    );

    assert_eq!(harness.wizard.step(), Step::Preview);
    assert_eq!(harness.foreground(0, 0), ACCENT);
}

/// The four keys that reach the short description with the type `feat` and the sparkles gitmoji.
fn to_short_description() -> Keys {
    Keys::new()
        .text("feat")
        .enter()
        .enter()
        .text(":sparkles:")
        .enter()
}

#[test]
fn a_shorter_limit_cuts_the_first_line_earlier() {
    let config = Config {
        short_message_limit: Some(MessageLimit::characters(50).expect("50 is the minimum")),
        ..config()
    };
    let mut harness = Harness::new(config, "limit-50");

    harness.drive(to_short_description().text(&"x".repeat(80)));

    // The prefix is `feat: ✨ `, so it holds 8 of the 50 characters.
    assert_eq!(harness.wizard.short_input().text().chars().count(), 42);
    assert_eq!(harness.wizard.message().short_message.chars().count(), 50);
    assert!(
        harness.screen().contains("42/42 (+8)"),
        "{}",
        harness.screen()
    );
}

#[test]
fn a_longer_limit_takes_a_longer_first_line() {
    let config = Config {
        short_message_limit: Some(MessageLimit::characters(100).expect("100 works")),
        ..config()
    };
    let mut harness = Harness::new(config, "limit-100");

    harness.drive(to_short_description().text(&"x".repeat(120)));

    assert_eq!(harness.wizard.message().short_message.chars().count(), 100);
}

#[test]
fn no_limit_leaves_the_first_line_free() {
    let config = Config {
        short_message_limit: Some(MessageLimit::NONE),
        ..config()
    };
    let mut harness = Harness::new(config, "limit-none");

    harness.drive(to_short_description().text(&"x".repeat(120)));

    assert_eq!(harness.wizard.short_input().text().chars().count(), 120);
    assert_eq!(harness.wizard.message().short_message.chars().count(), 128);
    assert_eq!(
        harness.wizard.short_input().counter_color(),
        None,
        "no limit draws no counter"
    );
    assert!(!harness.screen().contains("(+"), "{}", harness.screen());
}

#[test]
fn the_counter_shows_the_room_left_and_the_room_the_prefix_holds() {
    let mut harness = Harness::new(config(), "counter-prefix");

    harness.drive(to_short_description().text("add thing"));

    // `feat: ✨ ` holds 8 characters of the 72, so 64 are left.
    assert!(
        harness.screen().contains("9/64 (+8)"),
        "{}",
        harness.screen()
    );
}

#[test]
fn a_scope_moves_characters_from_the_counter_to_the_prefix() {
    let mut harness = Harness::new(config(), "counter-prefix-scope");

    // The third row of the scope menu takes a scope for this message alone.
    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .code(KeyCode::Down)
            .code(KeyCode::Down)
            .enter()
            .text("ui")
            .enter()
            .text("sparkles")
            .enter()
            .text("add thing"),
    );

    // `feat(ui): ✨ ` holds 12 characters of the 72.
    assert!(
        harness.screen().contains("9/60 (+12)"),
        "{}",
        harness.screen()
    );
}

#[test]
fn the_limit_follows_the_scope_of_the_step_before() {
    let config = Config {
        short_message_limit: Some(MessageLimit::characters(60).expect("60 works")),
        ..config()
    };
    let mut harness = Harness::new(config, "limit-scope");

    // The third row of the scope menu takes a scope that lives for this message alone.
    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .code(KeyCode::Down)
            .code(KeyCode::Down)
            .enter()
            .text("ui")
            .enter()
            .text("sparkles")
            .enter()
            .text(&"x".repeat(80)),
    );

    assert_eq!(harness.wizard.scope(), "ui");
    assert_eq!(harness.wizard.message().short_message.chars().count(), 60);
}

#[test]
fn the_preview_shows_the_message_and_esc_returns_to_the_last_step() {
    let mut harness = Harness::new(config(), "preview");

    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .enter()
            .text(":sparkles:")
            .enter()
            .text("add thing")
            .enter()
            .text("the body")
            .enter()
            .enter(),
    );

    assert_eq!(harness.wizard.step(), Step::Preview);
    let screen = harness.screen();
    assert!(screen.contains("feat: ✨ add thing"), "{screen}");
    assert!(screen.contains("the body"), "{screen}");

    harness.drive(Keys::new().esc());
    assert_eq!(harness.wizard.step(), Step::Breaking);
}

#[test]
fn the_wizard_ends_at_the_last_step_when_the_preview_is_off() {
    let config = Config {
        preview_on_finish: false,
        ..config()
    };
    let mut harness = Harness::new(config, "no-preview");

    let output = harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .enter()
            .text(":sparkles:")
            .enter()
            .text("add thing")
            .enter()
            .enter()
            .enter(),
    );

    let message = output.expect("the wizard ends").expect("a message");
    assert_eq!(message.short_message, "feat: ✨ add thing");
}

#[test]
fn the_scope_step_disappears_when_the_config_turns_it_off() {
    let config = Config {
        scope_step: false,
        ..config()
    };
    let mut harness = Harness::new(config, "no-scope-step");

    harness.drive(Keys::new().text("feat").enter());
    assert_eq!(harness.wizard.step(), Step::Gitmoji);

    let output = harness.drive(
        Keys::new()
            .text(":sparkles:")
            .enter()
            .text("add thing")
            .enter()
            .finish(),
    );

    assert_eq!(harness.wizard.scope(), "");
    let message = output.expect("the wizard ends").expect("a message");
    assert_eq!(message.short_message, "feat: ✨ add thing");
}

#[test]
fn the_gitmoji_step_disappears_when_the_config_turns_it_off() {
    let config = Config {
        gitmoji_step: false,
        ..config()
    };
    let mut harness = Harness::new(config, "no-gitmoji-step");

    harness.drive(Keys::new().text("feat").enter().enter());
    assert_eq!(harness.wizard.step(), Step::ShortDesc);

    let output = harness.drive(Keys::new().text("add thing").enter().finish());

    assert_eq!(harness.wizard.gitmoji(), "");
    let message = output.expect("the wizard ends").expect("a message");
    assert_eq!(message.short_message, "feat: add thing");
}

#[test]
fn the_two_body_steps_disappear_when_the_config_turns_them_off() {
    let config = Config {
        long_description_step: false,
        breaking_change_step: false,
        ..config()
    };
    let mut harness = Harness::new(config, "no-body-steps");

    harness.drive(to_short_description().text("add thing").enter());
    assert_eq!(harness.wizard.step(), Step::Preview);

    let output = harness.drive(Keys::new().enter());
    let message = output.expect("the wizard ends").expect("a message");
    assert_eq!(message.short_message, "feat: ✨ add thing");
    assert_eq!(message.long_message, "");
}

#[test]
fn the_breaking_change_step_disappears_on_its_own() {
    let config = Config {
        breaking_change_step: false,
        ..config()
    };
    let mut harness = Harness::new(config, "no-breaking-step");

    harness.drive(to_short_description().text("add thing").enter());
    assert_eq!(harness.wizard.step(), Step::LongDesc);

    harness.drive(Keys::new().text("the body").enter());
    assert_eq!(harness.wizard.step(), Step::Preview);
    assert_eq!(harness.wizard.message().long_message, "the body");

    // Esc from the preview lands on the last step the run shows.
    harness.drive(Keys::new().esc());
    assert_eq!(harness.wizard.step(), Step::LongDesc);
}

#[test]
fn esc_walks_back_over_the_hidden_steps() {
    let config = Config {
        scope_step: false,
        gitmoji_step: false,
        ..config()
    };
    let mut harness = Harness::new(config, "back-over-hidden");

    harness.drive(Keys::new().text("feat").enter().text("add thing"));
    assert_eq!(harness.wizard.step(), Step::ShortDesc);

    harness.drive(Keys::new().esc());
    assert_eq!(harness.wizard.step(), Step::Type);

    let output = harness.drive(Keys::new().esc());
    assert!(
        output.expect("the wizard ends").is_none(),
        "Esc on the first step quits"
    );
}

#[test]
fn the_wizard_ends_at_the_short_description_when_every_later_step_is_off() {
    let config = Config {
        long_description_step: false,
        breaking_change_step: false,
        preview_on_finish: false,
        ..config()
    };
    let mut harness = Harness::new(config, "short-only");

    let output = harness.drive(to_short_description().text("add thing").enter());

    let message = output.expect("the wizard ends").expect("a message");
    assert_eq!(message.short_message, "feat: ✨ add thing");
    assert_eq!(message.long_message, "");
}

#[test]
fn a_hidden_gitmoji_step_leaves_more_room_on_the_first_line() {
    let config = Config {
        gitmoji_step: false,
        ..config()
    };
    let mut harness = Harness::new(config, "limit-no-gitmoji");

    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .enter()
            .text(&"x".repeat(80)),
    );

    // `feat: ` holds 6 characters of the 72, so 66 are left.
    assert_eq!(harness.wizard.short_input().text().chars().count(), 66);
    assert_eq!(harness.wizard.message().short_message.chars().count(), 72);
}

#[test]
fn a_cancelled_new_scope_brings_the_scope_menu_back() {
    let mut harness = Harness::new(config(), "repeat");

    harness.drive(Keys::new().text("feat").enter());
    assert_eq!(harness.wizard.step(), Step::Scope);

    // The second row is "New scope".
    harness.drive(Keys::new().code(KeyCode::Down).enter());
    assert!(
        harness.wizard.current_select().is_none(),
        "the text input is open"
    );
    assert!(
        harness.screen().contains("Enter new scope"),
        "{}",
        harness.screen()
    );

    harness.drive(Keys::new().esc());
    assert_eq!(harness.wizard.step(), Step::Scope);
    assert!(
        harness.wizard.current_select().is_some(),
        "the menu is back"
    );
}

#[test]
fn a_new_scope_reaches_the_message_and_the_settings_file() {
    let mut harness = Harness::new(config(), "new-scope");

    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .code(KeyCode::Down)
            .enter()
            .text("cli")
            .enter(),
    );

    assert_eq!(harness.wizard.scope(), "cli");
    assert_eq!(harness.wizard.step(), Step::Gitmoji);

    let settings = harness.directory.join(".vscode/settings.json");
    let text = std::fs::read_to_string(&settings).expect("the settings file exists");
    assert!(text.contains("\"cli\""), "{text}");
}

#[test]
fn a_scope_used_once_does_not_reach_the_settings_file() {
    let mut harness = Harness::new(config(), "once");

    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .code(KeyCode::Down)
            .code(KeyCode::Down)
            .enter()
            .text("cli")
            .enter(),
    );

    assert_eq!(harness.wizard.scope(), "cli");
    assert!(!harness.directory.join(".vscode/settings.json").exists());
}

#[test]
fn a_scope_shortens_the_short_description_limit() {
    let mut harness = Harness::new(config(), "limit");

    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .code(KeyCode::Down)
            .code(KeyCode::Down)
            .enter()
            .text("a-long-scope-name")
            .enter()
            .text("sparkles")
            .enter()
            .text(&"x".repeat(80)),
    );

    let message = harness.wizard.message();
    assert_eq!(message.short_message.chars().count(), 72);
    assert!(
        message
            .short_message
            .starts_with("feat(a-long-scope-name): ✨ ")
    );
}

#[test]
fn no_scope_leaves_the_scope_out_of_the_message() {
    let mut harness = Harness::new(config(), "no-scope");

    let output = harness.drive(
        Keys::new()
            .text("chore")
            .enter()
            .enter()
            .text(":sparkles:")
            .enter()
            .text("add thing")
            .enter()
            .finish(),
    );

    assert_eq!(harness.wizard.commit_type(), "chore");
    assert_eq!(harness.wizard.scope(), "");
    let message = output.expect("the wizard ends").expect("a message");
    assert_eq!(message.short_message, "chore: ✨ add thing");
}

#[test]
fn the_editing_keys_work_inside_the_short_description() {
    let mut harness = Harness::new(config(), "editing");

    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .enter()
            .text("sparkles")
            .enter()
            .text("add the wrong thing"),
    );
    assert_eq!(harness.wizard.short_input().text(), "add the wrong thing");

    // Option and Backspace erase one word.
    harness.drive(Keys::new().option(KeyCode::Backspace));
    assert_eq!(harness.wizard.short_input().text(), "add the wrong ");

    // Cmd and Backspace erase the rest of the line.
    harness.drive(Keys::new().command(KeyCode::Backspace));
    assert_eq!(harness.wizard.short_input().text(), "");

    harness.drive(Keys::new().text("add thing").enter().finish());
    assert_eq!(harness.wizard.message().short_message, "feat: ✨ add thing");
}

#[test]
fn ctrl_a_and_ctrl_e_move_inside_the_short_description() {
    let mut harness = Harness::new(config(), "ends");

    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .enter()
            .text("sparkles")
            .enter()
            .text("thing")
            .ctrl('a')
            .text("add ")
            .ctrl('e')
            .text(" now"),
    );

    assert_eq!(harness.wizard.short_input().text(), "add thing now");
}

#[test]
fn cmd_with_a_letter_never_reaches_the_text_or_the_filter() {
    let mut harness = Harness::new(config(), "cmd-letter");

    // On the menu, Cmd and a letter leave the filter alone.
    harness.drive(
        Keys::new()
            .command(KeyCode::Char('k'))
            .command(KeyCode::Char('s')),
    );
    let select = harness.wizard.current_select().expect("the type menu");
    assert_eq!(select.visible().len(), commit_types().len());

    // In a text input, Cmd and a letter insert nothing.
    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .enter()
            .text("sparkles")
            .enter()
            .text("add")
            .command(KeyCode::Char('k'))
            .command(KeyCode::Char('v')),
    );
    assert_eq!(harness.wizard.short_input().text(), "add");
}

#[test]
fn the_editing_keys_work_inside_the_long_description() {
    let mut harness = Harness::new(config(), "editing-body");

    harness.drive(
        Keys::new()
            .text("feat")
            .enter()
            .enter()
            .text("sparkles")
            .enter()
            .text("add thing")
            .enter()
            .text("first line")
            .shift_enter()
            .text("second line with a mistake"),
    );

    // Cmd and Backspace erase the current line only.
    harness.drive(Keys::new().command(KeyCode::Backspace));
    harness.drive(Keys::new().text("second line"));

    harness.drive(Keys::new().finish());
    assert_eq!(
        harness.wizard.message().long_message,
        "first line\nsecond line"
    );
}

#[test]
fn a_menu_opens_with_an_empty_filter_after_back_navigation() {
    let mut harness = Harness::new(config(), "filter-reset");

    harness.drive(Keys::new().text("docs").enter());
    assert_eq!(harness.wizard.commit_type(), "docs");
    assert_eq!(harness.wizard.step(), Step::Scope);

    // Back on the type menu, the old filter is gone and the old answer stays highlighted.
    harness.drive(Keys::new().esc());
    let select = harness.wizard.current_select().expect("the type menu");
    assert_eq!(select.visible().len(), commit_types().len());
    assert!(select.visible()[select.cursor()].starts_with("docs"));

    // So a fresh filter matches on its own.
    harness.drive(Keys::new().text("feat").enter());
    assert_eq!(harness.wizard.commit_type(), "feat");
}
