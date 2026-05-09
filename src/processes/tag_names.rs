use std::collections::HashSet;

use anyhow::{Result, anyhow};

use crate::config::ControllerConfig;
use crate::processes::common::{parse_tags, torrent_hash};
use crate::qbit_api::{QbitClient, Torrent};

pub async fn process_tag_names(
    config: &ControllerConfig,
    qbit: &QbitClient,
    torrents: &mut [Torrent],
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
        let torrent_tags = parse_tags(&torrent.tags);

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
                qbit.add_tags(
                    &[torrent_hash(torrent)?],
                    &tags.into_iter().collect::<Vec<_>>(),
                )
                .await?;
            }
            torrent.tags = Some(sorted_tags_string(&new_tags));
        }
    }

    Ok(())
}

fn sorted_tags_string(tags: &HashSet<String>) -> String {
    let mut tags: Vec<&String> = tags.iter().collect();
    tags.sort();
    tags.into_iter().cloned().collect::<Vec<_>>().join(", ")
}
