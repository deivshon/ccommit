use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;

use crate::message::ConventionalCommit;
use crate::theme::Colors;
use crate::ui;

pub fn draw(frame: &mut Frame, area: Rect, message: &ConventionalCommit, colors: Colors) {
    let mut lines = vec![
        Line::from(Span::styled("The commit message", ui::title_style(colors))),
        Line::raw(""),
        Line::from(Span::styled(
            message.short_message.clone(),
            Style::new().add_modifier(Modifier::BOLD),
        )),
    ];

    if !message.long_message.is_empty() {
        lines.push(Line::raw(""));
        lines.extend(
            message
                .long_message
                .lines()
                .map(|line| Line::raw(line.to_string())),
        );
    }

    frame.render_widget(Paragraph::new(Text::from(lines)), area);
}
