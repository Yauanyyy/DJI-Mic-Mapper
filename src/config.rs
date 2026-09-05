use std::{fs, path::Path};

use serde::Deserialize;

use crate::{keymap::Chord, logger::Level};

pub const DJI_VENDOR_ID: u32 = 0x2CA3;
pub const DJI_PRODUCT_ID: u32 = 0x4011;

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub target: String,
    pub suppress_volume_up: bool,
    pub log_level: String,
    pub correlation_window_ms: u64,
    pub usage_page: u16,
    pub usage: u16,
    pub button_usage: u16,
    pub report_id: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            target: "F13".to_owned(),
            suppress_volume_up: true,
            log_level: "info".to_owned(),
            correlation_window_ms: 100,
            usage_page: 0x000C,
            usage: 0x0001,
            button_usage: 0x00E9,
            report_id: 6,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<(Self, Chord, Level), String> {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let config: Config = toml::from_str(&text)
            .map_err(|error| format!("invalid {}: {error}", path.display()))?;
        config.validate()?;
        let chord = Chord::parse(&config.target)?;
        let level = Level::parse(&config.log_level)?;
        Ok((config, chord, level))
    }

    fn validate(&self) -> Result<(), String> {
        if !(20..=500).contains(&self.correlation_window_ms) {
            return Err("correlation_window_ms must be between 20 and 500".to_owned());
        }
        if self.usage_page == 0 || self.usage == 0 || self.button_usage == 0 {
            return Err("usage_page, usage, and button_usage must be non-zero".to_owned());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_observed_device() {
        let config = Config::default();
        assert_eq!(config.usage_page, 0x000C);
        assert_eq!(config.usage, 0x0001);
        assert_eq!(config.button_usage, 0x00E9);
        assert_eq!(config.report_id, 6);
    }

    #[test]
    fn rejects_unreasonable_correlation_window() {
        let config = Config {
            correlation_window_ms: 5,
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }
}
