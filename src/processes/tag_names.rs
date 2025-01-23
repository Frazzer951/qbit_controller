use std::collections::hash_set::HashSet;

use anyhow::{anyhow, Result};
use qbit_rs::{model::Torrent, Qbit};

use crate::config::ControllerConfig;

pub async fn process_tag_names(
    config: &ControllerConfig,
    qbit: &Qbit,
    torrents: &Vec<Torrent>,
) -> Result<()> {
    let names_config = match &config.names {
        Some(names) => names,
        None => return Err(anyhow!("No names config found, skipping tag_names process")),
    };
    log::debug!("names_config: {names_config:?}",);

    for torrent in torrents {
        let torrent_name = match &torrent.name {
            Some(name) => name,
            None => continue,
        };
        let torrent_tags = match &torrent.tags {
            Some(tags) => tags.split(',').map(|s| s.trim().to_owned()).collect(),
            None => HashSet::new(),
        };

        let mut new_tags = torrent_tags.clone();

        for (name, name_config) in names_config.iter() {
            if torrent_name.to_lowercase().contains(&name.to_lowercase()) {
                if !config.settings.quiet {
                    log::info!("Found match for {name} in torrent {torrent_name}",);
                }
                new_tags.extend(name_config.tags.clone());
            }
        }

        if new_tags != torrent_tags {
            // Get only the new tags
            let tags: HashSet<String> = new_tags.difference(&torrent_tags).cloned().collect();
            log::info!("Adding tags for torrent {torrent_name}: {tags:?}");

            if !config.settings.dry_run {
                let tags = vec![tags.into_iter().collect::<Vec<_>>().join(",")];
                qbit.add_torrent_tags(vec![torrent.hash.clone().unwrap()], tags)
                    .await?;
            }
        }
    }

    Ok(())
}
