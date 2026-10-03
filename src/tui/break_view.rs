use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::colors;

/// Render a short break countdown.
pub fn render_short_break(f: &mut Frame, area: Rect, total: Duration, started_at: Instant) {
    let elapsed = started_at.elapsed();
    let remaining = if elapsed >= total {
        Duration::ZERO
    } else {
        total.checked_sub(elapsed).unwrap_or(Duration::ZERO)
    };

    let total_secs = remaining.as_secs();
    let m = total_secs / 60;
    let s = total_secs % 60;
    let time_str = format!("{m:02}:{s:02}");

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Short Break ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(colors::CYAN));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = Paragraph::new(Span::styled(
        time_str,
        Style::default().fg(colors::BRIGHT_BLUE).add_modifier(Modifier::BOLD),
    ))
    .alignment(Alignment::Center);

    let vertical = Layout::vertical([
        Constraint::Length((inner.height.saturating_sub(3)) / 2),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(inner);

    f.render_widget(text, vertical[1]);
}

/// Render the short break flash state (alternating colors).
pub fn render_short_break_flash(f: &mut Frame, area: Rect, flash_started_at: Instant) {
    let is_even_cycle = (flash_started_at.elapsed().as_millis() / 500).is_multiple_of(2);

    let block_style = if is_even_cycle {
        Style::default().fg(colors::CYAN).bg(colors::BRIGHT_BLUE)
    } else {
        Style::default()
            .fg(colors::BRIGHT_BLUE)
            .bg(colors::CYAN)
            .add_modifier(Modifier::REVERSED)
    };
    let text_fg = if is_even_cycle { colors::WHITE } else { colors::CYAN };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Break Over! ")
        .title_alignment(Alignment::Center)
        .style(block_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = Paragraph::new(Span::styled(
        "Press SPACE to resume",
        Style::default().fg(text_fg).add_modifier(Modifier::BOLD),
    ))
    .alignment(Alignment::Center);

    let vertical = Layout::vertical([
        Constraint::Length((inner.height.saturating_sub(3)) / 2),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(inner);

    f.render_widget(text, vertical[1]);
}

/// Render the endless break screen.
pub fn render_endless_break(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Endless Break ")
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(colors::CYAN));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = Paragraph::new(Span::styled(
        "Press SPACE to resume",
        Style::default().fg(colors::DIM_WHITE),
    ))
    .alignment(Alignment::Center);

    let vertical = Layout::vertical([
        Constraint::Length((inner.height.saturating_sub(3)) / 2),
        Constraint::Length(3),
        Constraint::Min(0),
    ])
    .split(inner);

    f.render_widget(text, vertical[1]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remaining_calculation() {
        let total = Duration::from_secs(600); // 10 min
        let started_at = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let elapsed = started_at.elapsed();
        let remaining = if elapsed >= total {
            Duration::ZERO
        } else {
            total - elapsed
        };
        assert!(remaining < total);
        assert!(remaining > Duration::ZERO);
    }

    #[test]
    fn test_remaining_zero_when_expired() {
        // Simulate expired break by setting total to 0
        let total = Duration::ZERO;
        let started_at = Instant::now();
        let elapsed = started_at.elapsed();
        let remaining = if elapsed >= total {
            Duration::ZERO
        } else {
            total - elapsed
        };
        assert_eq!(remaining, Duration::ZERO);
    }

    #[test]
    fn test_format_countdown() {
        let d = Duration::from_secs(605); // 10:05
        let m = d.as_secs() / 60;
        let s = d.as_secs() % 60;
        assert_eq!(format!("{m:02}:{s:02}"), "10:05");
    }
}
