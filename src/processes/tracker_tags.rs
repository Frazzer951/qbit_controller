use std::collections::HashSet;

use anyhow::{Result, anyhow};
use indexmap::IndexMap;

use crate::config::{ControllerConfig, TrackerRule};
use crate::processes::common::{dry_run_prefix, parse_tags, torrent_hash};
use crate::processes::stats::RunStats;
use crate::qbit_api::{QbitClient, Torrent, Tracker};

pub async fn process_tracker_tags(
    config: &ControllerConfig,
    qbit: &QbitClient,
    torrents: &mut [Torrent],
    stats: &mut RunStats,
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
            log::info!(
                "{}{:<10} '{torrent_name}' tags={tags_to_remove:?}",
                dry_run_prefix(config),
                "tag-remove",
            );
            stats.tracker_tags_removed += tags_to_remove.len();
            if !config.settings.dry_run {
                qbit.remove_tags(std::slice::from_ref(&hash), &tags_to_remove)
                    .await?;
            }
        }

        if !tags_to_add.is_empty() {
            log::info!(
                "{}{:<10} '{torrent_name}' tags={tags_to_add:?}",
                dry_run_prefix(config),
                "tag-add",
            );
            stats.tracker_tags_added += tags_to_add.len();
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
    let real_trackers: Vec<&Tracker> = trackers
        .iter()
        .filter(|tracker| is_real_tracker_url(&tracker.url))
        .collect();
    let has_working_tracker = real_trackers.iter().any(|tracker| tracker.status == 2);
    let has_failing_tracker = real_trackers.iter().any(|tracker| tracker.status == 4);
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

    if config.processes.tracker_errors && has_failing_tracker && !has_working_tracker {
        desired.insert(config.settings.tracker_error_tag.clone());
    }

    desired
}

fn is_real_tracker_url(url: &str) -> bool {
    matches!(
        url.split(':').next().unwrap_or(""),
        "http" | "https" | "udp" | "ws" | "wss"
    )
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

    #[test]
    fn does_not_tag_when_trackers_are_not_yet_contacted_or_updating() {
        let rules = IndexMap::new();
        for status in [0, 1, 3] {
            let trackers = vec![Tracker {
                url: "https://tracker.example/announce".to_owned(),
                status,
                msg: String::new(),
            }];
            assert!(
                !desired_tracker_tags(&config(), &rules, &trackers).contains("issue"),
                "status {status} should not tag as issue"
            );
        }
    }

    #[test]
    fn does_not_tag_when_any_real_tracker_is_working() {
        let rules = IndexMap::new();
        let trackers = vec![
            Tracker {
                url: "https://working.example/announce".to_owned(),
                status: 2,
                msg: String::new(),
            },
            Tracker {
                url: "https://broken.example/announce".to_owned(),
                status: 4,
                msg: "down".to_owned(),
            },
        ];
        assert!(!desired_tracker_tags(&config(), &rules, &trackers).contains("issue"));
    }

    #[test]
    fn ignores_pseudo_trackers_for_error_state() {
        let rules = IndexMap::new();
        let trackers = vec![Tracker {
            url: "** [DHT] **".to_owned(),
            status: 4,
            msg: String::new(),
        }];
        assert!(!desired_tracker_tags(&config(), &rules, &trackers).contains("issue"));
    }
}
