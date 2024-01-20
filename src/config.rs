use config::{Config, ConfigError};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ControllerConfig {
    pub qbit: Qbit,
}

#[derive(Debug, Deserialize)]
pub struct Qbit {
    pub url: String,
    pub username: String,
    pub password: String,
}

pub fn load_config(config_path: &str) -> Result<ControllerConfig, ConfigError> {
    let settings = Config::builder()
        .add_source(config::File::with_name(config_path))
        .build()?;
    settings.try_deserialize()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants;

    #[test]
    fn test_load_config() {
        let config_path = constants::CONFIG_DIR.to_owned() + constants::CONFIG_EXAMPLE_FILE;

        let config = load_config(config_path.as_str()).unwrap();

        assert_eq!(config.qbit.url, "http://{{ip/url}}:{{port}}");
        assert_eq!(config.qbit.username, "username");
        assert_eq!(config.qbit.password, "password");
    }
}
