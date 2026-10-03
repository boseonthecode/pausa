use crate::config::Config;

pub fn default_config() -> Config {
    Config {
        short_break_minutes: 10,
        reset_time: "00:00".to_string(),
    }
}
