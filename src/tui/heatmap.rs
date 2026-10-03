use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use rusqlite::Connection;

use crate::calendar;
use crate::calendar::CalendarDay;
use crate::calendar::grid::{YearGrid, build_grid, date_to_grid, grid_to_date};
use crate::tui::colors;

/// Return the background colour for a given heatmap intensity level.
fn intensity_color(intensity: u8, is_today: bool) -> ratatui::style::Color {
    if is_today {
        return colors::HEATMAP_TODAY;
    }
    match intensity {
        0 => colors::HEATMAP_0,
        1 => colors::HEATMAP_1,
        2 => colors::HEATMAP_2,
        3 => colors::HEATMAP_3,
        _ => colors::HEATMAP_4,
    }
}

/// Build the month‑label header line.
fn build_month_label_line(grid: &YearGrid) -> Line<'static> {
    let month_names: [&str; 13] = [
        "", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    // Find the first column for each month (where the 1st falls).
    let mut month_cols: Vec<(usize, &str)> = Vec::new();
    for m in 1..=12 {
        if let Some(first) = NaiveDate::from_ymd_opt(grid.year, m, 1)
            && let Some(pos) = date_to_grid(grid, first)
        {
            month_cols.push((pos.col, month_names[m as usize]));
        }
    }

    let cell_w: usize = 2;
    let day_label_w: usize = 4;
    let mut spans = vec![Span::raw(" ".repeat(day_label_w))];

    let col_width = cell_w;
    let mut col = 0_usize;
    for &(m_col, label) in &month_cols {
        while col < m_col {
            spans.push(Span::raw(" ".repeat(col_width)));
            col += 1;
        }
        // The month label may be wider than `col_width`; it will overlap
        // into the next cell, which is fine for terminal rendering.
        spans.push(Span::styled(
            format!("{label:>col_width$}"),
            Style::default().fg(colors::DIM_WHITE),
        ));
        col += 1;
    }
    while col < grid.total_cols {
        spans.push(Span::raw(" ".repeat(col_width)));
        col += 1;
    }

    Line::from(spans)
}

/// Build the legend bar: "Less  ██ ██ ██ ██ ██  More"
fn build_legend() -> Line<'static> {
    let intensities = [0, 1, 2, 3, 4];
    let mut spans: Vec<Span> = vec![Span::styled("Less ", Style::default().fg(colors::DIM_WHITE))];
    for level in intensities {
        let bg = intensity_color(level, false);
        spans.push(Span::styled("  ", Style::default().bg(bg)));
    }
    spans.push(Span::styled(" More", Style::default().fg(colors::DIM_WHITE)));
    Line::from(spans)
}

/// Minimum terminal width (in columns) required to render the heatmap grid.
const MIN_WIDTH: u16 = 60;

/// Render the full‑screen heatmap calendar for the given year.
pub fn render_heatmap(f: &mut Frame, area: Rect, conn: &Connection, year: i32) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Calendar — {year} "))
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(colors::CYAN));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Check for narrow terminal before building the grid.
    if inner.width < MIN_WIDTH {
        let msg = Paragraph::new(Line::from(Span::styled(
            format!("Terminal too narrow (width {}) — resize to view heatmap", inner.width),
            Style::default().fg(colors::AMBER),
        )))
        .alignment(Alignment::Center);
        let vertical = Layout::vertical([
            Constraint::Length((inner.height.saturating_sub(3)) / 2),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(inner);
        f.render_widget(msg, vertical[1]);
        return;
    }

    let grid = build_grid(year);
    let days = calendar::load_year(conn, year).unwrap_or_default();
    let day_map: HashMap<NaiveDate, &CalendarDay> = days.iter().map(|d| (d.date, d)).collect();
    let today = chrono::Local::now().date_naive();

    let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mut lines: Vec<Line> = Vec::new();

    // Month label row
    lines.push(build_month_label_line(&grid));

    // Seven day‑of‑week rows
    for (row, name) in day_names.iter().enumerate() {
        let mut spans: Vec<Span> = vec![Span::styled(
            format!("{name:3} "),
            Style::default().fg(colors::DIM_WHITE),
        )];
        for col in 0..grid.total_cols {
            if let Some(date) = grid_to_date(&grid, col, row) {
                if date.year() == grid.year {
                    let is_today = date == today;
                    let intensity = day_map.get(&date).map_or(0, |d| d.intensity);
                    let bg = intensity_color(intensity, is_today);
                    spans.push(Span::styled("  ", Style::default().bg(bg)));
                } else {
                    spans.push(Span::raw("  "));
                }
            } else {
                spans.push(Span::raw("  "));
            }
        }
        lines.push(Line::from(spans));
    }

    // Spacer + legend
    lines.push(Line::from(""));
    lines.push(build_legend());

    let content = Paragraph::new(lines);
    f.render_widget(content, inner);
}
