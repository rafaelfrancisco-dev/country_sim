#![allow(dead_code)]

use std::{error::Error as StdError, fmt, path::PathBuf};

use serde::Deserialize;
use tracing::error;

mod keys;
mod paths;
mod style;

use keys::KeyBindings;
pub use paths::{PROJECT_NAME, get_config_dir, get_data_dir};
use style::Styles;

const CONFIG: &str = include_str!("../../.config/config.json5");

#[derive(Clone, Debug, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub data_dir: PathBuf,
    #[serde(default)]
    pub config_dir: PathBuf,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default, flatten)]
    pub config: AppConfig,
    #[serde(default)]
    pub keybindings: KeyBindings,
    #[serde(default)]
    pub styles: Styles,
}

#[derive(Debug)]
pub enum ConfigLoadError {
    ParseDefaults(json5::Error),
    Build(config::ConfigError),
    SerializePath(PathBuf),
}

impl fmt::Display for ConfigLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigLoadError::ParseDefaults(err) => {
                write!(f, "failed to parse embedded default config: {err}")
            }
            ConfigLoadError::Build(err) => write!(f, "failed to load config: {err}"),
            ConfigLoadError::SerializePath(path) => {
                write!(f, "failed to serialize config path: {}", path.display())
            }
        }
    }
}

impl StdError for ConfigLoadError {}

impl Config {
    pub fn new() -> Result<Self, ConfigLoadError> {
        let default_config: Config = json5::from_str(CONFIG).map_err(ConfigLoadError::ParseDefaults)?;
        let data_dir = get_data_dir();
        let config_dir = get_config_dir();

        let data_dir_str = data_dir
            .to_str()
            .ok_or_else(|| ConfigLoadError::SerializePath(data_dir.clone()))?;
        let config_dir_str = config_dir
            .to_str()
            .ok_or_else(|| ConfigLoadError::SerializePath(config_dir.clone()))?;

        let mut builder = config::Config::builder()
            .set_default("data_dir", data_dir_str)
            .map_err(ConfigLoadError::Build)?
            .set_default("config_dir", config_dir_str)
            .map_err(ConfigLoadError::Build)?;

        let config_files = [
            ("config.json5", config::FileFormat::Json5),
            ("config.json", config::FileFormat::Json),
            ("config.yaml", config::FileFormat::Yaml),
            ("config.toml", config::FileFormat::Toml),
            ("config.ini", config::FileFormat::Ini),
        ];

        let mut found_config = false;
        for (file, format) in &config_files {
            let source = config::File::from(config_dir.join(file))
                .format(*format)
                .required(false);
            builder = builder.add_source(source);
            if config_dir.join(file).exists() {
                found_config = true;
            }
        }

        if !found_config {
            error!("No configuration file found. Application may not behave as expected");
        }

        let mut cfg: Self = builder
            .build()
            .map_err(ConfigLoadError::Build)?
            .try_deserialize()
            .map_err(ConfigLoadError::Build)?;

        for (mode, default_bindings) in &default_config.keybindings.0 {
            let user_bindings = cfg.keybindings.0.entry(*mode).or_default();
            for (key, cmd) in default_bindings {
                user_bindings.entry(key.clone()).or_insert_with(|| cmd.clone());
            }
        }
        for (mode, default_styles) in &default_config.styles.0 {
            let user_styles = cfg.styles.0.entry(*mode).or_default();
            for (style_key, style) in default_styles {
                user_styles.entry(style_key.clone()).or_insert(*style);
            }
        }

        Ok(cfg)
    }
}

lazy_static::lazy_static! {
    pub static ref LOG_ENV: String = format!("{}_LOG_LEVEL", PROJECT_NAME.clone());
    pub static ref LOG_FILE: String = format!("{}.log", env!("CARGO_PKG_NAME"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{action::Action, app::Mode};
    use crate::config::keys::parse_key_sequence;

    #[test]
    fn test_config() {
        let c = Config::new().expect("load config");
        assert_eq!(
            c.keybindings
                .0
                .get(&Mode::Home)
                .expect("home bindings")
                .get(&parse_key_sequence("<q>").expect("parse q"))
                .expect("q action"),
            &Action::Quit
        );
    }
}
