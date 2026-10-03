use std::time::{Duration, Instant};

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::colors;

/// Formats a duration as `HH:MM:SS`.
pub fn format_duration(d: Duration) -> String {
    let total = d.as_secs();
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

/// A simple stopwatch that tracks elapsed time.
pub struct Stopwatch {
    elapsed: Duration,
    is_running: bool,
    started_at: Option<Instant>,
}

impl Stopwatch {
    pub fn new() -> Self {
        Self {
            elapsed: Duration::ZERO,
            is_running: false,
            started_at: None,
        }
    }

    /// Toggle between running and paused.
    pub fn toggle(&mut self) {
        if self.is_running {
            if let Some(start) = self.started_at {
                self.elapsed += start.elapsed();
            }
            self.started_at = None;
        } else {
            self.started_at = Some(Instant::now());
        }
        self.is_running = !self.is_running;
    }

    /// Return the total elapsed time, including the current run if running.
    pub fn current_elapsed(&self) -> Duration {
        let base = self.elapsed;
        match self.started_at {
            Some(start) => base + start.elapsed(),
            None => base,
        }
    }

    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// Reset the stopwatch to zero.
    pub fn reset(&mut self) {
        self.elapsed = Duration::ZERO;
        self.started_at = None;
        self.is_running = false;
    }

    /// Render the stopwatch centered within the given area.
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let time_str = format_duration(self.current_elapsed());
        let fg = if self.is_running {
            colors::BRIGHT_BLUE
        } else {
            colors::AMBER
        };
        let status = if self.is_running { "RUNNING" } else { "PAUSED" };

        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Stopwatch ")
            .title_alignment(Alignment::Center)
            .style(Style::default().fg(colors::CYAN));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let time = Paragraph::new(Span::styled(
            time_str,
            Style::default().fg(fg).add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center);

        let indicator =
            Paragraph::new(Span::styled(status, Style::default().fg(colors::DIM_WHITE))).alignment(Alignment::Center);

        let chunks = Layout::vertical([
            Constraint::Length((inner.height.saturating_sub(6)) / 2),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

        f.render_widget(time, chunks[1]);
        f.render_widget(indicator, chunks[2]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_zero() {
        assert_eq!(format_duration(Duration::ZERO), "00:00:00");
    }

    #[test]
    fn test_format_duration_seconds() {
        assert_eq!(format_duration(Duration::from_secs(3661)), "01:01:01");
    }

    #[test]
    fn test_format_duration_large() {
        assert_eq!(format_duration(Duration::from_secs(86399)), "23:59:59");
    }

    #[test]
    fn test_stopwatch_starts_at_zero() {
        let sw = Stopwatch::new();
        assert_eq!(sw.current_elapsed(), Duration::ZERO);
        assert!(!sw.is_running());
    }

    #[test]
    fn test_toggle_starts_and_stops() {
        let mut sw = Stopwatch::new();
        sw.toggle();
        assert!(sw.is_running());
        sw.toggle();
        assert!(!sw.is_running());
    }

    #[test]
    fn test_reset_clears() {
        let mut sw = Stopwatch::new();
        sw.toggle();
        std::thread::sleep(std::time::Duration::from_millis(10));
        sw.toggle();
        assert!(sw.current_elapsed() > Duration::ZERO);
        sw.reset();
        assert_eq!(sw.current_elapsed(), Duration::ZERO);
        assert!(!sw.is_running());
    }

    #[test]
    fn test_current_elapsed_increases_when_running() {
        let mut sw = Stopwatch::new();
        sw.toggle();
        let e1 = sw.current_elapsed();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let e2 = sw.current_elapsed();
        assert!(e2 >= e1);
    }
}
