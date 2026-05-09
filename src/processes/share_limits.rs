use std::collections::HashSet;

use anyhow::{Result, anyhow};

use crate::config::{ControllerConfig, ShareLimit};
use crate::processes::common::{
    dry_run_prefix, is_complete, parse_duration_minutes, parse_tags, torrent_hash,
};
use crate::processes::stats::RunStats;
use crate::qbit_api::{MinuteLimit, QbitClient, RatioLimit, Torrent};

#[derive(Debug, Clone)]
struct ShareLimitGroup<'a> {
    name: &'a str,
    config: &'a ShareLimit,
}

pub async fn process_share_limits(
    config: &ControllerConfig,
    qbit: &QbitClient,
    torrents: &mut [Torrent],
    stats: &mut RunStats,
) -> Result<()> {
    let share_limits = match &config.share_limits {
        Some(share_limits) => share_limits,
        None => {
            return Err(anyhow!(
                "No share_limits config found, skipping share_limits process"
            ));
        }
    };

    let mut groups: Vec<ShareLimitGroup<'_>> = share_limits
        .iter()
        .map(|(name, limit)| ShareLimitGroup {
            name: name.as_str(),
            config: limit,
        })
        .collect();
    groups.sort_by_key(|group| group.config.priority);

    for torrent in torrents {
        if config.settings.share_limits_filter_completed && !is_complete(torrent) {
            continue;
        }

        let torrent_name = match &torrent.name {
            Some(name) => name,
            None => continue,
        };
        let hash = torrent_hash(torrent)?;
        let current_tags = parse_tags(&torrent.tags);
        let group = match groups
            .iter()
            .find(|group| matches_share_limit_group(group.config, torrent, &current_tags))
        {
            Some(group) => group,
            None => continue,
        };

        let desired_group_tag = share_limit_group_tag(config, group.name, group.config.priority);
        let mut desired_managed_tags = HashSet::from([desired_group_tag.clone()]);
        let protection = protection_reason(config, group.config, torrent)?;
        if let Some(protection_tag) = protection.protection_tag() {
            desired_managed_tags.insert(protection_tag);
        }

        sync_managed_tags(
            config,
            qbit,
            &hash,
            torrent_name,
            &current_tags,
            &desired_managed_tags,
            stats,
        )
        .await?;

        if should_cleanup(group.config, torrent, protection.is_protected())? {
            log::info!(
                "{}{:<10} '{torrent_name}' group={} (limit reached)",
                dry_run_prefix(config),
                "delete",
                group.name,
            );
            stats.share_limits_cleaned_up += 1;
            if !config.settings.dry_run {
                qbit.delete_torrents(std::slice::from_ref(&hash), true)
                    .await?;
            }
            continue;
        }

        let (ratio_limit, seeding_time_limit) = if protection.is_protected() {
            (RatioLimit::NoLimit, MinuteLimit::NoLimit)
        } else {
            (
                ratio_limit(group.config.max_ratio),
                seeding_time_limit(&group.config.max_seeding_time)?,
            )
        };

        let share_limits_match = ratio_limit_matches(ratio_limit, torrent.ratio_limit)
            && seeding_time_limit_matches(seeding_time_limit, torrent.seeding_time_limit);
        let desired_upload_bytes = group.config.limit_upload_speed.map(upload_limit_bytes);
        let upload_limit_match =
            desired_upload_bytes.is_none_or(|bytes| upload_limit_matches(bytes, torrent.up_limit));

        if !share_limits_match || !upload_limit_match {
            log::info!(
                "{}{:<10} '{torrent_name}' group={}",
                dry_run_prefix(config),
                "apply",
                group.name,
            );
            stats.share_limits_applied += 1;
            if !config.settings.dry_run {
                if !share_limits_match {
                    qbit.set_share_limits(
                        std::slice::from_ref(&hash),
                        ratio_limit,
                        seeding_time_limit,
                        MinuteLimit::Global,
                    )
                    .await?;
                }

                if let Some(bytes) = desired_upload_bytes
                    && !upload_limit_match
                {
                    qbit.set_upload_limit(std::slice::from_ref(&hash), bytes)
                        .await?;
                }

                if group.config.resume_torrent_after_change {
                    qbit.start_torrents(std::slice::from_ref(&hash)).await?;
                }
            }
        }

        let mut new_tags = current_tags;
        new_tags.retain(|tag| !is_managed_share_limit_tag(config, tag));
        new_tags.extend(desired_managed_tags);
        torrent.tags = Some(sorted_tags_string(&new_tags));
    }

    Ok(())
}

