use std::collections::hash_set::HashSet;

use anyhow::{Result, anyhow};
use qbit_rs::{Qbit, model::Torrent};

use crate::config::ControllerConfig;

pub async fn process_cat_moves(
    config: &ControllerConfig,
    qbit: &Qbit,
    torrents: &Vec<Torrent>,
) -> Result<()> {
    let cat_moves_config = match &config.cat_moves {
        Some(names) => names,
        None => {
            return Err(anyhow!(
                "No cat_moves config found, skipping cat_moves process"
            ));
        }
    };
    log::debug!("cat_moves_config: {cat_moves_config:?}",);

    for torrent in torrents {
        let torrent_name = match &torrent.name {
            Some(name) => name,
            None => continue,
        };
        let torrent_tags: HashSet<String> = match &torrent.tags {
            Some(tags) => tags.split(',').map(|s| s.trim().to_owned()).collect(),
            None => HashSet::new(),
        };
        let torrent_category = match &torrent.category {
            Some(cat) => cat.to_owned(),
            None => "".to_owned(),
        };

        let mut new_category = torrent_category.clone();

        for (config_name, cat_move_config) in cat_moves_config.iter() {
            // Both tags and categories must match if both are specified
            let categories_match = match &cat_move_config.categories {
                Some(categories) => categories.contains(&torrent_category),
                None => true, // No categories specified = matches all
            };

            let tags_match = match &cat_move_config.tags {
                Some(required_tags) => {
                    let required_set: HashSet<String> = required_tags.iter().cloned().collect();
                    required_set.is_subset(&torrent_tags)
                }
                None => true, // No tags specified = matches all
            };

            // Ensure at least one condition was specified
            if cat_move_config.categories.is_none() && cat_move_config.tags.is_none() {
                return Err(anyhow!(
                    "Config '{config_name}' must specify at least categories or tags"
                ));
            }

            if categories_match && tags_match {
                new_category = cat_move_config.new_category.clone();
                break;
            }
        }

        // Update category if changed
        if new_category != torrent_category {
            log::info!(
                "Setting category for '{torrent_name}' from '{torrent_category}' to '{new_category}'"
            );
            if !config.settings.dry_run {
                qbit.set_torrent_category(vec![torrent.hash.clone().unwrap()], &new_category)
                    .await?;
            }
        }
    }

    Ok(())
}
