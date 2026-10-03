// Casts are safe here because grid dimensions (max ~371) are well within
// all target integer ranges.
#![allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap, clippy::cast_sign_loss)]

use chrono::{Datelike, NaiveDate};

/// A position in the heatmap grid (0-indexed).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridCoords {
    pub col: usize,
    pub row: usize,
}

/// Describes the grid layout for a single year.
pub struct YearGrid {
    pub year: i32,
    /// Column 0 starts on this date (the Monday on or before Jan 1).
    pub start_date: NaiveDate,
    /// Total number of week-columns (52 or 53).
    pub total_cols: usize,
}

/// Build the grid descriptor for a given year.
///
/// The grid is anchored to the Monday on or before January 1.
/// Row 0 = Monday, Row 6 = Sunday.
#[must_use]
pub fn build_grid(year: i32) -> YearGrid {
    let jan_1 = NaiveDate::from_ymd_opt(year, 1, 1).expect("valid year");
    let dec_31 = NaiveDate::from_ymd_opt(year, 12, 31).expect("valid year");
    let jan_1_dow = i64::from(jan_1.weekday().num_days_from_monday());
    let start_date = jan_1 - chrono::Duration::days(jan_1_dow);
    let total_days = (dec_31 - start_date).num_days() + 1;
    // total_days is 365-371, safe for usize even on 32-bit
    let total_cols = (total_days + 6) as usize / 7;
    YearGrid {
        year,
        start_date,
        total_cols,
    }
}

/// Map a date to its grid position.
///
/// Returns `None` if the date is before the grid's start date or at/beyond
/// the end of the grid.
#[must_use]
pub fn date_to_grid(grid: &YearGrid, date: NaiveDate) -> Option<GridCoords> {
    let days = (date - grid.start_date).num_days();
    if days < 0 {
        return None;
    }
    // days is non-negative and fits in usize for reasonable grid sizes
    let days_u = days as usize;
    let col = days_u / 7;
    let row = days_u % 7;
    if col >= grid.total_cols {
        return None;
    }
    Some(GridCoords { col, row })
}

/// Map a grid position back to its calendar date.
///
/// Returns `None` if the position is outside the grid bounds.
#[must_use]
pub fn grid_to_date(grid: &YearGrid, col: usize, row: usize) -> Option<NaiveDate> {
    if col >= grid.total_cols || row >= 7 {
        return None;
    }
    let days = col * 7 + row;
    Some(grid.start_date + chrono::Duration::days(days as i64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_grid_2026() {
        let grid = build_grid(2026);
        assert_eq!(grid.year, 2026);
        assert_eq!(grid.start_date, NaiveDate::from_ymd_opt(2025, 12, 29).unwrap());
        assert_eq!(grid.total_cols, 53);
    }

    #[test]
    fn test_date_to_grid_jan_1_2026() {
        let grid = build_grid(2026);
        let d = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let pos = date_to_grid(&grid, d).unwrap();
        assert_eq!(pos.col, 0);
        assert_eq!(pos.row, 3);
    }

    #[test]
    fn test_date_to_grid_dec_31_2026() {
        let grid = build_grid(2026);
        let d = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        let pos = date_to_grid(&grid, d).unwrap();
        assert_eq!(pos.col, 52);
        assert_eq!(pos.row, 3);
    }

    #[test]
    fn test_grid_roundtrip() {
        let grid = build_grid(2026);
        let d = NaiveDate::from_ymd_opt(2026, 6, 5).unwrap();
        let pos = date_to_grid(&grid, d).unwrap();
        let roundtrip = grid_to_date(&grid, pos.col, pos.row).unwrap();
        assert_eq!(roundtrip, d);
    }

    #[test]
    fn test_date_out_of_bounds_before() {
        let grid = build_grid(2026);
        let d = NaiveDate::from_ymd_opt(2025, 12, 28).unwrap();
        assert!(date_to_grid(&grid, d).is_none());
    }

    #[test]
    fn test_previous_year_date_in_first_week() {
        let grid = build_grid(2026);
        let d = NaiveDate::from_ymd_opt(2025, 12, 29).unwrap();
        let pos = date_to_grid(&grid, d).unwrap();
        assert_eq!(pos.col, 0);
        assert_eq!(pos.row, 0);
    }

    #[test]
    fn test_grid_to_date_out_of_bounds() {
        let grid = build_grid(2026);
        assert!(grid_to_date(&grid, 99, 0).is_none());
        assert!(grid_to_date(&grid, 0, 9).is_none());
    }
}