async fn sync_managed_tags(
    config: &ControllerConfig,
    qbit: &QbitClient,
    hash: &String,
    torrent_name: &str,
    current_tags: &HashSet<String>,
    desired_managed_tags: &HashSet<String>,
    stats: &mut RunStats,
) -> Result<()> {
    let tags_to_remove: Vec<String> = current_tags
        .iter()
        .filter(|tag| is_managed_share_limit_tag(config, tag))
        .filter(|tag| !desired_managed_tags.contains(*tag))
        .cloned()
        .collect();
    let tags_to_add: Vec<String> = desired_managed_tags
        .difference(current_tags)
        .cloned()
        .collect();

    if !tags_to_remove.is_empty() {
        log::info!(
            "{}{:<10} '{torrent_name}' tags={tags_to_remove:?}",
            dry_run_prefix(config),
            "tag-remove",
        );
        stats.share_limit_tags_removed += tags_to_remove.len();
        if !config.settings.dry_run {
            qbit.remove_tags(std::slice::from_ref(hash), &tags_to_remove)
                .await?;
        }
    }

    if !tags_to_add.is_empty() {
        log::info!(
            "{}{:<10} '{torrent_name}' tags={tags_to_add:?}",
            dry_run_prefix(config),
            "tag-add",
        );
        stats.share_limit_tags_added += tags_to_add.len();
        if !config.settings.dry_run {
            qbit.add_tags(std::slice::from_ref(hash), &tags_to_add)
                .await?;
        }
    }

    Ok(())
}

