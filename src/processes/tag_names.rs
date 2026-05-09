use std::collections::HashSet;

use anyhow::{Result, anyhow};

use crate::config::ControllerConfig;
use crate::processes::common::{dry_run_prefix, parse_tags, torrent_hash};
use crate::processes::stats::RunStats;
use crate::qbit_api::{QbitClient, Torrent};

pub async fn process_tag_names(
    config: &ControllerConfig,
    qbit: &QbitClient,
    torrents: &mut [Torrent],
    stats: &mut RunStats,
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
                new_tags.extend(name_config.tags.clone());
            }
        }

        if new_tags != torrent_tags {
            let tags: HashSet<String> = new_tags.difference(&torrent_tags).cloned().collect();
            let tags_sorted: Vec<&String> = {
                let mut v: Vec<&String> = tags.iter().collect();
                v.sort();
                v
            };
            log::info!(
                "{}{:<10} '{torrent_name}' tags={tags_sorted:?}",
                dry_run_prefix(config),
                "tag-add",
            );
            stats.name_tags_added += tags.len();

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
