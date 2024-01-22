use anyhow::{anyhow, Result};
use qbit_rs::{model::Torrent, Qbit};

use crate::config::ControllerConfig;

pub async fn process_tag_names(
    config: ControllerConfig,
    qbit: Qbit,
    torrents: Vec<Torrent>,
) -> Result<()> {
    let names_config = match config.names {
        Some(names) => names,
        None => return Err(anyhow!("No names config found, skipping tag_names process")),
    };

    for (name, name_config) in names_config {
        let tags = name_config.tags;
        log::debug!("Processing name: {}, {:?}", name, tags);
    }

    Ok(())
}
