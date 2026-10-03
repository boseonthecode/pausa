use std::time::{Duration, Instant};

use chrono::{Datelike, Timelike};

use crate::config::Config;
use crate::db;
use crate::tui;
use crate::tui::break_view;
use crate::tui::dialogs;
use crate::tui::stopwatch::Stopwatch;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Paragraph};
use rusqlite::Connection;

/// The possible screens in the application.
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Stopwatch,
    ShortBreak { total: Duration, started_at: Instant },
    ShortBreakFlash { flash_started_at: Instant },
    EndlessBreak,
    Heatmap,
}

/// Core application state machine.
pub struct App {
    pub state: AppState,
    pub stopwatch: Stopwatch,
    pub showing_reset_dialog: bool,
    pub conn: Connection,
    pub config: Config,
    pub current_session_start: Option<Instant>,
    pub current_break_start: Option<Instant>,
    pub last_saved_elapsed: Duration,
    pub last_reset_date: Option<String>,
    pub last_reset_check: Instant,
    pub last_error: Option<String>,
}

impl App {
    /// Create a new application in the Stopwatch state.
    pub fn new(conn: Connection, config: Config) -> Self {
        Self {
            state: AppState::Stopwatch,
            stopwatch: Stopwatch::new(),
            showing_reset_dialog: false,
            conn,
            config,
            current_session_start: None,
            current_break_start: None,
            last_saved_elapsed: Duration::ZERO,
            last_reset_date: None,
            last_reset_check: Instant::now(),
            last_error: None,
        }
    }

    /// Run the main event loop until the user quits.
    pub fn run(&mut self, terminal: &mut tui::Tui) -> Result<()> {
        loop {
            self.update();
            terminal.draw(|f| self.render(f))?;

            if let Some(event) = tui::poll_key()?
                && self.handle_key(event)
            {
                break;
            }
        }
        Ok(())
    }

    /// Parse the config `reset_time` ("HH:MM") into total minutes.
    fn parse_reset_time(s: &str) -> u32 {
        let parts: Vec<&str> = s.split(':').collect();
        let h: u32 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let m: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        h * 60 + m
    }

    /// Check whether the configured daily reset time has passed and trigger a reset if so.
    fn check_reset(&mut self) {
        let now = chrono::Local::now();
        let today = now.format("%Y-%m-%d").to_string();

        // Already reset today
        if self.last_reset_date.as_ref() == Some(&today) {
            return;
        }

        let reset_minutes = Self::parse_reset_time(&self.config.reset_time);
        let current_minutes = now.hour() * 60 + now.minute();

        if current_minutes >= reset_minutes {
            self.save_in_flight_session();
            self.stopwatch.reset();
            self.last_saved_elapsed = Duration::ZERO;
            self.current_session_start = None;
            self.last_reset_date = Some(today);
        }
    }

    /// Update state transitions (called before each render).
    fn update(&mut self) {
        // Check for daily reset at most once per second
        if self.last_reset_check.elapsed() >= Duration::from_secs(1) {
            self.check_reset();
            self.last_reset_check = Instant::now();
        }
        if let AppState::ShortBreak { total, started_at } = self.state
            && started_at.elapsed() >= total
        {
            self.state = AppState::ShortBreakFlash {
                flash_started_at: Instant::now(),
            };
        }
    }

    /// Save the current study session (if any) to the database.
    /// Errors are captured in `last_error` for display.
    fn save_current_study_session(&mut self) {
        let elapsed = self.stopwatch.current_elapsed();
        let new_duration = elapsed.saturating_sub(self.last_saved_elapsed);
        if new_duration.is_zero() {
            return;
        }

        let now = chrono::Local::now();
        let date = now.format("%Y-%m-%d").to_string();
        let end_time = now.format("%Y-%m-%dT%H:%M:%S").to_string();
        let duration_secs = new_duration.as_secs().cast_signed();
        let start_time = (now - chrono::TimeDelta::seconds(duration_secs))
            .format("%Y-%m-%dT%H:%M:%S")
            .to_string();

        let session = db::sessions::Session {
            id: None,
            date,
            start_time,
            end_time: Some(end_time),
            duration_seconds: Some(duration_secs),
            kind: db::sessions::SessionType::Study,
        };
        if let Err(e) = db::sessions::insert(&self.conn, &session) {
            self.last_error = Some(format!("Failed to save study session: {e}"));
            return;
        }
        if let Err(e) = db::daily::aggregate_today(&self.conn) {
            self.last_error = Some(format!("Failed to update daily total: {e}"));
            return;
        }
        self.last_saved_elapsed = elapsed;
        self.current_session_start = None;
    }

