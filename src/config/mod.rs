mod defaults;

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default = "default_short_break_minutes")]
    pub short_break_minutes: u64,
    #[serde(default = "default_reset_time")]
    pub reset_time: String,
}

fn default_short_break_minutes() -> u64 {
    10
}

fn default_reset_time() -> String {
    "00:00".to_string()
}

impl Config {
    /// Load config from the default config file path.
    /// If the file does not exist, write defaults and return them.
    pub fn load() -> Result<Config> {
        let path = config_path();
        if !path.exists() {
            let cfg = defaults::default_config();
            save_default()?;
            return Ok(cfg);
        }
        let content =
            fs::read_to_string(&path).with_context(|| format!("failed to read config at {}", path.display()))?;
        let cfg: Config = toml::from_str(&content).with_context(|| "failed to parse config")?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Validate config values, returning an error if any are invalid.
    fn validate(&self) -> Result<()> {
        if self.short_break_minutes == 0 {
            bail!("short_break_minutes must be greater than 0");
        }
        let parts: Vec<&str> = self.reset_time.split(':').collect();
        if parts.len() != 2 {
            bail!("reset_time must be in HH:MM format (e.g. \"00:00\")");
        }
        let h: u32 = parts[0]
            .parse()
            .map_err(|_| anyhow::anyhow!("reset_time hour must be a number (0-23)"))?;
        let m: u32 = parts[1]
            .parse()
            .map_err(|_| anyhow::anyhow!("reset_time minute must be a number (0-59)"))?;
        if h > 23 {
            bail!("reset_time hour must be between 0 and 23");
        }
        if m > 59 {
            bail!("reset_time minute must be between 0 and 59");
        }
        Ok(())
    }

    /// Return the contents of the default config as a TOML string.
    fn default_toml() -> String {
        let cfg = defaults::default_config();
        format!(
            r#"short_break_minutes = {}

reset_time = "{}"
"#,
            cfg.short_break_minutes, cfg.reset_time
        )
    }
}

/// Return the path to the config file.
pub fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("pausa").join("config.toml")
}

/// Write the default config file.
pub fn save_default() -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create config directory")?;
    }
    fs::write(&path, Config::default_toml()).context("failed to write default config")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_toml_contains_keys() {
        let toml = Config::default_toml();
        assert!(toml.contains("short_break_minutes"));
        assert!(toml.contains("reset_time"));
    }

    #[test]
    fn test_parse_valid_toml() {
        let toml = r#"short_break_minutes = 5
reset_time = "01:00"
"#;
        let cfg: Config = toml::from_str(toml).unwrap();
        assert_eq!(cfg.short_break_minutes, 5);
        assert_eq!(cfg.reset_time, "01:00");
    }

    #[test]
    fn test_parse_default_toml() {
        let toml = Config::default_toml();
        let cfg: Config = toml::from_str(&toml).unwrap();
        assert_eq!(cfg.short_break_minutes, 10);
        assert_eq!(cfg.reset_time, "00:00");
    }

    #[test]
    fn test_config_path_ends_correctly() {
        let path = config_path();
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(filename, "config.toml");
    }

    #[test]
    fn test_validate_valid() {
        let cfg = Config {
            short_break_minutes: 10,
            reset_time: "04:00".to_string(),
        };
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_validate_zero_minutes() {
        let cfg = Config {
            short_break_minutes: 0,
            reset_time: "00:00".to_string(),
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_hour() {
        let cfg = Config {
            short_break_minutes: 10,
            reset_time: "25:00".to_string(),
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_minute() {
        let cfg = Config {
            short_break_minutes: 10,
            reset_time: "12:60".to_string(),
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_validate_bad_format() {
        let cfg = Config {
            short_break_minutes: 10,
            reset_time: "not-a-time".to_string(),
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_load_creates_defaults() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Override config path by setting XDG_CONFIG_HOME
        // SAFETY: test only, single-threaded
        unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp.path()) };
        let path = config_path();
        assert!(!path.exists());
        let cfg = Config::load().unwrap();
        assert_eq!(cfg.short_break_minutes, 10);
        assert!(path.exists());
    }
}
