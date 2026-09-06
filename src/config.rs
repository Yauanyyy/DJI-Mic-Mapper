use std::{fs, path::Path};

use serde::Deserialize;

use crate::{keymap::Chord, logger::Level};

pub const DJI_VENDOR_ID: u32 = 0x2CA3;
pub const DJI_PRODUCT_ID: u32 = 0x4011;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VolumeUpMode {
    Off,
    BestEffort,
    BlockAll,
}

impl VolumeUpMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::BestEffort => "best_effort",
            Self::BlockAll => "block_all",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub target: String,
    pub volume_up_mode: Option<VolumeUpMode>,
    // Backward compatibility with the first MVP config. New files should use
    // volume_up_mode instead.
    pub suppress_volume_up: Option<bool>,
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
            volume_up_mode: None,
            suppress_volume_up: None,
            log_level: "info".to_owned(),
            correlation_window_ms: 10,
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
        if self.volume_up_mode.is_some() && self.suppress_volume_up.is_some() {
            return Err(
                "use either volume_up_mode or legacy suppress_volume_up, not both".to_owned(),
            );
        }
        if !(5..=100).contains(&self.correlation_window_ms) {
            return Err("correlation_window_ms must be between 5 and 100".to_owned());
        }
        if self.usage_page == 0 || self.usage == 0 || self.button_usage == 0 {
            return Err("usage_page, usage, and button_usage must be non-zero".to_owned());
        }
        Ok(())
    }

    pub fn effective_volume_up_mode(&self) -> VolumeUpMode {
        self.volume_up_mode.unwrap_or_else(|| {
            if self.suppress_volume_up.unwrap_or(true) {
                VolumeUpMode::BestEffort
            } else {
                VolumeUpMode::Off
            }
        })
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
        assert_eq!(config.effective_volume_up_mode(), VolumeUpMode::BestEffort);
    }

    #[test]
    fn validates_correlation_window_range() {
        for correlation_window_ms in [5, 100] {
            let config = Config {
                correlation_window_ms,
                ..Config::default()
            };
            assert!(config.validate().is_ok());
        }

        for correlation_window_ms in [4, 101] {
            let config = Config {
                correlation_window_ms,
                ..Config::default()
            };
            assert!(config.validate().is_err());
        }
    }

    #[test]
    fn legacy_suppression_setting_remains_supported() {
        let config = Config {
            suppress_volume_up: Some(false),
            ..Config::default()
        };
        assert_eq!(config.effective_volume_up_mode(), VolumeUpMode::Off);
    }

    #[test]
    fn rejects_two_volume_mode_settings() {
        let config = Config {
            volume_up_mode: Some(VolumeUpMode::BlockAll),
            suppress_volume_up: Some(true),
            ..Config::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn parses_block_all_mode() {
        let config: Config = toml::from_str("volume_up_mode = 'block_all'").unwrap();
        assert_eq!(config.effective_volume_up_mode(), VolumeUpMode::BlockAll);
    }
}
