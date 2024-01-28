use anyhow::Result;
use config::Config;
use fs_err as fs;
use serde::Deserialize;
use std::{collections::HashMap, path::Path};

use crate::constants;

const EXAMPLE_CONFIG: &str = include_str!("../config/example_config.yml");

#[derive(Debug, Deserialize)]
pub struct ControllerConfig {
    pub qbit: Qbit,
    pub settings: Settings,
    pub processes: Processes,

    pub names: Option<HashMap<String, Name>>,
}

#[derive(Debug, Deserialize)]
pub struct Qbit {
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct Processes {
    pub tag_names: bool,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub dry_run: bool,
    pub enable_auto_management: bool,
}

#[derive(Debug, Deserialize)]
pub struct Name {
    pub tags: Vec<String>,
}

pub fn load_config(config_path: &str) -> Result<ControllerConfig> {
    let example_config_path = constants::CONFIG_DIR.to_owned() + constants::CONFIG_EXAMPLE_FILE;
    if !Path::new(&example_config_path).exists() {
        log::info!("Creating example config file at {}", example_config_path);
        fs::write(example_config_path, EXAMPLE_CONFIG)?;
    }

    let settings = Config::builder()
        .add_source(config::File::with_name(config_path))
        .build()?;
    Ok(settings.try_deserialize()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants;

    #[test]
    fn test_load_config() {
        let config_path = constants::CONFIG_DIR.to_owned() + constants::CONFIG_EXAMPLE_FILE;

        load_config(config_path.as_str()).unwrap();
    }
}
