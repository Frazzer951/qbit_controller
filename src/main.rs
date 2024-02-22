mod config;
mod constants;
mod processes;

use anyhow::Result;
use config::{load_config, ControllerConfig};
use processes::tag_names;
use qbit_rs::model::{Credential, GetTorrentListArg, Torrent};
use qbit_rs::Qbit;

async fn process_torrents(
    config: ControllerConfig,
    qbit: Qbit,
    torrents: Vec<Torrent>,
) -> Result<()> {
    if config.settings.enable_auto_management {
        if !config.settings.quiet {
            log::info!("Enabling auto management for all torrents");
        }
        if !config.settings.dry_run {
            let hashes: Vec<String> = torrents.iter().map(|t| t.hash.clone().unwrap()).collect();
            qbit.set_auto_management(hashes, true).await?;
        }
    }

    if config.processes.tag_names {
        tag_names::process_tag_names(&config, &qbit, &torrents).await?;
    }

    Ok(())
}

async fn run() -> Result<()> {
    let config_path = constants::CONFIG_DIR.to_owned() + constants::CONFIG_FILE;

    let config = load_config(config_path.as_str())?;
    log::debug!("{:#?}", config);

    if !config.settings.quiet {
        log::info!("Starting qbit-controller");
    }

    if config.settings.dry_run {
        log::info!("Dry run enabled, no changes will be made");
    }

    let credential = Credential::new(&config.qbit.username, &config.qbit.password);
    let qbit = Qbit::new(config.qbit.url.as_str(), credential);

    let qbit_version = qbit.get_version().await?;
    if !config.settings.quiet {
        log::info!("QbitTorrent Version: {qbit_version}");
    }

    let torrents = qbit.get_torrent_list(GetTorrentListArg::default()).await?;

    process_torrents(config, qbit, torrents).await?;

    Ok(())
}

pub fn setup_logger() -> Result<()> {
    log4rs::init_file("log_config.yml", Default::default())
}

#[tokio::main]
async fn main() {
    setup_logger().unwrap();

    if let Err(e) = run().await {
        log::error!("{e}");
        std::process::exit(1);
    }
}