    /// Save the current break session (if any) to the database.
    /// Errors are captured in `last_error` for display.
    fn save_current_break(&mut self) {
        if let Some(start) = self.current_break_start {
            let duration = start.elapsed().as_secs().cast_signed();
            let now = chrono::Local::now();
            let date = now.format("%Y-%m-%d").to_string();
            let end_time = now.format("%Y-%m-%dT%H:%M:%S").to_string();
            let start_time = (now - chrono::TimeDelta::seconds(duration))
                .format("%Y-%m-%dT%H:%M:%S")
                .to_string();

            let kind = match self.state {
                AppState::EndlessBreak => db::sessions::SessionType::EndlessBreak,
                _ => db::sessions::SessionType::ShortBreak,
            };

            let session = db::sessions::Session {
                id: None,
                date,
                start_time,
                end_time: Some(end_time),
                duration_seconds: Some(duration),
                kind,
            };
            if let Err(e) = db::sessions::insert(&self.conn, &session) {
                self.last_error = Some(format!("Failed to save break session: {e}"));
                return;
            }
            self.current_break_start = None;
        }
    }

    /// Save any in-flight session (study or break) before quitting.
    fn save_in_flight_session(&mut self) {
        match self.state {
            AppState::Stopwatch => {
                self.save_current_study_session();
            }
            AppState::ShortBreak { .. } | AppState::ShortBreakFlash { .. } | AppState::EndlessBreak => {
                self.save_current_break();
            }
            AppState::Heatmap => {}
        }
    }

    /// Handle a key press. Returns `true` if the app should quit.
    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if tui::is_quit_key(&key) {
            self.save_in_flight_session();
            return true;
        }
        match key.code {
            KeyCode::Char(' ') => match (&self.state, self.showing_reset_dialog) {
                (AppState::ShortBreak { .. } | AppState::ShortBreakFlash { .. } | AppState::EndlessBreak, _) => {
                    self.save_current_break();
                    self.state = AppState::Stopwatch;
                    self.current_session_start = Some(Instant::now());
                }
                (AppState::Stopwatch, false) => {
                    self.stopwatch.toggle();
                    if self.stopwatch.is_running() {
                        self.current_session_start = Some(Instant::now());
                    }
                }
                _ => {}
            },
            KeyCode::Char('b') if self.state == AppState::Stopwatch && !self.showing_reset_dialog => {
                self.save_current_study_session();
                let total = Duration::from_secs(self.config.short_break_minutes * 60);
                self.state = AppState::ShortBreak {
                    total,
                    started_at: Instant::now(),
                };
                self.current_break_start = Some(Instant::now());
            }
            KeyCode::Char('B') if self.state == AppState::Stopwatch && !self.showing_reset_dialog => {
                self.save_current_study_session();
                self.state = AppState::EndlessBreak;
                self.current_break_start = Some(Instant::now());
            }
            KeyCode::Char('h') if self.state == AppState::Stopwatch && !self.showing_reset_dialog => {
                self.state = AppState::Heatmap;
            }
            KeyCode::Char('h') if self.state == AppState::Heatmap => {
                self.state = AppState::Stopwatch;
            }
            KeyCode::Char('r') if self.state == AppState::Stopwatch && !self.showing_reset_dialog => {
                if self.stopwatch.is_running() {
                    self.stopwatch.toggle();
                }
                self.showing_reset_dialog = true;
            }
            KeyCode::Char('y') if self.showing_reset_dialog => {
                self.save_current_study_session();
                self.stopwatch.reset();
                self.last_saved_elapsed = Duration::ZERO;
                self.current_session_start = None;
                self.showing_reset_dialog = false;
            }
            KeyCode::Char('n') if self.showing_reset_dialog => {
                self.showing_reset_dialog = false;
            }
            _ => {}
        }
        false
    }

    /// Render the current state to the screen.
    fn render(&self, f: &mut Frame) {
        let area = f.area();
        match self.state {
            AppState::Stopwatch => {
                self.stopwatch.render(f, area);
                if self.showing_reset_dialog {
                    dialogs::render_confirm_reset(f, area);
                }
            }
            AppState::ShortBreak { total, started_at } => {
                break_view::render_short_break(f, area, total, started_at);
            }
            AppState::ShortBreakFlash { flash_started_at } => {
                break_view::render_short_break_flash(f, area, flash_started_at);
            }
            AppState::EndlessBreak => {
                break_view::render_endless_break(f, area);
            }
            AppState::Heatmap => tui::heatmap::render_heatmap(f, area, &self.conn, chrono::Local::now().year()),
        }
        if let Some(ref err) = self.last_error {
            let err_bar = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
            let text = Paragraph::new(Span::styled(
                format!(" Error: {err} "),
                Style::default().fg(tui::colors::AMBER),
            ));
            f.render_widget(text, err_bar[1]);
        }
    }
}

