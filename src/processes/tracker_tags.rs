use std::collections::HashSet;

use anyhow::{Result, anyhow};
use indexmap::IndexMap;

use crate::config::{ControllerConfig, TrackerRule};
use crate::processes::common::{parse_tags, torrent_hash};
use crate::qbit_api::{QbitClient, Torrent, Tracker};

pub async fn process_tracker_tags(
    config: &ControllerConfig,
    qbit: &QbitClient,
    torrents: &mut [Torrent],
) -> Result<()> {
    let tracker_config = match &config.trackers {
        Some(trackers) => trackers,
        None => {
            return Err(anyhow!(
                "No trackers config found, skipping tracker_tags process"
            ));
        }
    };

    let owned_tags = owned_tracker_tags(config, tracker_config);

    for torrent in torrents {
        let torrent_name = match &torrent.name {
            Some(name) => name,
            None => continue,
        };
        let hash = torrent_hash(torrent)?;
        let trackers = qbit.get_trackers(&hash).await?;
        let desired_tags = desired_tracker_tags(config, tracker_config, &trackers);
        let current_tags = parse_tags(&torrent.tags);

        let tags_to_remove: Vec<String> = current_tags
            .intersection(&owned_tags)
            .filter(|tag| !desired_tags.contains(*tag))
            .cloned()
            .collect();
        let tags_to_add: Vec<String> = desired_tags.difference(&current_tags).cloned().collect();

        if !tags_to_remove.is_empty() {
            log::info!("Removing tracker tags from '{torrent_name}': {tags_to_remove:?}");
            if !config.settings.dry_run {
                qbit.remove_tags(std::slice::from_ref(&hash), &tags_to_remove)
                    .await?;
            }
        }

        if !tags_to_add.is_empty() {
            log::info!("Adding tracker tags to '{torrent_name}': {tags_to_add:?}");
            if !config.settings.dry_run {
                qbit.add_tags(std::slice::from_ref(&hash), &tags_to_add)
                    .await?;
            }
        }

        let mut new_tags = current_tags;
        for tag in tags_to_remove {
            new_tags.remove(&tag);
        }
        new_tags.extend(tags_to_add);
        torrent.tags = Some(sorted_tags_string(&new_tags));
    }

    Ok(())
}

fn desired_tracker_tags(
    config: &ControllerConfig,
    tracker_config: &IndexMap<String, TrackerRule>,
    trackers: &[Tracker],
) -> HashSet<String> {
    let urls: Vec<&str> = trackers
        .iter()
        .map(|tracker| tracker.url.as_str())
        .filter(|url| !url.is_empty())
        .collect();
    let has_working_tracker = trackers.iter().any(|tracker| tracker.status == 2);
    let mut desired = HashSet::new();
    let mut matched_tracker_rule = false;

    for (rule_key, rule) in tracker_config {
        if rule_key == "other" {
            continue;
        }

        let matched = rule_key
            .split('|')
            .map(str::trim)
            .filter(|pattern| !pattern.is_empty())
            .any(|pattern| urls.iter().any(|url| url.contains(pattern)));

        if matched {
            matched_tracker_rule = true;
            desired.extend(rule.tags.iter().cloned());
        }
    }

    if !matched_tracker_rule
        && !urls.is_empty()
        && let Some(other_rule) = tracker_config.get("other")
    {
        desired.extend(other_rule.tags.iter().cloned());
    }

    if config.processes.tracker_errors && !has_working_tracker {
        desired.insert(config.settings.tracker_error_tag.clone());
    }

    desired
}

fn owned_tracker_tags(
    config: &ControllerConfig,
    tracker_config: &IndexMap<String, TrackerRule>,
) -> HashSet<String> {
    let mut owned: HashSet<String> = tracker_config
        .values()
        .flat_map(|rule| rule.tags.iter().cloned())
        .collect();

    if config.processes.tracker_errors {
        owned.insert(config.settings.tracker_error_tag.clone());
    }

    owned
}

fn sorted_tags_string(tags: &HashSet<String>) -> String {
    let mut tags: Vec<&String> = tags.iter().collect();
    tags.sort();
    tags.into_iter().cloned().collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Processes, Qbit, Settings};

    fn config() -> ControllerConfig {
        ControllerConfig {
            qbit: Qbit {
                url: "http://localhost:8080".to_owned(),
                username: "user".to_owned(),
                password: "pass".to_owned(),
            },
            settings: Settings::default(),
            processes: Processes {
                tracker_errors: true,
                ..Processes::default()
            },
            names: None,
            cat_moves: None,
            trackers: None,
            share_limits: None,
        }
    }

    #[test]
    fn matches_tracker_url_contains_and_other() {
        let mut rules = IndexMap::new();
        rules.insert(
            "iptorrents|stackoverflow.tech".to_owned(),
            TrackerRule {
                tags: vec!["iptorrents".to_owned()],
            },
        );
        rules.insert(
            "other".to_owned(),
            TrackerRule {
                tags: vec!["other".to_owned()],
            },
        );

        let trackers = vec![Tracker {
            url: "https://routing.bgp.technology/announce".to_owned(),
            status: 2,
            msg: String::new(),
        }];
        assert_eq!(
            desired_tracker_tags(&config(), &rules, &trackers),
            HashSet::from_iter(["other".to_owned()])
        );

        let trackers = vec![Tracker {
            url: "https://iptorrents.example/announce".to_owned(),
            status: 2,
            msg: String::new(),
        }];
        assert_eq!(
            desired_tracker_tags(&config(), &rules, &trackers),
            HashSet::from_iter(["iptorrents".to_owned()])
        );
    }

    #[test]
    fn tags_tracker_errors() {
        let rules = IndexMap::new();
        let trackers = vec![Tracker {
            url: "https://tracker.example/announce".to_owned(),
            status: 4,
            msg: "not working".to_owned(),
        }];

        assert!(desired_tracker_tags(&config(), &rules, &trackers).contains("issue"));
    }
}
