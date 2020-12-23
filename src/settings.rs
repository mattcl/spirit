use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;

use colorsys::Rgb;
use config;
use dirs;
use govee_rs::schema::{Color};
use serde::Deserialize;

use crate::error::{Result, SpiritError};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub default: Option<String>,
    pub devices: Option<Vec<DeviceSetting>>,
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
                    return Err(SpiritError::Error(
                        "Could not make global config file path".to_string(),
                    ));
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

    pub fn device_settings(&self) -> DeviceSettingMap {
        let mut map = HashMap::new();
        if let Some(ref devices) = self.devices {
            for setting in devices {
                map.insert(setting.name.clone(), setting.clone());
            }
        }

        DeviceSettingMap(map)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeviceSetting {
    pub name: String,
    pub color: Option<String>,
    pub success: Option<String>,
    pub fail: Option<String>,
}

impl DeviceSetting {
    pub fn color(&self) -> Result<Option<Color>> {
        if let Some(ref color_str) = self.color {
            let parsed = Rgb::from_hex_str(&color_str)?;
            Ok(Some(
                Color {
                    r: parsed.get_red() as u32,
                    g: parsed.get_green() as u32,
                    b: parsed.get_blue() as u32,
                }
            ))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Default)]
pub struct DeviceSettingMap(pub HashMap<String, DeviceSetting>);

impl DeviceSettingMap {
    pub fn get(&self, name: &str) -> Option<&DeviceSetting> {
        self.0.get(name)
    }

    pub fn default_color(&self, name: &str, force: Option<&str>, default: Option<&str>) -> Result<Option<Color>> {
        let device_color = self.get(name).and_then(|s| s.color.clone());
        self.pick_color(force, device_color, default)
    }

    pub fn success_color(&self, name: &str, default: Option<&str>) -> Result<Option<Color>> {
        let device_color = self.get(name).and_then(|s| s.success.clone());
        self.pick_color(None, device_color, default)
    }

    pub fn fail_color(&self, name: &str, default: Option<&str>) -> Result<Option<Color>> {
        let device_color = self.get(name).and_then(|s| s.fail.clone());
        self.pick_color(None, device_color, default)
    }

    fn pick_color(&self, force: Option<&str>, device: Option<String>, default: Option<&str>) -> Result<Option<Color>> {
        let color = if let Some(color_str) = force {
            Some(color_str.to_string())
        } else if let Some(device_color) = device {
            Some(device_color)
        } else if let Some(default) = default {
            Some(default.to_string())
        } else {
            None
        };

        if let Some(color_str) = color {
            let parsed = Rgb::from_hex_str(&color_str)?;
            Ok(Some(Color {
                r: parsed.get_red() as u32,
                g: parsed.get_green() as u32,
                b: parsed.get_blue() as u32,
            }))
        } else {
            Ok(None)
        }
    }
}



