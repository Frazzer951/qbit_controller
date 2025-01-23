use anyhow::Result;
use config::Config;
use fs_err as fs;
use serde::Deserialize;
use std::collections::HashMap;

use crate::constants;

const EXAMPLE_CONFIG: &str = include_str!("../config/example_config.yml");
const CONFIG_SCHEMA: &str = include_str!("../config/config_schema.json");

#[derive(Debug, Deserialize)]
pub struct ControllerConfig {
    pub qbit: Qbit,
    pub settings: Settings,
    pub processes: Processes,

    pub names: Option<HashMap<String, Name>>,
    pub cat_moves: Option<HashMap<String, CatMove>>,
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
    pub cat_move: bool,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub dry_run: bool,
    pub enable_auto_management: bool,
    pub quiet: bool,
}

#[derive(Debug, Deserialize)]
pub struct Name {
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CatMove {
    pub categories: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub new_category: String,
}

fn write_if_different(path: &str, contents: &str) -> Result<()> {
    if let Ok(existing) = fs::read_to_string(path) {
        if existing == contents {
            return Ok(());
        }
    }
    log::info!("Writing file at {}", path);
    Ok(fs::write(path, contents)?)
}

pub fn load_config(config_path: &str) -> Result<ControllerConfig> {
    let example_config_path = constants::CONFIG_DIR.to_owned() + constants::CONFIG_EXAMPLE_FILE;
    let config_schema_path = constants::CONFIG_DIR.to_owned() + constants::CONFIG_SCHEMA_FILE;
    write_if_different(&example_config_path,EXAMPLE_CONFIG)?;
    write_if_different(&config_schema_path,CONFIG_SCHEMA)?;

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