/// Generic placeholder for non-implemented states.
fn render_placeholder(f: &mut Frame, label: &str) {
    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {label} "))
        .title_alignment(Alignment::Center)
        .style(Style::default().fg(tui::colors::CYAN));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text = Paragraph::new(Span::styled(
        "Coming soon\u{2026}",
        Style::default().fg(tui::colors::DIM_WHITE),
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
    use crate::config::Config;
    use crate::db::create_in_memory;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn make_app() -> App {
        let conn = create_in_memory().unwrap();
        let config = Config {
            short_break_minutes: 10,
            reset_time: "00:00".to_string(),
        };
        App::new(conn, config)
    }

    fn make_key(code: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(code), KeyModifiers::NONE)
    }

    fn make_ctrl_c() -> KeyEvent {
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)
    }

    #[test]
    fn test_app_initial_state() {
        let app = make_app();
        assert_eq!(app.state, AppState::Stopwatch);
        assert!(!app.stopwatch.is_running());
        assert!(!app.showing_reset_dialog);
    }

    #[test]
    fn test_quit_key_q_returns_true() {
        let mut app = make_app();
        assert!(app.handle_key(make_key('q')));
    }

    #[test]
    fn test_ctrl_c_returns_true() {
        let mut app = make_app();
        assert!(app.handle_key(make_ctrl_c()));
    }

    #[test]
    fn test_space_toggles_stopwatch() {
        let mut app = make_app();
        app.handle_key(make_key(' '));
        assert!(app.stopwatch.is_running());
        app.handle_key(make_key(' '));
        assert!(!app.stopwatch.is_running());
    }

    #[test]
    fn test_b_starts_short_break() {
        let mut app = make_app();
        app.handle_key(make_key('b'));
        assert!(matches!(app.state, AppState::ShortBreak { .. }));
    }

    #[test]
    fn test_cap_b_starts_endless_break() {
        let mut app = make_app();
        app.handle_key(make_key('B'));
        assert_eq!(app.state, AppState::EndlessBreak);
    }

    #[test]
    fn test_space_resumes_from_break() {
        let mut app = make_app();
        app.handle_key(make_key('b'));
        app.handle_key(make_key(' '));
        assert_eq!(app.state, AppState::Stopwatch);
    }

    #[test]
    fn test_short_break_transitions_to_flash() {
        let mut app = make_app();
        app.state = AppState::ShortBreak {
            total: Duration::from_millis(1),
            started_at: Instant::now(),
        };
        std::thread::sleep(std::time::Duration::from_millis(10));
        app.update();
        assert!(matches!(app.state, AppState::ShortBreakFlash { .. }));
    }

    #[test]
    fn test_r_shows_dialog() {
        let mut app = make_app();
        app.handle_key(make_key('r'));
        assert!(app.showing_reset_dialog);
    }

    #[test]
    fn test_y_resets_timer() {
        let mut app = make_app();
        app.stopwatch.toggle();
        std::thread::sleep(std::time::Duration::from_millis(10));
        app.stopwatch.toggle();
        assert!(app.stopwatch.current_elapsed() > Duration::ZERO);

        app.showing_reset_dialog = true;
        app.handle_key(make_key('y'));
        assert!(!app.showing_reset_dialog);
        assert_eq!(app.stopwatch.current_elapsed(), Duration::ZERO);
    }

    #[test]
    fn test_n_dismisses_dialog() {
        let mut app = make_app();
        app.stopwatch.toggle();
        app.showing_reset_dialog = true;
        app.handle_key(make_key('n'));
        assert!(!app.showing_reset_dialog);
        assert!(app.stopwatch.is_running());
    }

    #[test]
    fn test_space_saves_study_session_on_break() {
        let mut app = make_app();
        // Start stopwatch and let some time pass
        app.handle_key(make_key(' '));
        std::thread::sleep(std::time::Duration::from_millis(5));
        // Start a short break — should save the study session
        app.handle_key(make_key('b'));
        let sessions =
            db::sessions::get_for_date(&app.conn, &chrono::Local::now().format("%Y-%m-%d").to_string()).unwrap();
        let study_sessions: Vec<_> = sessions
            .iter()
            .filter(|s| s.kind == db::sessions::SessionType::Study)
            .collect();
        assert_eq!(study_sessions.len(), 1);
    }

    #[test]
    fn test_break_saves_break_session_on_resume() {
        let mut app = make_app();
        // Enter short break
        app.handle_key(make_key('b'));
        std::thread::sleep(std::time::Duration::from_millis(2));
        // Resume — should save the break session
        app.handle_key(make_key(' '));
        let sessions =
            db::sessions::get_for_date(&app.conn, &chrono::Local::now().format("%Y-%m-%d").to_string()).unwrap();
        let break_sessions: Vec<_> = sessions
            .iter()
            .filter(|s| s.kind == db::sessions::SessionType::ShortBreak)
            .collect();
        assert_eq!(break_sessions.len(), 1);
    }

    #[test]
    fn test_reset_saves_study_session() {
        let mut app = make_app();
        // Start stopwatch and let time pass
        app.handle_key(make_key(' '));
        std::thread::sleep(std::time::Duration::from_millis(5));
        // Open reset dialog
        app.handle_key(make_key('r'));
        // Confirm reset — should save study session
        app.handle_key(make_key('y'));
        let sessions =
            db::sessions::get_for_date(&app.conn, &chrono::Local::now().format("%Y-%m-%d").to_string()).unwrap();
        let study_sessions: Vec<_> = sessions
            .iter()
            .filter(|s| s.kind == db::sessions::SessionType::Study)
            .collect();
        assert_eq!(study_sessions.len(), 1);
    }

    #[test]
    fn test_quit_saves_in_flight_study_session() {
        let mut app = make_app();
        // Start stopwatch and let time pass
        app.handle_key(make_key(' '));
        std::thread::sleep(std::time::Duration::from_millis(5));
        // Quit — should save study session
        assert!(app.handle_key(make_key('q')));
        let sessions =
            db::sessions::get_for_date(&app.conn, &chrono::Local::now().format("%Y-%m-%d").to_string()).unwrap();
        let study_sessions: Vec<_> = sessions
            .iter()
            .filter(|s| s.kind == db::sessions::SessionType::Study)
            .collect();
        assert_eq!(study_sessions.len(), 1);
    }

    #[test]
    fn test_quit_saves_in_flight_break_session() {
        let mut app = make_app();
        // Enter an endless break
        app.handle_key(make_key('B'));
        std::thread::sleep(std::time::Duration::from_millis(2));
        // Quit — should save break session
        assert!(app.handle_key(make_key('q')));
        let sessions =
            db::sessions::get_for_date(&app.conn, &chrono::Local::now().format("%Y-%m-%d").to_string()).unwrap();
        let break_sessions: Vec<_> = sessions
            .iter()
            .filter(|s| s.kind == db::sessions::SessionType::EndlessBreak)
            .collect();
        assert_eq!(break_sessions.len(), 1);
    }

    #[test]
    fn test_parse_reset_time_default_midnight() {
        assert_eq!(App::parse_reset_time("00:00"), 0);
    }

    #[test]
    fn test_parse_reset_time_custom() {
        assert_eq!(App::parse_reset_time("04:00"), 240);
        assert_eq!(App::parse_reset_time("23:59"), 1439);
    }

    #[test]
    fn test_parse_reset_time_invalid_falls_back() {
        assert_eq!(App::parse_reset_time("not-a-time"), 0);
    }

    #[test]
    fn test_reset_time_triggers_after_threshold() {
        let conn = create_in_memory().unwrap();
        let config = Config {
            short_break_minutes: 10,
            reset_time: "00:00".to_string(),
        };
        let mut app = App::new(conn, config);
        // Reset check fires once per second; force timing by setting last_reset_check far back
        app.last_reset_check = Instant::now() - Duration::from_secs(2);
        // last_reset_date is None, so it should trigger at midnight (00:00)
        // Current time should be >= 00:00, so reset triggers
        app.check_reset();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        assert_eq!(app.last_reset_date.as_deref(), Some(today.as_str()));
    }

    #[test]
    fn test_reset_time_skips_after_first_reset() {
        let conn = create_in_memory().unwrap();
        let config = Config {
            short_break_minutes: 10,
            reset_time: "00:00".to_string(),
        };
        let mut app = App::new(conn, config);
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        app.last_reset_date = Some(today.clone());
        app.last_reset_check = Instant::now() - Duration::from_secs(2);
        app.check_reset();
        // Should not change since we already reset today
        assert_eq!(app.last_reset_date.as_deref(), Some(today.as_str()));
    }

    #[test]
    fn test_b_uses_config_break_duration() {
        let conn = create_in_memory().unwrap();
        let config = Config {
            short_break_minutes: 5,
            reset_time: "00:00".to_string(),
        };
        let mut app = App::new(conn, config);
        app.handle_key(make_key('b'));
        assert!(matches!(app.state, AppState::ShortBreak { total, .. } if total == Duration::from_secs(300)));
    }
}
