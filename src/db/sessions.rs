use anyhow::Result;
use rusqlite::{Connection, params};

/// The type of a session block.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionType {
    Study,
    ShortBreak,
    EndlessBreak,
}

impl SessionType {
    fn as_str(&self) -> &'static str {
        match self {
            SessionType::Study => "study",
            SessionType::ShortBreak => "short_break",
            SessionType::EndlessBreak => "endless_break",
        }
    }

    fn from_str(s: &str) -> Option<SessionType> {
        match s {
            "study" => Some(SessionType::Study),
            "short_break" => Some(SessionType::ShortBreak),
            "endless_break" => Some(SessionType::EndlessBreak),
            _ => None,
        }
    }
}

/// A single session block (study or break).
#[derive(Debug, Clone)]
pub struct Session {
    pub id: Option<i64>,
    pub date: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub duration_seconds: Option<i64>,
    pub kind: SessionType,
}

/// Insert a new session. Returns the row ID.
pub fn insert(conn: &Connection, session: &Session) -> Result<i64> {
    conn.execute(
        "INSERT INTO sessions (date, start_time, end_time, duration_seconds, session_type)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            session.date,
            session.start_time,
            session.end_time,
            session.duration_seconds,
            session.kind.as_str(),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Get all sessions for a given date.
pub fn get_for_date(conn: &Connection, date: &str) -> Result<Vec<Session>> {
    let mut stmt = conn.prepare(
        "SELECT id, date, start_time, end_time, duration_seconds, session_type
         FROM sessions WHERE date = ?1 ORDER BY start_time",
    )?;
    let rows = stmt.query_map(params![date], |row| {
        let kind_str: String = row.get(5)?;
        let kind = SessionType::from_str(&kind_str)
            .ok_or_else(|| rusqlite::Error::InvalidParameterName(format!("unknown session_type: {kind_str}")))?;
        Ok(Session {
            id: Some(row.get(0)?),
            date: row.get(1)?,
            start_time: row.get(2)?,
            end_time: row.get(3)?,
            duration_seconds: row.get(4)?,
            kind,
        })
    })?;
    let mut sessions = Vec::new();
    for row in rows {
        sessions.push(row?);
    }
    Ok(sessions)
}

/// Get the most recent session (by `start_time`).
pub fn get_latest(conn: &Connection) -> Result<Option<Session>> {
    let mut stmt = conn.prepare(
        "SELECT id, date, start_time, end_time, duration_seconds, session_type
         FROM sessions ORDER BY start_time DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map([], |row| {
        let kind_str: String = row.get(5)?;
        let kind = SessionType::from_str(&kind_str)
            .ok_or_else(|| rusqlite::Error::InvalidParameterName(format!("unknown session_type: {kind_str}")))?;
        Ok(Session {
            id: Some(row.get(0)?),
            date: row.get(1)?,
            start_time: row.get(2)?,
            end_time: row.get(3)?,
            duration_seconds: row.get(4)?,
            kind,
        })
    })?;
    match rows.next() {
        Some(Ok(s)) => Ok(Some(s)),
        _ => Ok(None),
    }
}

/// Update the `end_time` and duration of a session.
pub fn update_end_time(conn: &Connection, id: i64, end_time: &str, duration_seconds: i64) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET end_time = ?1, duration_seconds = ?2 WHERE id = ?3",
        params![end_time, duration_seconds, id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::create_in_memory;

    #[test]
    fn test_insert_and_retrieve() {
        let conn = create_in_memory().unwrap();
        let session = Session {
            id: None,
            date: "2026-06-05".to_string(),
            start_time: "2026-06-05T10:00:00".to_string(),
            end_time: Some("2026-06-05T11:30:00".to_string()),
            duration_seconds: Some(5400),
            kind: SessionType::Study,
        };
        let id = insert(&conn, &session).unwrap();
        assert!(id > 0);

        let sessions = get_for_date(&conn, "2026-06-05").unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].duration_seconds, Some(5400));
    }

    #[test]
    fn test_get_latest_empty() {
        let conn = create_in_memory().unwrap();
        let latest = get_latest(&conn).unwrap();
        assert!(latest.is_none());
    }

    #[test]
    fn test_get_latest_returns_most_recent() {
        let conn = create_in_memory().unwrap();
        let s1 = Session {
            id: None,
            date: "2026-06-05".to_string(),
            start_time: "2026-06-05T08:00:00".to_string(),
            end_time: None,
            duration_seconds: None,
            kind: SessionType::Study,
        };
        let s2 = Session {
            id: None,
            date: "2026-06-05".to_string(),
            start_time: "2026-06-05T10:00:00".to_string(),
            end_time: None,
            duration_seconds: None,
            kind: SessionType::Study,
        };
        insert(&conn, &s1).unwrap();
        insert(&conn, &s2).unwrap();
        let latest = get_latest(&conn).unwrap().unwrap();
        assert_eq!(latest.start_time, "2026-06-05T10:00:00");
    }

    #[test]
    fn test_update_end_time() {
        let conn = create_in_memory().unwrap();
        let session = Session {
            id: None,
            date: "2026-06-05".to_string(),
            start_time: "2026-06-05T10:00:00".to_string(),
            end_time: None,
            duration_seconds: None,
            kind: SessionType::Study,
        };
        let id = insert(&conn, &session).unwrap();
        update_end_time(&conn, id, "2026-06-05T11:30:00", 5400).unwrap();

        let sessions = get_for_date(&conn, "2026-06-05").unwrap();
        assert_eq!(sessions[0].end_time.as_deref(), Some("2026-06-05T11:30:00"));
        assert_eq!(sessions[0].duration_seconds, Some(5400));
    }
}
