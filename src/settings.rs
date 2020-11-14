use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::Path;

use config;
use dirs;
use serde::Deserialize;

use crate::error::{Result, SpiritError};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub devices: Option<HashSet<String>>,
    pub default: Option<String>,
    pub success: Option<String>,
    pub fail: Option<String>,
}

impl Settings {
    pub fn new() -> Result<Option<Self>> {
        let mut settings = config::Config::new();
        let mut loaded = false;

        if let Some(home) = dirs::home_dir() {
            let global_config = home.join(Path::new(OsStr::new("spirit.toml")));
            if global_config.exists() {
                if let Some(path) = global_config.to_str() {
                    settings.merge(config::File::with_name(path))?;
                    loaded = true;
                } else {
                    return Err(SpiritError::Error("Could not make global config file path".to_string()));
                }
            }
        }

        if Path::new(OsStr::new("spirit.toml")).exists() {
            settings.merge(config::File::with_name("spirit"))?;
            loaded = true;
        }

        if loaded {
            Ok(Some(settings.try_into()?))
        } else {
            Ok(None)
        }
    }
}
