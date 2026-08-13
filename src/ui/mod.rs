pub mod editor;
pub mod input;
pub mod preview;
pub mod select;

use std::borrow::Cow;
use std::io::stdout;

use ratatui::Frame;
use ratatui::buffer::{Buffer, CellDiffOption};
use ratatui::crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::supports_keyboard_enhancement;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::theme::Colors;

/// The event loop does not own the event source, so a test drives the same two methods.
pub enum Flow<T> {
    Continue,
    Done(T),
}

pub trait Screen {
    type Output;

    fn handle(&mut self, event: Event) -> Flow<Self::Output>;
    fn draw(&self, frame: &mut Frame);
}

/// Gives the key press inside the event. A release or a non-key event gives `None`.
pub fn key_press(event: Event) -> Option<KeyEvent> {
    match event {
        Event::Key(key) if key.kind != KeyEventKind::Release => Some(key),
        _ => None,
    }
}

/// Raw mode turns Ctrl+C into a key, so every screen has to quit by itself.
pub fn is_interrupt(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}

/// The footer shows the error when there is one, and the key help otherwise.
pub fn draw_footer(frame: &mut Frame, area: Rect, error: Option<&str>, help: &str, colors: Colors) {
    let line = match error {
        Some(error) => Line::from(Span::styled(
            format!("Failure: {error}"),
            Style::new().fg(colors.error),
        )),
        None => Line::from(Span::styled(help, Style::new().fg(colors.muted))),
    };
    frame.render_widget(Paragraph::new(line), area);
}

/// One menu row, with the cursor mark on the highlighted one.
///
/// The highlighted row is reversed, so the terminal puts the accent behind the text and its own
/// background color in front. The row therefore keeps its contrast under any accent color.
pub fn menu_row<'a>(text: impl Into<Cow<'a, str>>, highlighted: bool, colors: Colors) -> Line<'a> {
    if highlighted {
        Line::from(vec![
            Span::styled("> ", Style::new().fg(colors.accent)),
            Span::styled(
                text,
                Style::new()
                    .fg(colors.accent)
                    .add_modifier(Modifier::REVERSED | Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(vec![Span::raw("  "), Span::raw(text)])
    }
}

/// The style of a title or a prompt.
pub fn title_style(colors: Colors) -> Style {
    Style::new().fg(colors.accent).add_modifier(Modifier::BOLD)
}

/// Sets up the terminal, runs the screen, and restores the terminal.
///
/// `ratatui::try_init` installs a panic hook that restores the terminal first.
pub fn run<S: Screen>(screen: &mut S) -> Result<S::Output, String> {
    let mut terminal = ratatui::try_init().map_err(|error| format!("Terminal error: {error}"))?;
    let enhanced = push_key_modifiers();

    let result = event_loop(&mut terminal, screen);

    if enhanced {
        pop_key_modifiers();
    }
    ratatui::restore();
    result
}

/// Asks the terminal to report Cmd and Option as modifiers.
///
/// A terminal reports them only through the kitty keyboard protocol. Without the protocol, Cmd
/// never reaches the program, and the other keys still work.
fn push_key_modifiers() -> bool {
    if !matches!(supports_keyboard_enhancement(), Ok(true)) {
        return false;
    }

    if execute!(
        stdout(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    )
    .is_err()
    {
        return false;
    }

    // The hook from `try_init` restores the terminal. This one runs first and gives the protocol
    // back, so a panic cannot leave the terminal in the enhanced mode.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        pop_key_modifiers();
        hook(info);
    }));

    true
}

fn pop_key_modifiers() {
    let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
}

/// Marks every cell of the frame, which turns the next flush into a full repaint.
///
/// A wide grapheme such as an emoji owns two columns, and ratatui leaves the second one blank in
/// the buffer. A frame that drops the emoji therefore changes only the first column, and ratatui
/// trusts the terminal to erase the rest of the glyph. Zed keeps the old emoji on the screen, so
/// every frame writes every cell.
fn mark_for_repaint(buffer: &mut Buffer) {
    for cell in &mut buffer.content {
        if cell.diff_option == CellDiffOption::None {
            cell.diff_option = CellDiffOption::AlwaysUpdate;
        }
    }
}

fn event_loop<S: Screen>(
    terminal: &mut ratatui::DefaultTerminal,
    screen: &mut S,
) -> Result<S::Output, String> {
    loop {
        terminal
            .draw(|frame| {
                screen.draw(frame);
                mark_for_repaint(frame.buffer_mut());
            })
            .map_err(|error| format!("Draw error: {error}"))?;

        let event = event::read().map_err(|error| format!("Input error: {error}"))?;

        if let Flow::Done(output) = screen.handle(event) {
            return Ok(output);
        }
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::*;

    #[test]
    fn the_repaint_mark_covers_the_column_after_an_emoji() {
        let area = Rect::new(0, 0, 4, 1);
        let mut previous = Buffer::empty(area);
        previous.set_string(0, 0, "📝ab", Style::new());
        let mut next = Buffer::empty(area);

        let columns: Vec<u16> = previous.diff(&next).iter().map(|(x, ..)| *x).collect();
        assert_eq!(columns, vec![0, 2, 3], "column 1 holds the emoji tail");

        mark_for_repaint(&mut next);

        let columns: Vec<u16> = previous.diff(&next).iter().map(|(x, ..)| *x).collect();
        assert_eq!(columns, vec![0, 1, 2, 3]);
    }
}
