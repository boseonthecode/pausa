use anyhow::Result;
use rusqlite::{Connection, params};

/// Aggregated daily study total (for heatmap).
#[derive(Debug, Clone)]
pub struct DailySession {
    pub date: String,
    pub total_seconds: i64,
}

/// Upsert a daily session record.
pub fn upsert(conn: &Connection, daily: &DailySession) -> Result<()> {
    conn.execute(
        "INSERT INTO daily_sessions (date, total_seconds, created_at, updated_at)
         VALUES (?1, ?2, datetime('now'), datetime('now'))
         ON CONFLICT(date) DO UPDATE SET
           total_seconds = excluded.total_seconds,
           updated_at = datetime('now')",
        params![daily.date, daily.total_seconds],
    )?;
    Ok(())
}

/// Get a daily session record for a specific date.
pub fn get_for_date(conn: &Connection, date: &str) -> Result<Option<DailySession>> {
    let mut stmt = conn.prepare("SELECT date, total_seconds FROM daily_sessions WHERE date = ?1")?;
    let mut rows = stmt.query_map(params![date], |row| {
        Ok(DailySession {
            date: row.get(0)?,
            total_seconds: row.get(1)?,
        })
    })?;
    match rows.next() {
        Some(Ok(d)) => Ok(Some(d)),
        _ => Ok(None),
    }
}

/// Sum all study sessions for today and upsert the daily total.
pub fn aggregate_today(conn: &Connection) -> Result<()> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let total: i64 = conn.query_row(
        "SELECT COALESCE(SUM(duration_seconds), 0) FROM sessions WHERE date = ?1 AND session_type = 'study'",
        params![today],
        |row| row.get(0),
    )?;
    let daily = DailySession {
        date: today,
        total_seconds: total,
    };
    upsert(conn, &daily)
}

/// Get all daily sessions for a given year.
pub fn get_year(conn: &Connection, year: i32) -> Result<Vec<DailySession>> {
    let start = format!("{year}-01-01");
    let end = format!("{year}-12-31");
    let mut stmt = conn.prepare(
        "SELECT date, total_seconds FROM daily_sessions
         WHERE date >= ?1 AND date <= ?2
         ORDER BY date",
    )?;
    let rows = stmt.query_map(params![start, end], |row| {
        Ok(DailySession {
            date: row.get(0)?,
            total_seconds: row.get(1)?,
        })
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::create_in_memory;

    #[test]
    fn test_upsert_creates_new() {
        let conn = create_in_memory().unwrap();
        let daily = DailySession {
            date: "2026-06-05".to_string(),
            total_seconds: 3600,
        };
        upsert(&conn, &daily).unwrap();

        let retrieved = get_for_date(&conn, "2026-06-05").unwrap().unwrap();
        assert_eq!(retrieved.total_seconds, 3600);
    }

    #[test]
    fn test_upsert_updates_existing() {
        let conn = create_in_memory().unwrap();
        let daily = DailySession {
            date: "2026-06-05".to_string(),
            total_seconds: 3600,
        };
        upsert(&conn, &daily).unwrap();

        let updated = DailySession {
            date: "2026-06-05".to_string(),
            total_seconds: 7200,
        };
        upsert(&conn, &updated).unwrap();

        let retrieved = get_for_date(&conn, "2026-06-05").unwrap().unwrap();
        assert_eq!(retrieved.total_seconds, 7200);
    }

    #[test]
    fn test_get_year_returns_only_that_year() {
        let conn = create_in_memory().unwrap();
        upsert(
            &conn,
            &DailySession {
                date: "2025-12-31".to_string(),
                total_seconds: 100,
            },
        )
        .unwrap();
        upsert(
            &conn,
            &DailySession {
                date: "2026-01-01".to_string(),
                total_seconds: 200,
            },
        )
        .unwrap();
        upsert(
            &conn,
            &DailySession {
                date: "2026-06-05".to_string(),
                total_seconds: 300,
            },
        )
        .unwrap();
        upsert(
            &conn,
            &DailySession {
                date: "2027-01-01".to_string(),
                total_seconds: 400,
            },
        )
        .unwrap();

        let year_data = get_year(&conn, 2026).unwrap();
        assert_eq!(year_data.len(), 2);
        assert!(year_data.iter().all(|d| d.date.starts_with("2026")));
    }

    #[test]
    fn test_aggregate_today_creates_row() {
        let conn = create_in_memory().unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        // Insert a study session for today
        conn.execute(
            "INSERT INTO sessions (date, start_time, end_time, duration_seconds, session_type)
             VALUES (?1, 'now', 'now', 3600, 'study')",
            params![today],
        )
        .unwrap();
        aggregate_today(&conn).unwrap();
        let daily = get_for_date(&conn, &today).unwrap().unwrap();
        assert_eq!(daily.total_seconds, 3600);
    }

    #[test]
    fn test_aggregate_today_updates_existing() {
        let conn = create_in_memory().unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        // Insert sessions and aggregate
        conn.execute(
            "INSERT INTO sessions (date, start_time, end_time, duration_seconds, session_type)
             VALUES (?1, 'now', 'now', 1800, 'study')",
            params![today],
        )
        .unwrap();
        aggregate_today(&conn).unwrap();
        // Insert another session and aggregate again
        conn.execute(
            "INSERT INTO sessions (date, start_time, end_time, duration_seconds, session_type)
             VALUES (?1, 'now', 'now', 900, 'study')",
            params![today],
        )
        .unwrap();
        aggregate_today(&conn).unwrap();
        let daily = get_for_date(&conn, &today).unwrap().unwrap();
        assert_eq!(daily.total_seconds, 2700);
    }

    #[test]
    fn test_aggregate_ignores_break_sessions() {
        let conn = create_in_memory().unwrap();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        // Insert a break session
        conn.execute(
            "INSERT INTO sessions (date, start_time, end_time, duration_seconds, session_type)
             VALUES (?1, 'now', 'now', 600, 'short_break')",
            params![today],
        )
        .unwrap();
        aggregate_today(&conn).unwrap();
        let daily = get_for_date(&conn, &today).unwrap().unwrap();
        assert_eq!(daily.total_seconds, 0);
    }

    #[test]
    fn test_get_for_date_missing() {
        let conn = create_in_memory().unwrap();
        let result = get_for_date(&conn, "2026-06-05").unwrap();
        assert!(result.is_none());
    }
}