fn matches_share_limit_group(
    group: &ShareLimit,
    torrent: &Torrent,
    torrent_tags: &HashSet<String>,
) -> bool {
    if let Some(categories) = &group.categories {
        let category = torrent.category.as_deref().unwrap_or_default();
        if !categories.iter().any(|allowed| allowed == category) {
            return false;
        }
    }

    if let Some(required_tags) = &group.include_all_tags
        && !required_tags.iter().all(|tag| torrent_tags.contains(tag))
    {
        return false;
    }

    if let Some(any_tags) = &group.include_any_tags
        && !any_tags.iter().any(|tag| torrent_tags.contains(tag))
    {
        return false;
    }

    if let Some(excluded_tags) = &group.exclude_all_tags
        && excluded_tags.iter().all(|tag| torrent_tags.contains(tag))
    {
        return false;
    }

    if let Some(excluded_tags) = &group.exclude_any_tags
        && excluded_tags.iter().any(|tag| torrent_tags.contains(tag))
    {
        return false;
    }

    true
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Protection {
    None,
    MinSeedingTime(String),
    MinSeeds(String),
}

impl Protection {
    fn is_protected(&self) -> bool {
        !matches!(self, Self::None)
    }

    fn protection_tag(&self) -> Option<String> {
        match self {
            Self::None => None,
            Self::MinSeedingTime(tag) | Self::MinSeeds(tag) => Some(tag.clone()),
        }
    }
}

fn protection_reason(
    config: &ControllerConfig,
    group: &ShareLimit,
    torrent: &Torrent,
) -> Result<Protection> {
    if let Some(min_seeding_time) = &group.min_seeding_time {
        let min_seconds = parse_duration_minutes(min_seeding_time)? * 60;
        if min_seconds > 0 && torrent.seeding_time.unwrap_or_default() < min_seconds {
            return Ok(Protection::MinSeedingTime(
                config.settings.share_limits_min_seeding_time_tag.clone(),
            ));
        }
    }

    if let Some(min_num_seeds) = group.min_num_seeds
        && min_num_seeds > 0
        && torrent.num_complete.unwrap_or_default() < min_num_seeds
    {
        return Ok(Protection::MinSeeds(
            config.settings.share_limits_min_num_seeds_tag.clone(),
        ));
    }

    Ok(Protection::None)
}

fn should_cleanup(group: &ShareLimit, torrent: &Torrent, is_protected: bool) -> Result<bool> {
    if !group.cleanup || is_protected {
        return Ok(false);
    }

    let max_ratio_reached = group
        .max_ratio
        .filter(|max_ratio| *max_ratio >= 0.0)
        .is_some_and(|max_ratio| torrent.ratio.unwrap_or_default() >= max_ratio);
    let max_time_reached = match &group.max_seeding_time {
        Some(max_seeding_time) => {
            let minutes = parse_duration_minutes(max_seeding_time)?;
            minutes >= 0 && torrent.seeding_time.unwrap_or_default() >= minutes * 60
        }
        None => false,
    };

    Ok(max_ratio_reached || max_time_reached)
}

fn ratio_limit(value: Option<f64>) -> RatioLimit {
    match value {
        Some(value) if value < 0.0 => RatioLimit::NoLimit,
        Some(value) => RatioLimit::Limited(value),
        None => RatioLimit::Global,
    }
}

fn seeding_time_limit(value: &Option<crate::config::DurationValue>) -> Result<MinuteLimit> {
    match value {
        Some(value) => match parse_duration_minutes(value)? {
            minutes if minutes < 0 => Ok(MinuteLimit::NoLimit),
            minutes => Ok(MinuteLimit::Limited(minutes as u64)),
        },
        None => Ok(MinuteLimit::Global),
    }
}

fn upload_limit_bytes(kib_per_second: i64) -> u64 {
    if kib_per_second < 0 {
        0
    } else {
        kib_per_second as u64 * 1024
    }
}

fn ratio_limit_matches(desired: RatioLimit, current: Option<f64>) -> bool {
    let current = match current {
        Some(value) => value,
        None => return false,
    };
    match desired {
        RatioLimit::Global => current == -2.0,
        RatioLimit::NoLimit => current == -1.0,
        RatioLimit::Limited(value) => (current - value).abs() < 0.001,
    }
}

fn seeding_time_limit_matches(desired: MinuteLimit, current: Option<i64>) -> bool {
    let current = match current {
        Some(value) => value,
        None => return false,
    };
    match desired {
        MinuteLimit::Global => current == -2,
        MinuteLimit::NoLimit => current == -1,
        MinuteLimit::Limited(value) => current >= 0 && current as u64 == value,
    }
}

fn upload_limit_matches(desired_bytes: u64, current: Option<i64>) -> bool {
    let current = match current {
        Some(value) => value,
        None => return false,
    };
    if desired_bytes == 0 {
        current <= 0
    } else {
        current >= 0 && current as u64 == desired_bytes
    }
}

fn share_limit_group_tag(config: &ControllerConfig, name: &str, priority: u32) -> String {
    format!("{}_{}.{}", config.settings.share_limits_tag, priority, name)
}

fn is_managed_share_limit_tag(config: &ControllerConfig, tag: &str) -> bool {
    tag.starts_with(&format!("{}_", config.settings.share_limits_tag))
        || tag == config.settings.share_limits_min_seeding_time_tag
        || tag == config.settings.share_limits_min_num_seeds_tag
}

fn sorted_tags_string(tags: &HashSet<String>) -> String {
    let mut tags: Vec<&String> = tags.iter().collect();
    tags.sort();
    tags.into_iter().cloned().collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DurationValue, Processes, Qbit, Settings};

    fn config() -> ControllerConfig {
        ControllerConfig {
            qbit: Qbit {
                url: "http://localhost:8080".to_owned(),
                username: "user".to_owned(),
                password: "pass".to_owned(),
            },
            settings: Settings::default(),
            processes: Processes::default(),
            names: None,
            cat_moves: None,
            trackers: None,
            share_limits: None,
        }
    }

    fn torrent(tags: &str, category: &str) -> Torrent {
        Torrent {
            added_on: None,
            auto_tmm: None,
            category: Some(category.to_owned()),
            hash: Some("hash".to_owned()),
            name: Some("torrent".to_owned()),
            num_complete: Some(20),
            progress: Some(1.0),
            ratio: Some(2.0),
            ratio_limit: None,
            seeding_time: Some(2 * 24 * 60 * 60),
            seeding_time_limit: None,
            state: Some("uploading".to_owned()),
            tags: Some(tags.to_owned()),
            tracker: None,
            up_limit: None,
        }
    }

    #[test]
    fn matches_tags_and_categories() {
        let group = ShareLimit {
            priority: 1,
            cleanup: false,
            max_ratio: None,
            max_seeding_time: None,
            min_seeding_time: None,
            min_num_seeds: None,
            limit_upload_speed: None,
            resume_torrent_after_change: true,
            categories: Some(vec!["movies-complete".to_owned()]),
            include_all_tags: Some(vec!["iptorrents".to_owned()]),
            include_any_tags: None,
            exclude_all_tags: None,
            exclude_any_tags: None,
        };

        assert!(matches_share_limit_group(
            &group,
            &torrent("iptorrents, other", "movies-complete"),
            &parse_tags(&Some("iptorrents, other".to_owned()))
        ));
        assert!(!matches_share_limit_group(
            &group,
            &torrent("DigitalCore", "movies-complete"),
            &parse_tags(&Some("DigitalCore".to_owned()))
        ));
    }

    #[test]
    fn cleanup_waits_for_min_seeds() {
        let group = ShareLimit {
            priority: 1,
            cleanup: true,
            max_ratio: Some(1.0),
            max_seeding_time: None,
            min_seeding_time: None,
            min_num_seeds: Some(10),
            limit_upload_speed: None,
            resume_torrent_after_change: true,
            categories: None,
            include_all_tags: None,
            include_any_tags: None,
            exclude_all_tags: None,
            exclude_any_tags: None,
        };
        let mut torrent = torrent("tag", "movies-complete");
        torrent.num_complete = Some(5);

        let protection = protection_reason(&config(), &group, &torrent).unwrap();
        assert!(protection.is_protected());
        assert!(!should_cleanup(&group, &torrent, protection.is_protected()).unwrap());
    }

    #[test]
    fn cleanup_when_ratio_or_seed_time_reached() {
        let mut group = ShareLimit {
            priority: 1,
            cleanup: true,
            max_ratio: Some(5.0),
            max_seeding_time: Some(DurationValue::Text("1d".to_owned())),
            min_seeding_time: None,
            min_num_seeds: None,
            limit_upload_speed: None,
            resume_torrent_after_change: true,
            categories: None,
            include_all_tags: None,
            include_any_tags: None,
            exclude_all_tags: None,
            exclude_any_tags: None,
        };
        assert!(should_cleanup(&group, &torrent("tag", "movies-complete"), false).unwrap());

        group.max_seeding_time = None;
        assert!(!should_cleanup(&group, &torrent("tag", "movies-complete"), false).unwrap());
    }

    #[test]
    fn formats_group_tag() {
        assert_eq!(
            share_limit_group_tag(&config(), "CROSS_SEED", 1),
            "z_1.CROSS_SEED"
        );
    }

    #[test]
    fn ratio_limit_match_detects_each_variant() {
        assert!(ratio_limit_matches(RatioLimit::Global, Some(-2.0)));
        assert!(ratio_limit_matches(RatioLimit::NoLimit, Some(-1.0)));
        assert!(ratio_limit_matches(RatioLimit::Limited(2.5), Some(2.5)));
        assert!(ratio_limit_matches(RatioLimit::Limited(2.5), Some(2.5001)));
        assert!(!ratio_limit_matches(RatioLimit::Limited(2.5), Some(3.0)));
        assert!(!ratio_limit_matches(RatioLimit::Global, Some(-1.0)));
        assert!(!ratio_limit_matches(RatioLimit::Limited(2.5), None));
    }

    #[test]
    fn seeding_time_limit_match_detects_each_variant() {
        assert!(seeding_time_limit_matches(MinuteLimit::Global, Some(-2)));
        assert!(seeding_time_limit_matches(MinuteLimit::NoLimit, Some(-1)));
        assert!(seeding_time_limit_matches(
            MinuteLimit::Limited(120),
            Some(120)
        ));
        assert!(!seeding_time_limit_matches(
            MinuteLimit::Limited(120),
            Some(60)
        ));
        assert!(!seeding_time_limit_matches(MinuteLimit::Global, Some(-1)));
        assert!(!seeding_time_limit_matches(MinuteLimit::Limited(120), None));
    }

    #[test]
    fn upload_limit_match_treats_zero_and_negative_as_unlimited() {
        assert!(upload_limit_matches(0, Some(0)));
        assert!(upload_limit_matches(0, Some(-1)));
        assert!(upload_limit_matches(1024, Some(1024)));
        assert!(!upload_limit_matches(1024, Some(2048)));
        assert!(!upload_limit_matches(1024, Some(-1)));
        assert!(!upload_limit_matches(1024, None));
    }
}
