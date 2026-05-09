pub mod config;
pub mod constants;
pub mod processes;
pub mod qbit_api;

use anyhow::Result;
use config::{ControllerConfig, load_config};
use processes::stats::{RunStats, log_section};
use processes::{
    cat_moves,
    common::{dry_run_prefix, parse_tags},
    share_limits, tag_names, tracker_tags,
};
use qbit_api::{QbitClient, Torrent};

async fn process_torrents(
    config: &ControllerConfig,
    qbit: &QbitClient,
    torrents: &mut Vec<Torrent>,
    stats: &mut RunStats,
) -> Result<()> {
    if config.settings.enable_auto_management {
        let ignore_tags = &config.settings.auto_management_ignore_tags;
        let hashes: Vec<String> = torrents
            .iter()
            .filter(|torrent| torrent.auto_tmm != Some(true))
            .filter(|torrent| {
                let tags = parse_tags(&torrent.tags);
                !ignore_tags.iter().any(|tag| tags.contains(tag))
            })
            .filter_map(|t| t.hash.clone())
            .collect();
        if !hashes.is_empty() {
            if !config.settings.quiet {
                log_section("Auto management");
                log::info!(
                    "{}{:<10} {} torrent(s)",
                    dry_run_prefix(config),
                    "auto-tmm",
                    hashes.len()
                );
            }
            stats.auto_management_enabled = hashes.len();
            if !config.settings.dry_run {
                qbit.set_auto_management(&hashes, true).await?;
            }
        }
    }

    if config.processes.tracker_tags || config.processes.tracker_errors {
        if !config.settings.quiet {
            log_section("Tracker tags");
        }
        tracker_tags::process_tracker_tags(config, qbit, torrents, stats).await?;
    }

    if config.processes.share_limits {
        if !config.settings.quiet {
            log_section("Share limits");
        }
        share_limits::process_share_limits(config, qbit, torrents, stats).await?;
    }

    if config.processes.tag_names {
        if !config.settings.quiet {
            log_section("Tag names");
        }
        tag_names::process_tag_names(config, qbit, torrents, stats).await?;
    }

    if config.processes.cat_move {
        if !config.settings.quiet {
            log_section("Category moves");
        }
        cat_moves::process_cat_moves(config, qbit, torrents, stats).await?;
    }

    Ok(())
}

pub async fn run() -> Result<()> {
    let config_path = constants::CONFIG_DIR.to_owned() + constants::CONFIG_FILE;

    let config = load_config(config_path.as_str())?;
    log::debug!("{:#?}", config);

    if !config.settings.quiet {
        log::info!("Starting qbit-controller");
    }

    if config.settings.dry_run {
        log::info!("Dry run enabled, no changes will be made");
    }

    let qbit = QbitClient::new(&config.qbit)?;
    qbit.login().await?;

    let qbit_version = qbit.get_version().await?;
    if !config.settings.quiet {
        log::info!("QbitTorrent Version: {qbit_version}");
    }
    let webapi_version = qbit.get_webapi_version().await?;
    if !config.settings.quiet {
        log::info!("qBittorrent Web API Version: {webapi_version}");
    }

    let mut torrents = qbit.get_torrents().await?;

    let mut stats = RunStats {
        torrents_total: torrents.len(),
        ..RunStats::default()
    };

    process_torrents(&config, &qbit, &mut torrents, &mut stats).await?;

    if !config.settings.quiet {
        stats.log_summary(config.settings.dry_run);
    }

    Ok(())
}
