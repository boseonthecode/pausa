pub mod grid;

use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::Connection;

/// A single day's study data for the heatmap.
#[derive(Debug, Clone)]
pub struct CalendarDay {
    pub date: NaiveDate,
    pub total_seconds: i64,
    pub intensity: u8,
}

/// Determine the heatmap intensity level (0–4) based on total study seconds.
///
/// Thresholds:
///   0: 0 s
///   1: 1 – 1 800 s   (≤30 min)
///   2: 1 801 – 7 200 s (30 min – 2 h)
///   3: 7 201 – 14 400 s (2 h – 4 h)
///   4: ≥14 401 s         (≥4 h)
#[must_use]
pub fn compute_intensity(seconds: i64) -> u8 {
    if seconds == 0 {
        0
    } else if seconds <= 1_800 {
        1
    } else if seconds <= 7_200 {
        2
    } else if seconds <= 14_400 {
        3
    } else {
        4
    }
}

/// Fetch all days for a year, filling in missing days with zero.
pub fn load_year(conn: &Connection, year: i32) -> Result<Vec<CalendarDay>> {
    let raw = super::db::daily::get_year(conn, year)?;
    let mut map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for d in &raw {
        map.insert(d.date.clone(), d.total_seconds);
    }

    let start = NaiveDate::from_ymd_opt(year, 1, 1).expect("valid year");
    let end = NaiveDate::from_ymd_opt(year, 12, 31).expect("valid year");
    let mut days = Vec::with_capacity(366);
    let mut current = start;
    while current <= end {
        let key = current.format("%Y-%m-%d").to_string();
        let seconds = map.get(&key).copied().unwrap_or(0);
        days.push(CalendarDay {
            date: current,
            total_seconds: seconds,
            intensity: compute_intensity(seconds),
        });
        current = current.succ_opt().expect("valid date");
    }
    Ok(days)
}

/// Find the maximum daily total for a given year.
pub fn max_daily(conn: &Connection, year: i32) -> Result<i64> {
    let start = format!("{year}-01-01");
    let end = format!("{year}-12-31");
    let max: i64 = conn.query_row(
        "SELECT COALESCE(MAX(total_seconds), 0) FROM daily_sessions WHERE date >= ?1 AND date <= ?2",
        rusqlite::params![start, end],
        |row| row.get(0),
    )?;
    Ok(max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::db::daily;
    use chrono::Datelike;

    fn seed_db(conn: &Connection) {
        daily::upsert(
            conn,
            &daily::DailySession {
                date: "2026-06-05".to_string(),
                total_seconds: 3600,
            },
        )
        .unwrap();
        daily::upsert(
            conn,
            &daily::DailySession {
                date: "2026-12-25".to_string(),
                total_seconds: 18000,
            },
        )
        .unwrap();
    }

    #[test]
    fn test_compute_intensity_zero() {
        assert_eq!(compute_intensity(0), 0);
    }

    #[test]
    fn test_compute_intensity_boundaries() {
        assert_eq!(compute_intensity(1), 1);
        assert_eq!(compute_intensity(1_800), 1);
        assert_eq!(compute_intensity(1_801), 2);
        assert_eq!(compute_intensity(7_200), 2);
        assert_eq!(compute_intensity(7_201), 3);
        assert_eq!(compute_intensity(14_400), 3);
        assert_eq!(compute_intensity(14_401), 4);
    }

    #[test]
    fn test_load_year_fills_missing_days() {
        let conn = db::create_in_memory().unwrap();
        seed_db(&conn);
        let days = load_year(&conn, 2026).unwrap();
        // 2026 is a common year (365 days)
        assert_eq!(days.len(), 365);
        // Missing days should have intensity 0
        let jan_1 = days.iter().find(|d| d.date.month() == 1 && d.date.day() == 1).unwrap();
        assert_eq!(jan_1.intensity, 0);
        assert_eq!(jan_1.total_seconds, 0);
        // Known days should have correct data
        let jun_5 = days.iter().find(|d| d.date.month() == 6 && d.date.day() == 5).unwrap();
        assert_eq!(jun_5.total_seconds, 3600);
        assert_eq!(jun_5.intensity, 2);
        let dec_25 = days
            .iter()
            .find(|d| d.date.month() == 12 && d.date.day() == 25)
            .unwrap();
        assert_eq!(dec_25.total_seconds, 18000);
        assert_eq!(dec_25.intensity, 4);
    }

    #[test]
    fn test_load_year_empty() {
        let conn = db::create_in_memory().unwrap();
        let days = load_year(&conn, 2026).unwrap();
        assert_eq!(days.len(), 365);
        assert!(days.iter().all(|d| d.total_seconds == 0));
    }

    #[test]
    fn test_max_daily() {
        let conn = db::create_in_memory().unwrap();
        seed_db(&conn);
        let max = max_daily(&conn, 2026).unwrap();
        assert_eq!(max, 18000);
    }

    #[test]
    fn test_max_daily_empty() {
        let conn = db::create_in_memory().unwrap();
        let max = max_daily(&conn, 2026).unwrap();
        assert_eq!(max, 0);
    }
}
