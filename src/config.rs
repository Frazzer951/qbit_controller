use anyhow::Result;
use config::Config;
use fs_err as fs;
use serde::Deserialize;
use std::collections::HashMap;

use indexmap::IndexMap;

use crate::constants;

const EXAMPLE_CONFIG: &str = include_str!("../config/example_config.yml");
const CONFIG_SCHEMA: &str = include_str!("../config/config_schema.json");

#[derive(Debug, Deserialize)]
pub struct ControllerConfig {
    pub qbit: Qbit,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub processes: Processes,

    pub names: Option<HashMap<String, Name>>,
    #[serde(alias = "cat_move")]
    pub cat_moves: Option<IndexMap<String, CatMove>>,
    pub trackers: Option<IndexMap<String, TrackerRule>>,
    pub share_limits: Option<IndexMap<String, ShareLimit>>,
}

#[derive(Debug, Deserialize)]
pub struct Qbit {
    pub url: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct Processes {
    #[serde(default)]
    pub tag_names: bool,
    #[serde(default)]
    pub cat_move: bool,
    #[serde(default)]
    pub tracker_tags: bool,
    #[serde(default)]
    pub tracker_errors: bool,
    #[serde(default)]
    pub share_limits: bool,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub enable_auto_management: bool,
    #[serde(default)]
    pub auto_management_ignore_tags: Vec<String>,
    #[serde(default)]
    pub quiet: bool,
    #[serde(default = "default_share_limits_tag")]
    pub share_limits_tag: String,
    #[serde(default = "default_tracker_error_tag")]
    pub tracker_error_tag: String,
    #[serde(default = "default_min_seeding_time_tag")]
    pub share_limits_min_seeding_time_tag: String,
    #[serde(default = "default_min_num_seeds_tag")]
    pub share_limits_min_num_seeds_tag: String,
    #[serde(default)]
    pub share_limits_filter_completed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dry_run: false,
            enable_auto_management: false,
            auto_management_ignore_tags: Vec::new(),
            quiet: false,
            share_limits_tag: default_share_limits_tag(),
            tracker_error_tag: default_tracker_error_tag(),
            share_limits_min_seeding_time_tag: default_min_seeding_time_tag(),
            share_limits_min_num_seeds_tag: default_min_num_seeds_tag(),
            share_limits_filter_completed: true,
        }
    }
}

fn default_share_limits_tag() -> String {
    "z".to_owned()
}

fn default_tracker_error_tag() -> String {
    "issue".to_owned()
}

fn default_min_seeding_time_tag() -> String {
    "MinSeedTimeNotReached".to_owned()
}

fn default_min_num_seeds_tag() -> String {
    "MinSeedsNotMet".to_owned()
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

#[derive(Debug, Deserialize)]
pub struct TrackerRule {
    #[serde(deserialize_with = "deserialize_string_or_vec")]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ShareLimit {
    pub priority: u32,
    #[serde(default)]
    pub cleanup: bool,
    pub max_ratio: Option<f64>,
    pub max_seeding_time: Option<DurationValue>,
    pub min_seeding_time: Option<DurationValue>,
    pub min_num_seeds: Option<i64>,
    pub limit_upload_speed: Option<i64>,
    #[serde(default = "default_true")]
    pub resume_torrent_after_change: bool,
    pub categories: Option<Vec<String>>,
    pub include_all_tags: Option<Vec<String>>,
    pub include_any_tags: Option<Vec<String>>,
    pub exclude_all_tags: Option<Vec<String>>,
    pub exclude_any_tags: Option<Vec<String>>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DurationValue {
    Minutes(i64),
    Text(String),
}

fn deserialize_string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        String(String),
        Vec(Vec<String>),
    }

    match StringOrVec::deserialize(deserializer)? {
        StringOrVec::String(value) => Ok(vec![value]),
        StringOrVec::Vec(values) => Ok(values),
    }
}

fn write_if_different(path: &str, contents: &str) -> Result<()> {
    if let Ok(existing) = fs::read_to_string(path)
        && existing == contents
    {
        return Ok(());
    }
    log::info!("Writing file at {}", path);
    Ok(fs::write(path, contents)?)
}

pub fn load_config(config_path: &str) -> Result<ControllerConfig> {
    let example_config_path = constants::CONFIG_DIR.to_owned() + constants::CONFIG_EXAMPLE_FILE;
    let config_schema_path = constants::CONFIG_DIR.to_owned() + constants::CONFIG_SCHEMA_FILE;
    write_if_different(&example_config_path, EXAMPLE_CONFIG)?;
    write_if_different(&config_schema_path, CONFIG_SCHEMA)?;

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
