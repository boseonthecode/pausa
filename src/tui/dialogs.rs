use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::tui::colors;

/// Render a centered confirmation dialog over the current screen.
pub fn render_confirm_reset(f: &mut Frame, area: Rect) {
    let dialog_width = area.width.min(44);
    let dialog_height = 7;
    let x = (area.width.saturating_sub(dialog_width)) / 2;
    let y = (area.height.saturating_sub(dialog_height)) / 2;

    let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

    // Clear the area behind the dialog
    f.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Reset ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(colors::AMBER));

    let inner = block.inner(dialog_area);
    f.render_widget(block, dialog_area);

    let text = Paragraph::new(Span::styled(
        "Reset today's timer?\n\n(y/n)",
        Style::default().fg(colors::WHITE),
    ))
    .alignment(Alignment::Center);

    f.render_widget(text, inner);
}
